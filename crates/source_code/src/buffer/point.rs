//! 2D point coordinate in the underlying text buffer.

use std::cmp::Ordering;

/// A (row, column) coordinate in the text buffer, 0-indexed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct BufferPoint {
    pub row: u32,
    pub column: u32,
}

impl BufferPoint {
    pub const ZERO: Self = Self { row: 0, column: 0 };

    #[inline]
    pub const fn new(row: u32, column: u32) -> Self {
        Self { row, column }
    }
}

impl PartialOrd for BufferPoint {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BufferPoint {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.row
            .cmp(&other.row)
            .then_with(|| self.column.cmp(&other.column))
    }
}

impl std::fmt::Display for BufferPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.row, self.column)
    }
}
