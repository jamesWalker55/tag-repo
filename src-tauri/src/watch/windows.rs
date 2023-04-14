use std::path::{Path, PathBuf};
use std::time::Duration;

use notify::event::ModifyKind::Name;
use notify::event::{CreateKind, EventAttributes, RemoveKind, RenameMode};
use notify::EventKind::{Create, Modify, Remove};
use notify::{
    Config, Event, EventHandler, ReadDirectoryChangesWatcher, RecursiveMode, Watcher, WatcherKind,
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tokio::time::{timeout_at, Instant};

/// A wrapper for `ReadDirectoryChangesWatcher`.
///
/// The structure of this wrapper is like this:
///
/// ```plain
/// Watcher
///   |
///   v
/// Handler
///   |
///   v
/// Output
/// ```
///
/// The **watcher** is a `Watcher` from the `notify` crate. It spawns events to be processed by the
/// **handler**.
///
/// The **handler** processes incoming events from the watcher. What it does depends on the kind of
/// event it received:
///
/// - Delete events: It defers these events for later processing. This is because both file
///   deletions and file moves are returned as deletions on Windows. This watcher delays any delete
///   events to see if any create events with the same name are created later, then treats the event
///   as a rename if so.
/// - Create events: It either returns a create event or a rename event. See the above point.
/// - Other events: It returns the events as-is.
///
/// ## How to stop watching
///
/// Just drop this struct. It should automatically clean up everything. When this struct drops, it
/// drops the `notify` watcher, which in turn makes the event handler task stop since the task
/// is receiving events from the watcher.
#[derive(Debug)]
pub struct WindowsNormWatcher {
    /// The actual watcher instance.
    watcher: ReadDirectoryChangesWatcher,
}

impl Watcher for WindowsNormWatcher {
    fn new<F: EventHandler>(event_handler: F, config: Config) -> notify::Result<Self>
    where
        Self: Sized,
    {
        // Spawn the watcher
        let (watcher_tx, watcher_rx) = unbounded_channel();

        let watcher =
            ReadDirectoryChangesWatcher::new(move |res| watcher_tx.send(res).unwrap(), config)?;

        // Spawn the event handler
        // Don't need to store the JoinHandle, it should naturally terminate once the watcher drops
        tokio::spawn(async move {
            event_handler_loop(watcher_rx, event_handler).await;
        });

        Ok(Self { watcher })
    }

    fn watch(&mut self, path: &Path, recursive_mode: RecursiveMode) -> notify::Result<()> {
        self.watcher.watch(path, recursive_mode)
    }

    fn unwatch(&mut self, path: &Path) -> notify::Result<()> {
        self.watcher.unwatch(path)
    }

    fn kind() -> WatcherKind
    where
        Self: Sized,
    {
        WatcherKind::ReadDirectoryChangesWatcher
    }
}

async fn event_handler_loop(
    mut watcher_rx: UnboundedReceiver<notify::Result<Event>>,
    mut event_handler: impl EventHandler,
) {
    fn clear_expired_records(
        recent_deleted_paths: &mut Vec<(Instant, PathBuf, EventAttributes)>,
        event_handler: &mut impl EventHandler,
    ) {
        let now = Instant::now();
        let mut i = 0;
        loop {
            if i == recent_deleted_paths.len() {
                break;
            }
            {
                let (expires_at, _, _) = recent_deleted_paths.get(i).unwrap();
                if expires_at <= &now {
                    let (_, path, attrs) = recent_deleted_paths.remove(i);
                    let evt = Event {
                        kind: Remove(RemoveKind::Any),
                        paths: vec![path],
                        attrs,
                    };
                    event_handler.handle_event(Ok(evt));
                } else {
                    // not expired, move on to next record
                    i += 1;
                }
            }
        }
    }
    let mut last_rename_from: Option<PathBuf> = None;
    let mut recent_deleted_paths: Vec<(Instant, PathBuf, EventAttributes)> = vec![];
    let mut res;
    loop {
        // If we have paths in the database, timeout until the next path's instant
        if recent_deleted_paths.len() > 0 {
            let next_wake_time = recent_deleted_paths.get(0).unwrap().0;
            match timeout_at(next_wake_time, watcher_rx.recv()).await {
                Ok(x) => {
                    // Didn't timeout, assign the return value to res
                    res = x;
                }
                Err(_) => {
                    // Timeout occurred, clear expired records from database and wait again
                    clear_expired_records(&mut recent_deleted_paths, &mut event_handler);
                    continue;
                }
            }
        } else {
            // No paths in database, just wait for next record indefinitely
            res = watcher_rx.recv().await;
        }
        match res {
            Some(evt) => {
                if evt.is_err() {
                    event_handler.handle_event(evt);
                    continue;
                }
                let evt = evt.unwrap();
                match evt {
                    Event {
                        kind: Modify(Name(RenameMode::From)), mut paths, ..
                    } => {
                        if let Some(_) = last_rename_from {
                            panic!("Got multiple 'Rename From' events in a row!")
                        }
                        let path = paths.pop().unwrap();
                        last_rename_from = Some(path);
                        continue;
                    }
                    Event {
                        kind: Modify(Name(RenameMode::To)),
                        mut paths,
                        attrs,
                    } => {
                        let from_path = last_rename_from.take().expect(
                        "Got 'Rename To' event, but no 'Rename From' event happened before this!",
                    );
                        let to_path = paths.pop().unwrap();
                        let evt = Event {
                            kind: Modify(Name(RenameMode::Both)),
                            paths: vec![from_path, to_path],
                            attrs,
                        };
                        event_handler.handle_event(Ok(evt));
                    }
                    Event { kind: Remove(RemoveKind::Any), mut paths, attrs } => {
                        assert_eq!(
                            paths.len(),
                            1,
                            "Number of created paths is not 1: {}",
                            paths.len()
                        );
                        let removed_path = paths.pop().unwrap();
                        let expires_at = Instant::now() + Duration::from_millis(10);
                        recent_deleted_paths.push((expires_at, removed_path, attrs));
                    }
                    Event { kind: Create(CreateKind::Any), mut paths, attrs } => {
                        assert_eq!(
                            paths.len(),
                            1,
                            "Number of created paths is not 1: {}",
                            paths.len()
                        );
                        let created_path = paths.pop().unwrap();
                        let mut deleted_path_match_id: Option<usize> = None;
                        for i in 0..recent_deleted_paths.len() {
                            let deleted_path = &recent_deleted_paths.get(i).unwrap().1;
                            let created_name = created_path
                                .file_name()
                                .expect("Path doesn't have file name");
                            let deleted_name = deleted_path
                                .file_name()
                                .expect("Path doesn't have file name");
                            if created_name == deleted_name {
                                deleted_path_match_id = Some(i);
                                break;
                            }
                        }
                        match deleted_path_match_id {
                            Some(i) => {
                                let deleted_path_match = recent_deleted_paths.remove(i).1;
                                let evt = Event {
                                    kind: Modify(Name(RenameMode::Both)),
                                    paths: vec![deleted_path_match, created_path],
                                    attrs,
                                };
                                event_handler.handle_event(Ok(evt));
                            }
                            None => {
                                let evt = Event {
                                    kind: Create(CreateKind::Any),
                                    paths: vec![created_path],
                                    attrs,
                                };
                                event_handler.handle_event(Ok(evt));
                            }
                        }
                    }
                    _ => event_handler.handle_event(Ok(evt)),
                }
            }
            None => {
                // send remaining deleted paths to output
                for (_, path, attrs) in recent_deleted_paths {
                    let evt = Event {
                        kind: Remove(RemoveKind::Any),
                        paths: vec![path],
                        attrs,
                    };
                    event_handler.handle_event(Ok(evt));
                }
                break;
            }
        }
    }
}
