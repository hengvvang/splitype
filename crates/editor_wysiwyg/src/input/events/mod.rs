//! WYSIWYG block-level event dispatch and action routing.

pub mod interactions;
pub mod structure_ops;
pub mod table_events;
pub mod text_edits;

pub use crate::document::protocol::{BlockEvent, BlockEventCategory};
