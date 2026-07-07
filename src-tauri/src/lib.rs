// Fase 1: pure ports. Not wired into Tauri commands yet (Fase 2/3).
// Allow dead code until the UI layers consume these modules.
#![allow(dead_code)]

pub mod config;
pub mod pet;
pub mod petdex;
pub mod sprite;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("error while running deskbuddy");
}
