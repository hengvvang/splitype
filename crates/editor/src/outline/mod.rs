//! Outline coordination in editor.

pub(crate) mod render;

use gpui::App;

use crate::editor::Editor;
use core_contracts::EditorDocument;

/// The editor entity implements the minimal document view the modes read.
impl EditorDocument for Editor {
    fn serialize_markdown(&self, cx: &App) -> String {
        self.serialized_document_text(cx)
    }
}


