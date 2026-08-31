//! Rendered-quote metadata refresh and multi-line quote check helpers.

use gpui::*;

use crate::model::block::Block;

/// Checks whether a block is related to quote structures.
pub fn is_block_quote_structure_related(block: &Entity<Block>, cx: &App) -> bool {
    let block_ref = block.read(cx);
    block_ref.kind().is_quote_container()
        || block_ref.quote_depth > 0
        || block_ref.quote_group_id.is_some()
}

/// Checks whether rendered quote text requires a full reparse.
pub fn rendered_quote_text_requires_reparse(block: &Entity<Block>, cx: &App) -> bool {
    let block_ref = block.read(cx);
    if block_ref.quote_depth == 0 && !block_ref.kind().is_quote_container() {
        return false;
    }

    let text = block_ref.display_text();
    if !text.contains('\n') {
        return false;
    }

    text.split('\n').skip(1).any(|line| {
        let trimmed_end = line.trim_end();
        if trimmed_end.is_empty() {
            return false;
        }
        let leading_spaces = trimmed_end.bytes().take_while(|b| *b == b' ').count();
        trimmed_end[leading_spaces..].starts_with('>')
    })
}

