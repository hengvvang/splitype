//! editor_preview — the read-only rendered Markdown preview pane.
//!
//! Standard-first: the preview tree is built from the CommonMark parse
//! (100% CommonMark), unlike the WYSIWYG 1:1-line parser. Rendering
//! styles come from `editor_wysiwyg`'s public presentation services;
//! the preview never touches WYSIWYG editing internals.
//!
//! The pane state implements [`editor_model::Pane`]. The crate owns its full
//! presentation (block renderers, footnote section, quote guides) and
//! input handling (drag selection); the coordinating crate only refreshes
//! the tree, routes focus and hands over the scroll shell through
//! [`editor_model::PaneRenderContext`].

pub mod node;
pub mod render;
pub mod outline;
pub mod search;
mod context;
mod input;
mod selection;
mod state;

pub use context::{build_preview_footnote_registry, sync_preview_block_context};
pub use input::{handle_mouse_down, handle_mouse_move, handle_mouse_up, selected_text};
pub use node::{PreviewBlock, blocks_to_preview_tree};
pub use render::render_preview_pane;
pub use selection::{PreviewEndpoint, PreviewSelectionRange};
pub use state::PreviewState;
