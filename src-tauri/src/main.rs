// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::manager::{ManagerStatus, RepoManager};
use crate::repo::{Item, OpenError, QueryError, Repo, SearchError};
use serde::{Serialize, Serializer};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tauri::{AppHandle, Manager, Runtime, Wry};
use thiserror::Error;
use tokio::sync::{Mutex, RwLock};
use tokio::time::sleep;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use window_shadows::set_shadow;

mod diff;
mod helpers;
mod manager;
mod query;
mod repo;
mod scan;
#[cfg(test)]
mod tests;
pub(crate) mod watch;

struct AppState {
    repo: Mutex<Option<Repo>>,
    manager: RwLock<Option<RepoManager<Wry>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self { repo: Mutex::new(None), manager: RwLock::new(None) }
    }
}

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn temp() {
    println!("Sleeping 3 seconds...");
    sleep(Duration::from_secs(3)).await;
    println!("Woke up!");
}

#[tauri::command]
async fn current_path(state: tauri::State<'_, AppState>) -> Result<Option<PathBuf>, ()> {
    // async commands that use state MUST return a Result:
    // https://github.com/tauri-apps/tauri/issues/2533
    let opt = state.manager.read().await;
    match &*opt {
        Some(manager) => Ok(Some(manager.path().to_path_buf())),
        None => Ok(None),
    }
}

#[tauri::command]
async fn open_repo(
    mut state: tauri::State<'_, AppState>,
    app_handle: AppHandle<Wry>,
    path: &str,
) -> Result<(), String> {
    // discard the existing connection first
    {
        let mut opt = state.manager.write().await;
        *opt = None;
    }

    app_handle
        .emit_all("repo-path-changed", None::<PathBuf>)
        .expect("Failed to emit event");

    // then open the repo
    let manager = RepoManager::new(&path, app_handle.clone()).map_err(|x| x.to_string())?;

    // assign manager to state NOW, to let #current_status() check the manager's status
    {
        let mut opt = state.manager.write().await;
        *opt = Some(manager);
    }

    app_handle
        .emit_all("repo-path-changed", Some(PathBuf::from(path)))
        .expect("Failed to emit event");

    // now try to resync the manager
    let rv = {
        let manager = state.manager.read().await;
        let Some(manager) = &*manager else {
            return Err(String::from(
                "race condition occurred! manager was deleted between this and the previous lock"
            ));
        };
        manager.watch().await.unwrap();
        manager.resync().await.map_err(|x| x.to_string())
    };

    // if resyncing failed, discard the manager
    // otherwise, continue on
    match rv {
        Ok(_) => {
            // resync ok, emit event
            app_handle
                .emit_all("repo-resynced", Some(PathBuf::from(path)))
                .expect("Failed to emit event");
        }
        Err(err) => {
            // error occurred, discard the manager from the app state
            let mut opt = state.manager.write().await;
            app_handle
                .emit_all("repo-path-changed", None::<PathBuf>)
                .expect("Failed to emit event");
            *opt = None;
            return Err(err);
        }
    }

    Ok(())
}

#[tauri::command]
async fn close_repo(mut state: tauri::State<'_, AppState>) -> Result<(), ()> {
    let mut opt = state.manager.write().await;
    *opt = None;
    Ok(())
}

#[tauri::command]
async fn current_status(state: tauri::State<'_, AppState>) -> Result<Option<ManagerStatus>, ()> {
    let manager = state.manager.read().await;
    let Some(manager) = &*manager else {
        return Ok(None);
    };
    Ok(Some(manager.status().await))
}

macro_rules! impl_serialize_to_string {
    ($t:ty) => {
        impl Serialize for $t {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(self.to_string().as_str())
            }
        }
    };
}

