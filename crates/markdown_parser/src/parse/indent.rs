//! Pure utility functions for Markdown I/O.
//!
//! These helpers are extracted from the parser module and operate on raw
//! string slices — they carry no dependency on GPUI or the runtime editor.

/// Strip up to 3 leading spaces from a code-fence candidate line.
///
/// Returns `None` if the indent exceeds 3 spaces, per CommonMark §4.5.
pub fn strip_fence_indent(line: &str) -> Option<&str> {
    let indent = line.bytes().take_while(|b| *b == b' ').count();
    (indent <= 3).then_some(&line[indent..])
}

/// Advance from `start` until the first blank line (or end of input).
///
/// Returns the index of the first blank line (or `lines.len()`).
pub fn collect_until_blank_line<S: AsRef<str>>(lines: &[S], start: usize) -> usize {
    let mut index = start + 1;
    while index < lines.len() && !lines[index].as_ref().trim().is_empty() {
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

/// Dedent every line by at least `columns` display columns.
///
/// Lines with insufficient leading whitespace are passed through unchanged.
pub fn dedent_lines<S: AsRef<str>>(lines: &[S], columns: usize) -> Vec<String> {
    lines
        .iter()
        .map(|line| {
            let line = line.as_ref();
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
