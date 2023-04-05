use crate::repo::{OpenError, Repo, SyncError};
use crate::scan::{scan_dir, Options, ScanError};
use futures::channel::mpsc::UnboundedReceiver;
use relative_path::RelativePathBuf;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use serde::Serialize;
use tokio::sync::{Mutex, RwLock};

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

pub enum ManagerCommand {
    ReSync,
}

// async fn manage_repo(
//     path: impl AsRef<Path>,
//     event_recv: UnboundedReceiver<ManagerCommand>,
//     event_send: UnboundedSender<ManagerCommand>,
// ) {
//     let repo = Repo::open(path.as_ref())?;
//     while let Some(command) = event_recv.recv().await {
//         match command {
//             ManagerCommand::ReSync => {
//
//             }
//         }
//     }
// }

#[derive(Debug)]
pub struct RepoManager {
    repo: Mutex<Repo>,
    status: RwLock<ManagerStatus>,
    path: PathBuf,
}

impl RepoManager {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, OpenError> {
        let path = path.as_ref();
        let repo = Repo::open(path.clone())?;
        let manager = Self {
            repo: Mutex::new(repo),
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
