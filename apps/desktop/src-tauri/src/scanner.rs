use serde::Serialize;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

const MAX_DEPTH: usize = 5;
const ACTIVE_FOLDER_MAX: usize = 98;
const ARCHIVE_NAME: &str = "99_ARCHIVE";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanReport {
    pub root: String,
    pub total_folders: usize,
    pub max_depth: usize,
    pub depth_violations: usize,
    pub numbering_violations: usize,
    pub duplicate_number_groups: usize,
    pub archive_misuse: usize,
    pub active_folder_limit_violations: usize,
    pub unreadable_folders: usize,
}

#[derive(Default)]
struct ScanState {
    total_folders: usize,
    max_depth: usize,
    depth_violations: usize,
    numbering_violations: usize,
    duplicate_number_groups: usize,
    archive_misuse: usize,
    active_folder_limit_violations: usize,
    unreadable_folders: usize,
}

#[derive(Debug, PartialEq, Eq)]
struct ParsedName {
    valid: bool,
    number: Option<u8>,
    archive_misuse: bool,
}

pub fn scan_workspace(root: PathBuf) -> Result<ScanReport, String> {
    if !root.exists() { return Err("Workspace path does not exist.".into()); }
    if !root.is_dir() { return Err("Workspace path is not a directory.".into()); }
    let mut state = ScanState::default();
    walk(&root, 0, &mut state);
    Ok(ScanReport {
        root: root.to_string_lossy().into_owned(),
        total_folders: state.total_folders,
        max_depth: state.max_depth,
        depth_violations: state.depth_violations,
        numbering_violations: state.numbering_violations,
        duplicate_number_groups: state.duplicate_number_groups,
        archive_misuse: state.archive_misuse,
        active_folder_limit_violations: state.active_folder_limit_violations,
        unreadable_folders: state.unreadable_folders,
    })
}

fn walk(path: &Path, depth: usize, state: &mut ScanState) {
    let entries = match fs::read_dir(path) { Ok(entries) => entries, Err(_) => { state.unreadable_folders += 1; return; } };
    let mut child_directories: Vec<(PathBuf, String)> = Vec::new();
    for entry_result in entries {
        let entry = match entry_result { Ok(entry) => entry, Err(_) => { state.unreadable_folders += 1; continue; } };
        let file_type = match entry.file_type() { Ok(file_type) => file_type, Err(_) => { state.unreadable_folders += 1; continue; } };
        if !file_type.is_dir() || file_type.is_symlink() { continue; }
        child_directories.push((entry.path(), entry.file_name().to_string_lossy().into_owned()));
    }
    inspect_siblings(&child_directories, state);
    for (child_path, child_name) in child_directories {
        let child_depth = depth + 1;
        state.total_folders += 1;
        state.max_depth = state.max_depth.max(child_depth);
        if child_depth > MAX_DEPTH { state.depth_violations += 1; }
        let parsed = parse_name(&child_name);
        if !parsed.valid { state.numbering_violations += 1; }
        if parsed.archive_misuse { state.archive_misuse += 1; }
        walk(&child_path, child_depth, state);
    }
}

fn inspect_siblings(children: &[(PathBuf, String)], state: &mut ScanState) {
    let mut managed_numbers: HashMap<u8, usize> = HashMap::new();
    let active_count = children.iter().filter(|(_, name)| name != ARCHIVE_NAME).count();
    for (_, name) in children {
        let parsed = parse_name(name);
        if let Some(number) = parsed.number { *managed_numbers.entry(number).or_insert(0) += 1; }
    }
    state.duplicate_number_groups += managed_numbers.values().filter(|count| **count > 1).count();
    if active_count > ACTIVE_FOLDER_MAX { state.active_folder_limit_violations += 1; }
}

fn parse_name(name: &str) -> ParsedName {
    if name == ARCHIVE_NAME { return ParsedName { valid: true, number: Some(99), archive_misuse: false }; }
    let bytes = name.as_bytes();
    if bytes.len() < 4 || !bytes[0].is_ascii_digit() || !bytes[1].is_ascii_digit() || bytes[2] != b'_' {
        return ParsedName { valid: false, number: None, archive_misuse: false };
    }
    let number = ((bytes[0] - b'0') * 10 + (bytes[1] - b'0')) as u8;
    let label = &name[3..];
    if label.trim().is_empty() { return ParsedName { valid: false, number: Some(number), archive_misuse: number == 99 }; }
    if number == 99 { return ParsedName { valid: false, number: Some(number), archive_misuse: true }; }
    if !(1..=98).contains(&number) { return ParsedName { valid: false, number: Some(number), archive_misuse: false }; }
    ParsedName { valid: true, number: Some(number), archive_misuse: false }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_active_folder() { assert_eq!(parse_name("03_Project"), ParsedName { valid: true, number: Some(3), archive_misuse: false }); }
    #[test]
    fn accepts_archive_only_at_99() { assert!(parse_name("99_ARCHIVE").valid); let misuse = parse_name("99_Project"); assert!(!misuse.valid); assert!(misuse.archive_misuse); }
    #[test]
    fn rejects_zero_and_missing_number() { assert!(!parse_name("00_Misc").valid); assert!(!parse_name("Media").valid); }
}
