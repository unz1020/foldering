use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};

static TRANSACTION_COUNTER: AtomicU64 = AtomicU64::new(1);
const MAX_DEPTH: usize = 5;
const RECENT_LIMIT: usize = 100;
const ARCHIVE_NAME: &str = "99_ARCHIVE";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileTransaction {
    pub id: String,
    pub kind: String,
    pub source: String,
    pub destination: String,
    pub created_at_ms: u128,
    pub original_transaction_id: Option<String>,
}

#[tauri::command]
pub fn move_file_review(
    app: AppHandle,
    workspace_root: String,
    watch_locations: Vec<String>,
    source_path: String,
    destination_dir: String,
) -> Result<FileTransaction, String> {
    let root = fs::canonicalize(&workspace_root)
        .map_err(|error| format!("Could not resolve Workspace: {error}"))?;
    let destination_input = PathBuf::from(&destination_dir);
    let destination_metadata = fs::symlink_metadata(&destination_input)
        .map_err(|error| format!("Could not read destination folder: {error}"))?;
    if destination_metadata.file_type().is_symlink() {
        return Err("Symlink destinations are not allowed in Review Mode.".into());
    }
    let destination = fs::canonicalize(&destination_input)
        .map_err(|error| format!("Could not resolve destination folder: {error}"))?;

    if !destination.is_dir() || !destination.starts_with(&root) {
        return Err("Destination must be an existing folder inside the Workspace.".into());
    }

    let depth = destination
        .strip_prefix(&root)
        .map_err(|_| "Destination is outside the Workspace.".to_string())?
        .components()
        .count();
    if depth == 0 || depth > MAX_DEPTH {
        return Err("Destination must be between Level 1 and Level 5.".into());
    }
    if destination
        .strip_prefix(&root)
        .ok()
        .map(|relative| relative.components().any(|component| component.as_os_str().to_string_lossy() == ARCHIVE_NAME))
        .unwrap_or(false)
    {
        return Err("Archive destinations are disabled in V0.2.1 Review Mode.".into());
    }

    let source = PathBuf::from(&source_path);
    let source_metadata = fs::symlink_metadata(&source)
        .map_err(|error| format!("Could not read source file: {error}"))?;
    if !source_metadata.file_type().is_file() || source_metadata.file_type().is_symlink() {
        return Err("Only regular files can be moved in Review Mode.".into());
    }

    let canonical_source = fs::canonicalize(&source)
        .map_err(|error| format!("Could not resolve source file: {error}"))?;
    let source_parent = canonical_source
        .parent()
        .ok_or_else(|| "Source file has no parent folder.".to_string())?;

    let allowed = watch_locations.iter().any(|location| {
        fs::canonicalize(location)
            .ok()
            .map(|allowed_path| allowed_path == source_parent)
            .unwrap_or(false)
    });
    if !allowed {
        return Err("Source must be a top-level file in an enabled watch location.".into());
    }

    let file_name = canonical_source
        .file_name()
        .ok_or_else(|| "Source file has no file name.".to_string())?;
    let target = destination.join(file_name);
    if target.exists() {
        return Err("A file with the same name already exists in the destination. Nothing was overwritten.".into());
    }

    let mut log = open_transaction_log(&app)?;
    safe_move(&canonical_source, &target)?;

    let transaction = FileTransaction {
        id: new_transaction_id(),
        kind: "MOVE".into(),
        source: canonical_source.to_string_lossy().into_owned(),
        destination: target.to_string_lossy().into_owned(),
        created_at_ms: now_ms(),
        original_transaction_id: None,
    };

    if let Err(log_error) = write_transaction(&mut log, &transaction) {
        return match safe_move(&target, &canonical_source) {
            Ok(()) => Err(format!("Transaction log failed; file move was rolled back: {log_error}")),
            Err(rollback_error) => Err(format!(
                "Transaction log failed and automatic rollback also failed. File is at {}. Log error: {log_error}; rollback error: {rollback_error}",
                target.display()
            )),
        };
    }
    Ok(transaction)
}

