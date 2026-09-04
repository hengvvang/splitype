//! Pure utility functions for Markdown I/O.
//!
//! These helpers are extracted from the parser module and operate on raw
//! string slices — they carry no dependency on GPUI or the runtime editor.

use crate::parse::lines::Lines;

/// Strip up to 3 leading spaces from a code-fence candidate line.
///
/// Returns `None` if the indent exceeds 3 spaces, per CommonMark §4.5.
pub fn strip_fence_indent(line: &str) -> Option<&str> {
    let indent = line.bytes().take_while(|b| *b == b' ').count();
    (indent <= 3).then_some(&line[indent..])
}

/// Advance from `start` until the first blank line (or end of input).
///
/// Returns the index of the first blank line (or `lines.line_count()`).
pub fn collect_until_blank_line<L: Lines + ?Sized>(lines: &L, start: usize) -> usize {
    let mut index = start + 1;
    while index < lines.line_count() && !lines.line(index).trim().is_empty() {
        index += 1;
    }
    index
}

/// Count leading indent columns and bytes for a line, treating tabs as
/// aligning to the next multiple-of-4 column.
pub fn leading_indent_columns_and_bytes(line: &str) -> (usize, usize) {
    let mut columns = 0usize;
    let mut bytes = 0usize;
    for ch in line.chars() {
        match ch {
            ' ' => {
                columns += 1;
                bytes += 1;
            }
            '\t' => {
                columns += 4 - (columns % 4);
                bytes += 1;
            }
            _ => break,
        }
    }
    (columns, bytes)
}

/// Compute the display width (in "columns") of a string, treating tabs
/// as aligning to the next multiple-of-4 column.
pub fn display_columns(value: &str) -> usize {
    let mut columns = 0usize;
    for ch in value.chars() {
        match ch {
            '\t' => columns += 4 - (columns % 4),
            _ => columns += 1,
        }
    }
    columns
}

/// Strip a specific number of display columns from the start of a line.
///
/// Returns `None` if the line does not have enough leading whitespace
/// (tabs counted as 4-column stops). Blank lines return `Some("")`.
pub fn strip_leading_columns(line: &str, columns: usize) -> Option<&str> {
    if columns == 0 {
        return Some(line);
    }
    if line.trim().is_empty() {
        return Some("");
    }

    let mut consumed_columns = 0usize;
    for (idx, ch) in line.char_indices() {
        let bytes_after_char = idx + ch.len_utf8();
        match ch {
            ' ' => {
                consumed_columns += 1;
            }
            '\t' => {
                consumed_columns += 4 - (consumed_columns % 4);
            }
            _ => break,
        }

        if consumed_columns >= columns {
            return Some(&line[bytes_after_char..]);
        }
    }

    None
}

/// Longest common byte prefix and suffix of `a` and `b`, snapped to UTF-8
/// character boundaries on both sides. Raw byte equality can diverge (or
/// resume matching) inside a multi-byte character — e.g. `E4 B8 AD` vs
/// `E4 B8 AE` — so the byte-level scan alone does not yield sliceable
/// positions; both results are stepped back until each input's cut point
/// falls on a character boundary. The unstripped middle therefore covers
/// every differing byte, possibly plus one partially shared character per
/// side.
pub fn common_affix(a: &str, b: &str) -> (usize, usize) {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    let mut prefix = 0usize;
    while prefix < a_bytes.len() && prefix < b_bytes.len() && a_bytes[prefix] == b_bytes[prefix] {
        prefix += 1;
    }
    while prefix > 0 && !(a.is_char_boundary(prefix) && b.is_char_boundary(prefix)) {
        prefix -= 1;
    }
    let mut suffix = 0usize;
    while suffix < a_bytes.len() - prefix
        && suffix < b_bytes.len() - prefix
        && a_bytes[a_bytes.len() - 1 - suffix] == b_bytes[b_bytes.len() - 1 - suffix]
    {
        suffix += 1;
    }
    while suffix > 0
        && !(a.is_char_boundary(a.len() - suffix) && b.is_char_boundary(b.len() - suffix))
    {
        suffix -= 1;
    }
    (prefix, suffix)
}

/// Dedent every line by at least `columns` display columns.
///
/// Lines with insufficient leading whitespace are passed through unchanged.
pub fn dedent_lines<L: Lines + ?Sized>(lines: &L, columns: usize) -> Vec<String> {
    (0..lines.line_count())
        .map(|index| {
            let line = lines.line(index);
            strip_leading_columns(line, columns)
                .unwrap_or(line)
                .to_string()
        })
        .collect()
}

/// Strip an indented-code-block prefix (4 spaces or 1 tab) from a line.
///
/// Returns `None` if the line does not start with an indented-code prefix.
pub fn strip_indented_code_prefix(line: &str) -> Option<&str> {
    if let Some(rest) = line.strip_prefix('\t') {
        Some(rest)
    } else {
        line.strip_prefix("    ")
    }
}

/// Check if a line starts a blockquote (`>` with ≤ 3 leading spaces).
pub fn is_quote_start(line: &str) -> bool {
    let trimmed_end = line.trim_end();
    let leading_spaces = trimmed_end.bytes().take_while(|b| *b == b' ').count();
    leading_spaces <= 3 && trimmed_end[leading_spaces..].starts_with('>')
}

/// Strip one level of blockquote prefix from a line.
///
/// Returns the inner text with the leading `>` (and optional following
/// space) removed. Returns `None` if the line is not a quote line.
pub fn strip_one_quote_level(line: &str) -> Option<String> {
    let leading_spaces = line.bytes().take_while(|b| *b == b' ').count();
    if leading_spaces > 3 {
        return None;
    }

    let rest = &line[leading_spaces..];
    if !rest.starts_with('>') {
        return None;
    }

    Some(
        rest[1..]
            .strip_prefix(' ')
            .unwrap_or(&rest[1..])
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::common_affix;

    #[test]
    fn snapshots_prefix_and_suffix_to_char_boundaries() {
        // '中' = E4 B8 AD; U+4E2E = E4 B8 AE. The bytes diverge at index 2,
        // inside the character: the prefix must snap back to 0.
        assert_eq!(common_affix("中", "\u{4E2E}"), (0, 0));
        // Shared suffix where the cut would land inside a character.
        assert_eq!(common_affix("x中", "y中"), (0, 3));
        // Ordinary ASCII case.
        assert_eq!(common_affix("abXcd", "abYcd"), (2, 2));
        // Identical inputs: full prefix, no suffix.
        assert_eq!(common_affix("same", "same"), (4, 0));
        // One input a prefix of the other.
        assert_eq!(common_affix("ab", "abc"), (2, 0));
    }
}
