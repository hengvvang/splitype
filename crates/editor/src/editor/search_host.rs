//! Search host proxies: the seams the search panel uses to reach the editor.

use std::sync::Arc;

use gpui::{App, Bounds, Pixels, WeakEntity, Window};

use crate::editor::Editor;

/// Search panel host: every coordination action re-enters the editor entity.
pub struct EditorSearchHost {
    editor: WeakEntity<Editor>,
}

impl EditorSearchHost {
    pub fn new(editor: WeakEntity<Editor>) -> Arc<Self> {
        Arc::new(Self { editor })
    }
}

impl editor_contracts::SearchHost for EditorSearchHost {
    fn toggle_show_replace(&self, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                editor.search.show_replace = !editor.search.show_replace;
                cx.notify();
            });
        }
    }

    fn toggle_match_case(&self, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                editor.search.match_case = !editor.search.match_case;
                editor.execute_search(cx);
            });
        }
    }

    fn toggle_whole_word(&self, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                editor.search.whole_word = !editor.search.whole_word;
                editor.execute_search(cx);
            });
        }
    }

    fn toggle_use_regex(&self, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                editor.search.use_regex = !editor.search.use_regex;
                editor.execute_search(cx);
            });
        }
    }

    fn toggle_preserve_case(&self, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                editor.search.preserve_case = !editor.search.preserve_case;
                cx.notify();
            });
        }
    }

    fn toggle_scope(&self, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                editor.search.scope =
                    if editor.search.scope == editor_contracts::SearchScope::Worktree {
                        editor_contracts::SearchScope::CurrentTab
                    } else {
                        editor_contracts::SearchScope::Worktree
                    };
                editor.execute_search(cx);
            });
        }
    }

    fn focus_query(&self, window: &mut Window, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                editor.search.active_field = editor_contracts::SearchActiveField::Query;
                window.focus(&editor.search.search_focus_handle, cx);
                cx.notify();
            });
        }
    }

    fn focus_replace(&self, window: &mut Window, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                editor.search.active_field = editor_contracts::SearchActiveField::Replace;
                window.focus(&editor.search.replace_focus_handle, cx);
                cx.notify();
            });
        }
    }

    fn handle_key_down(&self, event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                editor.handle_search_key_down(event, window, cx);
            });
        }
    }

    fn prev_match(&self, window: &mut Window, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                editor.search.prev_match();
                editor.jump_to_active_search_match(window, cx);
            });
        }
    }

    fn next_match(&self, window: &mut Window, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                editor.search.next_match();
                editor.jump_to_active_search_match(window, cx);
            });
        }
    }

    fn activate_match(&self, index: usize, window: &mut Window, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                editor.search.active_match_index = Some(index);
                editor.jump_to_active_search_match(window, cx);
            });
        }
    }

    fn replace_current(&self, window: &mut Window, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                editor.replace_current_search_match(window, cx);
            });
        }
    }

    fn replace_all(&self, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| editor.replace_all_search_matches(cx));
        }
    }

    fn toggle_match_expanded(&self, index: usize, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                editor.search.toggle_match_expanded(index);
                cx.notify();
            });
        }
    }

    fn collapse_results(&self, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                editor.search.results_expanded = false;
                cx.notify();
            });
        }
    }

    fn set_input_last_bounds(
        &self,
        field: editor_contracts::SearchActiveField,
        bounds: Bounds<Pixels>,
        cx: &mut App,
    ) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, _cx| {
                let input = match field {
                    editor_contracts::SearchActiveField::Query => &mut editor.search.search_input,
                    editor_contracts::SearchActiveField::Replace => {
                        &mut editor.search.replace_input
                    }
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

impl editor_contracts::SearchStateView for EditorSearchView {
    fn snapshot(
        &self,
        field: editor_contracts::SearchActiveField,
        cx: &App,
    ) -> editor_contracts::SearchInputSnapshot {
        let editor = self.editor.upgrade();
        let Some(editor) = editor else {
            return editor_contracts::SearchInputSnapshot::default();
        };
        let search = &editor.read(cx).search;
        let (input, focus_handle) = match field {
            editor_contracts::SearchActiveField::Query => {
                (&search.search_input, &search.search_focus_handle)
            }
            editor_contracts::SearchActiveField::Replace => {
                (&search.replace_input, &search.replace_focus_handle)
            }
        };
        editor_contracts::SearchInputSnapshot {
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

impl editor_contracts::SearchIme for EditorSearchIme {
    fn handle_input(
        &self,
        field: editor_contracts::SearchActiveField,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(entity) = self.editor.upgrade() else {
            return;
        };
        let focus_handle = entity.read(cx).search.search_focus_handle.clone();
        let focus_handle = match field {
            editor_contracts::SearchActiveField::Query => focus_handle,
            editor_contracts::SearchActiveField::Replace => {
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
