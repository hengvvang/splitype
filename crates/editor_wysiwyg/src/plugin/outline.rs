//! WYSIWYG pane outline extraction and heading navigation.

use gpui::App;
use editor_outline::OutlineNode;
use crate::model::Document;
use crate::markdown::parse::BlockKind;

/// Extracts all heading nodes from the WYSIWYG document blocks.
pub fn extract_outline_headings(doc: &Document, cx: &App) -> Vec<OutlineNode> {
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
