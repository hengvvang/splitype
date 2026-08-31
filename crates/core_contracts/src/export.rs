//! Canonical document export formats and error types.

use serde::{Deserialize, Serialize};
use std::io;

/// Supported document export file formats.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    Html,
    Pdf,
}

impl ExportFormat {
    #[inline]
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Html => "html",
            Self::Pdf => "pdf",
        }
    }

    #[inline]
    pub fn file_filter_name(&self) -> &'static str {
        match self {
            Self::Html => "HTML Document",
            Self::Pdf => "PDF Document",
        }
    }
}

/// Errors occurring during document export pipelines.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("I/O error during export: {0}")]
    Io(#[from] io::Error),
    #[error("Render error: {0}")]
    Render(String),
    #[error(
        "No Chrome/Chromium executable was found. Please install Chrome, Chromium, Edge, or Brave to export PDF."
    )]
    ChromiumNotFound,
    #[error("Failed to launch headless browser for PDF export: {0}")]
    ChromiumLaunch(String),
    #[error("PDF export timed out")]
    Timeout,
    #[error("Failed to initialize PDF runtime: {0}")]
    RuntimeInit(String),
    #[error("Export error: {0}")]
    Other(#[from] anyhow::Error),
}
