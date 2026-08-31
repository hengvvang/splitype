//! HTML document generation for Markdown export.

pub mod document;
pub mod rewriter;
pub mod styles;

pub use document::{
    contains_tibetan_text, render_chromium_pdf_html_with_base_dir, render_html,
    render_html_with_base_dir,
};
