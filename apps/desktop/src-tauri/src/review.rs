use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Serialize;
use std::{
    cmp::Ordering,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

const MAX_DEPTH: usize = 5;
const MAX_CANDIDATES: usize = 500;
const MAX_RECOMMENDATIONS: usize = 3;
const MIN_STRUCTURED_SCORE: f64 = 0.78;
const MIN_GENERIC_SCORE: f64 = 0.62;
const MIN_STRUCTURED_MARGIN: f64 = 0.025;
const MIN_GENERIC_MARGIN: f64 = 0.08;
const MAX_BINARY_PREVIEW_BYTES: u64 = 12 * 1024 * 1024;
const MAX_TEXT_PREVIEW_BYTES: usize = 120 * 1024;
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
pub struct ParsedFileName {
    pub structured: bool,
    pub agency: Option<String>,
    pub advertiser: Option<String>,
    pub subject: Option<String>,
    pub date: Option<String>,
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
    pub parsed: ParsedFileName,
    pub recommendations: Vec<FolderRecommendation>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilePreview {
    pub kind: String,
    pub mime_type: Option<String>,
    pub data_url: Option<String>,
    pub text: Option<String>,
    pub message: Option<String>,
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

            let parsed = parse_file_name(&file_name);
            let recommendations = recommend_destinations(&file_name, &parsed, &destinations);
            if !is_relevant_candidate(&parsed, &recommendations) {
                // Hard-to-classify files are intentionally invisible. We do not ask the user
                // about Downloads/Desktop files unless the filename and existing folder path
                // provide a strong, unambiguous filing signal.
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

            candidates.push(FileCandidate {
                path: entry.path().to_string_lossy().into_owned(),
                file_name,
                extension,
                size_bytes: metadata.len(),
                modified_at_ms,
                source_label: source_label.clone(),
                parsed,
                recommendations,
            });
        }
    }

    candidates.sort_by(|a, b| b.modified_at_ms.cmp(&a.modified_at_ms));
    candidates.truncate(MAX_CANDIDATES);
    Ok(candidates)
}

#[tauri::command]
pub fn get_file_preview(
    file_path: String,
    watch_locations: Vec<String>,
) -> Result<FilePreview, String> {
    let path = validated_candidate_path(&file_path, &watch_locations)?;
    let metadata = fs::metadata(&path).map_err(|error| format!("Could not read file: {error}"))?;
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if let Some(mime_type) = image_mime(&extension) {
        if metadata.len() > MAX_BINARY_PREVIEW_BYTES {
            return Ok(unsupported_preview("이미지가 12MB를 초과해 앱 안에서 미리보지 않습니다. 원본 열기를 사용하세요."));
        }
        let bytes = fs::read(&path).map_err(|error| format!("Could not read preview: {error}"))?;
        return Ok(FilePreview {
            kind: "image".into(),
            mime_type: Some(mime_type.into()),
            data_url: Some(format!("data:{mime_type};base64,{}", STANDARD.encode(bytes))),
            text: None,
            message: None,
        });
    }

    if extension == "pdf" {
        if metadata.len() > MAX_BINARY_PREVIEW_BYTES {
            return Ok(unsupported_preview("PDF가 12MB를 초과해 앱 안에서 미리보지 않습니다. 원본 열기를 사용하세요."));
        }
        let bytes = fs::read(&path).map_err(|error| format!("Could not read preview: {error}"))?;
        return Ok(FilePreview {
            kind: "pdf".into(),
            mime_type: Some("application/pdf".into()),
            data_url: Some(format!("data:application/pdf;base64,{}", STANDARD.encode(bytes))),
            text: None,
            message: None,
        });
    }

    if is_text_extension(&extension) {
        let bytes = fs::read(&path).map_err(|error| format!("Could not read preview: {error}"))?;
        let truncated = bytes.len() > MAX_TEXT_PREVIEW_BYTES;
        let visible = &bytes[..bytes.len().min(MAX_TEXT_PREVIEW_BYTES)];
        let mut text = String::from_utf8_lossy(visible).into_owned();
        if truncated {
            text.push_str("\n\n… 미리보기는 앞부분 120KB까지만 표시합니다.");
        }
        return Ok(FilePreview {
            kind: "text".into(),
            mime_type: Some("text/plain".into()),
            data_url: None,
            text: Some(text),
            message: None,
        });
    }

    Ok(unsupported_preview(
        "이 파일 형식은 앱 내부 미리보기를 지원하지 않습니다. 원본 열기로 직접 확인할 수 있습니다.",
    ))
}

#[tauri::command]
pub fn open_candidate_file(
    app: AppHandle,
    file_path: String,
    watch_locations: Vec<String>,
) -> Result<(), String> {
    let path = validated_candidate_path(&file_path, &watch_locations)?;
    let path_string = path.to_string_lossy().into_owned();
    app.opener()
        .open_path(path_string, None::<&str>)
        .map_err(|error| format!("Could not open file: {error}"))
}

fn unsupported_preview(message: &str) -> FilePreview {
    FilePreview {
        kind: "unsupported".into(),
        mime_type: None,
        data_url: None,
        text: None,
        message: Some(message.into()),
    }
}

fn validated_candidate_path(file_path: &str, watch_locations: &[String]) -> Result<PathBuf, String> {
    let source = fs::canonicalize(file_path)
        .map_err(|error| format!("Could not open candidate file: {error}"))?;
    if !source.is_file() {
        return Err("Candidate path is not a file.".into());
    }

    let source_parent = source
        .parent()
        .ok_or_else(|| "Candidate file has no parent directory.".to_string())?;
    let allowed = watch_locations.iter().any(|location| {
        fs::canonicalize(location)
            .ok()
            .map(|watch_root| watch_root == source_parent)
            .unwrap_or(false)
    });

    if !allowed {
        return Err("Preview/open is allowed only for top-level watched files.".into());
    }
    Ok(source)
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
        if name == ARCHIVE_NAME {
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

fn parse_file_name(file_name: &str) -> ParsedFileName {
    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(file_name);
    let mut parts: Vec<String> = stem
        .split('_')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect();

    let date = parts
        .last()
        .filter(|value| is_date_token(value))
        .cloned();
    if date.is_some() {
        parts.pop();
    }

    let structured = parts.len() >= 3;
    if !structured {
        return ParsedFileName {
            structured: false,
            agency: None,
            advertiser: None,
            subject: None,
            date,
        };
    }

    ParsedFileName {
        structured: true,
        agency: Some(parts[0].clone()),
        advertiser: Some(parts[1].clone()),
        subject: Some(parts[2..].join("_")),
        date,
    }
}

fn is_date_token(value: &str) -> bool {
    if !matches!(value.len(), 6 | 8) || !value.chars().all(|character| character.is_ascii_digit()) {
        return false;
    }
    let (month_start, day_start) = if value.len() == 6 { (2, 4) } else { (4, 6) };
    let month = value[month_start..month_start + 2].parse::<u8>().ok();
    let day = value[day_start..day_start + 2].parse::<u8>().ok();
    matches!(month, Some(1..=12)) && matches!(day, Some(1..=31))
}

fn recommend_destinations(
    file_name: &str,
    parsed: &ParsedFileName,
    destinations: &[Destination],
) -> Vec<FolderRecommendation> {
    let mut scored = if parsed.structured {
        recommend_structured(parsed, destinations)
    } else {
        recommend_generic(file_name, destinations)
    };

    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| b.matched_labels.len().cmp(&a.matched_labels.len()))
            .then_with(|| b.depth.cmp(&a.depth))
    });
    scored.truncate(MAX_RECOMMENDATIONS);
    scored
}

