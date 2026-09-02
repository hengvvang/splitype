//! Text buffer projection: byte-offset line indexing over the pane's local
//! document copy.
//!
//! The pane projects the shared document buffer (the process-level single
//! source of truth) into a local text copy for editing. `LineMap` indexes
//! line starts so that cursor movement, hit-testing, and search stay
//! O(log n) without a rope: Markdown documents are small enough that a flat
//! offset vector is both simple and fast.

/// Fast line-to-byte-offset index.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LineMap {
    /// Byte offset of each line start, sorted ascending. Always starts with
    /// `0`; text ending in `\n` does not create a trailing line.
    line_starts: Vec<usize>,
    /// Total text byte length at index time.
    text_len: usize,
    /// Whether the indexed text ends with a newline (which therefore does
    /// not count toward its final line's length).
    ends_with_newline: bool,
}

impl LineMap {
    /// Builds the index from text.
    pub fn new(text: &str) -> Self {
        let mut line_starts = vec![0usize];
        for (idx, ch) in text.char_indices() {
            if ch == '\n' && idx + 1 < text.len() {
                line_starts.push(idx + 1);
            }
        }
        Self {
            line_starts,
            text_len: text.len(),
            ends_with_newline: text.ends_with('\n'),
        }
    }

    /// Total number of lines (at least 1).
    #[inline]
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Byte offset at which line `row` starts; out-of-range rows clamp to
    /// the last line.
    #[inline]
    pub fn line_start(&self, row: usize) -> usize {
        let idx = row.min(self.line_starts.len() - 1);
        self.line_starts[idx]
    }

    /// Byte length of line `row` excluding the trailing newline.
    #[inline]
    pub fn line_len(&self, row: usize) -> usize {
        let start = self.line_start(row);
        let next = if row + 1 < self.line_starts.len() {
            // The next line starts right after this line's newline.
            self.line_starts[row + 1] - 1
        } else if self.ends_with_newline {
            self.text_len - 1
        } else {
            self.text_len
        };
        next.saturating_sub(start)
    }

    /// The (row, byte column) point of a byte offset. An offset pointing at
    /// a newline resolves to the end of the preceding line.
    pub fn offset_to_point(&self, offset: usize) -> BufferPoint {
        let offset = offset.min(self.text_len);
        let row = match self
            .line_starts
            .binary_search_by(|start| start.cmp(&offset))
        {
            Ok(idx) => idx,
            Err(idx) => idx.saturating_sub(1),
        };
        let start = self.line_starts[row];
        let column = offset.saturating_sub(start).min(self.line_len(row));
        BufferPoint::new(row as u32, column as u32)
    }

    /// The byte offset of a (row, byte column) point; clamps to line ends.
    pub fn point_to_offset(&self, point: BufferPoint) -> usize {
        let row = (point.row as usize).min(self.line_starts.len() - 1);
        self.line_start(row) + (point.column as usize).min(self.line_len(row))
    }
}

/// A (row, column) coordinate in the text buffer, 0-indexed, column in
/// bytes within the line.
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

impl std::fmt::Display for BufferPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.row, self.column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexes_lines_and_offsets() {
        let map = LineMap::new("ab\ncd\nef");
        assert_eq!(map.line_count(), 3);
        assert_eq!(map.line_start(0), 0);
        assert_eq!(map.line_start(1), 3);
        assert_eq!(map.line_start(2), 6);
        assert_eq!(map.line_len(0), 2);
        assert_eq!(map.line_len(2), 2);
        assert_eq!(map.offset_to_point(4), BufferPoint::new(1, 1));
        assert_eq!(map.point_to_offset(BufferPoint::new(1, 1)), 4);
    }

    #[test]
    fn newline_offsets_resolve_to_line_end() {
        let map = LineMap::new("ab\ncd");
        assert_eq!(map.offset_to_point(2), BufferPoint::new(0, 2));
        assert_eq!(map.offset_to_point(3), BufferPoint::new(1, 0));
    }

    #[test]
    fn empty_and_trailing_newline() {
        assert_eq!(LineMap::new("").line_count(), 1);
        let map = LineMap::new("ab\n");
        assert_eq!(map.line_count(), 1);
        assert_eq!(map.line_len(0), 2);
        assert_eq!(map.offset_to_point(3), BufferPoint::new(0, 2));
    }
}
