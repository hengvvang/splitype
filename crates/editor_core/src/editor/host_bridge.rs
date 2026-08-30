//! Implementation of pane and search host proxy seams.

use std::sync::Arc;

use gpui::{App, Bounds, KeyDownEvent, Pixels, Point, WeakEntity, Window};
use editor_model::{AutoscrollStrategy, PaneHost, PaneId, PaneKindId};

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

/// Search panel host: every coordination action re-enters the editor entity.
pub struct EditorSearchHost {
    editor: WeakEntity<Editor>,
}

impl EditorSearchHost {
    pub fn new(editor: WeakEntity<Editor>) -> Arc<Self> {
        Arc::new(Self { editor })
    }
}

impl editor_search::SearchHost for EditorSearchHost {
    fn execute_search(&self, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| editor.execute_search(cx));
        }
    }

    fn notify(&self, cx: &mut App) {
        cx.notify(self.editor.entity_id());
    }

    fn toggle_show_replace(&self, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| {
                editor.search.show_replace = !editor.search.show_replace;
                cx.notify();
            });
        }
    }

    fn toggle_match_case(&self, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| {
                editor.search.match_case = !editor.search.match_case;
                editor.execute_search(cx);
            });
        }
    }

    fn toggle_whole_word(&self, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| {
                editor.search.whole_word = !editor.search.whole_word;
                editor.execute_search(cx);
            });
        }
    }

    fn toggle_use_regex(&self, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| {
                editor.search.use_regex = !editor.search.use_regex;
                editor.execute_search(cx);
            });
        }
    }

    fn toggle_preserve_case(&self, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| {
                editor.search.preserve_case = !editor.search.preserve_case;
                cx.notify();
            });
        }
    }

    fn toggle_scope(&self, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| {
                editor.search.scope = if editor.search.scope == editor_search::SearchScope::Worktree {
                    editor_search::SearchScope::CurrentTab
                } else {
                    editor_search::SearchScope::Worktree
                };
                editor.execute_search(cx);
            });
        }
    }

    fn focus_query(&self, window: &mut Window, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| {
                editor.search.active_field = editor_search::SearchActiveField::Query;
                window.focus(&editor.search.search_focus_handle, cx);
            });
        }
    }

    fn focus_replace(&self, window: &mut Window, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| {
                editor.search.active_field = editor_search::SearchActiveField::Replace;
                window.focus(&editor.search.replace_focus_handle, cx);
            });
        }
    }

    fn handle_key_down(
        &self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| {
                editor.handle_search_key_down(event, window, cx);
            });
        }
    }

    fn prev_match(&self, window: &mut Window, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| {
                editor.search.prev_match();
                editor.jump_to_active_search_match(window, cx);
            });
        }
    }

    fn next_match(&self, window: &mut Window, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| {
                editor.search.next_match();
                editor.jump_to_active_search_match(window, cx);
            });
        }
    }

    fn activate_match(&self, index: usize, window: &mut Window, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| {
                editor.search.active_match_index = Some(index);
                editor.jump_to_active_search_match(window, cx);
            });
        }
    }

    fn replace_current(&self, window: &mut Window, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| {
                editor.replace_current_search_match(window, cx);
            });
        }
    }

    fn replace_all(&self, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| editor.replace_all_search_matches(cx));
        }
    }

    fn toggle_match_expanded(&self, index: usize, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| {
                editor.search.toggle_match_expanded(index);
                cx.notify();
            });
        }
    }

    fn collapse_results(&self, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| {
                editor.search.results_expanded = false;
                cx.notify();
            });
        }
    }

    fn set_input_last_bounds(
        &self,
        field: editor_search::SearchActiveField,
        bounds: Bounds<Pixels>,
        cx: &mut App,
    ) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, _cx| {
                let input = match field {
                    editor_search::SearchActiveField::Query => &mut editor.search.search_input,
                    editor_search::SearchActiveField::Replace => &mut editor.search.replace_input,
                };
                input.last_bounds = Some(bounds);
            });
        }
    }
}

/// Search input field snapshots for the input element.
pub struct EditorSearchView {
    editor: WeakEntity<Editor>,
}

impl EditorSearchView {
    pub fn new(editor: WeakEntity<Editor>) -> Arc<Self> {
        Arc::new(Self { editor })
    }
}

impl editor_search::SearchStateView for EditorSearchView {
    fn snapshot(
        &self,
        field: editor_search::SearchActiveField,
        cx: &App,
    ) -> editor_search::SearchInputSnapshot {
        let editor = self.editor.upgrade();
        let Some(editor) = editor else {
            return editor_search::SearchInputSnapshot::default();
        };
        let search = &editor.read(cx).search;
        let (input, focus_handle) = match field {
            editor_search::SearchActiveField::Query => (
                &search.search_input,
                &search.search_focus_handle,
            ),
            editor_search::SearchActiveField::Replace => (
                &search.replace_input,
                &search.replace_focus_handle,
            ),
        };
        editor_search::SearchInputSnapshot {
            text: input.text.clone(),
            marked_range: input.marked_range.clone(),
            selection_range: input.selection_range(),
            cursor_offset: input.cursor(),
            focus_handle: Some(focus_handle.clone()),
        }
    }
}

/// Search input IME registration: binds the platform input handler to the editor entity.
pub struct EditorSearchIme {
    editor: WeakEntity<Editor>,
}

impl EditorSearchIme {
    pub fn new(editor: WeakEntity<Editor>) -> Arc<Self> {
        Arc::new(Self { editor })
    }
}

impl editor_search::SearchIme for EditorSearchIme {
    fn handle_input(
        &self,
        field: editor_search::SearchActiveField,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(entity) = self.editor.upgrade() else {
            return;
        };
        let focus_handle = entity.read(cx).search.search_focus_handle.clone();
        let focus_handle = match field {
            editor_search::SearchActiveField::Query => focus_handle,
            editor_search::SearchActiveField::Replace => {
                entity.read(cx).search.replace_focus_handle.clone()
            }
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
            let _ = editor.update(cx, |editor, cx| {
                if let Some(state) = editor.pane_state_ref(pane_id) {
                    if let Some(text) = state.pane.serialize_text(cx) {
                        editor.rebuild_document_from_markdown(&text, cx);
                    }
                }
            });
        }
    }

    fn set_source_last_bounds(&self, _pane_id: PaneId, _bounds: Bounds<Pixels>, _cx: &mut App) {}

    fn undo(&self, window: &mut Window, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| {
                editor.on_undo(&crate::actions::defs::Undo, window, cx)
            });
        }
    }

    fn redo(&self, window: &mut Window, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| {
                editor.on_redo(&crate::actions::defs::Redo, window, cx)
            });
        }
    }

    fn preview_mouse_down(
        &self,
        _pane_id: PaneId,
        _block_index: usize,
        _position: Point<Pixels>,
        _cx: &mut App,
    ) {
    }

    fn preview_mouse_move(
        &self,
        _pane_id: PaneId,
        _block_index: usize,
        _position: Point<Pixels>,
        _cx: &mut App,
    ) {
    }

    fn preview_mouse_up(&self, _pane_id: PaneId, _cx: &mut App) {}

    fn navigate_to_outline(&self, _pane_id: PaneId, index: usize, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            let _ = editor.update(cx, |editor, cx| {
                let theme = cx.global::<theme::ThemeManager>().current_arc();
                editor.navigate_to_outline_index(index, PaneKindId(""), &theme, cx);
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
            let _ = editor.update(cx, |editor, cx| {
                editor.set_outline_hovered(hovered, window, cx);
            });
        }
    }
}
