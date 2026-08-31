//! WYSIWYG input processing: typing, focus, pointer events, selection, paste, and undo history.

pub mod events;
pub mod focus;
pub mod history;
pub mod paste;
pub mod selection;
pub mod typing;

pub use focus::{reset_block_cursor, set_block_selected_range};
pub use typing::is_setext_heading_target;


