//! `splitype-render` — Syntax highlighting, document exporters (PDF, HTML, Markdown), and rendering engines (LaTeX, Mermaid).

pub mod export;
pub mod plugins;

pub use export::{html, pdf};
pub use plugins::{code_highlight, latex_render, mermaid_render};

#[cfg(test)]
mod tests;

