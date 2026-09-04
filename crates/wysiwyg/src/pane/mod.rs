//! The WYSIWYG pane: its [`PaneView`] contract adapter and submodules.

pub mod actions;
pub mod controller;
pub mod search;
pub mod shell;

use editor_contracts::OutlineNode;
use editor_contracts::{EditTransaction, PaneCapabilities, PaneRenderContext, PaneView};
use editor_contracts::{SearchMatch, SearchQuery};
use gpui::{AnyElement, App, AppContext, FocusHandle, Window};
use theme::Theme;

use crate::pane::controller::WysiwygDocumentController;
use crate::pane::shell::WysiwygPane;

impl PaneView for WysiwygPane {
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
    ) -> Option<EditTransaction> {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |c, cx| c.replace_match(match_item, replace_with, cx))
    }

    fn navigate_to_search_match(&mut self, match_item: &SearchMatch, cx: &mut App) -> Option<f32> {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |c, cx| c.navigate_to_search_match(match_item, cx))
    }

    fn selected_text(&self, cx: &App) -> Option<String> {
        self.controller
            .as_ref()
            .and_then(|c| c.read(cx).selected_text(cx))
    }

    fn delete_selection(&mut self, cx: &mut App) -> Option<EditTransaction> {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |c, cx| c.delete_selection(cx))
    }

    fn insert_text(&mut self, text: &str, cx: &mut App) -> Option<EditTransaction> {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |c, cx| c.insert_text(text, cx))
    }

    fn select_all(&mut self, cx: &mut App) {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |c, cx| c.select_all(cx));
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
