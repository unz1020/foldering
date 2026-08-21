use serde::Serialize;
use std::{
    cmp::Ordering,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};

const MAX_DEPTH: usize = 5;
const MAX_CANDIDATES: usize = 500;
const MAX_RECOMMENDATIONS: usize = 3;
const ARCHIVE_NAME: &str = "99_ARCHIVE";
const PARTIAL_EXTENSIONS: &[&str] = &["crdownload", "part", "download", "tmp"];

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchLocations {
    pub downloads: Option<String>,
    pub desktop: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderRecommendation {
    pub path: String,
    pub relative_path: String,
    pub depth: usize,
    pub score: f64,
    pub managed: bool,
    pub matched_labels: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileCandidate {
    pub path: String,
    pub file_name: String,
    pub extension: String,
    pub size_bytes: u64,
    pub modified_at_ms: u128,
    pub source_label: String,
    pub recommendations: Vec<FolderRecommendation>,
}

#[derive(Clone)]
struct Destination {
    path: PathBuf,
    relative_path: String,
    depth: usize,
    managed: bool,
    labels: Vec<String>,
}

#[tauri::command]
pub fn get_default_watch_locations(app: AppHandle) -> WatchLocations {
    WatchLocations {
        downloads: app
            .path()
            .download_dir()
            .ok()
            .map(|path| path.to_string_lossy().into_owned()),
        desktop: app
            .path()
            .desktop_dir()
            .ok()
            .map(|path| path.to_string_lossy().into_owned()),
    }
}

#[tauri::command]
pub fn get_review_queue(
    workspace_root: String,
    watch_locations: Vec<String>,
) -> Result<Vec<FileCandidate>, String> {
    let root = fs::canonicalize(&workspace_root)
        .map_err(|error| format!("Could not open Workspace: {error}"))?;
    if !root.is_dir() {
        return Err("Workspace path is not a directory.".into());
    }

    let destinations = collect_destinations(&root);
    let mut candidates = Vec::new();

    for location in watch_locations {
        let location_path = PathBuf::from(&location);
        if !location_path.is_dir() {
            continue;
        }

        let source_label = location_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("Source")
            .to_string();

        let entries = match fs::read_dir(&location_path) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry_result in entries {
            let entry = match entry_result {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_) => continue,
            };
            if !file_type.is_file() || file_type.is_symlink() {
                continue;
            }

            let file_name = entry.file_name().to_string_lossy().into_owned();
            if should_ignore_file(&file_name) {
                continue;
            }

            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };

            let modified_at_ms = metadata
                .modified()
                .ok()
                .and_then(system_time_to_ms)
                .unwrap_or_default();
            let extension = entry
                .path()
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("")
                .to_string();

            let recommendations = recommend_destinations(&file_name, &destinations);
            candidates.push(FileCandidate {
                path: entry.path().to_string_lossy().into_owned(),
                file_name,
                extension,
                size_bytes: metadata.len(),
                modified_at_ms,
                source_label: source_label.clone(),
                recommendations,
            });
        }
    }

    candidates.sort_by(|a, b| b.modified_at_ms.cmp(&a.modified_at_ms));
    candidates.truncate(MAX_CANDIDATES);
    Ok(candidates)
}

fn collect_destinations(root: &Path) -> Vec<Destination> {
    let mut result = Vec::new();
    let mut labels = Vec::new();
    walk_destinations(root, root, 0, true, false, &mut labels, &mut result);
    result
}

fn walk_destinations(
    root: &Path,
    path: &Path,
    depth: usize,
    parent_managed: bool,
    inside_archive: bool,
    labels: &mut Vec<String>,
    result: &mut Vec<Destination>,
) {
    if depth >= MAX_DEPTH || inside_archive {
        return;
    }

    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry_result in entries {
        let entry = match entry_result {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(_) => continue,
        };
        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().into_owned();
        let child_depth = depth + 1;
        let is_archive = name == ARCHIVE_NAME;
        if is_archive {
            // Archive is never a normal filing recommendation.
            continue;
        }

        let (component_managed, label) = managed_label(&name);
        let managed = parent_managed && component_managed;
        labels.push(label);

        let relative_path = entry
            .path()
            .strip_prefix(root)
            .ok()
            .map(path_for_display)
            .unwrap_or_else(|| name.clone());

        result.push(Destination {
            path: entry.path(),
            relative_path,
            depth: child_depth,
            managed,
            labels: labels.clone(),
        });

        walk_destinations(
            root,
            &entry.path(),
            child_depth,
            managed,
            false,
            labels,
            result,
        );
        labels.pop();
    }
}

