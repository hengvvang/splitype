//! Inline footnote reference types.
//!
//! Footnote references appear in inline text as `[^id]` syntax.
//! Each reference links to a footnote definition block elsewhere in the document.

/// Inline footnote reference parsed from `[^id]` syntax.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InlineFootnoteReference {
    /// Footnote identifier without the `[^` and `]` markers.
    pub id: String,
    /// Zero-based occurrence count within the block.
    pub occurrence_index: usize,
}

/// Hit-test payload for a rendered footnote reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InlineFootnoteHit {
    pub id: String,
    pub occurrence_index: usize,
}

impl InlineFootnoteReference {
    /// Reconstruct the raw `[^id]` Markdown for this reference.
    pub fn raw_markdown(&self) -> String {
        format!("[^{}]", self.id)
    }

    /// Produce a hit-test payload for the rendered reference.
    pub(crate) fn hit(&self) -> Option<InlineFootnoteHit> {
        Some(InlineFootnoteHit {
            id: self.id.clone(),
            occurrence_index: self.occurrence_index,
        })
    }
}

/// Returns true when `id` is a valid footnote identifier.
pub(crate) fn is_valid_footnote_id(id: &str) -> bool {
    !id.is_empty()
        && !id
            .chars()
            .any(|ch| matches!(ch, ' ' | '\t' | '\n' | '\r' | '^' | '[' | ']' | ':'))
}

/// Parses an inline footnote reference `[^id]` from Markdown text,
/// returning the footnote id.
pub(crate) fn parse_inline_footnote_reference(markdown: &str) -> Option<String> {
    let rest = markdown.strip_prefix("[^")?;
    let bracket_end = rest.find(']')?;
    let id = &rest[..bracket_end];
    is_valid_footnote_id(id).then(|| id.to_string())
}
