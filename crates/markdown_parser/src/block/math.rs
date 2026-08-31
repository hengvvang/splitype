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
/// Single-line for newline-free formulas, fenced for empty or multi-line ones.
pub fn serialize_display_math_source(body: &str) -> (String, Range<usize>) {
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