#[derive(Error, Debug)]
enum GetItemError {
    #[error("no active repo")]
    NoOpenRepo,
    #[error("no item with given id found")]
    SearchError(#[from] SearchError),
}

impl_serialize_to_string!(GetItemError);

#[tauri::command]
async fn get_item(state: tauri::State<'_, AppState>, id: i64) -> Result<Item, GetItemError> {
    let manager = state.manager.read().await;
    let Some(manager) = &*manager else {
        return Err(GetItemError::NoOpenRepo);
    };
    let item = manager.get_item(id).await?;
    Ok(item)
}

#[derive(Error, Debug)]
enum QueryItemIdsError {
    #[error("no active repo")]
    NoOpenRepo,
    #[error("no item with given id found")]
    QueryError(#[from] QueryError),
}

impl_serialize_to_string!(QueryItemIdsError);

#[tauri::command]
async fn query_item_ids(
    state: tauri::State<'_, AppState>,
    query: String,
) -> Result<Vec<i64>, QueryItemIdsError> {
    let manager = state.manager.read().await;
    let Some(manager) = &*manager else {
        return Err(QueryItemIdsError::NoOpenRepo);
    };
    let item_ids = manager.query(query.as_str()).await?;
    Ok(item_ids)
}

#[derive(Error, Debug)]
enum RevealFileError {
    #[error("support for your operating system has not been implemented yet")]
    OperatingSystemNotSupported,
    #[error("failed to reveal file")]
    IOError(#[from] std::io::Error),
}

impl_serialize_to_string!(RevealFileError);

#[tauri::command]
fn reveal_file(path: String) -> Result<(), RevealFileError> {
    // for all target_os options, see:
    // https://doc.rust-lang.org/reference/conditional-compilation.html#target_os
    if cfg!(target_os = "windows") {
        Command::new("explorer")
            .args(["/select,", path.as_str()])
            .spawn()?;
    } else if cfg!(target_os = "macos") {
        Command::new("open").args(["-R", path.as_str()]).spawn()?;
    } else {
        return Err(RevealFileError::OperatingSystemNotSupported);
    };
    Ok(())
}

#[derive(Error, Debug)]
enum OpenFileError {
    #[error("failed to reveal file")]
    IOError(#[from] std::io::Error),
}

impl_serialize_to_string!(OpenFileError);

#[tauri::command]
fn open_file(path: String) -> Result<(), OpenFileError> {
    open::that(path)?;
    Ok(())
}

#[derive(Serialize)]
enum FileType {
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

#[derive(Error, Debug, Serialize)]
enum DetermineFileTypeError {
    #[error("malformed path")]
    InvalidPath,
}

#[tauri::command]
fn determine_filetype(path: String) -> Result<FileType, DetermineFileTypeError> {
    let path: &Path = path.as_ref();
    let Some(extension) = path.extension() else {
        return Ok(FileType::Unknown);
    };

    let extension = extension
        .to_str()
        .ok_or(DetermineFileTypeError::InvalidPath)?;

    if EXT_AUDIO.contains(&extension) {
        Ok(FileType::Audio)
    } else if EXT_DOCUMENT.contains(&extension) {
        Ok(FileType::Document)
    } else if EXT_IMAGE.contains(&extension) {
        Ok(FileType::Image)
    } else if EXT_VIDEO.contains(&extension) {
        Ok(FileType::Video)
    } else {
        Ok(FileType::Unknown)
    }
}

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::TRACE)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    tauri::Builder::default()
        .manage(AppState::default())
        .setup(|app| {
            let window = app.get_window("main").unwrap();
            set_shadow(&window, true).expect("Unsupported platform!");
            // app.listen_global("cool", |evt| {
            //     tokio::spawn(async move {
            //         println!("Sleeping a bit...");
            //         tokio::time::sleep(Duration::from_secs(2)).await;
            //         println!("Got payload: {:?}", evt.payload());
            //     });
            // });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            temp,
            current_path,
            open_repo,
            close_repo,
            current_status,
            query_item_ids,
            get_item,
            reveal_file,
            open_file,
            determine_filetype,
        ])
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod testsa {
    use super::*;
    use path_slash::PathExt;
    use std::env::current_dir;
    use std::path::PathBuf;

    #[test]
    fn my_test() {
        let x = PathBuf::from("testrepo");
        let mut cwd = current_dir().unwrap();
        cwd.push("testrepo");
        dbg!(cwd.to_slash());
    }
}
