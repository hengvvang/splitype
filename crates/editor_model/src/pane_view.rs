//! The universal plugin contract for editor panes.

use std::any::Any;

use gpui::{AnyElement, App, FocusHandle, Window};
use editor_outline::OutlineHeading;
use editor_search::{SearchMatch, SearchQuery};
use theme::Theme;

use crate::{EditorDocument, PaneHost, PaneId, PaneKindId, PaneRenderContext};

/// The plugin contract implemented by every editor pane kind.
pub trait PaneView: Any + Send + Sync + 'static {
    /// Which pane kind this state belongs to.
    fn kind(&self) -> PaneKindId;

    /// Pure markdown source of the active tab, as this mode sees it.
    fn document_source(&self, doc: &dyn EditorDocument, cx: &App) -> String;

    /// Render the pane's entire body viewport (including outline HUD / scroll if desired).
    fn render(
        &mut self,
        ctx: &PaneRenderContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement;

    /// Focus handle owned by this pane (if any).
    fn focus_handle(&self, _cx: &App) -> Option<FocusHandle> {
        None
    }

    /// Returns (line, column) 1-based cursor position if supported.
    fn cursor_position(&self, _cx: &App) -> Option<(usize, usize)> {
        None
    }

    /// Sync pane state from the canonical document text and revision.
    fn sync_document_text(&mut self, _text: &str, _revision: u64, _cx: &mut App) {}

    /// Serialize this pane's current content to Markdown.
    fn serialize_text(&self, _cx: &App) -> Option<String> {
        None
    }

    /// Extract outline headings for TOC navigation.
    fn outline_headings(&self, _cx: &App) -> Vec<OutlineHeading> {
        Vec::new()
    }

    /// Navigate to a specific outline heading item.
    fn navigate_to_outline(&mut self, _index: usize, _theme: &Theme, _cx: &mut App) {}

    /// Find search matches within this pane.
    fn search_matches(&self, _query: &SearchQuery, _cx: &App) -> Vec<SearchMatch> {
        Vec::new()
    }

    /// Jump to / highlight a search match in this pane.
    fn navigate_to_search_match(&mut self, _match_item: &SearchMatch, _cx: &mut App) {}

    /// Replace one search match item.
    fn replace_match(
        &mut self,
        _match_item: &SearchMatch,
        _replace_with: &str,
        _cx: &mut App,
    ) {}

    /// Replace all search matches.
    fn replace_all_matches(
        &mut self,
        _query: &SearchQuery,
        _replace_with: &str,
        _cx: &mut App,
    ) {}

    /// Apply line prefix (e.g. heading `# `, list `- `, task `- [ ] `).
    fn apply_line_prefix(&mut self, _prefix: &str, _cx: &mut App) {}

    /// Apply snippet / wrapper around selection (e.g. bold `****`, italic `**`, inline code ` `` `).
    fn apply_snippet(&mut self, _snippet: &str, _caret_offset: usize, _cx: &mut App) {}

    /// Wrap selection or insert template.
    fn apply_wrapped_or_template(
        &mut self,
        _empty_template: &str,
        _caret_offset_in_empty: usize,
        _wrap_prefix: &str,
        _wrap_suffix: &str,
        _cx: &mut App,
    ) {}

    /// Clear inline formatting from selection.
    fn apply_clear_format(&mut self, _cx: &mut App) {}

    /// Hook called when the shared document text has changed.
    fn on_document_changed(&mut self, _new_text: &str, _cx: &mut App) {}

    /// Handles key down event. Returns true if consumed.
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

    /// Handles mouse down event.
    fn handle_mouse_down(
        &mut self,
        _pane_id: PaneId,
        _event: &gpui::MouseDownEvent,
        _window: &mut Window,
        _cx: &mut App,
    ) {}

    /// Handles mouse move event.
    fn handle_mouse_move(
        &mut self,
        _pane_id: PaneId,
        _event: &gpui::MouseMoveEvent,
        _window: &mut Window,
        _cx: &mut App,
    ) {}

    /// Handles mouse up event.
    fn handle_mouse_up(
        &mut self,
        _pane_id: PaneId,
        _event: &gpui::MouseUpEvent,
        _window: &mut Window,
        _cx: &mut App,
    ) {}

    /// Type-erased access for downcasting to the concrete mode state.
    fn as_any(&self) -> &dyn Any;

    /// Type-erased mutable access for downcasting to the concrete mode state.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
