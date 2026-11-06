// A release build is a windowed app: no console window opens beside it.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bin;
mod clip_name;
mod db;
#[allow(dead_code)]
mod encode_args;
#[cfg(windows)]
#[allow(dead_code)]
mod encoder;
mod ffmpeg;
mod files;
mod jobs;
mod library;
mod media;
mod review;
mod settings;
#[allow(dead_code)]
mod verify;

use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_updater::UpdaterExt;

struct Db(Mutex<Connection>);

// The cache folder of the library's thumbnails.
struct Thumbnails(PathBuf);

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

#[tauri::command]
fn videos(db: State<Db>) -> Result<Vec<review::Video>, String> {
    review::videos(&db.0.lock().unwrap()).map_err(|e| e.to_string())
}

#[tauri::command]
fn video(db: State<Db>, key: String) -> Result<Option<review::VideoDetail>, String> {
    review::detail(&db.0.lock().unwrap(), &key).map_err(|e| e.to_string())
}

#[tauri::command]
fn clip_file_name(db: State<Db>, key: String, clip: review::ClipInput) -> Result<String, String> {
    review::clip_file_name(&db.0.lock().unwrap(), &key, &clip).map_err(|e| e.to_string())
}

#[tauri::command]
fn save(db: State<Db>, key: String, clip: review::ClipInput) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    review::save(&mut db.0.lock().unwrap(), &key, &clip, &now).map_err(|e| e.to_string())
}

// Moves the file to _Bin first, so a move that fails records no decision. A NAS copy can take a while.
#[tauri::command]
async fn discard(app: tauri::AppHandle, key: String) -> Result<(), String> {
    let work = tauri::async_runtime::spawn_blocking(move || {
        let db = app.state::<Db>();
        let root =
            settings::get(&db.0.lock().unwrap(), settings::LIBRARY).map_err(|e| e.to_string())?;
        let root = root.ok_or("No library folder chosen yet")?;
        bin::bin(&db.0, Path::new(&root), &key).map_err(|e| e.to_string())?;

        let now = chrono::Utc::now().to_rfc3339();
        review::discard(&db.0.lock().unwrap(), &key, &now).map_err(|e| e.to_string())
    });

    work.await.map_err(|e| e.to_string())?
}

#[tauri::command]
fn queue(db: State<Db>) -> Result<jobs::Queue, String> {
    jobs::queue(&db.0.lock().unwrap()).map_err(|e| e.to_string())
}

#[tauri::command]
fn reorder_jobs(db: State<Db>, ids: Vec<i64>) -> Result<(), String> {
    jobs::reorder(&mut db.0.lock().unwrap(), &ids).map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_job(db: State<Db>, id: i64) -> Result<(), String> {
    jobs::remove(&db.0.lock().unwrap(), id).map_err(|e| e.to_string())
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

        library::scan(&db.0, Path::new(&root), ffmpeg::probe).map_err(|e| e.to_string())
    });

    work.await.map_err(|e| e.to_string())?
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        // Library videos and their thumbnails for the page, read on a thread meant for blocking work.
        .register_asynchronous_uri_scheme_protocol("media", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            tauri::async_runtime::spawn_blocking(move || {
                let db = app.state::<Db>();
                let root = settings::get(&db.0.lock().unwrap(), settings::LIBRARY);
                let root = root.ok().flatten();

                let cache = &app.state::<Thumbnails>().0;
                let root = root.as_deref().map(Path::new);
                responder.respond(media::respond(root, cache, &request, ffmpeg::frame));
            });
        })
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let conn = db::open(&dir.join("shadow-plane.db"))?;
            app.manage(Db(Mutex::new(conn)));
            app.manage(Scanning(Mutex::new(())));
            app.manage(Thumbnails(app.path().app_cache_dir()?.join("thumbnails")));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            update,
            library,
            choose_library,
            scan,
            videos,
            video,
            clip_file_name,
            save,
            discard,
            queue,
            reorder_jobs,
            remove_job
        ])
        .run(tauri::generate_context!())
        .expect("Shadow Plane failed to start");
}
