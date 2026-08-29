//! App-side implementation of the [`PaneHost`] seam: a proxy that
//! re-enters the `Editor` entity through its captured weak handle.
//!
//! Pane mode crates render and process input against their own state; the
//! coordination-layer actions they need (focus routing, autoscroll, dirty
//! marking, source sync, undo/redo, preview selection) go through this
//! proxy so the modes never name the entity type.

use std::sync::Arc;

use gpui::{App, Point, Pixels, WeakEntity, Window};

use editor_core::{AutoscrollStrategy, PaneHost, PaneId};

use crate::editor::engine::controller::Editor;

/// Thin proxy implementing [`PaneHost`] on behalf of an `Editor` entity.
pub(crate) struct EditorPaneHost {
    editor: WeakEntity<Editor>,
}

impl EditorPaneHost {
    pub(crate) fn new(editor: WeakEntity<Editor>) -> Arc<Self> {
        Arc::new(Self { editor })
    }
}

impl PaneHost for EditorPaneHost {
    fn focus_pane(&self, pane_id: PaneId, window: &mut Window, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| editor.focus_pane(pane_id, window, cx));
        }
    }

    fn apply_pending_focus(&self, pane_id: PaneId, window: &mut Window, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| editor.apply_pending_focus(pane_id, window, cx));
        }
    }

    fn apply_pending_autoscroll(&self, pane_id: PaneId, window: &mut Window, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| editor.apply_pending_autoscroll(pane_id, window, cx));
        }
    }

    fn request_autoscroll(&self, pane_id: PaneId, strategy: AutoscrollStrategy, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| editor.request_autoscroll(pane_id, strategy, cx));
        }
    }

    fn mark_dirty(&self, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| editor.mark_dirty(cx));
        }
    }

    fn notify(&self, cx: &mut App) {
        cx.notify(self.editor.entity_id());
    }

    fn sync_source_edit(&self, pane_id: PaneId, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| editor.sync_source_edit_to_document(pane_id, cx));
        }
    }

    fn undo(&self, window: &mut Window, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| {
                editor.on_undo(&editor_wysiwyg::actions::Undo, window, cx)
            });
        }
    }

    fn redo(&self, window: &mut Window, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| {
                editor.on_redo(&editor_wysiwyg::actions::Redo, window, cx)
            });
        }
    }

    fn preview_mouse_down(
        &self,
        pane_id: PaneId,
        block_index: usize,
        position: Point<Pixels>,
        cx: &mut App,
    ) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| {
                editor.on_preview_mouse_down(pane_id, block_index, position, cx)
            });
        }
    }

    fn preview_mouse_move(
        &self,
        pane_id: PaneId,
        block_index: usize,
        position: Point<Pixels>,
        cx: &mut App,
    ) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| {
                editor.on_preview_mouse_move(pane_id, block_index, position, cx)
            });
        }
    }

    fn preview_mouse_up(&self, pane_id: PaneId, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| editor.on_preview_mouse_up(pane_id, cx));
        }
    }
}
