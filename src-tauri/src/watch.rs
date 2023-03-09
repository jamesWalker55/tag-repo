use futures::channel::oneshot;
use notify::event::ModifyKind::Name;
use notify::event::{CreateKind, ModifyKind, RemoveKind, RenameMode};
use notify::EventKind::{Create, Modify, Remove};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tokio::sync::{RwLock, RwLockWriteGuard};
use tokio::task;
use tokio::time::{timeout, timeout_at, Instant};

#[derive(Eq, PartialEq)]
enum PathRecordAction {
    Created,
    Removed,
}

#[derive(Debug)]
enum PathRecordCreationError {
    InvalidPath,
    InvalidEvent,
}

#[derive(Debug)]
struct PathRecord<'a> {
    path: &'a Path,
    file_name: &'a OsStr,
    action: PathRecordAction,
    sender: oneshot::Sender<ManagerResponse<'a>>,
    expires_at: Instant,
}

impl<'a> PathRecord<'a> {
    fn create(
        path: &'a Path,
        action: PathRecordAction,
        sender: oneshot::Sender<ManagerResponse<'a>>,
    ) -> Result<Self, PathRecordCreationError> {
        let expires_at = Instant::now() + Duration::from_millis(100);
        let file_name = path
            .file_name()
            .ok_or(PathRecordCreationError::InvalidPath)?;

        Ok(Self {
            path,
            file_name,
            action,
            sender,
            expires_at,
        })
    }
}

enum ManagerResponse<'a> {
    /// Respond that "This event is not a rename, treat it as the original create/remove event.
    NotRename,
    /// Respond that "This event is a rename, and create a new rename event".
    CreateRename(&'a Path),
    /// Respond that "This event is a rename, but skip this event", implying the pairing event will
    /// handle this.
    IgnoreRename,
}

async fn path_records_manager<'a>(mut rx: UnboundedReceiver<PathRecord<'a>>) {
    use ManagerResponse::*;

    let mut db: Vec<PathRecord<'a>> = vec![];
    let mut res = None;

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
                    let now = Instant::now();
                    db.retain(|x| x.expires_at >= now);
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

                // Clear expired records from database
                let now = Instant::now();
                db.retain(|x| x.expires_at >= now);

                // Scan records to find match
                let mut idx_to_remove = None;
                for (i, other_record) in db.iter().enumerate() {
                    // If both have the same path, and one is Created and other is Removed...
                    if record.file_name == other_record.file_name
                        && record.action != other_record.action
                    {
                        idx_to_remove = Some(i);
                        break;
                    }
                }
                if let Some(i) = idx_to_remove {
                    // Found match, send responses and remove from database
                    let other_record = db.remove(i);
                    record.sender.send(CreateRename(other_record.path));
                    other_record.sender.send(IgnoreRename);
                } else {
                    // No match, add to database
                    db.push(record);
                }
            }
            None => {
                // No more instructions, all senders have been dropped
                break;
            }
        }
    }
}

