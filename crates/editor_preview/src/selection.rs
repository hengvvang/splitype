//! Independent selection engine for the Preview pane.
//!
//! Standard-First: Works strictly on the read-only CommonMark AST snapshot.

use std::ops::Range;

/// Represents a character boundary point inside a specific preview block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreviewEndpoint {
    pub block_index: usize,
    pub offset: usize,
}

/// Continuous read-only text selection across preview blocks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreviewSelectionRange {
    pub start: PreviewEndpoint,
    pub end: PreviewEndpoint,
    pub is_reversed: bool,
}

impl PreviewSelectionRange {
    pub fn new(anchor: PreviewEndpoint, focus: PreviewEndpoint) -> Self {
        let is_reversed = focus.block_index < anchor.block_index
            || (focus.block_index == anchor.block_index && focus.offset < anchor.offset);
        let (start, end) = if is_reversed {
            (focus, anchor)
        } else {
            (anchor, focus)
        };
        Self {
            start,
            end,
            is_reversed,
        }
    }

    /// Whether this selection is empty (collapsed).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.start.block_index == self.end.block_index && self.start.offset == self.end.offset
    }

    /// Returns the character selection range for a specific block index, if it falls within the selection.
    pub fn range_for_block(&self, block_index: usize, block_len: usize) -> Option<Range<usize>> {
        if block_index < self.start.block_index || block_index > self.end.block_index {
            return None;
        }

        let start = if block_index == self.start.block_index {
            self.start.offset.min(block_len)
        } else {
            0
        };

        let end = if block_index == self.end.block_index {
            self.end.offset.min(block_len)
        } else {
            block_len
        };

        if start < end {
            Some(start..end)
        } else {
            None
        }
    }
}

