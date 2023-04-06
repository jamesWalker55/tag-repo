use crate::repo::{OpenError, RemoveError, Repo, SyncError};
use crate::scan::{classify_path, scan_dir, Options, PathType, ScanError, to_relative_path};
use crate::watch::{NormWatcher, ReadDirectoryChangesNormWatcher};
use futures::channel::mpsc::UnboundedReceiver;
use notify::event::{ModifyKind, RenameMode};
use notify::Event;
use notify::EventKind::{Create, Modify, Remove};
use relative_path::{RelativePath, RelativePathBuf};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tokio::time::timeout;

#[derive(Debug, Copy, Clone, Serialize)]
pub enum ManagerStatus {
    Idle,
    ScanningDirectory,
    UpdatingRepo,
}

impl Default for ManagerStatus {
    fn default() -> Self {
        Self::Idle
    }
}

// TODO: You need to rewrite your watcher to pass it a mpsc sender / follow notify's watcher api
// Right now, it's impossible to drop the watcher from a different thread

// async fn watcher_loop(repo: Arc<Mutex<Repo>>, repo_path: PathBuf, options: Options) {
//     let repo_path = repo_path.as_path();
//     let mut watcher = ReadDirectoryChangesNormWatcher::new().expect("failed to create watcher");
//     while let Some(evt) = watcher.recv().await {
//         let evt = evt.expect("unknown event error");
//         match evt {
//             evt if evt.kind == Modify(ModifyKind::Any) => { /* ignore */ }
//             Event { kind: Create(_), mut paths, .. } => {
//                 let path = paths.pop().expect("create event doesn't have a path");
//                 let PathType::Item(path) = classify_path(path, repo_path, &options) else {
//                     continue;
//                 };
//                 let repo = repo.lock().await;
//                 repo.insert_item(path.to_string(), "")
//                     .expect("failed to insert item");
//             }
//             Event { kind: Remove(_), mut paths, .. } => {
//                 let path = paths.pop().expect("remove event doesn't have a path");
//                 let path = to_relative_path(path.as_path(), repo_path);
//                 let repo = repo.lock().await;
//                 // TODO: Better handling here
//                 // Since removals are delayed, the item we are trying to remove may not be in the repo
//                 // Don't panic if the item isn't found
//                 // Only panic if there is some rusqlite error
//                 repo.remove_item_by_path(path.to_string())
//                     .expect("failed to remove item");
//             }
//             Event {
//                 kind: Modify(ModifyKind::Name(RenameMode::Both)),
//                 mut paths,
//                 ..
//             } => {
//                 let new_path = paths.pop().expect("rename event doesn't have any paths");
//                 let old_path = paths.pop().expect("rename event only has one path");
//                 let old_path = to_relative_path(old_path.as_path(), repo_path);
//                 let PathType::Item(new_path) = classify_path(new_path, repo_path, &options) else {
//                     continue;
//                 };
//                 let repo = repo.lock().await;
//                 repo.rename_path(old_path, new_path)
//                     .expect("failed to rename item");
//             }
//             _ => (),
//         }
//     }
// }

#[derive(Debug)]
pub struct RepoManager {
    repo: Arc<Mutex<Repo>>,
    status: RwLock<ManagerStatus>,
    path: PathBuf,
    // watcher_receiver:
}

impl RepoManager {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, OpenError> {
        let path = path.as_ref();
        let repo = Repo::open(path.clone())?;
        let manager = Self {
            repo: Arc::new(Mutex::new(repo)),
            status: RwLock::new(ManagerStatus::Idle),
            path: path.to_path_buf(),
        };
        Ok(manager)
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    pub async fn status(&self) -> ManagerStatus {
        *self.status.read().await
    }

    pub async fn update_status(&self, status: ManagerStatus) {
        *self.status.write().await = status;
    }

    pub async fn resync(&self) -> Result<(), SyncError> {
        self.update_status(ManagerStatus::ScanningDirectory).await;
        let path = self.path.clone();
        let new_paths = tokio::task::spawn_blocking(move || scan_dir(path, Options::default()))
            .await
            .expect("failed to join with thread that's scanning a directory")?;

        self.update_status(ManagerStatus::UpdatingRepo).await;
        {
            let mut repo = self.repo.lock().await;
            // TODO: Move this to a separate thread, when it blocks it makes the GUI lag!
            // UPDATE: Used block_in_place, not sure if it prevents the GUI lag
            tokio::task::block_in_place(move || repo.sync(new_paths))?;
            // tokio::task::spawn_blocking(move || repo.sync(new_paths))
            //     .await
            //     .expect("failed to join with thread that's batch-updating the database")?;
        }

        self.update_status(ManagerStatus::Idle).await;
        Ok(())
    }
}
