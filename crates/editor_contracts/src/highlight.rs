//! Highlight vocabulary shared by the document buffer and pane views:
//! token classes, byte-range spans, and the per-revision snapshot panes
//! render from.

use std::ops::Range;
use std::sync::Arc;

/// Token class category for code highlighting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CodeHighlightClass {
    /// Source code comment.
    Comment,
    /// Language keyword.
    Keyword,
    /// String literal.
    String,
    /// Numeric literal.
    Number,
    /// Type identifier.
    Type,
    /// Function or callable identifier.
    Function,
    /// Constant identifier.
    Constant,
    /// Variable identifier.
    Variable,
    /// Object or record property.
    Property,
    /// Operator token.
    Operator,
    /// Punctuation token.
    Punctuation,
    /// Markdown heading text (level 1..=6).
    MarkupHeading(u8),
    /// Markdown strong emphasis: rendered bold.
    MarkupBold,
    /// Markdown emphasis: rendered italic.
    MarkupItalic,
    /// Markdown inline code span: tinted background.
    MarkupCode,
    /// Markdown link text: colored and underlined.
    MarkupLink,
    /// Markdown link destination / autolink URI.
    MarkupUri,
    /// Markdown list markers and thematic breaks.
    MarkupList,
    /// Markdown block-quote markers.
    MarkupQuote,
    /// Backslash escapes and hard line breaks.
    MarkupEscape,
}

/// Highlighted byte range inside a document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodeHighlightSpan {
    pub range: Range<usize>,
    pub class: CodeHighlightClass,
}

/// Immutable highlight state for one document revision, shared with every
/// pane. `version` is the highlight map version the spans were computed
/// for; panes render `spans` regardless (stale-while-revalidate) and can
/// compare versions to detect freshness.
#[derive(Clone, Debug)]
pub struct HighlightSnapshot {
    pub version: u64,
    pub spans: Arc<[CodeHighlightSpan]>,
}
