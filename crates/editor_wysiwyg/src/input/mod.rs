//! WYSIWYG input processing: typing shortcuts, focus management.

pub mod focus;
pub mod typing;

pub use focus::{reset_block_cursor, set_block_selected_range};
pub use typing::is_setext_heading_target;
