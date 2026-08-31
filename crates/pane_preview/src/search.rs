//! Preview pane search matching and highlight navigation.

use crate::PreviewState;
use core_contracts::{SearchMatch, SearchQuery};

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

/// Calculates scroll Y offset to center the matched line/block in Preview.
pub fn calculate_scroll_offset_for_match(
    state: &PreviewState,
    match_item: &SearchMatch,
    line_height: f32,
) -> f32 {
    let line_number = match_item.line_number.saturating_sub(1);
    let block_count = state.blocks.len().max(1);
    let target_idx = line_number.min(block_count.saturating_sub(1));
    (target_idx as f32 * line_height * 2.0).max(0.0)
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
