//! Application views — window definitions and pane layout.
//!
//! Each top-level view (editor window, settings window, explorer sidebar)
//! is one module; the editor window further splits into its panels. The
//! tiled layout that splits and rearranges these views lives in `layout`.

pub(crate) mod editor;
pub(crate) mod explorer;
pub(crate) mod layout;
pub(crate) mod settings;
