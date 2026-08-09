//! Recent file history helpers.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context as _};

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

pub fn read_recent_files_with_dirs(
    dirs: &SplitypeConfigDirs,
) -> anyhow::Result<Vec<PathBuf>> {
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

pub fn read_recent_folders_with_dirs(
    dirs: &SplitypeConfigDirs,
) -> anyhow::Result<Vec<PathBuf>> {
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
    std::fs::write(target, content)
        .with_context(|| format!("failed to write '{}'", target.display()))
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
    if text.trim().is_empty() {
        return false;
    }

    !(is_inside_system_temp_dir(path) && has_splitype_temp_fixture_name(path))
}

fn is_inside_system_temp_dir(path: &Path) -> bool {
    let temp_dir = std::env::temp_dir();
    if cfg!(windows) {
        let path_text = normalize_windows_path_text(path);
        let mut temp_text = normalize_windows_path_text(&temp_dir);
        if !temp_text.ends_with('\\') {
            temp_text.push('\\');
        }
        return path_text.starts_with(&temp_text);
    }

    path.starts_with(temp_dir)
}

fn normalize_windows_path_text(path: &Path) -> String {
    path.to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_ascii_lowercase()
}

fn has_splitype_temp_fixture_name(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| {
            let name = name.to_ascii_lowercase();
            name.starts_with("splitype-drop-")
        })
        .unwrap_or(false)
}

fn same_recent_path(left: &Path, right: &Path) -> bool {
    if cfg!(windows) {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    } else {
        left == right
    }
}

#[cfg(test)]
mod tests {
    use super::{
        read_recent_files_with_dirs, record_recent_file_with_dirs, remove_recent_file_with_dirs,
        SplitypeConfigDirs, RECENT_FILES_LIMIT,
    };
    use std::path::{Path, PathBuf};

    #[test]
    fn missing_recent_history_file_returns_empty_list() {
        let root = std::env::temp_dir().join(format!("splitype-config-{}", uuid::Uuid::new_v4()));
        let dirs = SplitypeConfigDirs::from_root(&root);

        assert!(read_recent_files_with_dirs(&dirs).unwrap().is_empty());
        assert!(!dirs.history_file().exists());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn empty_recent_history_write_does_not_create_file() {
        let root = std::env::temp_dir().join(format!("splitype-config-{}", uuid::Uuid::new_v4()));
        let dirs = SplitypeConfigDirs::from_root(&root);

        super::write_recent_files_with_dirs(&[], &dirs).unwrap();

        assert!(!dirs.history_file().exists());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn blank_recent_file_path_is_rejected() {
        let root = std::env::temp_dir().join(format!("splitype-config-{}", uuid::Uuid::new_v4()));
        let dirs = SplitypeConfigDirs::from_root(&root);

        assert!(record_recent_file_with_dirs(Path::new("   "), &dirs).is_err());
        assert!(!dirs.history_file().exists());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn recent_history_filters_empty_lines_and_deduplicates() {
        let root = std::env::temp_dir().join(format!("splitype-config-{}", uuid::Uuid::new_v4()));
        let dirs = SplitypeConfigDirs::from_root(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            dirs.history_file(),
            "  \nC:\\one.md\n\nC:\\two.md\nC:\\one.md\n",
        )
        .unwrap();

        let paths = read_recent_files_with_dirs(&dirs).unwrap();
        assert_eq!(
            paths,
            vec![PathBuf::from("C:\\one.md"), PathBuf::from("C:\\two.md")]
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn recent_history_filters_legacy_splitype_temp_fixture_paths() {
        let root = std::env::temp_dir().join(format!("splitype-config-{}", uuid::Uuid::new_v4()));
        let dirs = SplitypeConfigDirs::from_root(&root);
        let fixture_path = std::env::temp_dir().join(format!(
            "splitype-drop-save-replace-{}-123.md",
            std::process::id()
        ));
        let real_path = PathBuf::from("C:\\notes\\real.md");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            dirs.history_file(),
            format!("{}\n{}\n", fixture_path.display(), real_path.display()),
        )
        .unwrap();

        let paths = read_recent_files_with_dirs(&dirs).unwrap();
        assert_eq!(paths, vec![real_path]);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn recording_splitype_temp_fixture_path_is_noop() {
        let root = std::env::temp_dir().join(format!("splitype-config-{}", uuid::Uuid::new_v4()));
        let dirs = SplitypeConfigDirs::from_root(&root);
        let fixture_path = std::env::temp_dir().join(format!(
            "splitype-drop-dirty-discard-{}-123.md",
            std::process::id()
        ));

        assert!(record_recent_file_with_dirs(&fixture_path, &dirs)
            .unwrap()
            .is_empty());
        assert!(!dirs.history_file().exists());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn ordinary_temp_markdown_file_can_still_be_recorded() {
        let root = std::env::temp_dir().join(format!("splitype-config-{}", uuid::Uuid::new_v4()));
        let dirs = SplitypeConfigDirs::from_root(&root);
        let path = std::env::temp_dir().join(format!("manual-note-{}.md", std::process::id()));

        let paths = record_recent_file_with_dirs(&path, &dirs).unwrap();

        assert_eq!(paths, vec![path]);
        assert!(dirs.history_file().exists());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn recording_recent_file_moves_it_to_front_and_truncates() {
        let root = std::env::temp_dir().join(format!("splitype-config-{}", uuid::Uuid::new_v4()));
        let dirs = SplitypeConfigDirs::from_root(&root);

        for index in 0..(RECENT_FILES_LIMIT + 2) {
            record_recent_file_with_dirs(&PathBuf::from(format!("file-{index}.md")), &dirs)
                .unwrap();
        }
        record_recent_file_with_dirs(&PathBuf::from("file-3.md"), &dirs).unwrap();

        let paths = read_recent_files_with_dirs(&dirs).unwrap();
        assert_eq!(paths.len(), RECENT_FILES_LIMIT);
        assert_eq!(paths[0], PathBuf::from("file-3.md"));
        assert_eq!(
            paths
                .iter()
                .filter(|path| path.as_path() == Path::new("file-3.md"))
                .count(),
            1
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn removing_recent_file_persists_history_without_it() {
        let root = std::env::temp_dir().join(format!("splitype-config-{}", uuid::Uuid::new_v4()));
        let dirs = SplitypeConfigDirs::from_root(&root);
        record_recent_file_with_dirs(&PathBuf::from("one.md"), &dirs).unwrap();
        record_recent_file_with_dirs(&PathBuf::from("two.md"), &dirs).unwrap();

        let paths = remove_recent_file_with_dirs(&PathBuf::from("one.md"), &dirs).unwrap();

        assert_eq!(paths, vec![PathBuf::from("two.md")]);
        assert_eq!(
            read_recent_files_with_dirs(&dirs).unwrap(),
            vec![PathBuf::from("two.md")]
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn removing_last_recent_file_deletes_history_file() {
        let root = std::env::temp_dir().join(format!("splitype-config-{}", uuid::Uuid::new_v4()));
        let dirs = SplitypeConfigDirs::from_root(&root);
        let path = PathBuf::from("only.md");
        record_recent_file_with_dirs(&path, &dirs).unwrap();
        assert!(dirs.history_file().exists());

        let paths = remove_recent_file_with_dirs(&path, &dirs).unwrap();

        assert!(paths.is_empty());
        assert!(!dirs.history_file().exists());
        assert!(read_recent_files_with_dirs(&dirs).unwrap().is_empty());

        let _ = std::fs::remove_dir_all(root);
    }
}
