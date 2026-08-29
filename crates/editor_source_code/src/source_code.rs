//! editor_source_code — core mode 2: the raw Markdown source editor.
//!
//! Owns the source-code pane state (text buffer, cursor, selection, drag,
//! search-match ranges, highlight cache), the pure text-run builder for
//! syntax highlighting, the virtualized rendering element, and the input
//! handling. The element reads state through the [`SourceStateView`]
//! snapshot interface and forwards IME registration through [`SourceIme`];
//! coordination-layer actions go through `editor::PaneHost`. The pane
//! state shares nothing with the WYSIWYG world — no markdown, no tree, no
//! highlight service (D14-B: each core self-hosts its own copies).

mod element;
mod highlight;
mod input;
mod state;

pub use element::{SourceCodeViewElement, SourceIme, SourceStateView, SourceViewSnapshot};
pub use highlight::build_line_text_runs;
pub use input::{handle_key_down, handle_mouse_down, handle_mouse_move, handle_mouse_up};
pub use state::SourceCodeState;
