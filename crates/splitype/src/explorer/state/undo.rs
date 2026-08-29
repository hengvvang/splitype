//! Explorer undo/redo — mirrors Zed's `crates/project_panel/src/undo.rs`.
//!
//! The history stores the forward operation; undoing executes its inverse
//! and pushes the same record onto the redo stack (so redo simply
//! re-executes it). Only reversible operations are recorded — permanent
//! deletes are not.

use std::path::{Path, PathBuf};

use super::utils::copy_dir_all;

#[derive(Debug, thiserror::Error)]
pub enum ExplorerError {
    #[error("Failed to create directory at {path:?}: {source}")]
    CreateDirFailed {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Failed to write file at {path:?}: {source}")]
    WriteFailed {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Failed to rename/move from {from:?} to {to:?}: {source}")]
    RenameFailed {
        from: PathBuf,
        to: PathBuf,
        source: std::io::Error,
    },
    #[error("Failed to delete {path:?}: {source}")]
    DeleteFailed {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Symlink error for {path:?}: {message}")]
    SymlinkError {
        path: PathBuf,
        message: String,
    },
}


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

pub fn remove_empty_dir_only(path: &Path) -> std::io::Result<()> {
    if path.is_dir() && std::fs::read_dir(path)?.next().is_none() {
        std::fs::remove_dir(path)?;
    }
    Ok(())
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
        ExplorerChange::DirCreated(path) => {
            std::fs::create_dir_all(path).map_err(|source| ExplorerError::CreateDirFailed {
                path: path.clone(),
                source,
            })?;
            Ok(())
        }
        ExplorerChange::DirRemoved(path) => {
            let _ = remove_empty_dir_only(path);
            Ok(())
        }
        ExplorerChange::Renamed { from, to } | ExplorerChange::Moved { from, to } => {
            if std::fs::rename(from, to).is_err() {
                // Fallback for cross-device moves: copy then delete source
                if from.is_dir() {
                    copy_dir_all(from, to).map_err(|source| ExplorerError::RenameFailed {
                        from: from.clone(),
                        to: to.clone(),
                        source,
                    })?;
                    let _ = remove_path_symlink_safe(from);
                } else {
                    std::fs::copy(from, to).map_err(|source| ExplorerError::RenameFailed {
                        from: from.clone(),
                        to: to.clone(),
                        source,
                    })?;
                    let _ = std::fs::remove_file(from);
                }
            }
            Ok(())
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
        ExplorerChange::Batch(changes) => {
            for change in changes {
                execute_explorer_change(change)?;
            }
            Ok(())
        }
    }
}

/// Execute the inverse of a recorded operation (undo).
pub fn execute_explorer_change_inverse(change: &ExplorerChange) -> Result<(), ExplorerError> {
    match change {
        ExplorerChange::Created { path, .. } => {
            // First try trash (recoverable), fallback to delete
            if let Err(_) = trash::delete(path) {
                remove_path_symlink_safe(path).map_err(|source| ExplorerError::DeleteFailed {
                    path: path.clone(),
                    source,
                })?;
            }
            Ok(())
        }
        ExplorerChange::DirCreated(path) => {
            let _ = remove_empty_dir_only(path);
            Ok(())
        }
        ExplorerChange::DirRemoved(path) => {
            std::fs::create_dir_all(path).map_err(|source| ExplorerError::CreateDirFailed {
                path: path.clone(),
                source,
            })?;
            Ok(())
        }
        ExplorerChange::Renamed { from, to } | ExplorerChange::Moved { from, to } => {
            if std::fs::rename(to, from).is_err() {
                // Fallback for cross-device moves: copy then delete source
                if to.is_dir() {
                    copy_dir_all(to, from).map_err(|source| ExplorerError::RenameFailed {
                        from: to.clone(),
                        to: from.clone(),
                        source,
                    })?;
                    let _ = remove_path_symlink_safe(to);
                } else {
                    std::fs::copy(to, from).map_err(|source| ExplorerError::RenameFailed {
                        from: to.clone(),
                        to: from.clone(),
                        source,
                    })?;
                    let _ = std::fs::remove_file(to);
                }
            }
            Ok(())
        }
        ExplorerChange::Copied { dest, .. } => {
            remove_path_symlink_safe(dest).map_err(|source| ExplorerError::DeleteFailed {
                path: dest.clone(),
                source,
            })
        }
        ExplorerChange::Batch(changes) => {
            for change in changes.iter().rev() {
                execute_explorer_change_inverse(change)?;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_undo_record_and_destination() {
        let mut history = ExplorerUndoHistory::default();
        let change1 = ExplorerChange::Created {
            path: PathBuf::from("/p/1.md"),
            is_dir: false,
        };
        let change2 = ExplorerChange::Created {
            path: PathBuf::from("/p/2.md"),
            is_dir: false,
        };
        let batch = ExplorerChange::Batch(vec![change1, change2]);
        assert_eq!(
            explorer_change_destination(&batch),
            Some(Path::new("/p/2.md"))
        );

        history.record(batch);
        assert!(history.can_undo());
        assert!(!history.can_redo());
    }
}
