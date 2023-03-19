use std::path::{Path, PathBuf};
use std::time::Duration;

use async_trait::async_trait;
use futures::channel::oneshot;
use notify::event::ModifyKind::Name;
use notify::event::{CreateKind, ModifyKind, RemoveKind, RenameMode};
use notify::EventKind::{Create, Modify, Remove};
use notify::{Config, Event, ReadDirectoryChangesWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::task;
use tokio::task::JoinHandle;
use tokio::time::{timeout_at, Instant};

use crate::watch::NormWatcher;

#[derive(Debug, Eq, PartialEq)]
enum PathRecordAction {
    Created,
    Removed,
}

#[derive(Debug)]
enum PathRecordCreationError {
    InvalidPath,
    // InvalidEvent,
}

#[derive(Debug)]
struct PathRecord {
    /// Any path that gets stored here MUST allow calling #file_name without error
    path: PathBuf,
    action: PathRecordAction,
    sender: oneshot::Sender<ManagerResponse>,
    expires_at: Instant,
}

impl PathRecord {
    fn create(
        path: PathBuf,
        action: PathRecordAction,
        sender: oneshot::Sender<ManagerResponse>,
    ) -> Result<Self, PathRecordCreationError> {
        // verify that path is valid
        path.file_name()
            .ok_or(PathRecordCreationError::InvalidPath)?;

        let expires_at = Instant::now() + Duration::from_millis(100);

        Ok(Self { path, action, sender, expires_at })
    }
}

#[derive(Debug)]
enum ManagerResponse {
    /// Respond that "This event is not a rename, treat it as the original create/remove event.
    NotRename,
    /// Respond that "This event is a rename, and create a new rename event".
    CreateRename(PathBuf),
    /// Respond that "This event is a rename, but skip this event", implying the pairing event will
    /// handle this.
    IgnoreRename,
}

async fn path_records_manager<'a>(mut rx: UnboundedReceiver<PathRecord>) {
    use ManagerResponse::*;

    fn clear_expired_records(db: &mut Vec<PathRecord>) {
        let now = Instant::now();
        let mut i = 0;
        loop {
            if i == db.len() {
                break;
            }
            let mut is_expired = false;
            {
                let x = db.get(i).unwrap();
                if x.expires_at <= now {
                    // record has expired, prepare to remove this record
                    // DON'T increment i to next record
                    is_expired = true;
                } else {
                    // not expired, move on to next record
                    i += 1;
                }
            }
            if is_expired {
                let x = db.remove(i);
                // send event, don't care if the receiver has been dropped
                let _ = x.sender.send(NotRename);
            }
        }
    }

    // TODO: Replace list with a binary heap
    let mut db: Vec<PathRecord> = vec![];
    let mut res;

    loop {
        // If we have paths in the database, timeout until the next path's instant
        if db.len() > 0 {
            let next_wake_time = db.get(0).unwrap().expires_at;
            match timeout_at(next_wake_time, rx.recv()).await {
                Ok(x) => {
                    // Didn't timeout, assign the return value to res
                    res = x;
                }
                Err(_) => {
                    // Timeout occurred, clear expired records from database and wait again
                    clear_expired_records(&mut db);
                    continue;
                }
            }
        } else {
            // No paths in database, just wait for next record indefinitely
            res = rx.recv().await;
        }

        match res {
            Some(record) => {
                // Got instructions to match this path record

                // Scan records to find match
                let mut idx_to_remove = None;
                for (i, other_record) in db.iter().enumerate() {
                    // If both have the same path, and one is Created and other is Removed...
                    let name_a = record.path.file_name().expect("Path has no filename");
                    let name_b = other_record.path.file_name().expect("Path has no filename");
                    if name_a == name_b && record.action != other_record.action {
                        idx_to_remove = Some(i);
                        break;
                    }
                }
                if let Some(i) = idx_to_remove {
                    // Found match, send responses and remove from database
                    let other_record = db.remove(i);
                    record.sender.send(CreateRename(other_record.path)).unwrap();
                    other_record.sender.send(IgnoreRename).unwrap();
                } else {
                    // No match, add to database
                    db.push(record);
                }

                // Clear expired records from database
                clear_expired_records(&mut db);
            }
            None => {
                // No more instructions, all senders have been dropped
                break;
            }
        }
    }
}

