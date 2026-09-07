//! Search host proxies: the seams the search panel uses to reach the editor.

use std::sync::Arc;

use gpui::{App, WeakEntity, Window};

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

    fn set_query(&self, query: String, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                if editor.search.search_input.text != query {
                    editor.search.search_input.text = query;
                    editor.execute_search(cx);
                }
            });
        }
    }

    fn set_replace(&self, replace: String, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                editor.search.replace_input.text = replace;
                cx.notify();
            });
        }
    }

    fn close(&self, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                editor.search.visible = false;
                editor.clear_search_highlights_from_document(cx);
                cx.notify();
            });
        }
    }

    fn history_prev(&self, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                if editor.search.search_input.history_prev() {
                    editor.execute_search(cx);
                }
            });
        }
    }

    fn history_next(&self, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                if editor.search.search_input.history_next() {
                    editor.execute_search(cx);
                }
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
}
