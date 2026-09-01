use crate::document::DocumentSnapshot;
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
    /// The pane can resolve document navigation targets.
    pub navigable: bool,
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

    /// Rebuilds the pane from the authoritative document snapshot. Called on
    /// creation and on every document commit for all panes except the one
    /// that originated the edit.
    fn sync_document(&mut self, document: &DocumentSnapshot, cx: &mut App);

    /// The pane's current serialization of the document, used by the editor
    /// to rebuild the authoritative text after pane-driven edits.
    fn serialize_text(&self, cx: &App) -> Option<String>;

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

    /// Gate: `editable`. Returns the new authoritative text, or `None` when
    /// the pane does not support the operation.
    fn apply_line_prefix(&mut self, _prefix: &str, _cx: &mut App) -> Option<String> {
        None
    }

    /// Gate: `editable`.
    fn apply_snippet(
        &mut self,
        _snippet: &str,
        _caret_offset: usize,
        _cx: &mut App,
    ) -> Option<String> {
        None
    }

    /// Gate: `editable`.
    fn apply_wrapped_or_template(
        &mut self,
        _empty_template: &str,
        _caret_offset_in_empty: usize,
        _wrap_prefix: &str,
        _wrap_suffix: &str,
        _cx: &mut App,
    ) -> Option<String> {
        None
    }

    /// Gate: `editable`.
    fn apply_clear_format(&mut self, _cx: &mut App) -> Option<String> {
        None
    }

    /// Gate: `searchable`.
    fn search_matches(&self, _query: &SearchQuery, _cx: &App) -> Vec<SearchMatch> {
        Vec::new()
    }

    /// Gate: `searchable`.
    fn navigate_to_search_match(&mut self, _match_item: &SearchMatch, _cx: &mut App) {}

    /// Gate: `replaceable`. Returns the new authoritative text, or `None`
    /// when the pane does not support replacement.
    fn replace_match(
        &mut self,
        _match_item: &SearchMatch,
        _replace_with: &str,
        _cx: &mut App,
    ) -> Option<String> {
        None
    }

    /// Gate: `replaceable`.
    fn replace_all_matches(
        &mut self,
        _query: &SearchQuery,
        _replace_with: &str,
        _cx: &mut App,
    ) -> Option<String> {
        None
    }

    /// Gate: `outline`.
    fn outline_headings(&self, _cx: &App) -> Vec<OutlineNode> {
        Vec::new()
    }

    /// Gate: `outline`.
    fn navigate_to_outline(&mut self, _index: usize, _theme: &Theme, _cx: &mut App) {}

    /// Gate: `navigable`.
    fn handle_navigation(
        &mut self,
        _target: &crate::document::NavigationTarget,
        _modifiers: gpui::Modifiers,
        _cx: &mut App,
    ) -> Option<crate::document::NavigationExecutionPlan> {
        None
    }

    // ── Reflection ──────────────────────────────────────────────────────────

    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
