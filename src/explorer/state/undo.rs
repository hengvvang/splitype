//! Explorer undo/redo — mirrors Zed's `crates/project_panel/src/undo.rs`.
//!
//! The history stores the forward operation; undoing executes its inverse
//! and pushes the same record onto the redo stack (so redo simply
//! re-executes it). Only reversible operations are recorded — permanent
//! deletes are not.

use std::path::{Path, PathBuf};

use super::utils::copy_dir_all;

/// One reversible file-tree operation recorded in the undo history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplorerChange {
    Created { path: PathBuf, is_dir: bool },
    Renamed { from: PathBuf, to: PathBuf },
    Moved { from: PathBuf, to: PathBuf },
    Copied { source: PathBuf, dest: PathBuf },
}

/// Undo/redo stacks for explorer file operations.
#[derive(Clone, Debug, Default)]
pub struct ExplorerUndoHistory {
    pub undo_stack: Vec<ExplorerChange>,
    pub redo_stack: Vec<ExplorerChange>,
}

impl ExplorerUndoHistory {
    pub fn record(&mut self, change: ExplorerChange) {
        self.undo_stack.push(change);
        // A fresh edit invalidates any forward history.
        self.redo_stack.clear();
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}

/// The destination path of a change (used to select the operation result).
pub fn explorer_change_destination(change: &ExplorerChange) -> Option<&Path> {
    match change {
        ExplorerChange::Created { path, .. } => Some(path),
        ExplorerChange::Renamed { to, .. } | ExplorerChange::Moved { to, .. } => Some(to),
        ExplorerChange::Copied { dest, .. } => Some(dest),
    }
}

pub fn remove_path_symlink_safe(path: &Path) -> std::io::Result<()> {
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

/// Execute a recorded file operation (redo).
pub fn execute_explorer_change(change: &ExplorerChange) {
    match change {
        ExplorerChange::Created { path, is_dir } => {
            if *is_dir {
                let _ = std::fs::create_dir_all(path);
            } else if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
                let _ = std::fs::write(path, "");
            }
        }
        ExplorerChange::Renamed { from, to } | ExplorerChange::Moved { from, to } => {
            let _ = std::fs::rename(from, to);
        }
        ExplorerChange::Copied { source, dest } => {
            let result = if source.is_dir() {
                copy_dir_all(source, dest)
            } else {
                std::fs::copy(source, dest).map(|_| ())
            };
            if let Err(err) = result {
                tracing::error!(path = %dest.display(), error = %err, "failed to redo copy");
            }
        }
    }
}

/// Execute the inverse of a recorded operation (undo).
pub fn execute_explorer_change_inverse(change: &ExplorerChange) {
    match change {
        ExplorerChange::Created { path, .. } => {
            if let Err(err) = remove_path_symlink_safe(path) {
                tracing::error!(path = %path.display(), error = %err, "failed to undo create");
            }
        }
        ExplorerChange::Renamed { from, to } | ExplorerChange::Moved { from, to } => {
            if let Err(err) = std::fs::rename(to, from) {
                tracing::error!(path = %to.display(), error = %err, "failed to undo rename");
            }
        }
        ExplorerChange::Copied { dest, .. } => {
            if let Err(err) = remove_path_symlink_safe(dest) {
                tracing::error!(path = %dest.display(), error = %err, "failed to undo copy");
            }
        }
    }
}
