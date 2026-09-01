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
}

/// Errors occurring during document export pipelines.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("I/O error during export: {0}")]
    Io(#[from] io::Error),
    #[error("Render error: {0}")]
    Render(String),
}
