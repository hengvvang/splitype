//! Editor inner panels — the `EditorInnerPanelKind` views (the welcome
//! panel plus the four editing panels WYSIWYG / source code / preview /
//! outline) and the inner panel layout rendering.
//!
//! This module also hosts the *state* of the top-level sidebar and settings
//! views: `explorer/` (file-tree model, owned by the Editor entity) and
//! `settings.rs` (settings panel state). The views themselves live in
//! `crate::explorer` / `crate::settings` and depend on this module one-way.

pub(crate) mod layout;
pub(crate) mod outline;
pub(crate) mod panel_types;
pub(crate) mod panels_state;
pub(crate) mod preview;
pub(crate) mod source_code;
pub(crate) mod wysiwyg;

pub(crate) mod explorer;
pub(crate) mod settings;

pub(crate) use preview::PreviewState;
pub(crate) use source_code::SourceCodePanelRuntime;
