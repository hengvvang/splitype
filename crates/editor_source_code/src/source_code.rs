//! editor_source_code — core mode 2: the raw Markdown source editor.
//!
//! Owns the source-code pane state (text buffer, cursor, selection, drag,
//! search-match ranges, highlight cache) and the pure text-run builder
//! for syntax highlighting. The rendering element and the editor-side
//! glue stay in the coordinating crate until the `Editor` entity
//! converges into `editor`.
//!
//! The pane state implements [`editor::Pane`]; it shares nothing with the
//! WYSIWYG world — no markdown, no tree, no highlight service (D14-B:
//! each core self-hosts its own copies).

mod highlight;
mod state;

pub use highlight::build_line_text_runs;
pub use state::SourceCodeState;
