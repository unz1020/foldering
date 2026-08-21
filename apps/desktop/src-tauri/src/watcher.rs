use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use std::{path::Path, sync::Mutex};
use tauri::{AppHandle, Emitter, State};

#[derive(Default)]
pub struct WatcherState { watcher: Mutex<Option<RecommendedWatcher>> }

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WatchChangedPayload { paths: Vec<String> }

#[tauri::command]
pub fn start_watcher(app: AppHandle, state: State<'_, WatcherState>, watch_locations: Vec<String>) -> Result<usize, String> {
    let app_for_events = app.clone();
    let mut watcher = notify::recommended_watcher(move |result: notify::Result<Event>| {
        let Ok(event) = result else { return; };
        if matches!(event.kind, EventKind::Access(_)) { return; }
        let paths = event.paths.iter().map(|path| path.to_string_lossy().into_owned()).collect::<Vec<_>>();
        if paths.is_empty() { return; }
        let _ = app_for_events.emit("watch-files-changed", WatchChangedPayload { paths });
    }).map_err(|error| format!("Could not create file watcher: {error}"))?;
    let mut watched = 0usize;
    for location in watch_locations {
        let path = Path::new(&location);
        if !path.is_dir() { continue; }
        watcher.watch(path, RecursiveMode::NonRecursive).map_err(|error| format!("Could not watch {}: {error}", path.display()))?;
        watched += 1;
    }
    let mut guard = state.watcher.lock().map_err(|_| "File watcher state is unavailable.".to_string())?;
    *guard = Some(watcher);
    Ok(watched)
}

#[tauri::command]
pub fn stop_watcher(state: State<'_, WatcherState>) -> Result<(), String> {
    let mut guard = state.watcher.lock().map_err(|_| "File watcher state is unavailable.".to_string())?;
    *guard = None;
    Ok(())
}
