use crate::document::DocumentSnapshot;
use crate::outline::OutlineHeading;
use crate::pane::{PaneHost, PaneId, PaneKind, PaneRenderContext};
use crate::search::{SearchMatch, SearchQuery};
use gpui::{AnyElement, App, FocusHandle, Window};
use std::any::Any;
use theme::Theme;

/// Optional behaviors a pane may offer. Hosts consult this before invoking
/// the corresponding [`PaneView`] methods so unsupported operations never
/// produce phantom edits, dirty state, or navigation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PaneCapabilities {
    /// The pane can mutate the document through `apply_*` commands.
    pub editable: bool,
    /// The pane can answer `search_matches`.
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
    fn kind(&self) -> PaneKind;

    /// The optional behaviors this pane supports. The default is empty —
    /// panes opt in explicitly.
    fn capabilities(&self) -> PaneCapabilities {
        PaneCapabilities::default()
    }

    fn render(&mut self, ctx: &PaneRenderContext, window: &mut Window, cx: &mut App) -> AnyElement;

    fn focus_handle(&self, _cx: &App) -> Option<FocusHandle> {
        None
    }

    fn cursor_position(&self, _cx: &App) -> Option<(usize, usize)> {
        None
    }

    fn sync_document(&mut self, _document: &DocumentSnapshot, _cx: &mut App) {}

    fn serialize_text(&self, _cx: &App) -> Option<String> {
        None
    }

    fn outline_headings(&self, _cx: &App) -> Vec<OutlineHeading> {
        Vec::new()
    }

    fn navigate_to_outline(&mut self, _index: usize, _theme: &Theme, _cx: &mut App) {}

    fn search_matches(&self, _query: &SearchQuery, _cx: &App) -> Vec<SearchMatch> {
        Vec::new()
    }

    fn navigate_to_search_match(&mut self, _match_item: &SearchMatch, _cx: &mut App) {}

    /// Replaces one search match and returns the new authoritative document
    /// text, or `None` when this pane does not support replacement.
    fn replace_match(
        &mut self,
        _match_item: &SearchMatch,
        _replace_with: &str,
        _cx: &mut App,
    ) -> Option<String> {
        None
    }

    fn replace_all_matches(
        &mut self,
        _query: &SearchQuery,
        _replace_with: &str,
        _cx: &mut App,
    ) -> Option<String> {
        None
    }

    fn apply_line_prefix(&mut self, _prefix: &str, _cx: &mut App) -> Option<String> {
        None
    }

    fn apply_snippet(
        &mut self,
        _snippet: &str,
        _caret_offset: usize,
        _cx: &mut App,
    ) -> Option<String> {
        None
    }

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

    fn apply_clear_format(&mut self, _cx: &mut App) -> Option<String> {
        None
    }

    fn on_document_changed(&mut self, _new_text: &str, _cx: &mut App) {}

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

    fn handle_navigation(
        &mut self,
        _target: &crate::document::NavigationTarget,
        _modifiers: gpui::Modifiers,
        _cx: &mut App,
    ) -> Option<crate::document::NavigationExecutionPlan> {
        None
    }

    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
