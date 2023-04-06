use crate::repo::{OpenError, RemoveError, Repo, SyncError};
use crate::scan::{classify_path, scan_dir, to_relative_path, Options, PathType, ScanError};
use crate::watch::WindowsNormWatcher;
use futures::executor::block_on;
use notify::event::{ModifyKind, RenameMode};
use notify::EventKind::{Create, Modify, Remove};
use notify::{Event, ReadDirectoryChangesWatcher, RecursiveMode, Watcher};
use relative_path::{RelativePath, RelativePathBuf};
use serde::Serialize;
use std::fmt::{Debug, Formatter, Pointer};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tokio::sync::{Mutex, RwLock};
use tokio::time::timeout;
use tracing::error;

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

// TODO: For now, use the default notify watcher for support on all OS

async fn event_handler(
    repo: Arc<Mutex<Repo>>,
    repo_path: PathBuf,
    mut receiver: UnboundedReceiver<notify::Result<Event>>,
    options: Options,
) {
    let repo_path = repo_path.as_path();
    while let Some(evt) = receiver.recv().await {
        let evt = evt.expect("unknown event error");
        match evt {
            evt if evt.kind == Modify(ModifyKind::Any) => { /* ignore */ }
            Event { kind: Create(_), mut paths, .. } => {
                let path = paths.pop().expect("create event doesn't have a path");
                let PathType::Item(path) = classify_path(path, repo_path, &options) else {
                    continue;
                };
                let repo = repo.lock().await;
                repo.insert_item(path.to_string(), "")
                    .expect("failed to insert item");
            }
            Event { kind: Remove(_), mut paths, .. } => {
                let path = paths.pop().expect("remove event doesn't have a path");
                let path = to_relative_path(path.as_path(), repo_path);
                let repo = repo.lock().await;
                // TODO: Better handling here
                // Since removals are delayed, the item we are trying to remove may not be in the repo
                // Don't panic if the item isn't found
                // Only panic if there is some rusqlite error
                repo.remove_item_by_path(path.to_string())
                    .expect("failed to remove item");
            }
            Event {
                kind: Modify(ModifyKind::Name(RenameMode::Both)),
                mut paths,
                ..
            } => {
                let new_path = paths.pop().expect("rename event doesn't have any paths");
                let old_path = paths.pop().expect("rename event only has one path");
                let old_path = to_relative_path(old_path.as_path(), repo_path);
                let PathType::Item(new_path) = classify_path(new_path, repo_path, &options) else {
                    continue;
                };
                let repo = repo.lock().await;
                repo.rename_path(old_path, new_path)
                    .expect("failed to rename item");
            }
            _ => (),
        }
    }
}

#[derive(Error, Debug)]
pub enum WatchError {
    #[error("failed to watch path")]
    CannotWatchPath(#[from] notify::Error),
    #[error("already watching path")]
    AlreadyWatching,
}

#[derive(Error, Debug)]
pub enum UnwatchError {
    #[error("not watching path, cannot unwatch")]
    NotWatching,
}

trait DebugWatcher: Watcher + Debug + Sync + Send {}

impl DebugWatcher for ReadDirectoryChangesWatcher {}

#[derive(Debug)]
pub struct RepoManager {
    repo: Arc<Mutex<Repo>>,
    status: RwLock<ManagerStatus>,
    path: PathBuf,
    // watcher: Option<Box<dyn DebugWatcher>>,
    watcher: RwLock<Option<Box<dyn DebugWatcher>>>,
}

impl RepoManager {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, OpenError> {
        let path = path.as_ref();
        let repo = Repo::open(path.clone())?;
        let manager = Self {
            repo: Arc::new(Mutex::new(repo)),
            status: RwLock::new(ManagerStatus::Idle),
            path: path.to_path_buf(),
            watcher: RwLock::new(None),
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
            // clone a reference to the repo
            let repo = self.repo.clone();
            // move the sync() call to a separate blocking thread
            tokio::task::spawn_blocking(move || {
                let mut repo = block_on(async { repo.lock().await });
                repo.sync(new_paths)
            })
            .await
            .expect("failed to join with thread that's batch-updating the database")?;
        }

        self.update_status(ManagerStatus::Idle).await;
        Ok(())
    }

    pub async fn watch(&self) -> Result<(), WatchError> {
        // check there isn't already a watcher
        {
            let watcher = self.watcher.read().await;
            // if there is already watcher, raise error
            if watcher.is_some() {
                return Err(WatchError::AlreadyWatching);
            }
        }

        // new unbounded channel for communication
        let (tx, rx) = unbounded_channel();

        // no need to store this thread's handle
        // the thread should stop when you drop the watcher
        {
            let repo = self.repo.clone();
            let path = self.path.clone();
            tokio::spawn(async move { event_handler(repo, path, rx, Options::default()) });
        }

        // create a new watcher
        let mut watcher =
            notify::recommended_watcher(move |res: notify::Result<Event>| match tx.send(res) {
                Ok(_) => {}
                Err(err) => {
                    let evt = err.0;
                    error!("reached maximum event limit, cannot send event: {:?}", evt);
                }
            })
            .unwrap();
        watcher.watch(self.path.as_ref(), RecursiveMode::Recursive)?;

        // drop the existing watcher
        {
            let mut watcher_opt = self.watcher.write().await;
            *watcher_opt = Some(Box::new(watcher));
        }

        Ok(())
    }

    pub async fn unwatch(&self) -> Result<(), UnwatchError> {
        let mut watcher = self.watcher.write().await;
        if watcher.is_none() {
            Err(UnwatchError::NotWatching)
        } else {
            *watcher = None;
            Ok(())
        }
    }
}
