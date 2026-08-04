//! Runtime cell position inside a native table.

/// Runtime-only location of a cell inside a native table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableCellPosition {
    /// Zero-based visual row. Header is row `0`; first body row is `1`.
    pub row: usize,
    pub column: usize,
}

impl TableCellPosition {
    pub fn is_header(self) -> bool {
        self.row == 0
    }

    pub fn body_row_index(self) -> Option<usize> {
        self.row.checked_sub(1)
    }
}
