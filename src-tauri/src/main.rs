// A release build is a windowed app: no console window opens beside it.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("Shadow Plane failed to start");
}
