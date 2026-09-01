//! Cross-block selection representation and normalization.

use crate::pane::state::CrossBlockSelectionEndpoint;

/// Cross-block selection with endpoints ordered by document position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NormalizedCrossBlockSelection {
    pub start: CrossBlockSelectionEndpoint,
    pub end: CrossBlockSelectionEndpoint,
    pub start_index: usize,
    pub end_index: usize,
    pub reversed: bool,
}

impl NormalizedCrossBlockSelection {
    /// Whether this selection is contained within a single block.
    #[inline]
    pub const fn is_single_block(&self) -> bool {
        self.start_index == self.end_index
    }

    /// Inclusive range of block indices spanned by this selection.
    #[inline]
    pub const fn block_index_range(&self) -> std::ops::RangeInclusive<usize> {
        self.start_index..=self.end_index
    }

    /// Whether the specified block index falls within this selection.
    #[inline]
    pub fn contains_block(&self, index: usize) -> bool {
        self.block_index_range().contains(&index)
    }
}
