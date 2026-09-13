//! Links widget host proxy: the seam the links widget uses to reach back into the editor.

use std::path::PathBuf;
use std::sync::Arc;

use gpui::{App, WeakEntity};

use crate::editor::Editor;
use crate::links::{LinksWidgetHost, LinksWidgetTab};

pub struct EditorLinksHost {
    editor: WeakEntity<Editor>,
}

impl EditorLinksHost {
    pub fn new(editor: WeakEntity<Editor>) -> Arc<Self> {
        Arc::new(Self { editor })
    }
}

impl LinksWidgetHost for EditorLinksHost {
    fn switch_tab(&self, tab: LinksWidgetTab, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                if let Some(ref mut widget) = editor.links_widget {
                    widget.active_tab = tab;
                    cx.notify();
                }
            });
        }
    }

    fn update_filter(&self, query: String, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                if let Some(ref mut widget) = editor.links_widget {
                    widget.search_query = query;
                    cx.notify();
                }
            });
        }
    }

    fn close_widget(&self, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                editor.links_widget = None;
                cx.notify();
            });
        }
    }

    fn navigate_to_backlink(
        &self,
        path: PathBuf,
        _line: usize,
        anchor: Option<String>,
        cx: &mut App,
    ) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                editor.links_widget = None;
                if editor
                    .open_file(&path, editor_contracts::TabKind::Persistent, cx)
                    .is_ok()
                {
                    if let Some(a) = anchor {
                        editor.jump_to_anchor(&a, cx);
                    }
                }
            });
        }
    }

    fn navigate_to_outgoing(&self, target: String, cx: &mut App) {
        if let Some(editor) = self.editor.upgrade() {
            editor.update(cx, |editor, cx| {
                editor.links_widget = None;
                editor.navigate_to_link(&target, cx);
            });
        }
    }
}
