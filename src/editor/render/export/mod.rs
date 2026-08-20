//! Document export helpers for HTML and PDF output.
//!
//! Export starts from the same Markdown text used by document saving. The
//! module owns format-specific rendering so editor code only chooses paths and
//! supplies the current theme.

use std::path::Path;

use crate::infra::theme::Theme;

mod html;
mod pdf;

/// Export target selected from the app menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExportFormat {
    /// Full HTML document with embedded theme CSS.
    Html,
    /// PDF bytes rendered from the themed HTML document.
    Pdf,
}

impl ExportFormat {
    /// File extension used for save-dialog defaults.
    pub(crate) fn extension(self) -> &'static str {
        match self {
            Self::Html => "html",
            Self::Pdf => "pdf",
        }
    }
}

pub(crate) use html::render_html_with_base_dir;

/// Typed error for document export failures.
#[derive(Debug)]
pub(crate) enum ExportError {
    /// Chromium headless browser executable was not found or failed to launch.
    ChromiumLaunchFailed(String),
    /// PDF rendering timed out.
    Timeout,
    /// Runtime initialization failure.
    RuntimeInit(String),
    /// File I/O or temporary file error.
    Io(std::io::Error),
    /// Generic rendering error with context.
    Render(String),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ChromiumLaunchFailed(msg) => write!(f, "Failed to launch Chromium for PDF export: {msg}"),
            Self::Timeout => write!(f, "PDF export timed out"),
            Self::RuntimeInit(msg) => write!(f, "Failed to initialize PDF export runtime: {msg}"),
            Self::Io(err) => write!(f, "I/O error during export: {err}"),
            Self::Render(msg) => write!(f, "PDF export rendering error: {msg}"),
        }
    }
}

impl std::error::Error for ExportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
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

/// Renders themed PDF bytes for the current document Markdown.
pub(crate) fn render_pdf(
    markdown: &str,
    theme: &Theme,
    title: &str,
    base_path: Option<&Path>,
) -> Result<Vec<u8>, ExportError> {
    pdf::render_pdf(markdown, theme, title, base_path)
}
