//! Recent file history helpers.

use std::path::{Path, PathBuf};

use anyhow::{Context as _, bail};

use super::dirs::SplitypeConfigDirs;

pub const RECENT_FILES_LIMIT: usize = 20;
pub const RECENT_FOLDERS_LIMIT: usize = 10;

pub fn read_recent_files() -> anyhow::Result<Vec<PathBuf>> {
    read_recent_files_with_dirs(&SplitypeConfigDirs::from_system()?)
}

pub fn record_recent_file(path: &Path) -> anyhow::Result<Vec<PathBuf>> {
    record_recent_file_with_dirs(path, &SplitypeConfigDirs::from_system()?)
}

pub fn remove_recent_file(path: &Path) -> anyhow::Result<Vec<PathBuf>> {
    remove_recent_file_with_dirs(path, &SplitypeConfigDirs::from_system()?)
}

pub fn read_recent_folders() -> anyhow::Result<Vec<PathBuf>> {
    read_recent_folders_with_dirs(&SplitypeConfigDirs::from_system()?)
}

pub fn record_recent_folder(path: &Path) -> anyhow::Result<Vec<PathBuf>> {
    record_recent_folder_with_dirs(path, &SplitypeConfigDirs::from_system()?)
}

pub fn read_recent_files_with_dirs(dirs: &SplitypeConfigDirs) -> anyhow::Result<Vec<PathBuf>> {
    let path = dirs.history_file();
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => {
            return Err(err).with_context(|| format!("failed to read '{}'", path.display()));
        }
    };

    Ok(normalize_recent_files(text.lines().map(PathBuf::from)))
}

pub fn record_recent_file_with_dirs(
    path: &Path,
    dirs: &SplitypeConfigDirs,
) -> anyhow::Result<Vec<PathBuf>> {
    if path.to_string_lossy().trim().is_empty() {
        bail!("recent file path cannot be empty");
    }
    if !is_recordable_recent_file_path(path) {
        return read_recent_files_with_dirs(dirs);
    }

    let mut paths = read_recent_files_with_dirs(dirs)?;
    let path = path.to_path_buf();
    paths.retain(|existing| !same_recent_path(existing, &path));
    paths.insert(0, path);
    paths.truncate(RECENT_FILES_LIMIT);
    write_recent_files_with_dirs(&paths, dirs)?;
    Ok(paths)
}

pub fn remove_recent_file_with_dirs(
    path: &Path,
    dirs: &SplitypeConfigDirs,
) -> anyhow::Result<Vec<PathBuf>> {
    let mut paths = read_recent_files_with_dirs(dirs)?;
    paths.retain(|existing| !same_recent_path(existing, path));
    write_recent_files_with_dirs(&paths, dirs)?;
    Ok(paths)
}

pub fn read_recent_folders_with_dirs(dirs: &SplitypeConfigDirs) -> anyhow::Result<Vec<PathBuf>> {
    let path = dirs.recent_folders_file();
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => {
            return Err(err).with_context(|| format!("failed to read '{}'", path.display()));
        }
    };
    Ok(normalize_recent_files(text.lines().map(PathBuf::from)))
}

pub fn record_recent_folder_with_dirs(
    path: &Path,
    dirs: &SplitypeConfigDirs,
) -> anyhow::Result<Vec<PathBuf>> {
    if path.to_string_lossy().trim().is_empty() {
        bail!("recent folder path cannot be empty");
    }
    let mut paths = read_recent_folders_with_dirs(dirs)?;
    let path = path.to_path_buf();
    paths.retain(|existing| !same_recent_path(existing, &path));
    paths.insert(0, path);
    paths.truncate(RECENT_FOLDERS_LIMIT);
    write_path_list(&paths, &dirs.recent_folders_file())?;
    Ok(paths)
}

fn write_recent_files_with_dirs(
    paths: &[PathBuf],
    dirs: &SplitypeConfigDirs,
) -> anyhow::Result<()> {
    write_path_list(paths, &dirs.history_file())
}

/// Shared persistence for a plain newline-separated path list (used by both
/// the recent-files history and the recent-folders list).
fn write_path_list(paths: &[PathBuf], target: &Path) -> anyhow::Result<()> {
    let normalized = normalize_recent_files(paths.iter().cloned());
    if normalized.is_empty() {
        match std::fs::remove_file(target) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => {
                return Err(err)
                    .with_context(|| format!("failed to remove '{}'", target.display()));
            }
        }
        return Ok(());
    }

    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create '{}'", parent.display()))?;
    }
    let mut content = String::new();
    for path in normalized {
        content.push_str(&path.to_string_lossy());
        content.push('\n');
    }
    let tmp_target = target.with_extension(format!("tmp.{}", std::process::id()));
    std::fs::write(&tmp_target, content)
        .with_context(|| format!("failed to write temporary file '{}'", tmp_target.display()))?;
    std::fs::rename(&tmp_target, target)
        .with_context(|| format!("failed to update '{}'", target.display()))
}

fn normalize_recent_files(paths: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
    let mut normalized: Vec<PathBuf> = Vec::new();
    for path in paths {
        let text = path.to_string_lossy();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            continue;
        }
        let path = PathBuf::from(trimmed);
        if !is_recordable_recent_file_path(&path) {
            continue;
        }
        if normalized
            .iter()
            .any(|existing| same_recent_path(existing, &path))
        {
            continue;
        }
        normalized.push(path);
        if normalized.len() == RECENT_FILES_LIMIT {
            break;
        }
    }
    normalized
}

fn is_recordable_recent_file_path(path: &Path) -> bool {
    let text = path.to_string_lossy();
    !text.trim().is_empty()
}

fn same_recent_path(left: &Path, right: &Path) -> bool {
    if cfg!(windows) {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    } else {
        left == right
    }
}

