//! Pane shell: the [`PaneView`] contract adapter for the source code editor.
//!
//! The pane is a thin shell around the [`SourceCodeEditor`] entity, which
//! owns all editing state (text projection, selections, folds, wrap, and
//! rendering caches). Undo history lives in the shared document buffer, so
//! the pane itself keeps none.

use gpui::{AnyElement, App, AppContext, Entity, FocusHandle, Window};
use theme::Theme;

use crate::editor::SourceCodeEditor;
use editor_contracts::{
    DocumentSnapshot, EditTransaction, OutlineNode, PaneCapabilities, PaneKind, PaneRenderContext,
    PaneView, SearchMatch, SearchQuery,
};

/// View state specific to a Source Code editor pane.
#[derive(Default)]
pub struct SourceCodeState {
    controller: Option<Entity<SourceCodeEditor>>,
    /// The most recent document snapshot, seeding a lazily created editor.
    latest_snapshot: Option<DocumentSnapshot>,
}

impl SourceCodeState {
    pub(crate) fn ensure_controller(&mut self, cx: &mut App) -> Entity<SourceCodeEditor> {
        if let Some(controller) = &self.controller {
            return controller.clone();
        }
        let document = self
            .latest_snapshot
            .clone()
            .unwrap_or_else(DocumentSnapshot::empty);
        let controller = cx.new(|cx| SourceCodeEditor::new(&document, cx));
        self.controller = Some(controller.clone());
        controller
    }
}

impl PaneView for SourceCodeState {
    fn kind(&self) -> PaneKind {
        PaneKind::from_static(crate::builder::PANE_KIND)
    }

    fn capabilities(&self) -> PaneCapabilities {
        PaneCapabilities {
            editable: true,
            searchable: true,
            replaceable: true,
            outline: true,
        }
    }

    fn sync_document(&mut self, document: &DocumentSnapshot, cx: &mut App) {
        if let Some(controller) = self.controller.clone() {
            controller.update(cx, |editor, cx| {
                editor.sync_document(document, cx);
            });
        } else {
            let document = document.clone();
            self.controller = Some(cx.new(|cx| SourceCodeEditor::new(&document, cx)));
        }
        self.latest_snapshot = Some(document.clone());
    }

    fn document_text(&self, cx: &App) -> Option<String> {
        self.controller
            .as_ref()
            .map(|controller| controller.read(cx).document_text())
    }

    fn focus_handle(&self, cx: &App) -> Option<FocusHandle> {
        self.controller
            .as_ref()
            .map(|controller| controller.read(cx).focus_handle.clone())
    }

    fn cursor_position(&self, cx: &App) -> Option<(usize, usize)> {
        self.controller
            .as_ref()
            .map(|controller| controller.read(cx).cursor_position_1based())
    }

    fn outline_headings(&self, cx: &App) -> Vec<OutlineNode> {
        self.controller
            .as_ref()
            .map(|controller| {
                let editor = controller.read(cx);
                crate::outline::extract_outline_headings(editor.text())
            })
            .unwrap_or_default()
    }

