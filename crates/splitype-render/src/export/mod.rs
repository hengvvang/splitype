//! Document export helpers for HTML and PDF output.
//!
//! Export starts from the same Markdown text used by document saving. The
//! module owns format-specific rendering so editor code only chooses paths and
//! supplies the current theme.

use std::path::Path;

use splitype_infra::theme::Theme;

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

pub use html::render_html_with_base_dir;
pub use splitype_infra::error::ExportError;

/// Renders themed PDF bytes for the current document Markdown.
pub fn render_pdf(
    markdown: &str,
    theme: &Theme,
    title: &str,
    base_path: Option<&Path>,
) -> Result<Vec<u8>, ExportError> {
    pdf::render_pdf(markdown, theme, title, base_path)
}
