pub(crate) mod close;
pub(crate) mod command;
pub(crate) mod document;
pub(crate) mod editor;
pub(crate) mod explorer;
pub(crate) mod export;
pub(crate) mod input;
pub(crate) mod layout;
pub(crate) mod render;
pub(crate) mod runtime;
pub(crate) mod selection;
pub(crate) mod source_map;
pub(crate) mod status_bar;
pub(crate) mod table_edit;
#[cfg(test)]
pub(crate) mod tests;
pub(crate) mod viewport;

// Re-export key types so submodules can still use `super::Editor`, etc.
pub(crate) use editor::{Editor, InfoDialogKind, RenderedSelectAllCycle, EditMode};
