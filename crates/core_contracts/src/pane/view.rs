use std::any::Any;
use gpui::{AnyElement, App, FocusHandle, Window};
use theme::Theme;
use crate::outline::OutlineHeading;
use crate::pane::{PaneHost, PaneId, PaneKind, PaneRenderContext};
use crate::search::{SearchMatch, SearchQuery};

pub trait PaneView: Any + Send + Sync + 'static {
    fn kind(&self) -> PaneKind;
    fn render(
        &mut self,
        ctx: &PaneRenderContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement;

    fn focus_handle(&self, _cx: &App) -> Option<FocusHandle> {
        None
    }

    fn cursor_position(&self, _cx: &App) -> Option<(usize, usize)> {
        None
    }

    fn sync_document_text(&mut self, _text: &str, _revision: u64, _cx: &mut App) {}

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

    fn replace_match(
        &mut self,
        _match_item: &SearchMatch,
        _replace_with: &str,
        _cx: &mut App,
    ) {}

    fn replace_all_matches(
        &mut self,
        _query: &SearchQuery,
        _replace_with: &str,
        _cx: &mut App,
    ) {}

    fn apply_line_prefix(&mut self, _prefix: &str, _cx: &mut App) {}

    fn apply_snippet(&mut self, _snippet: &str, _caret_offset: usize, _cx: &mut App) {}

    fn apply_wrapped_or_template(
        &mut self,
        _empty_template: &str,
        _caret_offset_in_empty: usize,
        _wrap_prefix: &str,
        _wrap_suffix: &str,
        _cx: &mut App,
    ) {}

    fn apply_clear_format(&mut self, _cx: &mut App) {}

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
    ) {}

    fn handle_mouse_move(
        &mut self,
        _pane_id: PaneId,
        _event: &gpui::MouseMoveEvent,
        _window: &mut Window,
        _cx: &mut App,
    ) {}

    fn handle_mouse_up(
        &mut self,
        _pane_id: PaneId,
        _event: &gpui::MouseUpEvent,
        _window: &mut Window,
        _cx: &mut App,
    ) {}

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
