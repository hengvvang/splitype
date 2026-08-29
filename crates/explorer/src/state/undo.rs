//! Explorer undo/redo — mirrors Zed's `crates/project_panel/src/undo.rs`.
//!
//! The history stores the forward operation; undoing executes its inverse
//! and pushes the same record onto the redo stack (so redo simply
//! re-executes it). Only reversible operations are recorded — permanent
//! deletes are not. All filesystem execution delegates to `explorer_fs`.

use std::path::{Path, PathBuf};

use explorer_fs::FsError;

/// One reversible file-tree operation recorded in the undo history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplorerChange {
    Created { path: PathBuf, is_dir: bool },
    Renamed { from: PathBuf, to: PathBuf },
    Moved { from: PathBuf, to: PathBuf },
    Copied { source: PathBuf, dest: PathBuf },
    DirCreated(PathBuf),
    DirRemoved(PathBuf),
    Batch(Vec<ExplorerChange>),
}

/// Undo/redo stacks for explorer file operations.
#[derive(Clone, Debug, Default)]
pub struct ExplorerUndoHistory {
    pub undo_stack: Vec<ExplorerChange>,
    pub redo_stack: Vec<ExplorerChange>,
}

impl ExplorerUndoHistory {
    pub fn record(&mut self, change: ExplorerChange) {
        if let ExplorerChange::Batch(ref changes) = change {
            if changes.is_empty() {
                return;
            }
        }
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
        ExplorerChange::DirCreated(path) => Some(path),
        ExplorerChange::DirRemoved(_) => None,
        ExplorerChange::Batch(changes) => changes.last().and_then(explorer_change_destination),
    }
}

/// Execute a recorded file operation (redo).
pub fn execute_explorer_change(change: &ExplorerChange) -> Result<(), FsError> {
    match change {
        ExplorerChange::Created { path, is_dir } => {
            if *is_dir {
                explorer_fs::create_dir_all(path)?;
            } else {
                explorer_fs::write_file(path, "")?;
            }
            Ok(())
        }
        ExplorerChange::DirCreated(path) => {
            explorer_fs::create_dir_all(path)?;
            Ok(())
        }
        ExplorerChange::DirRemoved(path) => {
            let _ = explorer_fs::remove_empty_dir_only(path);
            Ok(())
        }
        ExplorerChange::Renamed { from, to } | ExplorerChange::Moved { from, to } => {
            explorer_fs::rename(from, to)?;
            Ok(())
        }
        ExplorerChange::Copied { source, dest } => {
            explorer_fs::copy(source, dest)?;
            Ok(())
        }
        ExplorerChange::Batch(changes) => {
            for change in changes {
                execute_explorer_change(change)?;
            }
            Ok(())
        }
    }
}

/// Execute the inverse of a recorded operation (undo).
pub fn execute_explorer_change_inverse(change: &ExplorerChange) -> Result<(), FsError> {
    match change {
        ExplorerChange::Created { path, .. } => {
            // First try trash (recoverable), fallback to delete
            explorer_fs::trash(path).map_err(|source| FsError::DeleteFailed {
                path: path.clone(),
                source,
            })?;
            Ok(())
        }
        ExplorerChange::DirCreated(path) => {
            let _ = explorer_fs::remove_empty_dir_only(path);
            Ok(())
        }
        ExplorerChange::DirRemoved(path) => {
            explorer_fs::create_dir_all(path)?;
            Ok(())
        }
        ExplorerChange::Renamed { from, to } | ExplorerChange::Moved { from, to } => {
            explorer_fs::rename(to, from)?;
            Ok(())
        }
        ExplorerChange::Copied { dest, .. } => {
            explorer_fs::remove_symlink_safe(dest).map_err(|source| FsError::DeleteFailed {
                path: dest.clone(),
                source,
            })?;
            Ok(())
        }
        ExplorerChange::Batch(changes) => {
            for change in changes.iter().rev() {
                execute_explorer_change_inverse(change)?;
            }
            Ok(())
        }
    }
}

