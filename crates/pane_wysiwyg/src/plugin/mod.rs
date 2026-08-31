//! WYSIWYG Pane plugin implementation — PaneView contract and lifecycle.

pub mod actions;
pub mod controller;
pub mod outline;
pub mod search;
pub mod state;

pub use actions::*;
pub use controller::*;
pub use outline::*;
pub use search::*;
pub use state::*;

use std::sync::{Arc, Mutex};

use gpui::{
    AnyElement, App, AppContext, Entity, FocusHandle, Window,
};
use core_contracts::{
    PaneRenderContext, PaneView,
};
use core_contracts::OutlineNode;
use core_contracts::{SearchMatch, SearchQuery};
use theme::Theme;

/// View state specific to a WYSIWYG editor pane.
#[derive(Default)]
pub struct WysiwygPaneState {
    pub controller: Arc<Mutex<Option<Entity<WysiwygDocumentController>>>>,
    pub pending_text: Option<(String, u64)>,
}

impl WysiwygPaneState {
    fn ensure_controller(&self, cx: &mut App) -> Entity<WysiwygDocumentController> {
        let mut guard = self.controller.lock().unwrap();
        if let Some(controller) = guard.as_ref() {
            return controller.clone();
        }
        let (text, revision) = self.pending_text.clone().unwrap_or_else(|| (String::new(), 1));
        let controller = cx.new(|cx| WysiwygDocumentController::new(&text, revision, cx));
        *guard = Some(controller.clone());
        controller
    }
}

impl PaneView for WysiwygPaneState {
    fn kind(&self) -> core_contracts::PaneKind {
        core_contracts::PaneKind::new("wysiwyg")
    }

    fn sync_document_text(&mut self, text: &str, revision: u64, cx: &mut App) {
        let mut guard = self.controller.lock().unwrap();
        if let Some(controller) = guard.as_ref() {
            controller.update(cx, |c, cx| {
                c.sync_document_text(text, revision, cx);
            });
        } else {
            let controller = cx.new(|cx| WysiwygDocumentController::new(text, revision, cx));
            *guard = Some(controller);
        }
        self.pending_text = Some((text.to_string(), revision));
    }

    fn serialize_text(&self, cx: &App) -> Option<String> {
        let guard = self.controller.lock().unwrap();
        guard.as_ref().and_then(|c| c.read(cx).serialize_text(cx))
    }

    fn focus_handle(&self, cx: &App) -> Option<FocusHandle> {
        let guard = self.controller.lock().unwrap();
        guard.as_ref().and_then(|c| c.read(cx).focus_handle(cx))
    }

    fn outline_headings(&self, cx: &App) -> Vec<OutlineNode> {
        let guard = self.controller.lock().unwrap();
        guard.as_ref().map(|c| c.read(cx).outline_headings(cx)).unwrap_or_default()
    }

    fn navigate_to_outline(&mut self, index: usize, theme: &Theme, cx: &mut App) {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |c, cx| {
            c.navigate_to_outline(index, theme, cx);
        });
    }

    fn search_matches(&self, query: &SearchQuery, cx: &App) -> Vec<SearchMatch> {
        let guard = self.controller.lock().unwrap();
        guard.as_ref().map(|c| c.read(cx).search_matches(query, cx)).unwrap_or_default()
    }

    fn replace_match(&mut self, match_item: &SearchMatch, replace_with: &str, cx: &mut App) {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |c, cx| {
            c.replace_match(match_item, replace_with, cx);
        });
    }

    fn navigate_to_search_match(&mut self, match_item: &SearchMatch, cx: &mut App) {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |c, cx| {
            c.navigate_to_search_match(match_item, cx);
        });
    }

    fn apply_line_prefix(&mut self, prefix: &str, cx: &mut App) {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |c, cx| {
            c.apply_line_prefix(prefix, cx);
        });
    }

    fn apply_snippet(&mut self, snippet: &str, caret_offset: usize, cx: &mut App) {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |c, cx| {
            c.apply_snippet(snippet, caret_offset, cx);
        });
    }

    fn apply_wrapped_or_template(
        &mut self,
        empty_template: &str,
        caret_offset_in_empty: usize,
        wrap_prefix: &str,
        wrap_suffix: &str,
        cx: &mut App,
    ) {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |c, cx| {
            c.apply_wrapped_or_template(empty_template, caret_offset_in_empty, wrap_prefix, wrap_suffix, cx);
        });
    }

    fn apply_clear_format(&mut self, cx: &mut App) {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |c, cx| {
            c.apply_clear_format(cx);
        });
    }

    fn render(
        &mut self,
        ctx: &PaneRenderContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |c, cx| {
            c.render(ctx, window, cx)
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}


