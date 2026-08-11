//! Bidirectional offset map and edit result types for inline text.
//!
//! `SourceOffsetMap` maps between plain text offsets (the fragment tree, no
//! markers) and generated source positions (the serialized Markdown). The
//! "plain" side is the tree text; the "source" side is the Markdown text
//! produced by serialization. `InlineEditResult` captures the new tree and
//! the offset mapping after an edit operation.

use std::ops::Range;

use crate::inline::text::BlockText;

/// Bidirectional offset map between plain inline text and source Markdown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceOffsetMap {
    pub(crate) source: String,
    pub(crate) plain_to_source: Vec<usize>,
    pub(crate) source_to_plain: Vec<usize>,
}

impl SourceOffsetMap {
    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn plain_to_source_offset(&self, offset: usize) -> usize {
        self.plain_to_source
            .get(offset.min(self.plain_to_source.len().saturating_sub(1)))
            .copied()
            .unwrap_or(0)
    }

    pub fn plain_to_source_range(&self, range: Range<usize>) -> Range<usize> {
        self.plain_to_source_offset(range.start)..self.plain_to_source_offset(range.end)
    }

    pub fn source_to_plain_offset(&self, offset: usize) -> usize {
        self.source_to_plain
            .get(offset.min(self.source_to_plain.len().saturating_sub(1)))
            .copied()
            .unwrap_or(0)
    }

    pub fn source_to_plain_range(&self, range: Range<usize>) -> Range<usize> {
        self.source_to_plain_offset(range.start)..self.source_to_plain_offset(range.end)
    }
}

/// Result of a text replacement operation, containing the normalized tree and
/// a mapping from pre-edit text offsets to post-edit tree offsets.
#[derive(Clone, Debug)]
pub struct InlineEditResult {
    pub tree: BlockText,
    pub(crate) visible_to_normalized: Vec<usize>,
}

impl InlineEditResult {
    pub fn map_offset(&self, offset: usize) -> usize {
        self.visible_to_normalized
            .get(offset.min(self.visible_to_normalized.len().saturating_sub(1)))
            .copied()
            .unwrap_or(0)
    }

    pub fn map_range(&self, range: &Range<usize>) -> Range<usize> {
        self.map_offset(range.start)..self.map_offset(range.end)
    }
}
