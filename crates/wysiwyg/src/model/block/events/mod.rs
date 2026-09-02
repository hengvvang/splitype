//! Block-local event handlers: keyboard, mouse, clipboard, inline style,
//! structure navigation and table-grow interactions that mutate the
//! block itself and emit [`BlockEvent`]s for the editor to resolve.
//!
//! These implement [`Block`] (orphan rule: they must live in this crate);
//! the editor-side resolution of the emitted events lives in
//! [`crate::pane::controller`].

pub mod clipboard;
pub mod code_language;
pub mod inline_style;
pub mod mouse;
pub mod navigation;
pub mod table_grow;
pub mod text_edit;
