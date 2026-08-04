//! Panel-specific editing logic — one module per editor panel.
//!
//! All four panels (WYSIWYG, source code, preview, outline) share the same
//! document runtime; these modules hold the small amount of behavior that is
//! specific to a single panel, plus its panel-scoped state.

pub(crate) mod outline;
pub(crate) mod preview;
pub(crate) mod source_code;
pub(crate) mod wysiwyg;

pub(crate) use preview::PreviewState;
pub(crate) use source_code::SourcePanelState;
