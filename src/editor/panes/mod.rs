//! Editor panes: WYSIWYG, source code editor, preview, outline, and document view.

pub(crate) mod document_view;
pub(crate) mod outline;
pub mod preview;
pub(crate) mod source_code;
pub mod wysiwyg;

pub(crate) use preview::PreviewState;
