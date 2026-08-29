//! SearchQuery — the pure, precompiled matching engine.

use crate::SearchQuery;

#[test]
fn plain_query_matches_literally() {
    let query = SearchQuery::new("hello", false, false, false);
    let matches = query.find_matches("say hello, hello world", 0);
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].preview_match, "hello");
    assert_eq!(matches[0].byte_range, 4..9);
    assert_eq!(matches[0].column_number, 5);
    // Line numbers are relative to the caller-supplied base.
    assert_eq!(matches[0].line_number, 0);
}

#[test]
fn case_sensitivity_flag_controls_matching() {
    let insensitive = SearchQuery::new("Hello", false, false, false);
    assert_eq!(insensitive.find_matches("hello", 0).len(), 1);

    let sensitive = SearchQuery::new("Hello", true, false, false);
    assert_eq!(sensitive.find_matches("hello", 0).len(), 0);
    assert_eq!(sensitive.find_matches("Hello", 0).len(), 1);
}

#[test]
fn whole_word_requires_boundaries() {
    let query = SearchQuery::new("cat", false, true, false);
    let matches = query.find_matches("cat catalog scatter cat!", 0);
    assert_eq!(matches.len(), 2);
}

#[test]
fn regex_mode_uses_patterns() {
    let query = SearchQuery::new(r"\d{3}", false, false, true);
    let matches = query.find_matches("123 45 6789", 0);
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].preview_match, "123");
    assert_eq!(matches[1].preview_match, "678");
}

#[test]
fn invalid_regex_matches_nothing() {
    let query = SearchQuery::new("(", false, false, true);
    assert!(!query.is_valid());
    assert!(query.find_matches("(", 0).is_empty());
}

#[test]
fn line_numbers_are_based_from_argument() {
    let query = SearchQuery::new("needle", false, false, false);
    let matches = query.find_matches("first\nsecond needle\nthird", 10);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].line_number, 11);
}

#[test]
fn empty_query_matches_nothing() {
    let query = SearchQuery::new("", false, false, false);
    assert!(query.find_matches("any text", 0).is_empty());
}

#[test]
fn preview_prefix_and_suffix_are_capped() {
    let query = SearchQuery::new("x", false, false, false);
    let haystack = format!("{}x{}", "a".repeat(40), "b".repeat(40));
    let matches = query.find_matches(&haystack, 0);
    assert_eq!(matches.len(), 1);
    assert!(matches[0].preview_prefix.chars().count() <= 20);
    assert!(matches[0].preview_suffix.chars().count() <= 20);
}

#[test]
fn multiline_matches_flatten_newlines_in_preview() {
    let query = SearchQuery::new("a\nb", false, false, false);
    let matches = query.find_matches("x a\nb y", 0);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].preview_match, "a b");
}
