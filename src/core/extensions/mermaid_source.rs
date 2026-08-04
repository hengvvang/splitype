//! Mermaid fenced-block parsing helpers.

/// Opening fence metadata for a Mermaid fenced code block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MermaidFence {
    /// Fence marker, either backtick or tilde.
    pub marker: char,
    /// Opening fence run length.
    pub len: usize,
}

/// Parsed Mermaid source preserved from Markdown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MermaidSource {
    /// Full Markdown source, including the opening and closing fences.
    pub raw: String,
    /// Mermaid diagram source between the fences.
    pub body: String,
    /// The full info string after the opening fence.
    pub info: String,
}

/// Returns true when a fenced code info string declares Mermaid content.
pub fn is_mermaid_info_string(info: Option<&str>) -> bool {
    info.and_then(|info| info.split_whitespace().next())
        .is_some_and(|first| {
            first.eq_ignore_ascii_case("mermaid") || first.eq_ignore_ascii_case("mmd")
        })
}

/// Parse a line as a Mermaid opening fence.
pub fn parse_mermaid_fence_start(line: &str) -> Option<MermaidFence> {
    let trimmed = strip_fence_indent(line)?.trim_end();
    let marker = trimmed.chars().next()?;
    if marker != '`' && marker != '~' {
        return None;
    }

    let len = trimmed.chars().take_while(|ch| *ch == marker).count();
    if len < 3 {
        return None;
    }

    let info = trimmed[marker.len_utf8() * len..].trim();
    if marker == '`' && info.contains('`') {
        return None;
    }

    is_mermaid_info_string((!info.is_empty()).then_some(info))
        .then_some(MermaidFence { marker, len })
}

/// Returns true when `line` closes the given Mermaid fence.
pub fn is_mermaid_closing_fence(line: &str, fence: MermaidFence) -> bool {
    let Some(trimmed) = strip_fence_indent(line).map(str::trim_end) else {
        return false;
    };
    if !trimmed.starts_with(fence.marker) {
        return false;
    }

    let len = trimmed.chars().take_while(|ch| *ch == fence.marker).count();
    len >= fence.len && trimmed[fence.marker.len_utf8() * len..].trim().is_empty()
}

/// Parse raw fenced Markdown into the Mermaid diagram source it contains.
pub fn parse_mermaid_fence_source(raw: &str) -> Option<MermaidSource> {
    let raw = raw.trim_matches('\n').to_string();
    let lines = raw.split('\n').collect::<Vec<_>>();
    if lines.len() < 2 {
        return None;
    }

    let opening = strip_fence_indent(lines[0])?.trim_end();
    let fence = parse_mermaid_fence_start(opening)?;
    let info = opening[fence.marker.len_utf8() * fence.len..]
        .trim()
        .to_string();
    if !is_mermaid_closing_fence(lines.last()?, fence) {
        return None;
    }

    let body = lines[1..lines.len() - 1].join("\n");
    Some(MermaidSource { raw, body, info })
}

fn strip_fence_indent(line: &str) -> Option<&str> {
    let indent = line.bytes().take_while(|b| *b == b' ').count();
    (indent <= 3).then_some(&line[indent..])
}

#[cfg(test)]
mod tests {
    use super::{
        is_mermaid_closing_fence, is_mermaid_info_string, parse_mermaid_fence_source,
        parse_mermaid_fence_start,
    };

    #[test]
    fn detects_mermaid_info_string() {
        assert!(is_mermaid_info_string(Some("mermaid")));
        assert!(is_mermaid_info_string(Some("mmd")));
        assert!(is_mermaid_info_string(Some("mermaid graph LR")));
        assert!(!is_mermaid_info_string(Some("python")));
        assert!(!is_mermaid_info_string(None));
    }

    #[test]
    fn parses_backtick_mermaid_fence() {
        let fence = parse_mermaid_fence_start("```mermaid").expect("backtick fence");
        assert_eq!(fence.marker, '`');
        assert_eq!(fence.len, 3);
    }

    #[test]
    fn parses_tilde_mmd_fence() {
        let fence = parse_mermaid_fence_start("~~~mmd").expect("tilde fence");
        assert_eq!(fence.marker, '~');
        assert_eq!(fence.len, 3);
    }

    #[test]
    fn rejects_unclosed_mermaid_fence() {
        let source = "```mermaid\ngraph LR\n";
        assert!(parse_mermaid_fence_source(source).is_none());
    }
}
