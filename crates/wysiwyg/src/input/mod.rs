//! WYSIWYG input processing: typing, focus, pointer events, paste.

pub mod focus;
pub mod paste;
pub mod typing;

pub use focus::{reset_block_cursor, set_block_selected_range};
pub use typing::is_setext_heading_target;
