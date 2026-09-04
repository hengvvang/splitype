//! Preview pane outline extraction and heading navigation.

use crate::block::PreviewBlock;
use crate::pane::PreviewPane;
use editor_contracts::OutlineNode;
use markdown_parser::parse::BlockKind;

/// Extracts all heading nodes directly from the snapshot preview blocks.
pub fn extract_preview_headings(blocks: &[PreviewBlock]) -> Vec<OutlineNode> {
    let mut list = Vec::new();
    for (block_index, block) in blocks.iter().enumerate() {
        let level = match block.kind() {
            BlockKind::Heading { level } => level,
            _ => continue,
        };
        let label = block.display_text().trim().to_string();
        list.push(OutlineNode {
            id: format!("outline:preview:block:{block_index}"),
            label: if label.is_empty() {
                format!("Heading {level}")
            } else {
                label
            },
            level,
            block_index,
            block_id: None,
        });
    }
    list
}

/// Calculates approximate scroll Y offset for the given outline node in the Preview render tree.
pub fn calculate_scroll_offset_for_node(
    state: &PreviewPane,
    node: &OutlineNode,
    line_height: f32,
) -> f32 {
    calculate_scroll_offset_for_block_index(state, node.block_index, line_height)
}

/// Approximate scroll Y offset of the start of `block_index` in the Preview
/// render tree, estimated from per-block-kind heights.
pub fn calculate_scroll_offset_for_block_index(
    state: &PreviewPane,
    block_index: usize,
    line_height: f32,
) -> f32 {
    let mut y = 0.0;
    for (i, block) in state.blocks.iter().enumerate() {
        if i >= block_index {
            break;
        }
        y += estimated_block_height(block, line_height);
    }
    y
}

fn estimated_block_height(block: &PreviewBlock, line_height: f32) -> f32 {
    match block.kind() {
        BlockKind::Heading { level } => match level {
            1 => line_height * 2.2 + 16.0,
            2 => line_height * 1.8 + 14.0,
            3 => line_height * 1.5 + 12.0,
            _ => line_height * 1.3 + 10.0,
        },
        BlockKind::Paragraph => {
            let lines = (block.display_len() / 60).max(1);
            (lines as f32) * line_height + 12.0
        }
        BlockKind::CodeBlock { .. } => {
            let lines = block.display_text().lines().count().max(1);
            (lines as f32) * line_height + 24.0
        }
        BlockKind::Table => line_height * 4.0 + 16.0,
        BlockKind::ThematicBreak => line_height * 1.0 + 8.0,
        _ => line_height * 1.5 + 8.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use markdown_parser::parse::BlockData;

    #[test]
    fn extracts_preview_headings_from_blocks() {
        let blocks = vec![
            PreviewBlock::new(BlockData::with_plain_text(
                BlockKind::Heading { level: 1 },
                "Main Header",
            )),
            PreviewBlock::new(BlockData::paragraph("Some body text")),
            PreviewBlock::new(BlockData::with_plain_text(
                BlockKind::Heading { level: 3 },
                "Sub Header",
            )),
        ];
        let headings = extract_preview_headings(&blocks);
        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0].label, "Main Header");
        assert_eq!(headings[0].level, 1);
        assert_eq!(headings[0].block_index, 0);
        assert_eq!(headings[1].label, "Sub Header");
        assert_eq!(headings[1].level, 3);
        assert_eq!(headings[1].block_index, 2);
    }
}
