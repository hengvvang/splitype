//! Pure, precompiled search query calculation engine.
//!
//! Decouples query parsing, regular expression compilation, and text scanning
//! from GPUI contexts and UI view rendering.

use std::ops::Range;
use regex::Regex;

use super::state::{ceil_char_boundary, floor_char_boundary};

/// A single matched substring occurrence with line/column coordinates and context slices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawMatch {
    /// 1-indexed line number in the source text.
    pub line_number: usize,
    /// 1-indexed column number in Unicode characters.
    pub column_number: usize,
    /// Byte range within the scanned text.
    pub byte_range: Range<usize>,
    /// Leading context snippet on the same line (up to 20 Unicode chars).
    pub preview_prefix: String,
    /// Exact matched substring (with newlines flattened to spaces).
    pub preview_match: String,
    /// Trailing context snippet on the same line (up to 20 Unicode chars).
    pub preview_suffix: String,
}

/// An immutable, precompiled search query.
#[derive(Clone, Debug)]
pub struct SearchQuery {
    raw_query: String,
    match_case: bool,
    whole_word: bool,
    use_regex: bool,
    compiled_regex: Option<Regex>,
}

impl SearchQuery {
    /// Constructs and precompiles a new `SearchQuery`.
    ///
    /// If regex syntax is invalid, `compiled_regex` is `None`, which safely matches nothing.
    pub fn new(
        query: impl Into<String>,
        match_case: bool,
        whole_word: bool,
        use_regex: bool,
    ) -> Self {
        let raw_query = query.into();
        if raw_query.is_empty() {
            return Self {
                raw_query,
                match_case,
                whole_word,
                use_regex,
                compiled_regex: None,
            };
        }

        let pattern = if use_regex {
            raw_query.clone()
        } else {
            let escaped = regex::escape(&raw_query);
            if whole_word {
                format!(r"\b{}\b", escaped)
            } else {
                escaped
            }
        };

        let mut builder = regex::RegexBuilder::new(&pattern);
        builder.case_insensitive(!match_case);
        let compiled_regex = builder.build().ok();

        Self {
            raw_query,
            match_case,
            whole_word,
            use_regex,
            compiled_regex,
        }
    }

    /// The raw query string.
    #[inline]
    pub fn raw_query(&self) -> &str {
        &self.raw_query
    }

    /// Whether case matching is enabled.
    #[inline]
    pub fn match_case(&self) -> bool {
        self.match_case
    }

    /// Whether whole word matching is enabled.
    #[inline]
    pub fn whole_word(&self) -> bool {
        self.whole_word
    }

    /// Whether regex mode is enabled.
    #[inline]
    pub fn use_regex(&self) -> bool {
        self.use_regex
    }

    /// Whether the query has a valid, compiled matcher.
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.compiled_regex.is_some()
    }

    /// Scans a text buffer and returns all matching occurrences with full Unicode safety.
    pub fn find_matches(&self, text: &str, base_line_number: usize) -> Vec<RawMatch> {
        if text.is_empty() {
            return Vec::new();
        }

        let Some(ref regex) = self.compiled_regex else {
            return Vec::new();
        };

        let mut results = Vec::new();

        for mat in regex.find_iter(text) {
            let range = mat.range();
            let start = floor_char_boundary(text, range.start);
            let end = ceil_char_boundary(text, range.end.max(start));
            let matched_slice = &text[start..end];

            let prefix_text = &text[..start];
            let relative_line = prefix_text.matches('\n').count();
            let actual_line = base_line_number + relative_line;

            let last_nl = prefix_text.rfind('\n').map(|p| p + 1).unwrap_or(0);
            let same_line_prefix = &prefix_text[last_nl..];
            let column_number = same_line_prefix.chars().count() + 1;

            let suffix_text = &text[end..];
            let next_nl = suffix_text.find('\n').unwrap_or(suffix_text.len());
            let same_line_suffix = &suffix_text[..next_nl];

            // Extract up to 20 Unicode chars on the same line before match
            let preview_prefix: String = same_line_prefix
                .chars()
                .rev()
                .take(20)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();

            // Extract up to 20 Unicode chars on the same line after match
            let preview_suffix: String = same_line_suffix
                .chars()
                .take(20)
                .collect();

            results.push(RawMatch {
                line_number: actual_line,
                column_number,
                byte_range: start..end,
                preview_prefix,
                preview_match: matched_slice.replace(['\r', '\n'], " "),
                preview_suffix,
            });
        }

        results
    }
}

/// Computes replacement string matching the casing style of `matched_slice` if `preserve_case` is true.
pub fn compute_preserve_case_replacement(matched_slice: &str, replacement: &str, preserve_case: bool) -> String {
    if !preserve_case || matched_slice.is_empty() || replacement.is_empty() {
        return replacement.to_string();
    }

    let is_all_upper = matched_slice.chars().all(|c| !c.is_alphabetic() || c.is_uppercase());
    let is_all_lower = matched_slice.chars().all(|c| !c.is_alphabetic() || c.is_lowercase());

    let is_title_case = {
        let mut chars = matched_slice.chars();
        match chars.next() {
            Some(first) if first.is_uppercase() => {
                chars.all(|c| !c.is_alphabetic() || c.is_lowercase())
            }
            _ => false,
        }
    };

    if is_all_upper {
        replacement.to_uppercase()
    } else if is_title_case {
        let mut chars = replacement.chars();
        match chars.next() {
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            None => String::new(),
        }
    } else if is_all_lower {
        replacement.to_lowercase()
    } else {
        replacement.to_string()
    }
}