fn recommend_structured(
    parsed: &ParsedFileName,
    destinations: &[Destination],
) -> Vec<FolderRecommendation> {
    let Some(advertiser) = parsed.advertiser.as_deref() else {
        return Vec::new();
    };
    let Some(subject) = parsed.subject.as_deref() else {
        return Vec::new();
    };

    destinations
        .iter()
        .filter_map(|destination| {
            let (advertiser_index, advertiser_score) = destination
                .labels
                .iter()
                .enumerate()
                .map(|(index, label)| (index, segment_similarity(advertiser, label)))
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal))?;

            if advertiser_score < 0.90 {
                return None;
            }

            let mut subject_best: f64 = 0.0;
            let mut subject_matches = Vec::new();
            for label in destination.labels.iter().skip(advertiser_index + 1) {
                let similarity = segment_similarity(subject, label);
                if similarity >= 0.72 {
                    subject_best = subject_best.max(similarity);
                    subject_matches.push(label.clone());
                }
            }
            if subject_matches.is_empty() {
                return None;
            }

            let leaf_score = destination
                .labels
                .last()
                .map(|label| segment_similarity(subject, label))
                .unwrap_or_default();
            if leaf_score < 0.60 && subject_matches.len() < 2 {
                return None;
            }

            let multi_match_bonus = if subject_matches.len() >= 2 { 0.04 } else { 0.0 };
            let managed_bonus = if destination.managed { 0.02 } else { 0.0 };
            let score = (0.58 * advertiser_score
                + 0.26 * subject_best
                + 0.10 * leaf_score
                + multi_match_bonus
                + managed_bonus)
                .min(0.99);

            let mut matched_labels = vec![destination.labels[advertiser_index].clone()];
            for label in subject_matches {
                if !matched_labels.contains(&label) {
                    matched_labels.push(label);
                }
            }

            Some(FolderRecommendation {
                path: destination.path.to_string_lossy().into_owned(),
                relative_path: destination.relative_path.clone(),
                depth: destination.depth,
                score,
                managed: destination.managed,
                matched_labels,
            })
        })
        .collect()
}

