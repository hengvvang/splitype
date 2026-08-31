//! Typing behavior: newline handling, table extension, and shortcut transformations.

use gpui::*;

use crate::markdown::inline::text::BlockText;
use crate::markdown::parse::BlockKind;
use crate::model::block::Block;

/// Checks if a block is a candidate for Setext heading promotion.
pub fn is_setext_heading_target(block: &Entity<Block>, cx: &App) -> bool {
    let block = block.read(cx);
    if block.kind() != BlockKind::Paragraph || !block.children.is_empty() {
        return false;
    }
    let text = block.data.text.plain_text();
    !text.trim().is_empty() && !text.contains('\n')
}

/// Applies Markdown paragraph prefix shortcuts (e.g. `# ` -> Heading, `- ` -> List, etc.).
pub fn apply_paragraph_shortcuts(
    kind: BlockKind,
    mut text: BlockText,
    cursor: usize,
) -> (BlockKind, BlockText, usize) {
    if kind == BlockKind::Paragraph {
        let plain_text = text.plain_text();
        if let Some((detected_kind, prefix_len)) = BlockKind::detect_markdown_shortcut(&plain_text)
        {
            text.remove_plain_prefix(prefix_len);
            return (detected_kind, text, cursor.saturating_sub(prefix_len));
        }
    }

    (kind, text, cursor)
}
