//! Document serialization — tree to Markdown text.
//!
//! Rendered mode serializes the semantic block tree back to normalized
//! Markdown. Source mode writes the raw source buffer directly so literal
//! delimiters are preserved. The save dialogs that use this text live in
//! `crate::editor::file`.

use gpui::*;

use crate::editor::engine::controller::Editor;

pub(crate) use crate::model::parse::fence::safe_code_fence_with_info;

impl Editor {
    pub(crate) fn serialized_document_text(&self, cx: &App) -> String {
        if self.is_source_code() {
            self.doc().serialize_source_text(cx)
        } else {
            self.doc().serialize_markdown(cx)
        }
    }
}

