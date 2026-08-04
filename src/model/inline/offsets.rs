//! Bidirectional offset map and edit result types for inline Markdown text.
//!
//! `InlineMarkdownOffsetMap` maps between visible text offsets and generated
//! Markdown source positions. `InlineEditResult` captures the new tree and
//! selected offset mapping after an edit operation.

use std::ops::Range;

use crate::model::inline::text::RichText;

/// Bidirectional offset map between source Markdown and visible inline text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InlineMarkdownOffsetMap {
    pub(crate) markdown: String,
    pub(crate) visible_to_markdown: Vec<usize>,
    pub(crate) markdown_to_visible: Vec<usize>,
}

impl InlineMarkdownOffsetMap {
    pub(crate) fn markdown(&self) -> &str {
        &self.markdown
    }

    pub(crate) fn visible_to_markdown_offset(&self, offset: usize) -> usize {
        self.visible_to_markdown
            .get(offset.min(self.visible_to_markdown.len().saturating_sub(1)))
            .copied()
            .unwrap_or(0)
    }

    pub(crate) fn visible_to_markdown_range(&self, range: Range<usize>) -> Range<usize> {
        self.visible_to_markdown_offset(range.start)..self.visible_to_markdown_offset(range.end)
    }

    pub(crate) fn markdown_to_visible_offset(&self, offset: usize) -> usize {
        self.markdown_to_visible
            .get(offset.min(self.markdown_to_visible.len().saturating_sub(1)))
            .copied()
            .unwrap_or(0)
    }

    pub(crate) fn markdown_to_visible_range(&self, range: Range<usize>) -> Range<usize> {
        self.markdown_to_visible_offset(range.start)..self.markdown_to_visible_offset(range.end)
    }
}

/// Result of a visible-text replacement operation, containing the
/// normalized tree and a mapping from pre-edit visible offsets to
/// post-edit tree offsets.
#[derive(Clone, Debug)]
pub struct InlineEditResult {
    pub tree: RichText,
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
