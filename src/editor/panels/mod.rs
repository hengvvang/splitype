//! Editor inner panels — the `EditorInnerPanelKind` views (the welcome
//! panel plus the four editing panels WYSIWYG / source code / preview /
//! outline) and the inner panel layout rendering.

pub(crate) mod layout;
pub(crate) mod outline;
pub(crate) mod preview;
pub(crate) mod source_code;
pub(crate) mod wysiwyg;

pub(crate) use preview::PreviewState;
pub(crate) use source_code::SourceCodePanelRuntime;
