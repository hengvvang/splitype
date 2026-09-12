//! Filesystem primitives — every function is background-thread safe.

use std::io;
use std::ops::Range;
use std::path::{Path, PathBuf};

use super::error::FsError;

/// Recursively copy a directory tree (`fs::copy` is file-only).
pub fn copy_dir_all(source: &Path, destination: &Path) -> io::Result<()> {
    if destination.starts_with(source) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "cannot copy directory into itself or its subtree",
        ));
    }
    std::fs::create_dir_all(destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let from = entry.path();
        let to = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// Disambiguate a copy destination: `"name copy.ext"`, `"name copy 1.ext"`, …
///
/// Returns the destination and, when a suffix was needed, the byte range of
/// the original name plus the disambiguation suffix — the caller selects
/// that range so the user can immediately retype the name.
pub fn disambiguated_paste_path(
    source: &Path,
    target_dir: &Path,
) -> (PathBuf, Option<Range<usize>>) {
    let Some(name) = source.file_name() else {
        return (target_dir.join("copy"), None);
    };
    let stem = source
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let extension = source
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let mut candidate = target_dir.join(name);
    let mut ix = 0usize;
    let mut disambiguation_range = None;
    while candidate.exists() {
        let suffix = if ix == 0 {
            " copy".to_string()
        } else {
            format!(" copy {ix}")
        };
        candidate = target_dir.join(format!("{stem}{suffix}{extension}"));
        // Select the original stem plus the suffix, leaving the extension
        // out — the rename editor pre-selects exactly what to replace.
        disambiguation_range = Some(0..(stem.len() + suffix.len()));
        ix += 1;
    }
    (candidate, disambiguation_range)
}

/// Remove a path, tolerating symlinks (a symlink to a directory must be
/// removed with `remove_file` on non-Windows, `remove_dir` on Windows).
pub fn remove_symlink_safe(path: &Path) -> io::Result<()> {
    let is_symlink = path
        .symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);
    if is_symlink {
        #[cfg(windows)]
        if path.is_dir() {
            std::fs::remove_dir(path)
        } else {
            std::fs::remove_file(path)
        }
        #[cfg(not(windows))]
        std::fs::remove_file(path)
    } else if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
}

/// Remove `path` only when it is an empty directory.
pub fn remove_empty_dir_only(path: &Path) -> io::Result<()> {
    if path.is_dir() && std::fs::read_dir(path)?.next().is_none() {
        std::fs::remove_dir(path)?;
    }
    Ok(())
}

/// Represents a file or directory that has been moved to the system trash,
/// retaining enough information to restore it to its original location.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TrashedEntry {
    pub id: std::ffi::OsString,
    pub name: std::ffi::OsString,
    pub original_parent: PathBuf,
}

impl From<trash::TrashItem> for TrashedEntry {
    fn from(item: trash::TrashItem) -> Self {
        Self {
            id: item.id,
            name: item.name,
            original_parent: item.original_parent,
        }
    }
}

impl TrashedEntry {
    pub fn into_trash_item(self) -> trash::TrashItem {
        trash::TrashItem {
            id: self.id,
            name: self.name,
            original_parent: self.original_parent,
            time_deleted: 0,
        }
    }
}

fn strip_windows_verbatim(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(stripped) = s.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{stripped}"))
    } else if let Some(stripped) = s.strip_prefix(r"\\?\") {
        PathBuf::from(stripped)
    } else {
        path.to_path_buf()
    }
}

/// Move `path` to the OS trash and retain enough info to restore it if undone.
pub fn trash_with_info(path: &Path) -> io::Result<TrashedEntry> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let normalized_path = strip_windows_verbatim(&canonical);
    let name = path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    let parent = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let normalized_parent = parent
        .canonicalize()
        .map(|p| strip_windows_verbatim(&p))
        .unwrap_or_else(|_| parent.clone());

    trash::delete(path).map_err(|e| io::Error::other(e.to_string()))?;

    #[cfg(any(
        target_os = "windows",
        all(
            unix,
            not(target_os = "macos"),
            not(target_os = "ios"),
            not(target_os = "android")
        )
    ))]
    {
        if let Ok(items) = trash::os_limited::list() {
            if let Some(item) = items.into_iter().rev().find(|item| {
                let match_name = item
                    .name
                    .to_string_lossy()
                    .eq_ignore_ascii_case(&name.to_string_lossy());
                let match_parent = item
                    .original_parent
                    .as_os_str()
                    .eq_ignore_ascii_case(normalized_parent.as_os_str())
                    || item
                        .original_parent
                        .as_os_str()
                        .eq_ignore_ascii_case(parent.as_os_str());
                let match_path = item
                    .original_path()
                    .as_os_str()
                    .eq_ignore_ascii_case(normalized_path.as_os_str())
                    || item
                        .original_path()
                        .as_os_str()
                        .eq_ignore_ascii_case(path.as_os_str());

                match_name && (match_parent || match_path)
            }) {
                return Ok(TrashedEntry::from(item));
            }
        }
    }

    Ok(TrashedEntry {
        id: std::ffi::OsString::new(),
        name,
        original_parent: normalized_parent,
    })
}

