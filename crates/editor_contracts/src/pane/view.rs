use crate::document::DocumentSnapshot;
use crate::edit::EditTransaction;
use crate::outline::OutlineNode;

use crate::pane::{PaneHost, PaneId, PaneKind, PaneRenderContext};
use crate::search::{SearchMatch, SearchQuery};
use gpui::{AnyElement, App, FocusHandle, Window};
use std::any::Any;
use theme::Theme;

/// Optional behaviors a pane may offer.
///
/// This is the single source of truth for what a host may invoke: every
/// optional [`PaneView`] method below maps to exactly one capability, and
/// hosts MUST check the matching flag before calling it. The default method
/// bodies only exist so minimal panes can ignore whole behavior families —
/// they are never a substitute for the capability check.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PaneCapabilities {
    /// The pane can mutate the document through `apply_*` commands.
    pub editable: bool,
    /// The pane can answer `search_matches` and jump to them.
    pub searchable: bool,
    /// The pane can apply search replacements through `replace_*`.
    pub replaceable: bool,
    /// The pane can produce `outline_headings` and navigate to them.
    pub outline: bool,
}

/// A pane instance runs on the GPUI UI thread only; it must not be shared
/// across threads. Descriptors stay `Send + Sync` because the registry may
/// live in a process-global lock.
pub trait PaneView: Any + 'static {
    // ── Identity and lifecycle (always required) ────────────────────────────

    fn kind(&self) -> PaneKind;

    /// The optional behaviors this pane supports. The default is empty —
    /// panes opt in explicitly.
    fn capabilities(&self) -> PaneCapabilities {
        PaneCapabilities::default()
    }

    /// Converges the pane to the given snapshot of its document. Called on
    /// creation and whenever the shared buffer notifies a change; the pane
    /// must no-op when it is already at this revision (the originating pane
    /// re-receives its own edit this way and simply records the revision).
    fn sync_document(&mut self, document: &DocumentSnapshot, cx: &mut App);

    /// The pane's current text of the document. The host commits it into the
    /// shared buffer after pane-driven edits; read-only panes return `None`.
    fn document_text(&self, cx: &App) -> Option<String>;

    fn render(&mut self, ctx: &PaneRenderContext, window: &mut Window, cx: &mut App) -> AnyElement;

    /// The pane's keyboard focus handle, when it owns one.
    fn focus_handle(&self, _cx: &App) -> Option<FocusHandle> {
        None
    }

    /// Cursor position as (1-based line, 1-based column) for status displays.
    fn cursor_position(&self, _cx: &App) -> Option<(usize, usize)> {
        None
    }

    /// Routes a raw key event to the pane; returns whether it was consumed.
    /// Input routing is universal — every pane receives events regardless of
    /// capabilities; the default means "not consumed".
    fn handle_key_down(
        &mut self,
        _pane_id: PaneId,
        _event: &gpui::KeyDownEvent,
        _window: &mut Window,
        _cx: &mut App,
        _host: &dyn PaneHost,
    ) -> bool {
        false
    }

    fn handle_mouse_down(
        &mut self,
        _pane_id: PaneId,
        _event: &gpui::MouseDownEvent,
        _window: &mut Window,
        _cx: &mut App,
    ) {
    }

    fn handle_mouse_move(
        &mut self,
        _pane_id: PaneId,
        _event: &gpui::MouseMoveEvent,
        _window: &mut Window,
        _cx: &mut App,
    ) {
    }

    fn handle_mouse_up(
        &mut self,
        _pane_id: PaneId,
        _event: &gpui::MouseUpEvent,
        _window: &mut Window,
        _cx: &mut App,
    ) {
    }

    // ── Optional behaviors, gated by [`PaneView::capabilities`] ─────────────

    /// Gate: `editable`. The text covered by the pane's current selection,
    /// or `None` when there is no selection. Used by the editor's unified
    /// Copy/Cut actions; panes never touch the platform clipboard directly.
    fn selected_text(&self, _cx: &App) -> Option<String> {
        None
    }

    /// Gate: `editable`. Deletes the pane's current selection and returns
    /// the resulting edit transaction (new document text, undo metadata),
    /// or `None` when there is nothing to delete. Used by the editor's Cut
    /// action after copying the selection to the clipboard.
    fn delete_selection(&mut self, _cx: &mut App) -> Option<EditTransaction> {
        None
    }

    /// Gate: `editable`. Inserts `text` at the pane's cursors (replacing
    /// any selection) and returns the resulting edit transaction, or `None`
    /// when the pane cannot accept text. Used by the editor's Paste action.
    fn insert_text(&mut self, _text: &str, _cx: &mut App) -> Option<EditTransaction> {
        None
    }

    /// Gate: `editable`. Selects the whole document; no text change, so no
    /// commit follows. Used by the editor's Select All action.
    fn select_all(&mut self, _cx: &mut App) {}

    /// Gate: `searchable`.
    fn search_matches(&self, _query: &SearchQuery, _cx: &App) -> Vec<SearchMatch> {
        Vec::new()
    }

    /// Gate: `searchable`. Brings the match into view inside the pane and
    /// returns the target scroll Y offset in content coordinates, or `None`
    /// when the pane cannot locate the match. The host applies the offset to
    /// the pane's scroll handle — the same convention as
    /// [`PaneView::navigate_to_outline`].
    fn navigate_to_search_match(
        &mut self,
        _match_item: &SearchMatch,
        _cx: &mut App,
    ) -> Option<f32> {
        None
    }

    /// Gate: `searchable`. Replaces the pane's rendered search-match
    /// decorations. `matches` holds the matches belonging to this pane's
    /// document; `active_index` marks the currently selected one.
    fn set_search_highlights(
        &mut self,
        _matches: &[SearchMatch],
        _active_index: Option<usize>,
        _cx: &mut App,
    ) {
    }

    /// Gate: `replaceable`. Returns the edit transaction carrying the new
    /// document text, or `None` when the pane does not support replacement.
    fn replace_match(
        &mut self,
        _match_item: &SearchMatch,
        _replace_with: &str,
        _cx: &mut App,
    ) -> Option<EditTransaction> {
        None
    }

    /// Gate: `replaceable`.
    fn replace_all_matches(
        &mut self,
        _query: &SearchQuery,
        _replace_with: &str,
        _cx: &mut App,
    ) -> Option<EditTransaction> {
        None
    }

    /// Gate: `outline`.
    fn outline_headings(&self, _cx: &App) -> Vec<OutlineNode> {
        Vec::new()
    }

    /// Gate: `outline`.
    fn navigate_to_outline(&mut self, _index: usize, _theme: &Theme, _cx: &mut App) -> Option<f32> {
        None
    }

    // ── Reflection ──────────────────────────────────────────────────────────

    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
