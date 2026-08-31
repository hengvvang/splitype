//! Free helper functions for background file operations — mirrors Zed's
//! `crates/project_panel/src/utils.rs`.
//!
//! Everything here runs on a background thread: no editor or window access,
//! only paths and filesystem facts. The pure filesystem primitives live in
//! `explorer_fs`; this module owns the operation bookkeeping (change
//! records for undo).

use std::path::{Path, PathBuf};

use super::undo::ExplorerChange;

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
        // Moving or copying a directory into its own subtree is an invalid circular operation.
        if source.is_dir() && target_dir.starts_with(source) {
            continue;
        }
        if is_cut {
            let destination = target_dir.join(source.file_name().unwrap_or_default());
            if source == &destination {
                continue;
            }
            if explorer_fs::rename(source, &destination).is_ok() {
                changes.push(ExplorerChange::Moved {
                    from: source.clone(),
                    to: destination,
                });
            }
        } else {
            let destination = if disambiguate {
                explorer_fs::disambiguated_paste_path(source, target_dir).0
            } else {
                target_dir.join(source.file_name().unwrap_or_default())
            };
            if explorer_fs::copy(source, &destination).is_ok() {
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

