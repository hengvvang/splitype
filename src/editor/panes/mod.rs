//! Editor panes: WYSIWYG, source code editor, preview, outline, and document pane container.

pub(crate) mod document_pane;
pub(crate) mod outline;
pub mod preview;
pub(crate) mod source_code;
pub mod wysiwyg;

pub(crate) use preview::PreviewState;
