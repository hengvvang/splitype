//! Cross-block selection, mouse drag tracking, and text mutation.

pub mod actions;
pub mod mouse_drag;
pub mod source_mutation;
pub mod state;

pub use source_mutation::{cross_block_selected_markdown, safe_source_slice};
pub use state::NormalizedCrossBlockSelection;
