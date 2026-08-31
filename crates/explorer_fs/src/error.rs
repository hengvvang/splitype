use std::path::PathBuf;

/// Typed filesystem operation failure.
#[derive(Debug, thiserror::Error)]
pub enum FsError {
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
    SymlinkError { path: PathBuf, message: String },
}
