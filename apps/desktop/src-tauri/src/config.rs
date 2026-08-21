use serde_json::Value;
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not resolve app data directory: {error}"))?;
    fs::create_dir_all(&dir)
        .map_err(|error| format!("Could not create app data directory: {error}"))?;
    Ok(dir.join("workspace.json"))
}

#[tauri::command]
pub fn load_workspace_config(app: AppHandle) -> Result<Option<Value>, String> {
    let path = config_path(&app)?;
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&path)
        .map_err(|error| format!("Could not read workspace config: {error}"))?;
    let value = serde_json::from_str(&content)
        .map_err(|error| format!("Workspace config is invalid JSON: {error}"))?;
    Ok(Some(value))
}

#[tauri::command]
pub fn save_workspace_config(app: AppHandle, config: Value) -> Result<(), String> {
    let path = config_path(&app)?;
    let content = serde_json::to_string_pretty(&config)
        .map_err(|error| format!("Could not serialize workspace config: {error}"))?;
    fs::write(path, content).map_err(|error| format!("Could not save workspace config: {error}"))
}