#[tauri::command]
pub fn undo_transaction(app: AppHandle, transaction_id: String) -> Result<FileTransaction, String> {
    let transactions = read_transactions(&app)?;
    let original = transactions
        .iter()
        .find(|transaction| transaction.id == transaction_id && transaction.kind == "MOVE")
        .cloned()
        .ok_or_else(|| "Move transaction was not found.".to_string())?;

    if transactions.iter().any(|transaction| {
        transaction.kind == "UNDO"
            && transaction.original_transaction_id.as_deref() == Some(original.id.as_str())
    }) {
        return Err("This move has already been undone.".into());
    }

    let current = PathBuf::from(&original.destination);
    let restore = PathBuf::from(&original.source);
    if !current.is_file() {
        return Err("The moved file no longer exists at its recorded destination.".into());
    }
    if restore.exists() {
        return Err("Undo was blocked because the original path is already occupied.".into());
    }
    let restore_parent = restore
        .parent()
        .ok_or_else(|| "Original path has no parent folder.".to_string())?;
    if !restore_parent.is_dir() {
        return Err("The original folder no longer exists.".into());
    }

    let mut log = open_transaction_log(&app)?;
    safe_move(&current, &restore)?;

    let undo = FileTransaction {
        id: new_transaction_id(),
        kind: "UNDO".into(),
        source: current.to_string_lossy().into_owned(),
        destination: restore.to_string_lossy().into_owned(),
        created_at_ms: now_ms(),
        original_transaction_id: Some(original.id),
    };

    if let Err(log_error) = write_transaction(&mut log, &undo) {
        return match safe_move(&restore, &current) {
            Ok(()) => Err(format!("Undo log failed; Undo was rolled back: {log_error}")),
            Err(rollback_error) => Err(format!(
                "Undo log failed and automatic rollback also failed. File is at {}. Log error: {log_error}; rollback error: {rollback_error}",
                restore.display()
            )),
        };
    }
    Ok(undo)
}

#[tauri::command]
pub fn list_recent_transactions(app: AppHandle) -> Result<Vec<FileTransaction>, String> {
    let mut transactions = read_transactions(&app)?;
    transactions.reverse();
    transactions.truncate(RECENT_LIMIT);
    Ok(transactions)
}

fn safe_move(source: &Path, destination: &Path) -> Result<(), String> {
    let metadata = fs::metadata(source)
        .map_err(|error| format!("Could not read source metadata: {error}"))?;
    let mut source_file = fs::File::open(source)
        .map_err(|error| format!("Could not open source file: {error}"))?;
    let mut destination_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|error| format!("Could not create destination without overwrite: {error}"))?;

    let copied = match io::copy(&mut source_file, &mut destination_file) {
        Ok(copied) => copied,
        Err(error) => {
            drop(destination_file);
            let _ = fs::remove_file(destination);
            return Err(format!("Could not copy file: {error}"));
        }
    };

    if copied != metadata.len() {
        drop(destination_file);
        let _ = fs::remove_file(destination);
        return Err("Copy verification failed; incomplete destination was removed.".into());
    }

    if let Err(error) = destination_file.sync_all() {
        drop(destination_file);
        let _ = fs::remove_file(destination);
        return Err(format!("Could not flush destination file; destination was removed: {error}"));
    }
    drop(destination_file);
    drop(source_file);

    let _ = fs::set_permissions(destination, metadata.permissions());

    if let Err(remove_error) = fs::remove_file(source) {
        let rollback = fs::remove_file(destination);
        return match rollback {
            Ok(()) => Err(format!("Could not remove source after verified copy; destination copy was rolled back: {remove_error}")),
            Err(rollback_error) => Err(format!(
                "Could not remove source and could not remove destination rollback copy. Source was preserved. remove error: {remove_error}; rollback error: {rollback_error}"
            )),
        };
    }

    Ok(())
}

fn transactions_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not resolve app data directory: {error}"))?;
    fs::create_dir_all(&dir)
        .map_err(|error| format!("Could not create app data directory: {error}"))?;
    Ok(dir.join("transactions.jsonl"))
}

fn open_transaction_log(app: &AppHandle) -> Result<fs::File, String> {
    let path = transactions_path(app)?;
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("Could not open transaction log: {error}"))
}

fn write_transaction(file: &mut fs::File, transaction: &FileTransaction) -> Result<(), String> {
    let json = serde_json::to_string(transaction)
        .map_err(|error| format!("Could not serialize transaction: {error}"))?;
    writeln!(file, "{json}")
        .and_then(|_| file.flush())
        .map_err(|error| format!("Could not write transaction log: {error}"))
}

fn read_transactions(app: &AppHandle) -> Result<Vec<FileTransaction>, String> {
    let path = transactions_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = fs::File::open(path).map_err(|error| format!("Could not read transaction log: {error}"))?;
    let reader = BufReader::new(file);
    let mut result = Vec::new();

    for line in reader.lines() {
        let line = line.map_err(|error| format!("Could not read transaction line: {error}"))?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(transaction) = serde_json::from_str::<FileTransaction>(&line) {
            result.push(transaction);
        }
    }
    Ok(result)
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn new_transaction_id() -> String {
    let counter = TRANSACTION_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}-{}-{}", now_ms(), std::process::id(), counter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_move_moves_a_regular_file_without_overwrite_logic() {
        let base = std::env::temp_dir().join(format!("fivelevel-test-{}", new_transaction_id()));
        fs::create_dir_all(&base).unwrap();
        let source = base.join("source.txt");
        let destination = base.join("destination.txt");
        fs::write(&source, b"hello 5-level").unwrap();

        safe_move(&source, &destination).unwrap();

        assert!(!source.exists());
        assert_eq!(fs::read(&destination).unwrap(), b"hello 5-level");
        let _ = fs::remove_dir_all(base);
    }
}
