//! Block editing semantics — the edit-mode enumeration.
//!
//! How a block edits: rendered rich text, verbatim source text, or raw
//! code block text. Splitting from `block.rs` keeps the block entity
//! file focused on data and behavior.

use markdown::parse::BlockKind;

/// Editing semantics for the current block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BlockEditMode {
    /// Markdown-delimited rich text with inline projection and shortcuts.
    RenderedRich,
    /// Source-text blocks (raw markdown, comments, HTML, math, mermaid)
    /// edited verbatim: no marker parsing, no inline shortcuts.
    Verbatim,
    /// Code blocks edited verbatim with line numbers and language chrome.
    CodeBlockRaw,
}

impl BlockEditMode {
    pub(crate) fn for_kind(kind: &BlockKind) -> Self {
        if kind.is_code_block() {
            Self::CodeBlockRaw
        } else if matches!(
            kind,
            BlockKind::RawMarkdown
                | BlockKind::HtmlComment
                | BlockKind::HtmlBlock
                | BlockKind::MathBlock
                | BlockKind::MermaidBlock
        ) {
            Self::Verbatim
        } else {
            Self::RenderedRich
        }
    }

    pub(crate) fn edits_verbatim_text(self) -> bool {
        matches!(self, Self::Verbatim | Self::CodeBlockRaw)
    }

    pub(crate) fn supports_inline_projection(self) -> bool {
        matches!(self, Self::RenderedRich)
    }
}