fn recommend_destinations(file_name: &str, destinations: &[Destination]) -> Vec<FolderRecommendation> {
    let normalized_file = compact(file_name.trim_end_matches(|c: char| c == '.'));
    let mut scored: Vec<FolderRecommendation> = destinations
        .iter()
        .filter_map(|destination| {
            let mut score = 0.0;
            let mut matched_labels = Vec::new();

            for (index, label) in destination.labels.iter().enumerate() {
                let normalized_label = compact(label);
                if normalized_label.chars().count() < 2 {
                    continue;
                }
                if normalized_file.contains(&normalized_label) {
                    let level_weight = match index {
                        0 => 0.34,
                        1 => 0.20,
                        2 => 0.18,
                        3 => 0.16,
                        _ => 0.12,
                    };
                    score += level_weight;
                    matched_labels.push(label.clone());
                }
            }

            if matched_labels.is_empty() {
                return None;
            }

            let destination_label = destination.labels.last().map(|value| compact(value));
            if !destination_label
                .as_deref()
                .map(|label| normalized_file.contains(label))
                .unwrap_or(false)
            {
                return None;
            }

            if matched_labels.len() >= 2 { score += 0.08; }
            if matched_labels.len() >= 3 { score += 0.06; }
            if destination.managed { score += 0.02; }
            score = f64::min(score, 0.99);

            Some(FolderRecommendation {
                path: destination.path.to_string_lossy().into_owned(),
                relative_path: destination.relative_path.clone(),
                depth: destination.depth,
                score,
                managed: destination.managed,
                matched_labels,
            })
        })
        .collect();

    scored.sort_by(|a, b| {
        b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal).then_with(|| b.depth.cmp(&a.depth))
    });
    scored.truncate(MAX_RECOMMENDATIONS);
    scored
}

fn managed_label(name: &str) -> (bool, String) {
    if name == ARCHIVE_NAME { return (true, "ARCHIVE".into()); }
    let bytes = name.as_bytes();
    if bytes.len() < 4 || !bytes[0].is_ascii_digit() || !bytes[1].is_ascii_digit() || bytes[2] != b'_' {
        return (false, name.to_string());
    }
    let number = ((bytes[0] - b'0') * 10 + (bytes[1] - b'0')) as u8;
    let label = name[3..].trim();
    if !(1..=98).contains(&number) || label.is_empty() { return (false, label.to_string()); }
    (true, label.to_string())
}

fn compact(value: &str) -> String {
    value.chars().filter(|character| character.is_alphanumeric()).flat_map(|character| character.to_lowercase()).collect()
}

fn should_ignore_file(file_name: &str) -> bool {
    if file_name.starts_with('.') || file_name.starts_with("~$") { return true; }
    let extension = Path::new(file_name).extension().and_then(|value| value.to_str()).unwrap_or("").to_ascii_lowercase();
    PARTIAL_EXTENSIONS.contains(&extension.as_str())
}

fn path_for_display(path: &Path) -> String {
    path.components().map(|component| component.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/")
}

fn system_time_to_ms(time: SystemTime) -> Option<u128> {
    time.duration_since(UNIX_EPOCH).ok().map(|duration| duration.as_millis())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strips_numbering_for_matching() {
        assert_eq!(managed_label("03_Channel"), (true, "Channel".into()));
        assert_eq!(managed_label("Channel"), (false, "Channel".into()));
    }
    #[test]
    fn ignores_partial_downloads() {
        assert!(should_ignore_file("report.pdf.crdownload"));
        assert!(should_ignore_file("~$report.xlsx"));
        assert!(!should_ignore_file("report.pdf"));
    }
}
