//! Explorer undo/redo — mirrors Zed's `crates/project_panel/src/undo.rs`.
//!
//! The history stores the forward operation; undoing executes its inverse
//! and pushes the same record onto the redo stack (so redo simply
//! re-executes it). Only reversible operations are recorded — permanent
//! deletes are not.

use std::path::{Path, PathBuf};

use crate::infra::error::ExplorerError;
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
pub fn execute_explorer_change(change: &ExplorerChange) -> Result<(), ExplorerError> {
    match change {
        ExplorerChange::Created { path, is_dir } => {
            if *is_dir {
                std::fs::create_dir_all(path).map_err(|source| ExplorerError::CreateDirFailed {
                    path: path.clone(),
                    source,
                })?;
            } else if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|source| ExplorerError::CreateDirFailed {
                    path: parent.to_path_buf(),
                    source,
                })?;
                std::fs::write(path, "").map_err(|source| ExplorerError::WriteFailed {
                    path: path.clone(),
                    source,
                })?;
            }
            Ok(())
        }
        ExplorerChange::Renamed { from, to } | ExplorerChange::Moved { from, to } => {
            std::fs::rename(from, to).map_err(|source| ExplorerError::RenameFailed {
                from: from.clone(),
                to: to.clone(),
                source,
            })
        }
        ExplorerChange::Copied { source, dest } => {
            if source.is_dir() {
                copy_dir_all(source, dest).map_err(|source_err| ExplorerError::WriteFailed {
                    path: dest.clone(),
                    source: source_err,
                })
            } else {
                std::fs::copy(source, dest)
                    .map(|_| ())
                    .map_err(|source_err| ExplorerError::WriteFailed {
                        path: dest.clone(),
                        source: source_err,
                    })
            }
        }
    }
}

/// Execute the inverse of a recorded operation (undo).
pub fn execute_explorer_change_inverse(change: &ExplorerChange) -> Result<(), ExplorerError> {
    match change {
        ExplorerChange::Created { path, .. } => {
            remove_path_symlink_safe(path).map_err(|source| ExplorerError::DeleteFailed {
                path: path.clone(),
                source,
            })
        }
        ExplorerChange::Renamed { from, to } | ExplorerChange::Moved { from, to } => {
            std::fs::rename(to, from).map_err(|source| ExplorerError::RenameFailed {
                from: to.clone(),
                to: from.clone(),
                source,
            })
        }
        ExplorerChange::Copied { dest, .. } => {
            remove_path_symlink_safe(dest).map_err(|source| ExplorerError::DeleteFailed {
                path: dest.clone(),
                source,
            })
        }
    }
}
