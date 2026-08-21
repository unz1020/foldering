mod config;
mod review;
mod scanner;
mod transactions;
mod watcher;

use config::{load_workspace_config, save_workspace_config};
use review::{
    get_default_watch_locations, get_file_preview, get_review_queue, open_candidate_file,
};
use scanner::{scan_workspace, ScanReport};
use std::path::PathBuf;
use transactions::{list_recent_transactions, move_file_review, undo_transaction};
use watcher::{start_watcher, stop_watcher, WatcherState};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

#[tauri::command]
fn scan_folder(root_path: String) -> Result<ScanReport, String> {
    scan_workspace(PathBuf::from(root_path))
}

#[tauri::command]
fn get_autostart_enabled(app: tauri::AppHandle) -> Result<bool, String> {
    app.autolaunch()
        .is_enabled()
        .map_err(|error| format!("Could not read autostart state: {error}"))
}

#[tauri::command]
fn set_autostart_enabled(app: tauri::AppHandle, enabled: bool) -> Result<bool, String> {
    let manager = app.autolaunch();
    if enabled {
        manager
            .enable()
            .map_err(|error| format!("Could not enable autostart: {error}"))?;
    } else {
        manager
            .disable()
            .map_err(|error| format!("Could not disable autostart: {error}"))?;
    }
    manager
        .is_enabled()
        .map_err(|error| format!("Could not confirm autostart state: {error}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(WatcherState::default())
        .invoke_handler(tauri::generate_handler![
            scan_folder,
            load_workspace_config,
            save_workspace_config,
            get_default_watch_locations,
            get_review_queue,
            get_file_preview,
            open_candidate_file,
            move_file_review,
            undo_transaction,
            list_recent_transactions,
            start_watcher,
            stop_watcher,
            get_autostart_enabled,
            set_autostart_enabled,
        ])
        .run(tauri::generate_context!())
        .expect("error while running 5-Level");
}
