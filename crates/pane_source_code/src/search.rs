//! Source code search matching and slice replacement.

use crate::state::SourceCodeState;
use core_contracts::{SearchMatch, SearchQuery};
use std::ops::Range;

/// Searches within the raw source text buffer.
pub fn search_in_source(text: &str, query: &SearchQuery) -> Vec<SearchMatch> {
    let raw_matches = query.find_matches(text, 1);
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

/// Replaces a search match within the source text buffer.
pub fn replace_source_match(
    state: &mut SourceCodeState,
    match_item: &SearchMatch,
    replace_with: &str,
) {
    let range = match_item.byte_range.clone();
    if range.start <= state.text.len() && range.end <= state.text.len() {
        state.text.replace_range(range.clone(), replace_with);
        state.rebuild_lines();
        state
            .selections
            .set_single_point(range.start + replace_with.len());
        state.highlight_cache = None;
    }
}

/// Replaces a byte range within the source text buffer and refreshes line indexing.
pub fn replace_in_source(state: &mut SourceCodeState, range: Range<usize>, replacement: &str) {
    if range.start <= state.text.len() && range.end <= state.text.len() {
        state.text.replace_range(range.clone(), replacement);
        state.rebuild_lines();
        state
            .selections
            .set_single_point(range.start + replacement.len());
        state.highlight_cache = None;
    }
}

/// Replaces multiple ranges in the source text in reverse order.
pub fn replace_all_in_source(
    state: &mut SourceCodeState,
    mut replacements: Vec<(Range<usize>, String)>,
) {
    replacements.sort_by_key(|(r, _)| std::cmp::Reverse(r.start));
    for (range, replacement) in replacements {
        if range.start <= state.text.len() && range.end <= state.text.len() {
            state.text.replace_range(range, &replacement);
        }
    }
    state.rebuild_lines();
    state.highlight_cache = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_finds_correct_matches_in_source() {
        let state = SourceCodeState::from_text("Hello Rust\nHello World");
        let query = SearchQuery::new("Hello", true, false, false);
        let matches = search_in_source(&state.text, &query);
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].line_number, 1);
        assert_eq!(matches[1].line_number, 2);
    }

    #[test]
    fn replace_updates_source_text_and_cursor() {
        let mut state = SourceCodeState::from_text("Foo Bar Baz");
        replace_in_source(&mut state, 4..7, "Rust");
        assert_eq!(state.text, "Foo Rust Baz");
        assert_eq!(state.cursor(), 8);
    }
}
