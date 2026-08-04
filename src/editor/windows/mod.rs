//! Editor window internals — the editor window's own sub-windows: chrome
//! state, window commands, the tiled layout, and the four content panels
//! (WYSIWYG, source code, preview, outline) with their shared block views.

pub mod actions;
pub mod blocks;
pub mod chrome;
pub mod commands;
pub mod file;
pub mod keybindings;
pub mod layout;
pub mod menu;
pub(crate) mod outline;
pub(crate) mod preview;
pub(crate) mod source_code;
pub(crate) mod wysiwyg;

pub(crate) use preview::PreviewState;
pub(crate) use source_code::SourcePanelState;
