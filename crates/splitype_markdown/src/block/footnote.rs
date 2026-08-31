//! Footnote definition-head parsing (pure Markdown syntax).

/// Returns true when `id` is a valid footnote identifier.
///
/// Colons are rejected so a definition's display text (`id: content`, where
/// the first line of content lives in the block text) can always be split
/// back into its id and content parts unambiguously.
pub fn is_valid_footnote_id(id: &str) -> bool {
    !id.is_empty()
        && !id
            .chars()
            .any(|ch| matches!(ch, ' ' | '\t' | '\n' | '\r' | '^' | '[' | ']' | ':'))
}

/// Splits a footnote definition's display text (`id: content`) back into its
/// id and content parts. The content is trimmed of its single leading space.
pub fn split_footnote_definition_text(text: &str) -> (&str, &str) {
    match text.split_once(':') {
        Some((id, content)) => (id, content.trim_start()),
        None => (text, ""),
    }
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

