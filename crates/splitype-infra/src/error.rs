//! Standardized Domain Error Hierarchy for Splitype.
//!
//! Provides rich, typed error enums for filesystem, configuration, export,
//! and UI operations, enabling structured error bubbling and contextual diagnostics.

use std::fmt;
use std::path::PathBuf;

/// Root domain error encompassing subsystem failures.
#[derive(Debug)]
pub enum SplitypeError {
    Explorer(ExplorerError),
    Config(ConfigError),
    Export(ExportError),
    I18n(I18nError),
    Io(std::io::Error),
}

impl fmt::Display for SplitypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Explorer(err) => write!(f, "Explorer error: {err}"),
            Self::Config(err) => write!(f, "Config error: {err}"),
            Self::Export(err) => write!(f, "Export error: {err}"),
            Self::I18n(err) => write!(f, "I18n error: {err}"),
            Self::Io(err) => write!(f, "I/O error: {err}"),
        }
    }
}

impl std::error::Error for SplitypeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Explorer(err) => Some(err),
            Self::Config(err) => Some(err),
            Self::Export(err) => Some(err),
            Self::I18n(err) => Some(err),
            Self::Io(err) => Some(err),
        }
    }
}

impl From<ExplorerError> for SplitypeError {
    fn from(err: ExplorerError) -> Self {
        Self::Explorer(err)
    }
}

impl From<ConfigError> for SplitypeError {
    fn from(err: ConfigError) -> Self {
        Self::Config(err)
    }
}

impl From<ExportError> for SplitypeError {
    fn from(err: ExportError) -> Self {
        Self::Export(err)
    }
}

impl From<I18nError> for SplitypeError {
    fn from(err: I18nError) -> Self {
        Self::I18n(err)
    }
}

impl From<std::io::Error> for SplitypeError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

/// Errors originating from workspace scanning, worktree indexing, and file manipulations.
#[derive(Debug)]
pub enum ExplorerError {
    NotFound { path: PathBuf },
    ReadFailed { path: PathBuf, source: std::io::Error },
    WriteFailed { path: PathBuf, source: std::io::Error },
    CreateDirFailed { path: PathBuf, source: std::io::Error },
    DeleteFailed { path: PathBuf, source: std::io::Error },
    RenameFailed { from: PathBuf, to: PathBuf, source: std::io::Error },
    InvalidPath(PathBuf),
}

impl fmt::Display for ExplorerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { path } => write!(f, "Path '{}' was not found", path.display()),
            Self::ReadFailed { path, source } => {
                write!(f, "Failed to read file '{}': {source}", path.display())
            }
            Self::WriteFailed { path, source } => {
                write!(f, "Failed to write file '{}': {source}", path.display())
            }
            Self::CreateDirFailed { path, source } => {
                write!(f, "Failed to create directory '{}': {source}", path.display())
            }
            Self::DeleteFailed { path, source } => {
                write!(f, "Failed to delete '{}': {source}", path.display())
            }
            Self::RenameFailed { from, to, source } => {
                write!(
                    f,
                    "Failed to rename from '{}' to '{}': {source}",
                    from.display(),
                    to.display()
                )
            }
            Self::InvalidPath(path) => write!(f, "Invalid path '{}'", path.display()),
        }
    }
}

impl std::error::Error for ExplorerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ReadFailed { source, .. }
            | Self::WriteFailed { source, .. }
            | Self::CreateDirFailed { source, .. }
            | Self::DeleteFailed { source, .. }
            | Self::RenameFailed { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Errors originating from configuration loading, parsing, and persistence.
#[derive(Debug)]
pub enum ConfigError {
    ReadFailed { path: PathBuf, source: std::io::Error },
    ParseFailed(String),
    SerializeFailed(String),
    WriteFailed { path: PathBuf, source: std::io::Error },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadFailed { path, source } => {
                write!(f, "Failed to read config file '{}': {source}", path.display())
            }
            Self::ParseFailed(msg) => write!(f, "Failed to parse config JSONC: {msg}"),
            Self::SerializeFailed(msg) => write!(f, "Failed to serialize config JSON: {msg}"),
            Self::WriteFailed { path, source } => {
                write!(f, "Failed to persist config file '{}': {source}", path.display())
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ReadFailed { source, .. } | Self::WriteFailed { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Errors originating from document export (HTML, PDF, etc.).
#[derive(Debug)]
pub enum ExportError {
    HtmlRenderFailed(String),
    ChromiumNotFound,
    ChromiumLaunchFailed(String),
    Timeout,
    RuntimeInit(String),
    PdfProcessFailed { status: Option<i32>, details: String },
    IoFailed { path: PathBuf, source: std::io::Error },
    Io(std::io::Error),
    TaskSpawnFailed(String),
    TaskAborted,
    Render(String),
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HtmlRenderFailed(msg) => write!(f, "Failed to render HTML: {msg}"),
            Self::ChromiumNotFound => {
                write!(f, "PDF export requires Chromium/Chrome, which was not found")
            }
            Self::ChromiumLaunchFailed(msg) => {
                write!(f, "Failed to launch Chromium for PDF export: {msg}")
            }
            Self::Timeout => write!(f, "PDF export timed out"),
            Self::RuntimeInit(msg) => write!(f, "Failed to initialize PDF export runtime: {msg}"),
            Self::PdfProcessFailed { status, details } => {
                write!(
                    f,
                    "PDF generator process failed with status {status:?}: {details}"
                )
            }
            Self::IoFailed { path, source } => {
                write!(f, "Export I/O failed for '{}': {source}", path.display())
            }
            Self::Io(err) => write!(f, "I/O error during export: {err}"),
            Self::TaskSpawnFailed(msg) => write!(f, "Failed to start export task: {msg}"),
            Self::TaskAborted => write!(f, "Export task stopped before reporting a result"),
            Self::Render(msg) => write!(f, "PDF export rendering error: {msg}"),
        }
    }
}

impl std::error::Error for ExportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoFailed { source, .. } => Some(source),
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ExportError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

/// Errors originating from language pack and translation catalog operations.
#[derive(Debug)]
pub enum I18nError {
    PackLoadFailed { language_id: String, details: String },
    BuiltinConflict(String),
}

impl fmt::Display for I18nError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PackLoadFailed { language_id, details } => {
                write!(f, "Failed to load language pack '{language_id}': {details}")
            }
            Self::BuiltinConflict(lang) => {
                write!(f, "Custom language ID '{lang}' cannot override built-in languages")
            }
        }
    }
}

impl std::error::Error for I18nError {}
