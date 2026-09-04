//! Preview pane search matching, highlight distribution, and navigation.

use crate::block::PreviewBlock;
use crate::outline::calculate_scroll_offset_for_block_index;
use crate::pane::PreviewPane;
use editor_contracts::{SearchMatch, SearchQuery};

/// Searches within the Markdown text source of the Preview pane.
pub fn search_in_preview(markdown: &str, query: &SearchQuery) -> Vec<SearchMatch> {
    let raw_matches = query.find_matches(markdown, 1);
    let mut results = Vec::with_capacity(raw_matches.len());

    for mat in raw_matches {
        results.push(SearchMatch {
            file_path: None,
            file_name: "Current File".to_string(),
            block_id: None,
            entity_id: None,
            line_number: mat.line_number,
            column_number: mat.column_number,
            byte_range: mat.byte_range,
            preview_prefix: mat.preview_prefix,
            preview_match: mat.preview_match,
            preview_suffix: mat.preview_suffix,
        });
    }

    results
}

/// The separator used when joining block texts into the searchable document
/// text; must match [`PreviewPane::search_matches`] joining.
const BLOCK_JOIN_SEPARATOR_LEN: usize = 2;

/// Locates the root block containing a document-text byte offset using the
/// same cumulative offsets as the joined search text.
fn block_index_for_offset(blocks: &[PreviewBlock], offset: usize) -> Option<usize> {
    let mut cumulative = 0usize;
    for (index, block) in blocks.iter().enumerate() {
        let len = block.display_len();
        if offset < cumulative + len {
            return Some(index);
        }
        cumulative += len + BLOCK_JOIN_SEPARATOR_LEN;
    }
    None
}

/// Distributes document-text match ranges onto the per-block local ranges the
/// preview renderer decorates.
pub fn distribute_search_highlights(
    blocks: &mut [PreviewBlock],
    matches: &[SearchMatch],
    active_index: Option<usize>,
) {
    for block in blocks.iter_mut() {
        block.search_matches.clear();
    }
    let mut cumulative = 0usize;
    for block in blocks.iter_mut() {
        let len = block.display_len();
        for (index, match_item) in matches.iter().enumerate() {
            let start = match_item
                .byte_range
                .start
                .max(cumulative)
                .saturating_sub(cumulative);
            let end = match_item
                .byte_range
                .end
                .min(cumulative + len)
                .saturating_sub(cumulative);
            if start < end {
                block
                    .search_matches
                    .push((start..end, Some(index) == active_index));
            }
        }
        cumulative += len + BLOCK_JOIN_SEPARATOR_LEN;
    }
}

/// Calculates the scroll Y offset of the block containing the match.
pub fn calculate_scroll_offset_for_match(
    state: &PreviewPane,
    match_item: &SearchMatch,
    line_height: f32,
) -> Option<f32> {
    let block_index = block_index_for_offset(&state.blocks, match_item.byte_range.start)?;
    Some(calculate_scroll_offset_for_block_index(
        state,
        block_index,
        line_height,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn searches_in_preview_markdown() {
        let md = "Line 1 with keyword\nLine 2 without\nLine 3 with keyword";
        let query = SearchQuery::new("keyword", false, false, false);
        let matches = search_in_preview(md, &query);
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].line_number, 1);
        assert_eq!(matches[1].line_number, 3);
    }
}
