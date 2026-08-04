//! Inline LaTeX math metadata for formatted text fragments.
//!
//! Inline math is stored as source-preserving metadata:
//! the original delimiters and body are kept so the fragment
//! can be serialized back to exact Markdown without loss.

/// Source-preserving inline LaTeX math metadata.
///
/// The `source` field keeps the full Markdown form including delimiters
/// (`$...$` or `\(...\)`), while `body` holds only the LaTeX content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InlineLatex {
    /// Full Markdown source including delimiters.
    pub source: String,
    /// LaTeX body between the delimiters.
    pub body: String,
    /// Delimiter syntax used by the source.
    pub delimiter: InlineLatexDelimiter,
}

/// Supported inline LaTeX math delimiter syntaxes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InlineLatexDelimiter {
    /// Dollar-delimited: `$...$`.
    Dollar,
    /// Parenthesis-delimited: `\(...\)`.
    Paren,
}
