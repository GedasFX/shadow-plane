// A release build is a windowed app: no console window opens beside it.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;
mod ffprobe;
mod library;
mod settings;

use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_updater::UpdaterExt;

struct Db(Mutex<Connection>);

// Held through a scan: a second scan waits, then works from the first one's index and the latest folder.
struct Scanning(Mutex<()>);

// On Windows the installer closes the app and starts the new version, so this returns only when
// there is nothing to install; restart() is for the other platforms.
#[tauri::command]
async fn update(app: tauri::AppHandle) -> Result<(), String> {
    let found = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;
    let Some(next) = found else {
        return Ok(());
    };
    next.download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| e.to_string())?;
    app.restart()
}

#[tauri::command]
fn library(db: State<Db>) -> Result<Option<String>, String> {
    settings::get(&db.0.lock().unwrap(), settings::LIBRARY).map_err(|e| e.to_string())
}

// Async, so the dialog blocks a worker thread rather than the window's.
#[tauri::command]
async fn choose_library(
    app: tauri::AppHandle,
    db: State<'_, Db>,
) -> Result<Option<String>, String> {
    let Some(picked) = app.dialog().file().blocking_pick_folder() else {
        return Ok(None);
    };
    let path = picked.into_path().map_err(|e| e.to_string())?;
    let path = path.to_string_lossy().into_owned();
    settings::set(&db.0.lock().unwrap(), settings::LIBRARY, &path).map_err(|e| e.to_string())?;
    Ok(Some(path))
}

// The scan waits on ffprobe for seconds, so it runs on a thread meant for blocking work.
#[tauri::command]
async fn scan(app: tauri::AppHandle) -> Result<library::Scan, String> {
    let work = tauri::async_runtime::spawn_blocking(move || {
        let scanning = app.state::<Scanning>();
        let _one_at_a_time = scanning.0.lock().unwrap();
        let db = app.state::<Db>();
        let root =
            settings::get(&db.0.lock().unwrap(), settings::LIBRARY).map_err(|e| e.to_string())?;
        let root = root.ok_or("No library folder chosen yet")?;

        library::scan(&db.0, Path::new(&root), ffprobe::probe).map_err(|e| e.to_string())
    });

    work.await.map_err(|e| e.to_string())?
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let conn = db::open(&dir.join("shadow-plane.db"))?;
            app.manage(Db(Mutex::new(conn)));
            app.manage(Scanning(Mutex::new(())));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            update,
            library,
            choose_library,
            scan
        ])
        .run(tauri::generate_context!())
        .expect("Shadow Plane failed to start");
}