/// Restore a previously trashed entry from the system recycle bin back to its original path.
pub fn restore_trash(entry: &TrashedEntry) -> io::Result<PathBuf> {
    let original_path = entry.original_parent.join(&entry.name);

    #[cfg(any(
        target_os = "windows",
        all(
            unix,
            not(target_os = "macos"),
            not(target_os = "ios"),
            not(target_os = "android")
        )
    ))]
    {
        if !entry.id.is_empty() {
            let item = entry.clone().into_trash_item();
            if trash::os_limited::restore_all([item]).is_ok() {
                return Ok(original_path);
            }
        }
        if let Ok(items) = trash::os_limited::list() {
            if let Some(item) = items.into_iter().rev().find(|i| {
                let match_name = i
                    .name
                    .to_string_lossy()
                    .eq_ignore_ascii_case(&entry.name.to_string_lossy());
                let match_parent = i
                    .original_parent
                    .as_os_str()
                    .eq_ignore_ascii_case(entry.original_parent.as_os_str());
                match_name && match_parent
            }) {
                trash::os_limited::restore_all([item])
                    .map_err(|e| io::Error::other(e.to_string()))?;
                return Ok(original_path);
            }
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!(
            "Item '{}' could not be restored from trash",
            original_path.display()
        ),
    ))
}

/// Move `path` to the OS trash (recoverable); falls back to a permanent
/// delete when trash is unavailable.
pub fn trash(path: &Path) -> io::Result<()> {
    match trash_with_info(path) {
        Ok(_) => Ok(()),
        Err(_) => remove_symlink_safe(path),
    }
}

/// Rename or move `from` to `to`, falling back to copy-then-remove for
/// cross-device moves.
pub fn rename(from: &Path, to: &Path) -> Result<(), FsError> {
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    // Cross-device fallback: copy then delete the source.
    if from.is_dir() {
        copy_dir_all(from, to).map_err(|source| FsError::RenameFailed {
            from: from.to_path_buf(),
            to: to.to_path_buf(),
            source,
        })?;
        let _ = remove_symlink_safe(from);
    } else {
        std::fs::copy(from, to).map_err(|source| FsError::RenameFailed {
            from: from.to_path_buf(),
            to: to.to_path_buf(),
            source,
        })?;
        let _ = std::fs::remove_file(from);
    }
    Ok(())
}

/// Copy a file or directory tree to `dest`.
pub fn copy(source: &Path, dest: &Path) -> Result<(), FsError> {
    if source.is_dir() {
        copy_dir_all(source, dest).map_err(|source_err| FsError::WriteFailed {
            path: dest.to_path_buf(),
            source: source_err,
        })
    } else {
        std::fs::copy(source, dest)
            .map(|_| ())
            .map_err(|source_err| FsError::WriteFailed {
                path: dest.to_path_buf(),
                source: source_err,
            })
    }
}

/// Recursively create `path` and its parents.
pub fn create_dir_all(path: &Path) -> Result<(), FsError> {
    std::fs::create_dir_all(path).map_err(|source| FsError::CreateDirFailed {
        path: path.to_path_buf(),
        source,
    })
}

/// Write `contents` to `path`, creating parent directories first.
pub fn write_file(path: &Path, contents: &str) -> Result<(), FsError> {
    if let Some(parent) = path.parent() {
        create_dir_all(parent)?;
    }
    std::fs::write(path, contents).map_err(|source| FsError::WriteFailed {
        path: path.to_path_buf(),
        source,
    })
}

/// Create an empty file at `path` exclusively (fails with
/// `AlreadyExists`-carrying [`FsError::WriteFailed`] when `path` exists).
pub fn create_new_file(path: &Path) -> Result<(), FsError> {
    if let Some(parent) = path.parent() {
        create_dir_all(parent)?;
    }
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map(|_| ())
        .map_err(|source| FsError::WriteFailed {
            path: path.to_path_buf(),
            source,
        })
}