fn recommend_generic(file_name: &str, destinations: &[Destination]) -> Vec<FolderRecommendation> {
    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(file_name);
    let normalized_file = compact(stem);

    destinations
        .iter()
        .filter_map(|destination| {
            let mut score: f64 = 0.0;
            let mut matched_labels = Vec::new();

            for (index, label) in destination.labels.iter().enumerate() {
                let normalized_label = compact(label);
                if normalized_label.chars().count() < 2 {
                    continue;
                }
                if normalized_file.contains(&normalized_label) {
                    score += match index {
                        0 => 0.30,
                        1 => 0.22,
                        2 => 0.18,
                        3 => 0.15,
                        _ => 0.12,
                    };
                    matched_labels.push(label.clone());
                }
            }

            if matched_labels.len() < 2 {
                return None;
            }

            let leaf_matches = destination
                .labels
                .last()
                .map(|label| normalized_file.contains(&compact(label)))
                .unwrap_or(false);
            if !leaf_matches {
                return None;
            }

            score += 0.08;
            if destination.managed {
                score += 0.02;
            }

            Some(FolderRecommendation {
                path: destination.path.to_string_lossy().into_owned(),
                relative_path: destination.relative_path.clone(),
                depth: destination.depth,
                score: score.min(0.99),
                managed: destination.managed,
                matched_labels,
            })
        })
        .collect()
}

fn is_relevant_candidate(parsed: &ParsedFileName, recommendations: &[FolderRecommendation]) -> bool {
    let Some(best) = recommendations.first() else {
        return false;
    };
    let second_score = recommendations.get(1).map(|item| item.score).unwrap_or_default();
    let margin = if recommendations.len() >= 2 {
        best.score - second_score
    } else {
        1.0
    };

    if parsed.structured {
        best.score >= MIN_STRUCTURED_SCORE
            && best.matched_labels.len() >= 2
            && margin >= MIN_STRUCTURED_MARGIN
    } else {
        best.score >= MIN_GENERIC_SCORE
            && best.matched_labels.len() >= 2
            && margin >= MIN_GENERIC_MARGIN
    }
}

fn segment_similarity(segment: &str, label: &str) -> f64 {
    let segment = compact(segment);
    let label = compact(label);
    if segment.chars().count() < 2 || label.chars().count() < 2 {
        return 0.0;
    }
    if segment == label {
        return 1.0;
    }
    if segment.contains(&label) || label.contains(&segment) {
        let shorter = segment.chars().count().min(label.chars().count()) as f64;
        let longer = segment.chars().count().max(label.chars().count()) as f64;
        let coverage = shorter / longer;
        return 0.82 + 0.16 * coverage;
    }
    0.0
}

