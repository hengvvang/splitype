//! Single selection range and cursor state.

use std::ops::Range;

/// A single cursor or selection range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    pub id: usize,
    /// The fixed anchor point of the selection.
    pub anchor: usize,
    /// The active head / cursor position of the selection.
    pub head: usize,
    /// Memorized target visual column for vertical navigation.
    pub goal_column: Option<u32>,
}

impl Selection {
    /// Creates a collapsed cursor at `offset`.
    pub fn point(id: usize, offset: usize) -> Self {
        Self {
            id,
            anchor: offset,
            head: offset,
            goal_column: None,
        }
    }

    /// Creates a selection from `anchor` to `head`.
    pub fn range(id: usize, anchor: usize, head: usize) -> Self {
        Self {
            id,
            anchor,
            head,
            goal_column: None,
        }
    }

    /// Is this selection collapsed to a single cursor point?
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    /// The lower byte offset of the selection range.
    #[inline]
    pub fn start(&self) -> usize {
        self.anchor.min(self.head)
    }

    /// The upper byte offset of the selection range.
    #[inline]
    pub fn end(&self) -> usize {
        self.anchor.max(self.head)
    }

    /// Returns the selection as an ordered byte range.
    #[inline]
    pub fn range_bounds(&self) -> Range<usize> {
        self.start()..self.end()
    }

    /// Is the cursor moving backward (head before anchor)?
    #[inline]
    pub fn is_reversed(&self) -> bool {
        self.head < self.anchor
    }
}
