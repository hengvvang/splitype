//! Content rendering pipelines — document content to visual artifacts.
//!
//! Syntax highlighting (`code_highlight`), LaTeX and Mermaid SVG rendering
//! (`latex_render`, `mermaid_render`), and themed HTML/PDF export (`export`).
//! All pipelines consume document content (plus theme) and produce display
//! or export output; they may use `infra` (e.g. remote image fetching).

pub(crate) mod code_highlight;
pub(crate) mod export;
pub(crate) mod latex_render;
pub(crate) mod mermaid_render;
