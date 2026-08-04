//! Panel-specific editing logic — one module per editor panel.
//!
//! All four panels (WYSIWYG, source code, preview, outline) share the same
//! document runtime; these modules hold the behavior and rendering of a
//! single panel. `blocks` holds the block views shared by the WYSIWYG and
//! preview panels.

pub(crate) mod blocks;
pub(crate) mod outline;
pub(crate) mod preview;
pub(crate) mod source_code;
pub(crate) mod wysiwyg;

pub(crate) use preview::PreviewState;
pub(crate) use source_code::SourcePanelState;