/// A wrapper for `ReadDirectoryChangesWatcher`.
///
/// The structure of this wrapper is like this:
///
/// ```plain
/// Watcher
///   |
///   v
/// Handler -> Manager
///   |   ^      |
///   |   +------+
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
/// - Create/Delete events: It sends these events to the **manager** to be further processed. It
///   later receives back a message from the **manager** then returns an event according to the
///   message.
///
///   This is because Windows shows file move events as Create/Delete events. The **manager** will
///   attempt to resolve these Create/Delete events into Rename events if possible, otherwise the
///   manager returns the original Create/Delete events.
///
/// - Other events: It returns the events as-is, without sending them to the manager
///
/// The **manager** maintains a list of recently-created/deleted paths. When a new path is
/// created/deleted, it scans this list to check if there are any similar deleted/created path. If
/// so, it tells the **handler** to treat the event as a rename event. If not found, it adds the
/// path to the list, then tells the **handler** to return the original event as-is.
///
/// ## How to stop watching
///
/// Just drop this struct. It should automatically clean up everything. If you really need to drop
/// it manually, here are some instructions / information:
///
/// - Drop the `notify` watcher first. Since the other tasks depend on receiving events from the
///   `notify` watcher, terminating the watcher will naturally cause the tasks to terminate
/// - Then `await` on the two task handlers. You want to ensure that the tasks have really
///   ended.
pub struct ReadDirectoryChangesNormWatcher {
    /// The actual watcher instance.
    watcher: ReadDirectoryChangesWatcher,
    /// Handle for the path record manager / debouncer thing. Its only purpose is to keep the handle
    /// in memory and only drop it when this struct is dropped.
    manager_handle: JoinHandle<()>,
    /// A receiver that receives processed events from the event handler. As the name suggests,
    /// these events are final ("output") events.
    output_rx: UnboundedReceiver<notify::Result<Event>>,
    /// Handle for the event handler. Its only purpose is to keep the handle in memory and only drop
    /// it when this struct is dropped.
    event_handler_handle: JoinHandle<()>,
}

impl ReadDirectoryChangesNormWatcher {
    pub fn new() -> notify::Result<Self> {
        // Spawn the watcher
        let (watcher_tx, watcher_rx) = unbounded_channel();

        let watcher = ReadDirectoryChangesWatcher::new(
            move |res| watcher_tx.send(res).unwrap(),
            Config::default(),
        )?;

        // Spawn the path manager
        let (manager_tx, manager_rx) = unbounded_channel();
        let manager_handle = tokio::spawn(async move {
            path_records_manager(manager_rx).await;
        });

        // Spawn the event handler
        let (output_tx, output_rx) = unbounded_channel();
        let event_handler_handle = tokio::spawn(async move {
            event_handler(watcher_rx, manager_tx, output_tx).await;
        });

        Ok(Self {
            watcher,
            manager_handle,
            output_rx,
            event_handler_handle,
        })
    }
}

#[async_trait]
impl NormWatcher for ReadDirectoryChangesNormWatcher {
    fn watch(&mut self, path: &Path, recursive_mode: RecursiveMode) -> notify::Result<()> {
        self.watcher.watch(path.as_ref(), recursive_mode)
    }

    async fn recv(&mut self) -> Option<notify::Result<Event>> {
        self.output_rx.recv().await
    }

    #[cfg(test)]
    fn stop_watching(&mut self) {
        let temp_watcher = ReadDirectoryChangesWatcher::new(|_res| {}, Config::default()).unwrap();
        let real_watcher = std::mem::replace(&mut self.watcher, temp_watcher);
        drop(real_watcher);
    }
}

