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
    Created(PathBuf),
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
        ExplorerChange::Created(path) => Some(path),
        ExplorerChange::Renamed { to, .. } | ExplorerChange::Moved { to, .. } => Some(to),
        ExplorerChange::Copied { dest, .. } => Some(dest),
    }
}

/// Execute a recorded file operation (redo).
pub fn execute_explorer_change(change: &ExplorerChange) {
    match change {
        ExplorerChange::Created(path) => {
            if path.is_dir() {
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
                eprintln!("failed to redo copy '{}': {err}", dest.display());
            }
        }
    }
}

/// Execute the inverse of a recorded operation (undo).
pub fn execute_explorer_change_inverse(change: &ExplorerChange) {
    match change {
        ExplorerChange::Created(path) => {
            let result = if path.is_dir() {
                std::fs::remove_dir_all(path)
            } else {
                std::fs::remove_file(path)
            };
            if let Err(err) = result {
                eprintln!("failed to undo create '{}': {err}", path.display());
            }
        }
        ExplorerChange::Renamed { from, to } | ExplorerChange::Moved { from, to } => {
            if let Err(err) = std::fs::rename(to, from) {
                eprintln!("failed to undo rename '{}': {err}", to.display());
            }
        }
        ExplorerChange::Copied { dest, .. } => {
            let result = if dest.is_dir() {
                std::fs::remove_dir_all(dest)
            } else {
                std::fs::remove_file(dest)
            };
            if let Err(err) = result {
                eprintln!("failed to undo copy '{}': {err}", dest.display());
            }
        }
    }
}
