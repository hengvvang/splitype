//! Inline filename editor for the explorer: buffer operations, validation,
//! and the custom GPUI input element. (The `EntityInputHandler` IME bridge
//! for the shell lives in `crates/app` — orphan rule.)

pub mod editor;
pub(crate) mod operations;

pub use editor::{FilenameEditor, FilenameEditorEvent};
