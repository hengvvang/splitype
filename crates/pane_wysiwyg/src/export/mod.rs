//! Document export helpers for HTML and PDF output.
//!
//! Export starts from the same Markdown text used by document saving. The
//! module owns format-specific rendering so editor code only chooses paths and
//! supplies the current theme.

use std::path::Path;
use theme::Theme;

pub mod html;
pub mod pdf;

/// Export target selected from the app menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    /// Full HTML document with embedded theme CSS.
    Html,
    /// PDF bytes rendered from the themed HTML document.
    Pdf,
}

impl ExportFormat {
    /// File extension used for save-dialog defaults.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Html => "html",
            Self::Pdf => "pdf",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("Failed to initialize PDF runtime: {0}")]
    RuntimeInit(String),
    #[error("Chromium executable not found. Please ensure Chrome, Edge, Chromium, or Brave is installed.")]
    ChromiumNotFound,
    #[error("Failed to launch Chromium: {0}")]
    ChromiumLaunchFailed(String),
    #[error("PDF rendering failed: {0}")]
    Render(String),
    #[error("PDF export timed out")]
    Timeout,
    #[error("I/O error during export: {0}")]
    Io(#[from] std::io::Error),
    #[error("Export error: {0}")]
    Other(#[from] anyhow::Error),
}


pub use html::{render_html, render_html_with_base_dir};

/// Renders themed PDF bytes for the current document Markdown.
pub fn render_pdf(
    markdown: &str,
    theme: &Theme,
    title: &str,
    base_path: Option<&Path>,
) -> Result<Vec<u8>, ExportError> {
    pdf::render_pdf(markdown, theme, title, base_path)
}

