//! Optional document-routing role for window panels.
//!
//! Panels that manage documents implement this trait, and their plugin
//! exports an adapter casting its panel view to this role; the composition
//! root registers that adapter by kind. The shell routes document operations
//! through it without knowing the concrete type. Any plugin can provide a
//! document panel and take over the built-in editor's role.

use crate::document::{DocumentHost, DocumentId, TabKind};
use crate::export::ExportFormat;
use gpui::{App, Window};
use platform_contracts::PanelView;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The document-routing role of a [`PanelView`]: manages tabs of editable
/// files and answers the shell's document lifecycle questions.
///
/// All methods take plain `App`/`Window` handles; implementations delegate
/// to their own entities. The shell only ever talks to panels through this
/// trait and [`PanelView`].
pub trait DocumentPanel: PanelView {
    /// Wires the shell's [`DocumentHost`] into this panel after the panel
    /// enters the shell. Implementations must re-sync any already-open
    /// documents once the host arrives.
    fn attach_document_host(&mut self, host: Arc<dyn DocumentHost>, cx: &mut App);

    /// Seeds the panel with the window's initial document: raw Markdown text
    /// and an optional backing path. An empty text with no path leaves the
    /// panel with an empty session.
    fn load_initial_document(&mut self, text: String, path: Option<PathBuf>, cx: &mut App);

    /// Opens `path` as a tab, activating an existing tab when already open.
    fn open_file(&mut self, path: &Path, kind: TabKind, window: &mut Window, cx: &mut App);

    /// Path of the active tab, if any.
    fn active_tab_path(&self, cx: &App) -> Option<PathBuf>;

    /// Display name of the tab at `index`, if it exists.
    fn tab_display_name(&self, index: usize, cx: &App) -> Option<String>;

    /// Buffer identities of every open document view in this panel
    /// (deduplicated), for window-level close-guard aggregation.
    fn document_buffer_ids(&self, _cx: &App) -> Vec<DocumentId> {
        Vec::new()
    }

    /// Save the tab at `index`.
    fn save_tab_at(&mut self, index: usize, window: &mut Window, cx: &mut App);

    /// Close the tab at `index` without prompting.
    fn close_tab(&mut self, index: usize, cx: &mut App);

    /// Discard unsaved changes of the tab at `index` and close it.
    fn discard_tab_at(&mut self, index: usize, cx: &mut App);

    /// Close every tab without prompting.
    fn clear_tabs(&mut self, cx: &mut App);

    // ── Unsaved-changes confirmation dialog ─────────────────────────────

    /// Whether any tab currently requests the unsaved-changes dialog.
    fn has_unsaved_dialog(&self, cx: &App) -> bool;

    /// Cancel the unsaved-changes dialog.
    fn cancel_close_dialog(&mut self, cx: &mut App);

    /// Save pending documents and finish the close flow.
    fn save_and_close_dialog(&mut self, window: &mut Window, cx: &mut App);

    /// Discard pending changes and finish the close flow.
    fn discard_and_close_dialog(&mut self, cx: &mut App);

    // ── Drop-replace confirmation dialog ────────────────────────────────

    /// Whether any tab currently requests the drop-replace dialog.
    fn has_drop_replace_dialog(&self, cx: &App) -> bool;

    /// Cancel the drop-replace dialog.
    fn cancel_drop_replace_dialog(&mut self, cx: &mut App);

    /// Save the current document, then apply the pending drop replacement.
    fn save_and_replace_pending_drop(&mut self, window: &mut Window, cx: &mut App);

    /// Discard the current document and apply the pending drop replacement.
    fn discard_pending_drop_replace(&mut self, window: &mut Window, cx: &mut App);

    // ── Menu-dispatched document commands ───────────────────────────────

    /// Save the active document, prompting for a path when untitled.
    fn request_save_document(&mut self, cx: &mut App);

    /// Save the active document to a new location.
    fn request_save_document_as(&mut self, cx: &mut App);

    /// Save the active document directly.
    fn save_document(&mut self, window: &mut Window, cx: &mut App);

    /// Save the active document to a new location directly.
    fn save_document_as(&mut self, window: &mut Window, cx: &mut App);

    /// Export the active document in the given format via a path prompt.
    fn export_document(&mut self, format: ExportFormat, window: &mut Window, cx: &mut App);
}
