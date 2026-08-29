//! Editor panes (WYSIWYG, Source Code, Preview): the mode coordination
//! shells. The document view shell (`document/`) and the outline HUD
//! (`outline/`) are editor-level, not panes, and live beside this module.

pub mod preview;
pub(crate) mod source_code;
pub mod wysiwyg;
