//! 2D point coordinate in the rendered display space.

use std::cmp::Ordering;

/// A (row, column) coordinate in display/screen space, 0-indexed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DisplayPoint {
    pub row: u32,
    pub column: u32,
}

impl DisplayPoint {
    pub const ZERO: Self = Self { row: 0, column: 0 };

    #[inline]
    pub const fn new(row: u32, column: u32) -> Self {
        Self { row, column }
    }
}

impl PartialOrd for DisplayPoint {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DisplayPoint {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.row
            .cmp(&other.row)
            .then_with(|| self.column.cmp(&other.column))
    }
}

impl std::fmt::Display for DisplayPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.row, self.column)
    }
}

