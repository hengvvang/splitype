//! editor_preview — the read-only rendered Markdown preview pane.
//!
//! Standard-first: the preview tree is built from the CommonMark parse
//! (100% CommonMark), unlike the WYSIWYG 1:1-line parser. Rendering
//! styles come from `editor_wysiwyg`'s public presentation services;
//! the preview never touches WYSIWYG editing internals.
//!
//! The pane state implements [`editor::Pane`]. Editor-facing glue
//! (refresh scheduling, mouse handling) stays in the coordinating crate
//! until the `Editor` entity converges.

pub mod node;
mod selection;
mod state;

pub use node::{PreviewBlock, blocks_to_preview_tree};
pub use selection::{PreviewEndpoint, PreviewSelectionRange};
pub use state::PreviewState;
