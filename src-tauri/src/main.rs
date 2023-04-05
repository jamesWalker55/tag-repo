// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::manager::{ManagerStatus, RepoManager};
use crate::repo::{Item, OpenError, Repo};
use std::path::PathBuf;
use std::time::Duration;
use tauri::Manager;
use tokio::sync::{Mutex, RwLock};
use tokio::time::sleep;
use tracing::Level;
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
    manager: RwLock<Option<RepoManager>>,
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
fn testrepo(path: &str) -> Result<(), String> {
    let repo = Repo::open(path).map_err(|err| "failed to open repo!")?;
    Ok(())
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
async fn open_path(mut state: tauri::State<'_, AppState>, path: &str) -> Result<Vec<Item>, String> {
    // discard the existing connection first
    {
        let mut opt = state.manager.write().await;
        *opt = None;
    }

    // then open the repo
    let manager = RepoManager::new(&path).map_err(|x| x.to_string())?;
    {
        let mut opt = state.manager.write().await;
        *opt = Some(manager);
    }
    let manager = state.manager.read().await;
    let Some(manager) = &*manager else {
        panic!("race condition occurred! attempted to resync after loading a new manager");
    };
    manager.resync().await.map_err(|x| x.to_string())?;
    // let items = repo.query_items("").expect("failed to query");

    Ok(vec![])
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
            testrepo,
            current_path,
            open_path,
            close_repo,
            current_status,
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
