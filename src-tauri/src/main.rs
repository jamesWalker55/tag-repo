// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use normpath::PathExt;

use rodio::{Decoder, OutputStream, PlayError, Sink, Source, StreamError};
use serde::{Serialize, Serializer};
use tauri::{AppHandle, Manager, PhysicalSize, Wry};
use thiserror::Error;
use tokio::sync::{Mutex, RwLock};
use tokio::time::sleep;
use tracing::{error, Level};
use tracing_subscriber::FmtSubscriber;
use window_shadows::{set_shadow, Error};

use crate::manager::{FileType, ItemDetails, ManagerStatus, RepoManager};
use crate::repo::{DirStructureError, QueryError, Repo, SearchError};
use crate::tree::FolderBuf;

mod diff;
mod helpers;
mod manager;
mod query;
mod repo;
mod scan;
#[cfg(test)]
mod tests;
mod tree;
pub(crate) mod watch;

#[derive(Error, Debug)]
enum CreateAudioOutputError {
    #[error("error when constructing output stream, {0}")]
    StreamError(#[from] StreamError),
    #[error("error when constructing output stream, {0}")]
    PlayError(#[from] PlayError),
}

fn get_output_stream_and_sink() -> Result<(OutputStream, Sink), CreateAudioOutputError> {
    let (stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;
    // lower the volume to prevent hearing damage
    sink.set_volume(0.5);
    Ok((stream, sink))
}

struct AppState {
    repo: Mutex<Option<Repo>>,
    manager: RwLock<Option<RepoManager<Wry>>>,
    // a wrapper around the audio stream? if this is dropped then audio will stop
    output_sink: Option<Sink>,
}

impl AppState {
    fn new(output_sink: Option<Sink>) -> Self {
        Self {
            repo: Mutex::new(None),
            manager: RwLock::new(None),
            output_sink,
        }
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
    state: tauri::State<'_, AppState>,
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
async fn close_repo(state: tauri::State<'_, AppState>) -> Result<(), ()> {
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
async fn get_item_details(
    state: tauri::State<'_, AppState>,
    id: i64,
) -> Result<ItemDetails, GetItemError> {
    let manager = state.manager.read().await;
    let Some(manager) = &*manager else {
        return Err(GetItemError::NoOpenRepo);
    };
    let item = manager.get_item_details(id).await?;
    Ok(item)
}

#[derive(Error, Debug)]
enum QueryItemIdsError {
    #[error("no active repo")]
    NoOpenRepo,
    #[error("failed to query items, {0}")]
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
enum GetFoldersError {
    #[error("no active repo")]
    NoOpenRepo,
    #[error("failed to query items, {0}")]
    DirStructureError(#[from] DirStructureError),
}

impl_serialize_to_string!(GetFoldersError);

#[tauri::command]
async fn get_dir_structure(
    state: tauri::State<'_, AppState>,
) -> Result<FolderBuf, GetFoldersError> {
    let manager = state.manager.read().await;
    let Some(manager) = &*manager else {
        return Err(GetFoldersError::NoOpenRepo);
    };
    let folders = manager.get_dir_structure().await?;
    Ok(folders)
}

#[derive(Error, Debug)]
enum InsertTagsError {
    #[error("no active repo")]
    NoOpenRepo,
    #[error("failed to insert tags, {0}")]
    InsertTagsError(#[from] repo::InsertTagsError),
}

impl_serialize_to_string!(InsertTagsError);

#[tauri::command]
async fn insert_tags(
    state: tauri::State<'_, AppState>,
    ids: Vec<i64>,
    tags: String,
) -> Result<(), InsertTagsError> {
    let manager = state.manager.read().await;
    let Some(manager) = &*manager else {
        return Err(InsertTagsError::NoOpenRepo);
    };
    let tags: Vec<_> = tags.split_whitespace().map(|x| x.to_string()).collect();
    if !tags.is_empty() {
        manager.insert_tags(ids, tags).await?;
    }
    Ok(())
}

#[derive(Error, Debug)]
enum RemoveTagsError {
    #[error("no active repo")]
    NoOpenRepo,
    #[error("failed to remove tags, {0}")]
    RemoveTagsError(#[from] repo::RemoveTagsError),
}

impl_serialize_to_string!(RemoveTagsError);

#[tauri::command]
async fn remove_tags(
    state: tauri::State<'_, AppState>,
    ids: Vec<i64>,
    tags: String,
) -> Result<(), RemoveTagsError> {
    let manager = state.manager.read().await;
    let Some(manager) = &*manager else {
        return Err(RemoveTagsError::NoOpenRepo);
    };
    let tags: Vec<_> = tags.split_whitespace().map(|x| x.to_string()).collect();
    if !tags.is_empty() {
        manager.remove_tags(ids, tags).await?;
    }
    Ok(())
}

#[derive(Error, Debug)]
enum RevealFileError {
    #[error("support for your operating system has not been implemented yet")]
    OperatingSystemNotSupported,
    #[error("failed to reveal file, {0}")]
    IOError(#[from] std::io::Error),
    #[error("malformed path, {0}")]
    MalformedPath(PathBuf),
}

impl_serialize_to_string!(RevealFileError);

// for all target_os options, see:
// https://doc.rust-lang.org/reference/conditional-compilation.html#target_os
#[cfg(target_os = "windows")]
#[tauri::command]
fn reveal_file(path: String) -> Result<(), RevealFileError> {
    let path: &Path = path.as_ref();
    // explorer can't find the file if you use forward slashes
    // normalise the path to remove forward slashes
    let path = path.normalize_virtually()?;
    let Some(path) = path.as_path().to_str() else {
            return Err(RevealFileError::MalformedPath(path.into_path_buf()));
        };
    Command::new("explorer").args(["/select,", path]).spawn()?;
    Ok(())
}

#[cfg(target_os = "macos")]
#[tauri::command]
fn reveal_file(path: String) -> Result<(), RevealFileError> {
    let path: &Path = path.as_ref();
    let path = path.normalize()?;
    let Some(path) = path.as_path().to_str() else {
        return Err(RevealFileError::MalformedPath(path.into_path_buf()));
    };
    Command::new("open").args(["-R", path]).spawn()?;
    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
#[tauri::command]
fn reveal_file(path: String) -> Result<(), RevealFileError> {
    let path: &Path = path.as_ref();
    return Err(RevealFileError::OperatingSystemNotSupported);
}

#[derive(Error, Debug)]
enum OpenFileError {
    #[error("failed to reveal file")]
    IOError(#[from] std::io::Error),
}

impl_serialize_to_string!(OpenFileError);

#[tauri::command]
fn launch_file(path: String) -> Result<(), OpenFileError> {
    open::that(path)?;
    Ok(())
}

#[tauri::command]
fn launch_manual() -> Result<(), OpenFileError> {
    open::that("https://jameswalker55.github.io/tag-repo-site/")
        .unwrap_or_else(|err| error!("failed to open browser to manual, {:?}", err));
    Ok(())
}

#[tauri::command]
fn determine_filetype(path: String) -> FileType {
    use crate::manager::determine_filetype;

    determine_filetype(path)
}

#[tauri::command]
fn supports_audio_playback(state: tauri::State<'_, AppState>) -> bool {
    state.output_sink.is_some()
}

#[derive(Error, Debug)]
enum PreviewAudioError {
    #[error("no audio device available")]
    NoOutputStream,
    #[error("failed to open file, {0}")]
    IOError(#[from] std::io::Error),
    #[error("failed to decode file, {0}")]
    DecodeError(#[from] rodio::decoder::DecoderError),
}

impl_serialize_to_string!(PreviewAudioError);

fn load_music(path: impl AsRef<Path>) -> Result<Decoder<BufReader<File>>, PreviewAudioError> {
    let path = path.as_ref();

    let file = BufReader::new(File::open(&path)?);
    let source = Decoder::new(file)?;
    Ok(source)
}

#[tauri::command]
fn preview_audio(
    state: tauri::State<'_, AppState>,
    path: String,
    skip_milliseconds: u64,
) -> Result<(), PreviewAudioError> {
    let Some(sink) = &state.output_sink else {
        return Err(PreviewAudioError::NoOutputStream)
    };
    // stop all current audio without pausing
    sink.stop();
    // try to load new audio
    match load_music(path) {
        Ok(music) => {
            if skip_milliseconds != 0 {
                sink.append(music.skip_duration(Duration::from_millis(skip_milliseconds)));
            } else {
                sink.append(music);
            }
            // ensure sink isn't paused
            sink.play();
            Ok(())
        }
        Err(err) => {
            error!("failed to preview audio, {}", &err);
            Err(err)
        }
    }
}

#[tauri::command]
fn stop_audio(state: tauri::State<'_, AppState>) -> Result<(), PreviewAudioError> {
    let Some(sink) = &state.output_sink else {
        return Err(PreviewAudioError::NoOutputStream)
    };
    // stop all current audio without pausing
    sink.stop();
    Ok(())
}

#[tauri::command]
fn get_audio_volume(state: tauri::State<'_, AppState>) -> Result<f32, PreviewAudioError> {
    let Some(sink) = &state.output_sink else {
        return Err(PreviewAudioError::NoOutputStream)
    };
    Ok(sink.volume())
}

#[tauri::command]
fn set_audio_volume(
    state: tauri::State<'_, AppState>,
    volume: f32,
) -> Result<(), PreviewAudioError> {
    let Some(sink) = &state.output_sink else {
        return Err(PreviewAudioError::NoOutputStream)
    };
    // stop all current audio without pausing
    sink.set_volume(volume);
    Ok(())
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

    // "stream" is the output audio stream, if this is dropped then audio will stop
    let (_stream, sink) = match get_output_stream_and_sink() {
        Ok((stream, sink)) => (Some(stream), Some(sink)),
        Err(err) => {
            error!("failed to create audio output stream, {0}", err);
            (None, None)
        }
    };

    let app_state = AppState::new(sink);

    tauri::Builder::default()
        .manage(app_state)
        .setup(|app| {
            let window = app
                .get_window("main")
                .expect("failed to get window with name 'main'");
            window
                .set_min_size(Some(PhysicalSize { width: 400, height: 270 }))
                .expect("failed to set min size of window");
            match set_shadow(&window, true) {
                Ok(_) => {}
                Err(err) => {
                    error!("failed to set window shadows, unsupported system. {}", err);
                }
            }
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
            get_item_details,
            reveal_file,
            launch_file,
            determine_filetype,
            insert_tags,
            remove_tags,
            get_dir_structure,
            supports_audio_playback,
            preview_audio,
            stop_audio,
            get_audio_volume,
            set_audio_volume,
            launch_manual,
        ])
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    error!("main thread has dropped!");
}

#[cfg(test)]
mod testsa {
    use std::env::current_dir;
    use std::path::PathBuf;

    use path_slash::PathExt;

    use super::*;

    #[test]
    fn my_test() {
        let x = PathBuf::from("testrepo");
        let mut cwd = current_dir().unwrap();
        cwd.push("testrepo");
        dbg!(cwd.to_slash());
    }
}
