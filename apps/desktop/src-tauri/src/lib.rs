mod config;
mod review;
mod scanner;
mod transactions;
mod watcher;

use config::{load_workspace_config, save_workspace_config};
use review::{get_default_watch_locations, get_review_queue};
use scanner::{scan_workspace, ScanReport};
use std::path::PathBuf;
use transactions::{list_recent_transactions, move_file_review, undo_transaction};
use watcher::{start_watcher, stop_watcher, WatcherState};

#[tauri::command]
fn scan_folder(root_path: String) -> Result<ScanReport, String> {
    scan_workspace(PathBuf::from(root_path))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(WatcherState::default())
        .invoke_handler(tauri::generate_handler![
            scan_folder,
            load_workspace_config,
            save_workspace_config,
            get_default_watch_locations,
            get_review_queue,
            move_file_review,
            undo_transaction,
            list_recent_transactions,
            start_watcher,
            stop_watcher,
        ])
        .run(tauri::generate_context!())
        .expect("error while running 5-Level");
}
