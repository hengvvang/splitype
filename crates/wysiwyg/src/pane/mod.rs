//! The WYSIWYG pane: its [`PaneView`] contract adapter and submodules.

pub mod actions;
pub mod controller;
pub mod outline;
pub mod search;
pub mod state;

use editor_contracts::OutlineNode;
use editor_contracts::{PaneCapabilities, PaneRenderContext, PaneView};
use editor_contracts::{SearchMatch, SearchQuery};
use gpui::{AnyElement, App, AppContext, FocusHandle, Window};
use theme::Theme;

use crate::pane::controller::WysiwygDocumentController;
use crate::pane::state::WysiwygPaneState;

impl PaneView for WysiwygPaneState {
    fn kind(&self) -> editor_contracts::PaneKind {
        editor_contracts::PaneKind::from_static(crate::builder::PANE_KIND)
    }

    fn capabilities(&self) -> PaneCapabilities {
        PaneCapabilities {
            editable: true,
            searchable: true,
            replaceable: true,
            outline: true,
        }
    }

    fn sync_document(&mut self, document: &editor_contracts::DocumentSnapshot, cx: &mut App) {
        if let Some(controller) = self.controller.clone() {
            controller.update(cx, |controller, cx| {
                controller.sync_document(document, cx);
            });
        } else {
            let document = document.clone();
            self.controller = Some(cx.new(|cx| WysiwygDocumentController::new(&document, cx)));
        }
        self.latest_snapshot = Some(document.clone());
    }

    fn document_text(&self, cx: &App) -> Option<String> {
        self.controller
            .as_ref()
            .and_then(|c| c.read(cx).document_text(cx))
    }

    fn focus_handle(&self, cx: &App) -> Option<FocusHandle> {
        self.controller
            .as_ref()
            .and_then(|c| c.read(cx).focus_handle(cx))
    }

    fn outline_headings(&self, cx: &App) -> Vec<OutlineNode> {
        self.controller
            .as_ref()
            .map(|c| c.read(cx).outline_headings(cx))
            .unwrap_or_default()
    }

    fn navigate_to_outline(&mut self, index: usize, theme: &Theme, cx: &mut App) -> Option<f32> {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |c, cx| c.navigate_to_outline(index, theme, cx))
    }

    fn search_matches(&self, query: &SearchQuery, cx: &App) -> Vec<SearchMatch> {
        self.controller
            .as_ref()
            .map(|c| c.read(cx).search_matches(query, cx))
            .unwrap_or_default()
    }

    fn replace_match(
        &mut self,
        match_item: &SearchMatch,
        replace_with: &str,
        cx: &mut App,
    ) -> Option<String> {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |c, cx| c.replace_match(match_item, replace_with, cx))
    }

    fn navigate_to_search_match(&mut self, match_item: &SearchMatch, cx: &mut App) -> Option<f32> {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |c, cx| c.navigate_to_search_match(match_item, cx))
    }

    fn apply_line_prefix(&mut self, prefix: &str, cx: &mut App) -> Option<String> {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |c, cx| c.apply_line_prefix(prefix, cx))
    }

    fn apply_snippet(
        &mut self,
        snippet: &str,
        caret_offset: usize,
        cx: &mut App,
    ) -> Option<String> {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |c, cx| c.apply_snippet(snippet, caret_offset, cx))
    }

    fn apply_wrapped_or_template(
        &mut self,
        empty_template: &str,
        caret_offset_in_empty: usize,
        wrap_prefix: &str,
        wrap_suffix: &str,
        cx: &mut App,
    ) -> Option<String> {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |c, cx| {
            c.apply_wrapped_or_template(
                empty_template,
                caret_offset_in_empty,
                wrap_prefix,
                wrap_suffix,
                cx,
            )
        })
    }

    fn apply_clear_format(&mut self, cx: &mut App) -> Option<String> {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |c, cx| c.apply_clear_format(cx))
    }

    fn render(&mut self, ctx: &PaneRenderContext, window: &mut Window, cx: &mut App) -> AnyElement {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |c, cx| c.render(ctx, window, cx))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
