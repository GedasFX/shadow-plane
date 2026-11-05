// A release build is a windowed app: no console window opens beside it.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;

use tauri::Manager;
use tauri_plugin_updater::UpdaterExt;

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

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            db::open(&dir.join("shadow-plane.db"))?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![update])
        .run(tauri::generate_context!())
        .expect("Shadow Plane failed to start");
}
