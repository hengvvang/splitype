//! Display math source parsing and serialization — `$$...$$` Markdown blocks.

use std::ops::Range;

/// Parsed display-math source preserved from Markdown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayMathSource {
    /// Full Markdown source, including `$$` delimiters.
    pub source: String,
    /// LaTeX body between the display delimiters.
    pub body: String,
}

/// Parse a raw `$$...$$` Markdown block into the LaTeX body it contains.
pub fn parse_display_math_source(raw: &str) -> Option<DisplayMathSource> {
    let raw = raw.trim_matches('\n').to_string();
    let lines = raw.split('\n').collect::<Vec<_>>();
    if lines.is_empty() {
        return None;
    }

    if lines.len() == 1 {
        let line = strip_display_indent(lines[0])?.trim_end();
        let body_and_close = line.strip_prefix("$$")?;
        let close = body_and_close.find("$$")?;
        let body = body_and_close[..close].trim().to_string();
        return Some(DisplayMathSource { source: raw, body });
    }

    let opener = strip_display_indent(lines[0])?.trim_end();
    let closer = lines.last()?.trim();
    if opener != "$$" || closer != "$$" {
        return None;
    }

    let body = lines[1..lines.len() - 1].join("\n");
    Some(DisplayMathSource { source: raw, body })
}

/// Serialize a display-math block body back to canonical Markdown, returning
/// the serialized source and the byte range of the formula body within it.
///
/// A body that already forms a complete `$$...$$` source is preserved
/// verbatim (defensive round-trip for legacy data); otherwise it is wrapped
/// in display delimiters — single-line for newline-free formulas, fenced for
/// empty or multi-line ones.
pub fn serialize_display_math_source(body: &str) -> (String, Range<usize>) {
    if let Some(source) = parse_display_math_source(body) {
        let start = source.source.find(&source.body).unwrap_or(0);
        return (source.source, start..start + source.body.len());
    }
    if body.is_empty() || body.contains('\n') {
        let wrapped = format!("$$\n{body}\n$$");
        (wrapped, "$$\n".len().."$$\n".len() + body.len())
    } else {
        (format!("$${body}$$"), "$$".len().."$$".len() + body.len())
    }
}

fn strip_display_indent(line: &str) -> Option<&str> {
    let indent = line.bytes().take_while(|byte| *byte == b' ').count();
    (indent <= 3).then_some(&line[indent..])
}

#[cfg(test)]
mod tests {
    use super::{parse_display_math_source, serialize_display_math_source};

    #[test]
    fn parses_single_line_display_math() {
        let parsed = parse_display_math_source("$$x^2$$").expect("display math");
        assert_eq!(parsed.body, "x^2");
        assert_eq!(parsed.source, "$$x^2$$");
    }

    #[test]
    fn parses_multiline_display_math() {
        let parsed = parse_display_math_source("$$\n\\int_0^1 x^2 dx\n$$").expect("display math");
        assert_eq!(parsed.body, "\\int_0^1 x^2 dx");
    }

    #[test]
    fn rejects_unclosed_display_math() {
        assert!(parse_display_math_source("$$\n\\frac{1}{2}").is_none());
    }

    #[test]
    fn serializes_newline_free_body_single_line() {
        let (source, body_range) = serialize_display_math_source("x^2");
        assert_eq!(source, "$$x^2$$");
        assert_eq!(body_range, 2..5);
    }

    #[test]
    fn serializes_multi_line_and_empty_bodies_fenced() {
        let (source, body_range) = serialize_display_math_source("a\nb");
        assert_eq!(source, "$$\na\nb\n$$");
        assert_eq!(body_range, 3..6);

        let (source, body_range) = serialize_display_math_source("");
        assert_eq!(source, "$$\n\n$$");
        assert_eq!(body_range, 3..3);
    }

    #[test]
    fn serialization_preserves_already_fenced_body() {
        let (source, body_range) = serialize_display_math_source("$$x^2$$");
        assert_eq!(source, "$$x^2$$");
        assert_eq!(body_range, 2..5);
    }
}
