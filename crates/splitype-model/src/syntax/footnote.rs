//! Footnote definition-head parsing (pure Markdown syntax).

/// Returns true when `id` is a valid footnote identifier.
pub fn is_valid_footnote_id(id: &str) -> bool {
    !id.is_empty()
        && !id
            .chars()
            .any(|ch| matches!(ch, ' ' | '\t' | '\n' | '\r' | '^' | '[' | ']'))
}

/// Parse a footnote definition head line `[^id]: body`, returning the id and
/// the remainder after the marker.
pub fn parse_footnote_definition_head(line: &str) -> Option<(String, String)> {
    let trimmed_end = line.trim_end();
    let leading_spaces = trimmed_end.bytes().take_while(|b| *b == b' ').count();
    if leading_spaces > 3 {
        return None;
    }

    let rest = &trimmed_end[leading_spaces..];
    let after_open = rest.strip_prefix("[^")?;
    let label_end = after_open.find("]:")?;
    let id = after_open[..label_end].to_string();
    if !is_valid_footnote_id(&id) {
        return None;
    }

    let remainder = after_open[label_end + 2..]
        .strip_prefix(' ')
        .unwrap_or(&after_open[label_end + 2..])
        .to_string();
    Some((id, remainder))
}

#[cfg(test)]
mod tests {
    use super::{is_valid_footnote_id, parse_footnote_definition_head};

    #[test]
    fn validates_footnote_ids() {
        assert!(is_valid_footnote_id("long-note"));
        assert!(!is_valid_footnote_id("bad id"));
    }

    #[test]
    fn parses_definition_head() {
        assert_eq!(
            parse_footnote_definition_head("[^ref-1]: body"),
            Some(("ref-1".to_string(), "body".to_string()))
        );
    }
}
