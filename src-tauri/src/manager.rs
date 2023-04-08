use crate::repo::{Item, OpenError, QueryError, RemoveError, Repo, SearchError, SyncError};
use crate::scan::{classify_path, scan_dir, to_relative_path, Options, PathType, ScanError};
use crate::watch::{BestWatcher, WindowsNormWatcher};
use futures::executor::block_on;
use notify::event::{ModifyKind, RenameMode};
use notify::EventKind::{Create, Modify, Remove};
use notify::{
    Config, Event, ReadDirectoryChangesWatcher, RecommendedWatcher, RecursiveMode, Watcher,
};
use relative_path::{RelativePath, RelativePathBuf};
use serde::Serialize;
use std::fmt::{Debug, Formatter, Pointer};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Manager, Runtime};
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tokio::sync::{Mutex, RwLock};
use tokio::time::timeout;
use tracing::{debug, error, info};

#[derive(Debug, Copy, Clone, Serialize)]
pub enum ManagerStatus {
    Idle,
    ScanningDirectory,
    UpdatingRepo,
    Querying,
}

impl Default for ManagerStatus {
    fn default() -> Self {
        Self::Idle
    }
}

// #[tracing::instrument]
async fn event_handler<R: Runtime>(
    repo: Arc<Mutex<Repo>>,
    repo_path: PathBuf,
    app_handle: AppHandle<R>,
    mut receiver: UnboundedReceiver<notify::Result<Event>>,
    options: Options,
) {
    debug!("watcher started!");
    let repo_path = repo_path.as_path();
    while let Some(evt) = receiver.recv().await {
        debug!("received event: {:?}", evt);
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
                app_handle
                    .emit_all("item-added", path.to_string())
                    .expect("Failed to emit event");
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
                app_handle
                    .emit_all("item-removed", path.to_string())
                    .expect("Failed to emit event");
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
                repo.rename_path(old_path.to_string(), new_path.to_string())
                    .expect("failed to rename item");
                app_handle
                    .emit_all("item-renamed", (old_path.to_string(), new_path.to_string()))
                    .expect("Failed to emit event");
            }
            _ => (),
        }
    }
    debug!("watcher ended!");
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

#[derive(Debug)]
pub struct RepoManager<R: Runtime> {
    repo: Arc<Mutex<Repo>>,
    status: RwLock<ManagerStatus>,
    path: PathBuf,
    watcher: RwLock<Option<BestWatcher>>,
    app_handle: AppHandle<R>,
}

impl<R: Runtime> RepoManager<R> {
    pub fn new(path: impl AsRef<Path>, app_handle: AppHandle<R>) -> Result<Self, OpenError> {
        let path = path.as_ref();
        let repo = Repo::open(&path)?;
        let manager = Self {
            repo: Arc::new(Mutex::new(repo)),
            status: RwLock::new(ManagerStatus::Idle),
            path: path.to_path_buf(),
            watcher: RwLock::new(None),
            app_handle,
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
        self.app_handle
            .emit_all("status-changed", status)
            .expect("Failed to emit event");
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

    pub async fn query(&self, query: &str) -> Result<Vec<i64>, QueryError> {
        self.update_status(ManagerStatus::Querying).await;
        let items = {
            // clone a reference to the repo
            let repo = self.repo.clone();
            let query = query.to_string();
            tokio::task::spawn_blocking(move || {
                let mut repo = block_on(async { repo.lock().await });
                repo.query_ids(&query)
            })
            .await
            .expect("failed to join with thread that's batch-updating the database")?
        };
        self.update_status(ManagerStatus::Idle).await;
        Ok(items)
    }

    pub async fn get_item(&self, id: i64) -> Result<Item, SearchError> {
        self.update_status(ManagerStatus::Querying).await;
        let item = {
            let repo = self.repo.lock().await;
            repo.get_item_by_id(id)
        };
        self.update_status(ManagerStatus::Idle).await;
        Ok(item?)
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
            let new_handle = self.app_handle.clone();
            tokio::spawn(async move {
                event_handler(repo, path, new_handle, rx, Options::default()).await
            });
        }

        // create a new watcher
        let mut watcher = BestWatcher::new(
            move |res: notify::Result<Event>| match tx.send(res) {
                Ok(_) => {}
                Err(err) => {
                    let evt = err.0;
                    error!("failed to send event to watcher loop: {:?}", evt);
                }
            },
            Config::default(),
        )
        .unwrap();
        watcher.watch(self.path.as_ref(), RecursiveMode::Recursive)?;

        // drop the existing watcher
        {
            let mut watcher_opt = self.watcher.write().await;
            *watcher_opt = Some(watcher);
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
