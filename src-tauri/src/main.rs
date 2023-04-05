// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::repo::{Item, OpenError, Repo};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, RwLock};
use std::thread::sleep;
use std::time::Duration;
use tauri::Manager;
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
}

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn testrepo(path: &str) -> Result<(), String> {
    let repo = Repo::open(path).map_err(|err| "failed to open repo!")?;
    Ok(())
}

#[tauri::command]
fn current_repo_path(state: tauri::State<AppState>) -> Option<PathBuf> {
    let opt = state
        .repo
        .lock()
        .expect("failed to lock repo when trying to get repo path");
    match &*opt {
        Some(repo) => Some(repo.path().to_path_buf()),
        None => None,
    }
}

#[tauri::command]
fn open_repo(mut state: tauri::State<AppState>, path: &str) -> Result<Vec<Item>, String> {
    // lock the state
    let mut opt = state
        .repo
        .lock()
        .expect("failed to lock repo when trying to set repo path");

    // discard the existing connection first
    *opt = None;

    // then open the repo
    let mut repo = Repo::open(&path).map_err(|x| "Open error".to_string())?;
    repo.sync_all().expect("Failed to sync!");
    let items = repo.query_items("").expect("failed to query");
    *opt = Some(repo);

    Ok(items)
}

fn main() {
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::TRACE)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    tauri::Builder::default()
        .manage(AppState { repo: Mutex::new(None) })
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
            testrepo,
            current_repo_path,
            open_repo,
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