async fn event_handler(
    mut watcher_rx: UnboundedReceiver<notify::Result<Event>>,
    manager_tx: UnboundedSender<PathRecord>,
    output_tx: UnboundedSender<notify::Result<Event>>,
) {
    let mut last_rename_from: Option<PathBuf> = None;
    while let Some(evt) = watcher_rx.recv().await {
        if evt.is_err() {
            output_tx.send(evt).unwrap();
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
            Event { kind: Modify(Name(RenameMode::To)), mut paths, .. } => {
                let from_path = last_rename_from.take().expect(
                    "Got 'Rename To' event, but no 'Rename From' event happened before this!",
                );
                let to_path = paths.pop().unwrap();
                let evt = Event {
                    kind: Modify(Name(RenameMode::Both)),
                    paths: vec![from_path, to_path],
                    attrs: evt.attrs.clone(),
                };
                output_tx.send(Ok(evt)).unwrap();
            }
            Event { kind: Remove(RemoveKind::Any), mut paths, attrs } => {
                assert_eq!(
                    paths.len(),
                    1,
                    "Number of created paths is not 1: {}",
                    paths.len()
                );
                let removed_path = paths.pop().unwrap();
                let (path_tx, path_rx) = oneshot::channel();
                let record =
                    PathRecord::create(removed_path.clone(), PathRecordAction::Removed, path_tx)
                        .unwrap();
                manager_tx.send(record).unwrap();
                let output_tx = output_tx.clone();

                task::spawn(async move {
                    match path_rx.await {
                        Ok(ManagerResponse::CreateRename(created_path)) => {
                            // found matching create, this is a file-move event
                            // we got a path, meaning we should handle this event
                            // we'll create a rename event:
                            let evt = Event {
                                kind: Modify(Name(RenameMode::Both)),
                                paths: vec![removed_path, created_path.to_path_buf()],
                                attrs,
                            };
                            output_tx.send(Ok(evt)).unwrap();
                        }
                        Ok(ManagerResponse::IgnoreRename) => {
                            // found matching create, this is a file-move event
                            // however we didn't get a path, meaning the paired create event will handle this
                            // we'll do nothing here
                        }
                        Ok(ManagerResponse::NotRename) | Err(_) => {
                            // Case (a): no paired path found, treat this as a remove
                            // Case (b): sender got dropped, the watcher has likely been dropped,
                            //           just treat it as a normal event
                            let evt = Event {
                                kind: Remove(RemoveKind::Any),
                                paths: vec![removed_path],
                                attrs,
                            };
                            output_tx.send(Ok(evt)).unwrap();
                        }
                    }
                });
            }
            Event { kind: Create(CreateKind::Any), paths, attrs } => {
                assert_eq!(
                    paths.len(),
                    1,
                    "Number of created paths is not 1: {}",
                    paths.len()
                );
                let created_path = paths.get(0).unwrap().clone();
                let (path_tx, path_rx) = oneshot::channel();
                let record =
                    PathRecord::create(created_path.clone(), PathRecordAction::Created, path_tx)
                        .unwrap();
                manager_tx.send(record).unwrap();
                let output_tx = output_tx.clone();

                task::spawn(async move {
                    match path_rx.await {
                        Ok(ManagerResponse::CreateRename(removed_path)) => {
                            // found matching remove, this is a file-move event
                            // we got a path, meaning we should handle this event
                            // we'll create a rename event:
                            let evt = Event {
                                kind: Modify(Name(RenameMode::Both)),
                                paths: vec![removed_path, created_path],
                                attrs,
                            };
                            output_tx.send(Ok(evt)).unwrap();
                        }
                        Ok(ManagerResponse::IgnoreRename) => {
                            // found matching remove, this is a file-move event
                            // however we didn't get a path, meaning the paired remove event will handle this
                            // we'll do nothing here
                        }
                        Ok(ManagerResponse::NotRename) | Err(_) => {
                            // Case (a): no paired path found, treat this as a create
                            // Case (b): sender got dropped, the watcher has likely been dropped,
                            //           just treat it as a normal event
                            let evt = Event {
                                kind: Create(CreateKind::Any),
                                paths: vec![created_path.to_path_buf()],
                                attrs,
                            };
                            output_tx.send(Ok(evt)).unwrap();
                        }
                    }
                });
            }
            Event { kind: Modify(ModifyKind::Any), .. } => {
                output_tx.send(Ok(evt)).unwrap();
            }
            _ => output_tx.send(Ok(evt)).unwrap(),
        }
    }
}
