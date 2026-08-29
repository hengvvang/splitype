//! Outline HUD — Notion-style floating outline ticks rail and hover TOC popover.
//!
//! Heading extraction from raw markdown is a `Pane` contract service
//! (`editor::outline_headings_from_markdown`); this module keeps only the
//! document-block-level parser and the `EditorDocument` implementation.

pub(crate) mod render;

use gpui::App;

use editor_wysiwyg::document::Document;
use crate::editor::engine::controller::Editor;
use editor_wysiwyg::markdown::parse::BlockKind;
use editor_core::{EditorDocument, OutlineNode};

/// The editor entity implements the minimal document view the modes read.
impl EditorDocument for Editor {
    fn serialize_markdown(&self, cx: &App) -> String {
        self.doc().serialize_markdown(cx)
    }

    fn outline_headings(&self, cx: &App) -> Vec<OutlineNode> {
        build_outline_headings_from_doc(self.doc(), cx)
    }
}

/// Extract all heading items from the active Document block entities.
pub(crate) fn build_outline_headings_from_doc(doc: &Document, cx: &App) -> Vec<OutlineNode> {
    let mut list = Vec::new();
    for (block_index, entry) in doc.blocks().iter().enumerate() {
        let block = entry.entity.read(cx);
        let level = match block.kind() {
            BlockKind::Heading { level } => level,
            _ => {
                let text = block.data.text.plain_text();
                if let Some((lvl, _)) = BlockKind::parse_atx_heading_line(text.trim()) {
                    lvl
                } else {
                    continue;
                }
            }
        };

        let heading_text = block.data.text.plain_text().trim().to_string();
        let entity_id = entry.entity.entity_id();
        let display_label = if let Some((_, parsed_text)) = BlockKind::parse_atx_heading_line(&heading_text) {
            parsed_text
        } else {
            heading_text
        };

        list.push(OutlineNode {
            id: format!("outline:{entity_id}"),
            label: if display_label.is_empty() {
                format!("Heading {level}")
            } else {
                display_label
            },
            level,
            block_index,
            block_id: Some(entity_id),
        });
    }
    list
}
