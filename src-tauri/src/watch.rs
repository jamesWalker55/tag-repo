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

enum RenamePartialPath<'a> {
    CreatedPath { path: &'a Path, waker: Waker },
    RemovedPath { path: &'a Path, waker: Waker },
}

// type RenamePathResolverDB<'a> = HashMap<&'a str, Vec<&'a PathBuf>>;

// enum RenamePathResolverState<'a> {
//     Initial,
//     WaitingDBWriteLock(
//         Option<Pin<Box<dyn Future<Output = RwLockWriteGuard<'a, RenamePathResolverDB<'a>>>>>>,
//     ),
// }
//
// struct RenamePathResolver<'a> {
//     path_info: RenamePartialPath<'a>,
//     path_info_db: Arc<RwLock<HashMap<&'a str, Vec<&'a PathBuf>>>>,
//     current_state: RenamePathResolverState<'a>,
// }
//
// impl<'a> Future for RenamePathResolver<'a> {
//     type Output = Option<&'a PathBuf>;
//
//     fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//         match &self.current_state {
//             RenamePathResolverState::Initial => {
//                 let future = self.path_info_db.write();
//                 future.poll(cx);
//                 self.current_state =
//                     RenamePathResolverState::WaitingDBWriteLock(Some(Box::pin(future)));
//                 // future.poll(cx)
//             }
//             RenamePathResolverState::WaitingDBWriteLock(lock) => {
//                 todo!()
//             }
//         }
//         // let x = cx.waker().clone();
//         todo!()
//     }
// }

// enum ResolveRenameError {
//     InvalidPath,
// }
//
// type AsyncPathList<'a> = RwLock<Vec<&'a PathBuf>>;
//
// struct TemporaryPathPusher<'a> {
//     path: &'a PathBuf,
//     vec: AsyncPathList<'a>,
//     already_pushed: bool,
// }
//
// impl<'a> TemporaryPathPusher<'a> {
//     fn new(path: &PathBuf, vec: AsyncPathList) -> Self {
//         Self {
//             path,
//             vec,
//             already_pushed: false,
//         }
//     }
//
//     fn activate(&mut self) {
//         if self.already_pushed {
//             panic!()
//         }
//         let w = self.vec.write().push(self.path);
//         self.already_pushed = true;
//     }
// }
//
// impl<'a> Drop for TemporaryPathPusher<'a> {
//     fn drop(&mut self) {
//         let idx = self.vec.iter().position(|x| x == &self.path).unwrap();
//         self.vec.remove(idx);
//     }
// }
//
// type RenamePathResolverDB<'a> = HashMap<&'a OsStr, AsyncPathList<'a>>;
//
// async fn resolve_create_to_rename_path<'a>(
//     created_path: &'a PathBuf,
//     rename_path_resolver: Arc<RwLock<RenamePathResolverDB<'a>>>,
// ) -> Result<Option<&'a PathBuf>, ResolveRenameError> {
//     let created_filename = created_path
//         .file_name()
//         .ok_or(ResolveRenameError::InvalidPath)?;
//     let mut path_pusher = None;
//     {
//         let mut rename_path_resolver = rename_path_resolver.write().await;
//         match rename_path_resolver.get_mut(created_filename) {
//             Some(paths) => {
//                 path_pusher = Some(TemporaryPathPusher {
//                     path: created_path,
//                     vec: paths,
//                     already_pushed: false,
//                 })
//             }
//             None => {
//                 let mut paths = vec![];
//                 paths.push(created_path);
//                 rename_path_resolver.insert(created_filename, paths);
//             }
//         }
//     }
//     Err(ResolveRenameError::InvalidPath)
// }

struct PathRecord<'a> {
    path: &'a Path,
    file_name: &'a OsStr,
    sender: oneshot::Sender<ManagerResponse<'a>>,
    expires_at: Instant,
}

// impl PathRecord {
//     fn create(path: &Path, sender: oneshot::Sender<ManagerResponse>) {
//         let file_name = path.file_name();
//         Self {
//             path,
//             file_name: path.file_name().expect()
//         }
//     }
// }

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
                    if record.file_name == other_record.file_name {
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
    let (tx, mut rx) = unbounded_channel();
    let tokio_handle = Handle::current();

    let mut watcher = RecommendedWatcher::new(
        move |res| {
            tokio_handle.block_on(async {
                tx.send(res).unwrap();
            })
        },
        Config::default(),
    )?;

    watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;

    let mut last_rename_from: Option<PathBuf> = None;
    // let paths_buffer = Arc::new(RwLock::new(HashMap::new()));

    while let Some(evt) = rx.recv().await {
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
            //     kind: Remove(RemoveKind::Any),
            //     paths: mut removed_paths,
            //     ..
            // } = evt
            // {
            //     assert_eq!(
            //         removed_paths.len(),
            //         1,
            //         "Number of removed paths is not 1: {}",
            //         removed_paths.len()
            //     );
            //     let removed_path = removed_paths.pop().unwrap();
            //     task::spawn(async move {
            //         let rv = timeout(Duration::from_millis(100), async {
            //             let paths_buffer = paths_buffer.read().await;
            //             let x = paths_buffer.contains_key(remove);
            //         })
            //         .await;
            //         match rv {
            //             Ok(Some(created_path)) => {
            //                 // found matching create, this is a file-move event
            //                 // we got a path, meaning we should handle this event
            //                 // we'll create a rename event:
            //                 let evt = Event {
            //                     kind: Modify(Name(RenameMode::Both)),
            //                     paths: vec![removed_path, created_path],
            //                     attrs: evt.attrs.clone(),
            //                 };
            //                 println!("{:?}", evt);
            //             }
            //             Ok(None) => {
            //                 // found matching create, this is a file-move event
            //                 // however we didn't get a path, meaning the paired "create" event will handle this
            //                 // we'll do nothing here
            //             }
            //             Err(e) => {
            //                 // timeout, no paired create found
            //                 // treat this as a remove
            //                 println!("{:?}", evt);
            //             }
            //         }
            //     });
            // } else if let Event {
            //     kind: Create(CreateKind::Any),
            //     paths: mut created_paths,
            //     ..
            // } = evt
            // {
            //     assert_eq!(
            //         created_paths.len(),
            //         1,
            //         "Number of created paths is not 1: {}",
            //         created_paths.len()
            //     );
            //     let created_path = created_paths.pop().unwrap();
            //     task::spawn(async move {
            //         let rv = timeout(Duration::from_millis(100), has_paired_delete(created_path)).await;
            //         match rv {
            //             Ok(Some(removed_path)) => {
            //                 // found matching remove, this is a file-move event
            //                 // we got a path, meaning we should handle this event
            //                 // we'll create a rename event:
            //                 let evt = Event {
            //                     kind: Modify(Name(RenameMode::Both)),
            //                     paths: vec![removed_path, created_path],
            //                     attrs: evt.attrs.clone(),
            //                 };
            //                 println!("{:?}", evt);
            //             }
            //             Ok(None) => {
            //                 // found matching remove, this is a file-move event
            //                 // however we didn't get a path, meaning the paired "remove" event will handle this
            //                 // we'll do nothing here
            //             }
            //             Err(e) => {
            //                 // timeout, no paired remove found
            //                 // treat this as a create
            //                 println!("{:?}", evt);
            //             }
            //         }
            //     });
            // } else if let Event {
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
