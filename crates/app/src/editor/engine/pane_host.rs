//! App-side implementation of the [`PaneHost`] seam: a proxy that
//! re-enters the `Editor` entity through its captured weak handle.
//!
//! Pane mode crates render and process input against their own state; the
//! coordination-layer actions they need (focus routing, autoscroll, dirty
//! marking, source sync, undo/redo, preview selection) go through this
//! proxy so the modes never name the entity type.

use std::sync::Arc;

use gpui::{App, Bounds, Point, Pixels, WeakEntity, Window};

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

/// Snapshot provider for the Source pane's rendering element: reads the
/// pane state off the editor entity.
pub(crate) struct EditorSourceView {
    editor: WeakEntity<Editor>,
}

impl EditorSourceView {
    pub(crate) fn new(editor: WeakEntity<Editor>) -> Arc<Self> {
        Arc::new(Self { editor })
    }
}

impl editor_source_code::SourceStateView for EditorSourceView {
    fn snapshot(
        &self,
        pane_id: PaneId,
        cx: &App,
    ) -> Option<editor_source_code::SourceViewSnapshot> {
        let editor = self.editor.upgrade()?;
        let state = editor.read(cx).pane_state_ref(pane_id)?.as_source_code()?;
        Some(editor_source_code::SourceViewSnapshot {
            text: state.text.clone(),
            line_ranges: state.line_ranges.clone(),
            cursor: state.cursor,
            selection: state.selection.clone(),
            highlight_spans: state
                .highlight_cache
                .as_ref()
                .map(|h| h.spans.clone())
                .unwrap_or_default(),
            focus_handle: state.focus_handle.clone(),
        })
    }
}

/// IME registration for the Source pane: re-enters the editor entity so
/// the platform input handler binds to it (gpui requires a concrete
/// entity type).
pub(crate) struct EditorSourceIme {
    editor: WeakEntity<Editor>,
}

impl EditorSourceIme {
    pub(crate) fn new(editor: WeakEntity<Editor>) -> Arc<Self> {
        Arc::new(Self { editor })
    }
}

/// Outline HUD host: navigation and hover re-enter the editor entity,
/// carrying the pane kind and theme captured at render time.
pub(crate) struct EditorOutlineHost {
    editor: WeakEntity<Editor>,
    kind: editor_core::EditorPaneKind,
    theme: theme::Theme,
}

impl EditorOutlineHost {
    pub(crate) fn new(
        editor: WeakEntity<Editor>,
        kind: editor_core::EditorPaneKind,
        theme: theme::Theme,
    ) -> Arc<Self> {
        Arc::new(Self { editor, kind, theme })
    }
}

impl editor_outline::OutlineHost for EditorOutlineHost {
    fn navigate_to(&self, index: usize, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let kind = self.kind;
            let theme = self.theme.clone();
            let _ = editor.update(cx, |editor, cx| {
                editor.navigate_to_outline_index(index, kind, &theme, cx)
            });
        }
    }

    fn set_hovered(&self, hovered: bool, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| editor.set_outline_hovered(hovered, cx));
        }
    }
}

impl editor_source_code::SourceIme for EditorSourceIme {
    fn handle_input(
        &self,
        pane_id: PaneId,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(entity) = self.editor.upgrade() else {
            return;
        };
        let Some(focus_handle) = entity
            .read(cx)
            .pane_state_ref(pane_id)
            .and_then(|p| p.as_source_code())
            .and_then(|s| s.focus_handle.clone())
        else {
            return;
        };
        window.handle_input(
            &focus_handle,
            gpui::ElementInputHandler::new(bounds, entity),
            cx,
        );
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

    fn set_source_last_bounds(&self, pane_id: PaneId, bounds: Bounds<Pixels>, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, _cx| {
                if let Some(source) = editor.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
                    source.last_bounds = Some(bounds);
                }
            });
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