async fn async_watch(path: impl AsRef<Path>) -> notify::Result<()> {
    // Spawn the watcher

    let (watcher_tx, mut watcher_rx) = unbounded_channel();
    let tokio_handle = Handle::current();

    let mut watcher = RecommendedWatcher::new(
        move |res| {
            tokio_handle.block_on(async {
                watcher_tx.send(res).unwrap();
            })
        },
        Config::default(),
    )?;

    watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;

    // Spawn the path manager
    let (manager_tx, manager_rx) = unbounded_channel();
    let manager_handle = tokio::spawn(async move {
        path_records_manager(manager_rx).await;
    });

    // Begin watching for events

    let mut last_rename_from: Option<PathBuf> = None;
    while let Some(evt) = watcher_rx.recv().await {
        let evt = evt.unwrap();
        if let Event {
            kind: Modify(Name(RenameMode::From)),
            mut paths,
            ..
        } = evt
        {
            if let Some(_) = last_rename_from {
                panic!("Got multiple 'Rename From' events in a row!")
            }
            let path = paths.pop().unwrap();
            last_rename_from = Some(path);
            continue;
        } else if let Event {
            kind: Modify(Name(RenameMode::To)),
            mut paths,
            ..
        } = evt
        {
            let from_path = last_rename_from
                .take()
                .expect("Got 'Rename To' event, but no 'Rename From' event happened before this!");
            let to_path = paths.pop().unwrap();
            let evt = Event {
                kind: Modify(Name(RenameMode::Both)),
                paths: vec![from_path, to_path],
                attrs: evt.attrs.clone(),
            };
            println!("{:?}", evt);
        } else if let Event {
            kind: Remove(RemoveKind::Any),
            paths: mut removed_paths,
            ..
        } = evt
        {
            assert_eq!(
                removed_paths.len(),
                1,
                "Number of created paths is not 1: {}",
                removed_paths.len()
            );
            let removed_path = removed_paths.pop().unwrap();
            let (path_tx, path_rx) = oneshot::channel();
            let record =
                PathRecord::create(removed_path.as_path(), PathRecordAction::Removed, path_tx).unwrap();
            manager_tx.send(record).unwrap();

            task::spawn(async move {
                match path_rx.await {
                    Ok(ManagerResponse::CreateRename(created_path)) => {
                        // found matching create, this is a file-move event
                        // we got a path, meaning we should handle this event
                        // we'll create a rename event:
                        let evt = Event {
                            kind: Modify(Name(RenameMode::Both)),
                            paths: vec![removed_path, created_path.to_path_buf()],
                            attrs: evt.attrs.clone(),
                        };
                        println!("{:?}", evt);
                    }
                    Ok(ManagerResponse::IgnoreRename) => {
                        // found matching create, this is a file-move event
                        // however we didn't get a path, meaning the paired create event will handle this
                        // we'll do nothing here
                    }
                    Ok(ManagerResponse::NotRename) => {
                        // no paired path found
                        // treat this as a remove
                        println!("{:?}", evt);
                    }
                    Err(e) => {
                        // the sender got dropped somehow?
                        panic!();
                    }
                }
            });
        } else if let Event {
            kind: Create(CreateKind::Any),
            paths: created_paths,
            ..
        } = evt
        {
            assert_eq!(
                created_paths.len(),
                1,
                "Number of created paths is not 1: {}",
                created_paths.len()
            );
            let created_path = created_paths.get(0).unwrap().as_path();
            let (path_tx, path_rx) = oneshot::channel();
            let record =
                PathRecord::create(created_path, PathRecordAction::Created, path_tx).unwrap();
            manager_tx.send(record).unwrap();

            task::spawn(async move {
                match path_rx.await {
                    Ok(ManagerResponse::CreateRename(removed_path)) => {
                        // found matching remove, this is a file-move event
                        // we got a path, meaning we should handle this event
                        // we'll create a rename event:
                        let evt = Event {
                            kind: Modify(Name(RenameMode::Both)),
                            paths: vec![removed_path.to_path_buf(), created_path.to_path_buf()],
                            attrs: evt.attrs.clone(),
                        };
                        println!("{:?}", evt);
                    }
                    Ok(ManagerResponse::IgnoreRename) => {
                        // found matching remove, this is a file-move event
                        // however we didn't get a path, meaning the paired remove event will handle this
                        // we'll do nothing here
                    }
                    Ok(ManagerResponse::NotRename) => {
                        // no paired path found
                        // treat this as a create
                        println!("{:?}", evt);
                    }
                    Err(e) => {
                        // the sender got dropped somehow?
                        panic!();
                    }
                }
            });
        } else if let Event {
            kind: Modify(ModifyKind::Any),
            ..
        } = evt
        {
            // ignore
            // println!("{:?}", evt);
        } else {
            println!("{:?}", evt);
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    let path = r"D:\Programming\rust-learning\temp";
    println!("watching {}", path);

    if let Err(e) = async_watch(path).await {
        println!("error: {:?}", e)
    }
}
