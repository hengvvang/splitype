//! Canonical document export formats shared with panel plugins.

use serde::{Deserialize, Serialize};

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
