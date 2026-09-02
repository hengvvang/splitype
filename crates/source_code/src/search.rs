//! Source code search matching and slice replacement.
//!
//! Matching is a pure function over document text; replacement flows
//! through the editor's `replace_range` / `replace_all_ranges` operations
//! so every change becomes a buffer-level edit transaction.

use editor_contracts::{SearchMatch, SearchQuery};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_finds_correct_matches_in_source() {
        let text = "Hello Rust\nHello World";
        let query = SearchQuery::new("Hello", true, false, false);
        let matches = search_in_source(text, &query);
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].line_number, 1);
        assert_eq!(matches[1].line_number, 2);
    }
}
