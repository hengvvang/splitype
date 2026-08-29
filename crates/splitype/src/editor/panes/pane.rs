//! Pane trait — the cross-mode plugin contract of the editor family.
//!
//! Every view mode (WYSIWYG, Source Code, Preview) implements [`Pane`];
//! the editor holds one pane state per split leaf and talks to the modes
//! only through this trait. The modes depend on `editor` (for [`Editor`],
//! [`EditorPaneKind`] and the pure data types below); `editor` never
//! depends on a mode.
//!
//! Cross-mode consumers (export, search, outline) read *pure data* through
//! [`Pane::document_source`] / [`Pane::outline_items`] and push *pure
//! ranges* through [`Pane::set_search_matches`] — no mode internals ever
//! cross a crate boundary.

use std::ops::Range;

use gpui::App;

use crate::editor::engine::controller::Editor;
use crate::editor::engine::session::EditorPaneKind;

/// A heading node in the outline HUD. Contract type owned by the `editor`
/// crate.
pub use editor_core::OutlineNode;

/// The plugin contract implemented by every editor pane kind.
pub trait Pane {
    /// Which pane kind this state belongs to.
    fn kind(&self) -> EditorPaneKind;

    /// Pure markdown source of the active tab, as this mode sees it.
    ///
    /// Export, search and outline consume this; the mode decides what
    /// "source" means (WYSIWYG serializes the block tree, Source Code
    /// returns its raw buffer, Preview serializes the shared document).
    fn document_source(&self, editor: &Editor, cx: &App) -> String;

    /// Push in-pane search matches as pure byte ranges (range, is-active).
    ///
    /// Modes highlight these in their own rendering. WYSIWYG highlights at
    /// the block level (the editor syncs `block.search_matches`) and
    /// Preview is read-only, so both are no-ops by design.
    fn set_search_matches(&mut self, matches: &[(Range<usize>, bool)]);

    /// Heading items for the outline HUD (pure data).
    fn outline_items(&self, editor: &Editor, cx: &App) -> Vec<OutlineNode>;
}