fn managed_label(name: &str) -> (bool, String) {
    if name == ARCHIVE_NAME {
        return (true, "ARCHIVE".into());
    }
    let bytes = name.as_bytes();
    if bytes.len() < 4
        || !bytes[0].is_ascii_digit()
        || !bytes[1].is_ascii_digit()
        || bytes[2] != b'_'
    {
        return (false, name.to_string());
    }
    let number = ((bytes[0] - b'0') * 10 + (bytes[1] - b'0')) as u8;
    let label = name[3..].trim();
    if !(1..=98).contains(&number) || label.is_empty() {
        return (false, label.to_string());
    }
    (true, label.to_string())
}

fn compact(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect()
}

fn image_mime(extension: &str) -> Option<&'static str> {
    match extension {
        "jpg" | "jpeg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "bmp" => Some("image/bmp"),
        _ => None,
    }
}

fn is_text_extension(extension: &str) -> bool {
    matches!(
        extension,
        "txt" | "md" | "csv" | "json" | "log" | "xml" | "html" | "css" | "js" | "ts"
    )
}

fn should_ignore_file(file_name: &str) -> bool {
    if file_name.starts_with('.') || file_name.starts_with("~$") {
        return true;
    }
    let extension = Path::new(file_name)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    PARTIAL_EXTENSIONS.contains(&extension.as_str())
}

fn path_for_display(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn system_time_to_ms(time: SystemTime) -> Option<u128> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn destination(path: &str, labels: &[&str]) -> Destination {
        Destination {
            path: PathBuf::from(path),
            relative_path: path.into(),
            depth: labels.len(),
            managed: true,
            labels: labels.iter().map(|value| value.to_string()).collect(),
        }
    }

    #[test]
    fn parses_jbr_style_filename() {
        let parsed = parse_file_name("JBR_자코모_DV360소재_260821.png");
        assert!(parsed.structured);
        assert_eq!(parsed.agency.as_deref(), Some("JBR"));
        assert_eq!(parsed.advertiser.as_deref(), Some("자코모"));
        assert_eq!(parsed.subject.as_deref(), Some("DV360소재"));
        assert_eq!(parsed.date.as_deref(), Some("260821"));
    }

    #[test]
    fn rejects_invalid_date_suffix_as_date() {
        let parsed = parse_file_name("JBR_자코모_DV360_991399.pdf");
        assert!(parsed.structured);
        assert_eq!(parsed.date, None);
        assert_eq!(parsed.subject.as_deref(), Some("DV360_991399"));
    }

    #[test]
    fn structured_match_requires_advertiser_and_work_path() {
        let parsed = parse_file_name("JBR_자코모_DV360소재_260821.png");
        let destinations = vec![
            destination("01_자코모/01_미디어/01_DV360/01_소재", &["자코모", "미디어", "DV360", "소재"]),
            destination("01_자코모/02_제작/01_소재", &["자코모", "제작", "소재"]),
            destination("02_교원웰스/01_미디어/01_DV360", &["교원웰스", "미디어", "DV360"]),
        ];
        let recommendations = recommend_destinations("JBR_자코모_DV360소재_260821.png", &parsed, &destinations);
        assert!(!recommendations.is_empty());
        assert!(recommendations[0].relative_path.contains("DV360"));
        assert!(is_relevant_candidate(&parsed, &recommendations));
    }

    #[test]
    fn ambiguous_subject_is_excluded() {
        let parsed = parse_file_name("JBR_자코모_소재_260821.png");
        let destinations = vec![
            destination("01_자코모/01_미디어/01_소재", &["자코모", "미디어", "소재"]),
            destination("01_자코모/02_제작/01_소재", &["자코모", "제작", "소재"]),
        ];
        let recommendations = recommend_destinations("JBR_자코모_소재_260821.png", &parsed, &destinations);
        assert!(!is_relevant_candidate(&parsed, &recommendations));
    }

    #[test]
    fn unrelated_file_is_excluded() {
        let parsed = parse_file_name("IMG_1234.png");
        let destinations = vec![destination("01_자코모/01_DV360", &["자코모", "DV360"])];
        let recommendations = recommend_destinations("IMG_1234.png", &parsed, &destinations);
        assert!(recommendations.is_empty());
        assert!(!is_relevant_candidate(&parsed, &recommendations));
    }

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
