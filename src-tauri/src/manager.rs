use crate::repo::{
    InsertTagsError, Item, OpenError, QueryError, RemoveError, RemoveTagsError, Repo, SearchError,
    SyncError,
};
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
    // Querying,
}

impl Default for ManagerStatus {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Serialize, Clone)]
pub struct ItemDetails {
    item: Item,
    filetype: FileType,
}

impl ItemDetails {
    fn from_item(item: Item) -> Self {
        let filetype = determine_filetype(&item.path);
        Self { item, filetype }
    }
}

#[derive(Serialize, Clone)]
pub enum FileType {
    Audio,
    Document,
    Image,
    Video,
    Unknown,
}

macro_rules! file_types {
    ($($file_type:tt),*) => {
        [$(stringify!($file_type)),*]
    };
}

const EXT_AUDIO: &'static [&'static str] = &file_types![
    aac, ac3, aif, aifc, aiff, au, cda, dts, fla, flac, it, m1a, m2a, m3u, m4a, mid, midi, mka,
    mod, mp2, mp3, mpa, ogg, opus, ra, rmi, snd, spc, umx, voc, wav, wma, xm
];

const EXT_DOCUMENT: &'static [&'static str] = &file_types![
    c, chm, cpp, csv, cxx, doc, docm, docx, dot, dotm, dotx, h, hpp, htm, html, hxx, ini, java,
    lua, mht, mhtml, odt, pdf, potm, potx, ppam, pps, ppsm, ppsx, ppt, pptm, pptx, rtf, sldm, sldx,
    thmx, txt, vsd, wpd, wps, wri, xlam, xls, xlsb, xlsm, xlsx, xltm, xltx, xml
];

const EXT_IMAGE: &'static [&'static str] =
    &file_types![ani, bmp, gif, ico, jpe, jpeg, jpg, pcx, png, psd, tga, tif, tiff, webp, wmf];

const EXT_VIDEO: &'static [&'static str] = &file_types![
    3g2, 3gp, 3gp2, 3gpp, amr, amv, asf, avi, bdmv, bik, d2v, divx, drc, dsa, dsm, dss, dsv, evo,
    f4v, flc, fli, flic, flv, hdmov, ifo, ivf, m1v, m2p, m2t, m2ts, m2v, m4b, m4p, m4v, mkv, mov,
    mp2v, mp4, mp4v, mpe, mpeg, mpg, mpls, mpv2, mpv4, mts, ogm, ogv, pss, pva, qt, ram, ratdvd,
    rm, rmm, rmvb, roq, rpm, smil, smk, swf, tp, tpr, ts, vob, vp6, webm, wm, wmp, wmv
];

pub fn determine_filetype(path: impl AsRef<Path>) -> FileType {
    let path: &Path = path.as_ref();
    let Some(extension) = path.extension() else {
        return FileType::Unknown;
    };

    let Some(extension) = extension.to_str() else {
        error!("cannot determine filetype of malformed path: {:?}", path);
        return FileType::Unknown;
    };

    if EXT_AUDIO.contains(&extension) {
        FileType::Audio
    } else if EXT_DOCUMENT.contains(&extension) {
        FileType::Document
    } else if EXT_IMAGE.contains(&extension) {
        FileType::Image
    } else if EXT_VIDEO.contains(&extension) {
        FileType::Video
    } else {
        FileType::Unknown
    }
}

// this prints a lot of text to the console
// either reduce the text or remove it entirely
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
                let inserted_item = repo
                    .insert_item(path.to_string(), "")
                    .expect("failed to insert item");
                app_handle
                    .emit_all("item-added", ItemDetails::from_item(inserted_item))
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
                let removed_item = repo
                    .remove_item_by_path(path.to_string())
                    .expect("failed to remove item");
                app_handle
                    .emit_all("item-removed", ItemDetails::from_item(removed_item))
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
                let old_path = old_path.to_string();
                let new_path = new_path.to_string();
                let repo = repo.lock().await;
                repo.rename_path(&old_path, &new_path)
                    .expect("failed to rename item");
                let renamed_item = repo
                    .get_item_by_path(&new_path)
                    .expect("failed to fetch renamed item");
                app_handle
                    .emit_all("item-renamed", ItemDetails::from_item(renamed_item))
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
        Ok(items)
    }

    pub async fn get_item_details(&self, id: i64) -> Result<ItemDetails, SearchError> {
        let item = {
            let repo = self.repo.lock().await;
            repo.get_item_by_id(id)
        }?;
        let details = ItemDetails::from_item(item);
        Ok(details)
    }

    pub async fn insert_tags(
        &self,
        ids: Vec<i64>,
        tags: Vec<String>,
    ) -> Result<(), InsertTagsError> {
        if ids.len() == 0 {
            return Ok(());
        }
        // clone a reference to the repo
        let repo = self.repo.clone();
        let app_handle = self.app_handle.clone();
        tokio::task::spawn_blocking(move || {
            let mut repo = block_on(async { repo.lock().await });
            let ids = ids;
            if ids.len() == 1 {
                let rv = repo.insert_tags(*ids.get(0).unwrap(), tags);
                let item = repo
                    .get_item_by_id(*ids.get(0).unwrap())
                    .expect("failed to get item after inserting tags");
                app_handle
                    .emit_all("item-tags-added", ItemDetails::from_item(item))
                    .expect("Failed to emit event");
                rv
            } else {
                let rv = repo.batch_insert_tags(&ids, tags);
                let items: Result<Vec<_>, _> = ids
                    .iter()
                    .map(|id| {
                        Ok::<_, SearchError>(ItemDetails::from_item(repo.get_item_by_id(*id)?))
                    })
                    .collect();
                let items = items.expect("failed to get items after batch-inserting tags");
                app_handle
                    .emit_all("batch-item-tags-added", items)
                    .expect("Failed to emit event");
                rv
            }
        })
        .await
        .expect("failed to join with thread that's inserting tags")?;
        Ok(())
    }

    pub async fn remove_tags(
        &self,
        ids: Vec<i64>,
        tags: Vec<String>,
    ) -> Result<(), RemoveTagsError> {
        // clone a reference to the repo
        let repo = self.repo.clone();
        let app_handle = self.app_handle.clone();
        tokio::task::spawn_blocking(move || {
            let mut repo = block_on(async { repo.lock().await });
            let ids = ids;
            if ids.len() == 1 {
                let rv = repo.remove_tags(*ids.get(0).unwrap(), tags);
                let item = repo
                    .get_item_by_id(*ids.get(0).unwrap())
                    .expect("failed to get item after removing tags");
                app_handle
                    .emit_all("item-tags-removed", ItemDetails::from_item(item))
                    .expect("Failed to emit event");
                rv
            } else {
                let rv = repo.batch_remove_tags(&ids, tags);
                let items: Result<Vec<_>, _> = ids
                    .iter()
                    .map(|id| {
                        Ok::<_, SearchError>(ItemDetails::from_item(repo.get_item_by_id(*id)?))
                    })
                    .collect();
                let items = items.expect("failed to get items after batch-removing tags");
                app_handle
                    .emit_all("batch-item-tags-removed", items)
                    .expect("Failed to emit event");
                rv
            }
        })
        .await
        .expect("failed to join with thread that's removing tags")?;
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
