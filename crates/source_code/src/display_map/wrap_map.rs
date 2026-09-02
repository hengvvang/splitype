//! Soft line wrapping: visual row segmentation of each buffer line.
//!
//! Wrapping is pixel-based like Zed's: a line breaks where its shaped width
//! would exceed the available viewport width. The editor recomputes wrap
//! points whenever the text, the wrap width, or the font metrics change.

/// Wrap state for the whole buffer.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WrapState {
    /// Viewport width in pixels the points were computed for.
    pub width_px: f32,
    /// Per buffer line: byte offsets where visual rows break. A line with
    /// `n` points renders as `n + 1` visual rows; an empty vec means the
    /// line is a single unwrapped row.
    pub points: Vec<Vec<usize>>,
}

impl WrapState {
    pub fn new(width_px: f32, points: Vec<Vec<usize>>) -> Self {
        Self { width_px, points }
    }

    /// Visual row count of one buffer line.
    #[inline]
    pub fn line_rows(&self, buffer_row: usize) -> u32 {
        self.points
            .get(buffer_row)
            .map(|points| points.len() as u32 + 1)
            .unwrap_or(1)
    }

    /// Byte range covered by one visual row of a buffer line.
    pub fn row_range(
        &self,
        buffer_row: usize,
        wrap_index: usize,
        line_range: std::ops::Range<usize>,
    ) -> std::ops::Range<usize> {
        let start = if wrap_index == 0 {
            line_range.start
        } else {
            self.points
                .get(buffer_row)
                .and_then(|points| points.get(wrap_index - 1))
                .copied()
                .unwrap_or(line_range.start)
        };
        let end = self
            .points
            .get(buffer_row)
            .and_then(|points| points.get(wrap_index))
            .copied()
            .unwrap_or(line_range.end);
        start..end
    }

    /// The wrap index (visual row) containing `offset` within a buffer line.
    pub fn wrap_index_of(&self, buffer_row: usize, offset_in_line: usize) -> usize {
        self.points
            .get(buffer_row)
            .map(|points| points.partition_point(|point| *point <= offset_in_line))
            .unwrap_or(0)
    }
}