    fn navigate_to_outline(&mut self, index: usize, theme: &Theme, cx: &mut App) -> Option<f32> {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |editor, cx| {
            let headings = crate::outline::extract_outline_headings(editor.text());
            let heading = headings.get(index)?;
            let offset = editor.line_start_offset(heading.block_index);
            editor.move_to(offset, false);
            editor.start_cursor_blink();
            let snapshot = editor.snapshot();
            let row = snapshot.offset_to_display_point(offset).row;
            let font_size = theme.typography.code_size.max(12.0);
            let line_height = (font_size * theme.typography.text_line_height).round();
            cx.notify();
            Some(row as f32 * line_height)
        })
    }

    fn search_matches(&self, query: &SearchQuery, cx: &App) -> Vec<SearchMatch> {
        self.controller
            .as_ref()
            .map(|controller| {
                let editor = controller.read(cx);
                crate::search::search_in_source(editor.text(), query)
            })
            .unwrap_or_default()
    }

    fn navigate_to_search_match(&mut self, match_item: &SearchMatch, cx: &mut App) -> Option<f32> {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |editor, cx| {
            let start = match_item.byte_range.start.min(editor.text().len());
            let end = match_item.byte_range.end.min(editor.text().len());
            editor.selections_mut().set_single_range(start, end);
            editor.start_cursor_blink();
            let snapshot = editor.snapshot();
            let row = snapshot.offset_to_display_point(start).row;
            let theme = cx.global::<theme::ThemeManager>().current_arc();
            let font_size = theme.typography.code_size.max(12.0);
            let line_height = (font_size * theme.typography.text_line_height).round();
            cx.notify();
            Some(row as f32 * line_height)
        })
    }

    fn set_search_highlights(
        &mut self,
        matches: &[SearchMatch],
        active_index: Option<usize>,
        cx: &mut App,
    ) {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |editor, cx| {
            editor.set_search_matches(
                matches
                    .iter()
                    .enumerate()
                    .map(|(i, m)| (m.byte_range.clone(), Some(i) == active_index))
                    .collect(),
            );
            cx.notify();
        });
    }

    fn replace_match(
        &mut self,
        match_item: &SearchMatch,
        replace_with: &str,
        cx: &mut App,
    ) -> Option<EditTransaction> {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |editor, cx| {
            editor.replace_range(match_item.byte_range.clone(), replace_with, cx)
        })
    }

    fn replace_all_matches(
        &mut self,
        query: &SearchQuery,
        replace_with: &str,
        cx: &mut App,
    ) -> Option<EditTransaction> {
        let controller = self.ensure_controller(cx);
        let matches = controller.update(cx, |editor, _cx| {
            crate::search::search_in_source(editor.text(), query)
                .into_iter()
                .map(|m| (m.byte_range, replace_with.to_string()))
                .collect::<Vec<_>>()
        });
        controller.update(cx, |editor, cx| editor.replace_all_ranges(matches, cx))
    }

    fn selected_text(&self, cx: &App) -> Option<String> {
        self.controller
            .as_ref()
            .and_then(|controller| controller.read(cx).selected_text())
    }

    fn delete_selection(&mut self, cx: &mut App) -> Option<EditTransaction> {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |editor, cx| editor.delete_selection(cx))
    }

    fn insert_text(&mut self, text: &str, cx: &mut App) -> Option<EditTransaction> {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |editor, cx| editor.insert_text(text, cx))
    }

    fn select_all(&mut self, cx: &mut App) {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |editor, cx| editor.select_all(cx));
    }

    fn handle_key_down(
        &mut self,
        _pane_id: editor_contracts::PaneId,
        event: &gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut App,
        _host: &dyn editor_contracts::PaneHost,
    ) -> bool {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |editor, cx| editor.handle_key_down(event, window, cx))
    }

    fn handle_mouse_down(
        &mut self,
        _pane_id: editor_contracts::PaneId,
        event: &gpui::MouseDownEvent,
        window: &mut Window,
        cx: &mut App,
    ) {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |editor, cx| {
            editor.handle_mouse_down(event, window, cx);
        });
    }

    fn handle_mouse_move(
        &mut self,
        _pane_id: editor_contracts::PaneId,
        event: &gpui::MouseMoveEvent,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |editor, cx| editor.handle_mouse_move(event, window, cx))
    }

    fn handle_mouse_up(
        &mut self,
        _pane_id: editor_contracts::PaneId,
        event: &gpui::MouseUpEvent,
        _window: &mut Window,
        cx: &mut App,
    ) {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |editor, cx| {
            editor.handle_mouse_up(event, cx);
        });
    }

    fn render(&mut self, ctx: &PaneRenderContext, window: &mut Window, cx: &mut App) -> AnyElement {
        let controller = self.ensure_controller(cx);
        controller.update(cx, |editor, cx| editor.render(ctx, window, cx))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
