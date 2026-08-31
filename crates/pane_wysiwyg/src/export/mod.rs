//! Document export helpers for HTML and PDF output.

use std::path::Path;
use theme::Theme;

pub mod html;
pub mod pdf;

use core_contracts::ExportError;
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
