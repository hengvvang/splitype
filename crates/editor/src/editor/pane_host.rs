//! Pane host proxy: the seam a pane uses to reach back into the editor.

use std::sync::Arc;

use editor_contracts::{EditTransaction, PaneHost, PaneId};
use gpui::{App, WeakEntity, Window};

use crate::editor::Editor;

/// Thin proxy implementing [`PaneHost`] on behalf of an `Editor` entity.
pub struct EditorPaneHost {
    editor: WeakEntity<Editor>,
}

impl EditorPaneHost {
    pub fn new(editor: WeakEntity<Editor>) -> Arc<Self> {
        Arc::new(Self { editor })
    }
}

impl PaneHost for EditorPaneHost {
    fn commit_edit(&self, edit: EditTransaction, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                editor.commit_document_edit(edit, cx);
            });
        }
    }

    fn navigate_to_outline(&self, pane_id: PaneId, index: usize, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                let theme = cx.global::<theme::ThemeManager>().current_arc();
                editor.navigate_to_outline_index(pane_id, index, &theme, cx);
            });
        }
    }

    fn scroll_pane_to_y(&self, pane_id: PaneId, y: f32, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                editor.scroll_pane_to_y(pane_id, y, cx);
            });
        }
    }

    fn set_outline_hovered(
        &self,
        _pane_id: PaneId,
        hovered: bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                editor.set_outline_hovered(hovered, window, cx);
            });
        }
    }

    fn handle_pane_key_down(
        &self,
        pane_id: PaneId,
        event: &gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                editor.handle_pane_key_down(pane_id, event, window, cx)
            })
        } else {
            false
        }
    }

    fn handle_pane_mouse_down(
        &self,
        pane_id: PaneId,
        event: &gpui::MouseDownEvent,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                editor.handle_pane_mouse_down(pane_id, event, window, cx);
            });
        }
    }

    fn handle_pane_mouse_move(
        &self,
        pane_id: PaneId,
        event: &gpui::MouseMoveEvent,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                editor.handle_pane_mouse_move(pane_id, event, window, cx);
            });
        }
    }

    fn handle_pane_mouse_up(
        &self,
        pane_id: PaneId,
        event: &gpui::MouseUpEvent,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                editor.handle_pane_mouse_up(pane_id, event, window, cx);
            });
        }
    }
}
