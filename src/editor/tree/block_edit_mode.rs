//! Block editing semantics — the edit-mode enumeration.
//!
//! How a block edits: rendered rich text, raw source, or raw code
//! block text. Splitting from `block.rs` keeps the block entity file
//! focused on data and behavior.

use crate::model::block::BlockKind;

/// Editing semantics for the current block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BlockEditMode {
    RenderedRich,
    SourceRaw,
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
            Self::SourceRaw
        } else {
            Self::RenderedRich
        }
    }

    pub(crate) fn uses_raw_text_editing(self) -> bool {
        matches!(self, Self::SourceRaw | Self::CodeBlockRaw)
    }

    pub(crate) fn supports_inline_projection(self) -> bool {
        matches!(self, Self::RenderedRich)
    }
}
