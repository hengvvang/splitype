//! Display math source parsing — `$$...$$` Markdown blocks.

/// Parsed display-math source preserved from Markdown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayMathSource {
    /// Full Markdown source, including `$$` delimiters.
    pub raw: String,
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
        return Some(DisplayMathSource { raw, body });
    }

    let opener = strip_display_indent(lines[0])?.trim_end();
    let closer = lines.last()?.trim();
    if opener != "$$" || closer != "$$" {
        return None;
    }

    let body = lines[1..lines.len() - 1].join("\n");
    Some(DisplayMathSource { raw, body })
}

fn strip_display_indent(line: &str) -> Option<&str> {
    let indent = line.bytes().take_while(|byte| *byte == b' ').count();
    (indent <= 3).then_some(&line[indent..])
}

#[cfg(test)]
mod tests {
    use super::parse_display_math_source;

    #[test]
    fn parses_single_line_display_math() {
        let parsed = parse_display_math_source("$$x^2$$").expect("display math");
        assert_eq!(parsed.body, "x^2");
        assert_eq!(parsed.raw, "$$x^2$$");
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
}
