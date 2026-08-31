//! Fast line-to-byte-offset indexing and conversions.

use std::ops::Range;

use crate::buffer::point::BufferPoint;

/// Fast line index mapping byte ranges and row/col points.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LineMap {
    /// Byte ranges for each 0-indexed line (excluding newline character).
    line_ranges: Vec<Range<usize>>,
    /// Total text byte length.
    text_len: usize,
}

impl LineMap {
    /// Builds a LineMap from text.
    pub fn new(text: &str) -> Self {
        let mut lines = Vec::new();
        let mut start = 0;
        for part in text.split('\n') {
            lines.push(start..start + part.len());
            start += part.len() + 1;
        }
        if lines.is_empty() {
            lines.push(0..0);
        }
        Self {
            line_ranges: lines,
            text_len: text.len(),
        }
    }

    /// Total number of lines.
    #[inline]
    pub fn line_count(&self) -> usize {
        if self.line_ranges.is_empty() { 1 } else { self.line_ranges.len() }
    }

    /// Returns byte range of a given 0-indexed line.
    #[inline]
    pub fn line_range(&self, line_index: usize) -> Range<usize> {
        if line_index < self.line_ranges.len() {
            self.line_ranges[line_index].clone()
        } else if let Some(last) = self.line_ranges.last() {
            last.end..last.end
        } else {
            0..0
        }
    }

    /// Returns the start byte offset of a given 0-indexed line.
    #[inline]
    pub fn line_start_offset(&self, line_index: usize) -> usize {
        self.line_range(line_index).start
    }

    /// Returns the end byte offset (before '\n') of a given 0-indexed line.
    #[inline]
    pub fn line_end_offset(&self, line_index: usize) -> usize {
        self.line_range(line_index).end
    }

    /// Returns the length in bytes of a given line.
    #[inline]
    pub fn line_len(&self, line_index: usize) -> usize {
        let r = self.line_range(line_index);
        r.end.saturating_sub(r.start)
    }

    /// Converts a byte offset to a BufferPoint (row, col).
    pub fn offset_to_point(&self, text: &str, offset: usize) -> BufferPoint {
        let offset = offset.min(text.len());
        if self.line_ranges.is_empty() {
            return BufferPoint::ZERO;
        }

        // Binary search to find the line
        let line_idx = match self.line_ranges.binary_search_by(|r| {
            if offset < r.start {
                std::cmp::Ordering::Greater
            } else if offset > r.end {
                // If it's on the newline character between r.end and next.start
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        }) {
            Ok(idx) => idx,
            Err(idx) => {
                if idx >= self.line_ranges.len() {
                    self.line_ranges.len() - 1
                } else if idx > 0 && offset == self.line_ranges[idx - 1].end + 1 {
                    idx
                } else {
                    idx.saturating_sub(1)
                }
            }
        };

        let start = self.line_ranges[line_idx].start;
        let line_len = self.line_len(line_idx);
        let col = offset.saturating_sub(start).min(line_len) as u32;

        BufferPoint::new(line_idx as u32, col)
    }

    /// Converts a BufferPoint (row, col) to a byte offset.
    pub fn point_to_offset(&self, _text: &str, point: BufferPoint) -> usize {
        let line_idx = (point.row as usize).min(self.line_count().saturating_sub(1));
        let range = self.line_range(line_idx);
        let line_len = range.end.saturating_sub(range.start);
        range.start + (point.column as usize).min(line_len)
    }
}
