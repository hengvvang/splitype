//! Free helper functions for background file operations — mirrors Zed's
//! `crates/project_panel/src/utils.rs`.
//!
//! Everything here runs on a background thread: no editor or window access,
//! only paths and filesystem facts.

use std::ops::Range;
use std::path::{Path, PathBuf};

use super::undo::ExplorerChange;

/// Recursively copy a directory tree (`fs::copy` is file-only).
pub fn copy_dir_all(source: &Path, destination: &Path) -> std::io::Result<()> {
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

/// Disambiguate a copy destination on a background thread (no editor
/// access): "name copy.ext", "name copy 1.ext", …
///
/// Returns the destination and, when a suffix was needed, the byte range of
/// the original name plus the disambiguation suffix — the inline rename
/// editor selects that range so the user can immediately retype the name
/// (mirrors Zed's `create_paste_path` returning a `disambiguation_range`).
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

/// Move (cut) or copy `items` into `target_dir` on a background thread;
/// returns the recorded changes for undo. Used by paste and drag-drop.
///
/// `disambiguate` selects the collision policy for copies: `true` derives a
/// "name copy.ext" destination (in-panel paste / drag-copy, which then opens
/// the inline rename editor), `false` overwrites same-named destinations
/// (external file drops, after the user confirmed the Replace prompt).
pub fn execute_entry_ops(
    items: &[PathBuf],
    target_dir: &Path,
    is_cut: bool,
    disambiguate: bool,
) -> Vec<ExplorerChange> {
    let mut changes = Vec::new();
    for source in items {
        if is_cut {
            // Moving into the entry's own subtree is an invalid circular move.
            if target_dir.starts_with(source) {
                continue;
            }
            let destination = target_dir.join(source.file_name().unwrap_or_default());
            if source == &destination {
                continue;
            }
            if std::fs::rename(source, &destination).is_ok() {
                changes.push(ExplorerChange::Moved {
                    from: source.clone(),
                    to: destination,
                });
            }
        } else {
            let destination = if disambiguate {
                disambiguated_paste_path(source, target_dir).0
            } else {
                target_dir.join(source.file_name().unwrap_or_default())
            };
            let result = if source.is_dir() {
                copy_dir_all(source, &destination)
            } else {
                std::fs::copy(source, &destination).map(|_| ())
            };
            if result.is_ok() {
                changes.push(ExplorerChange::Copied {
                    source: source.clone(),
                    dest: destination,
                });
            }
        }
    }
    changes
}

/// The copy modifier inverts a drag from move to copy (mirrors Zed's
/// `is_copy_modifier_set`: Alt on macOS, Ctrl elsewhere).
pub fn explorer_is_copy_modifier(modifiers: &gpui::Modifiers) -> bool {
    cfg!(target_os = "macos") && modifiers.alt
        || cfg!(not(target_os = "macos")) && modifiers.control
}
