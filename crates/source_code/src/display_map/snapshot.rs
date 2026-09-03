//! Frame-stable display transformation snapshot.
//!
//! The snapshot flattens the display pipeline — folds collapse hidden rows
//! and soft wraps expand lines into visual rows — into a `RowIndex` that
//! converts buffer offsets to display points and back in O(log n). The
//! editor rebuilds it whenever the text, folds, or wrap state change.

use std::sync::Arc;

use crate::display_map::display_point::DisplayPoint;
use crate::display_map::fold_map::FoldMap;
use crate::display_map::tab_map::TabMap;
use crate::display_map::wrap_map::WrapState;
use crate::text::Rope;

/// Immutable, frame-stable snapshot of all display transformation state.
#[derive(Clone, Debug)]
pub struct DisplaySnapshot<'a> {
    pub text: &'a Rope,
    pub tab_map: TabMap,
    pub fold_map: &'a FoldMap,
    pub wrap: &'a WrapState,
    pub rows: Arc<RowIndex>,
}

/// The flattened visual row index.
#[derive(Clone, Debug, Default)]
pub struct RowIndex {
    /// For each buffer row: the display row at which it starts. Rows hidden
    /// inside a fold reuse their fold header's start row, so positions
    /// inside folds clamp to the header.
    pub starts: Vec<u32>,
    /// Total number of display rows (at least 1).
    pub total: u32,
}

impl RowIndex {
    /// Builds the index from the fold map, wrap state, and line count.
    pub fn build(line_count: usize, folds: &FoldMap, wrap: &WrapState) -> Self {
        let mut starts = vec![0u32; line_count];
        let mut display_row = 0u32;
        let mut buffer_row = 0usize;
        while buffer_row < line_count {
            if let Some(fold) = folds.fold_at(buffer_row as u32) {
                // The fold header row itself is visible; everything after it
                // up to the fold end maps onto the header's start row.
                let header_start = display_row;
                starts[buffer_row] = header_start;
                display_row += wrap.line_rows(buffer_row);
                for hidden in buffer_row + 1..=(fold.end_row as usize).min(line_count - 1) {
                    starts[hidden] = header_start;
                }
                buffer_row = fold.end_row as usize + 1;
            } else {
                starts[buffer_row] = display_row;
                display_row += wrap.line_rows(buffer_row);
                buffer_row += 1;
            }
        }
        Self {
            starts,
            total: display_row.max(1),
        }
    }

    /// The visible buffer row starting at or before `display_row`. Shared
    /// starts (hidden rows inside a fold) resolve to the fold header, the
    /// lowest such row.
    pub fn buffer_row_at(&self, display_row: u32) -> u32 {
        if self.starts.is_empty() {
            return 0;
        }
        let idx = self
            .starts
            .partition_point(|start| *start <= display_row)
            .saturating_sub(1);
        // Hidden rows inside a fold share their header's start row; walk
        // back to the lowest row with the same start (the header itself).
        let start = self.starts[idx];
        let mut header = idx;
        while header > 0 && self.starts[header - 1] == start {
            header -= 1;
        }
        header as u32
    }
}

impl<'a> DisplaySnapshot<'a> {
    pub fn new(
        text: &'a Rope,
        tab_map: TabMap,
        fold_map: &'a FoldMap,
        wrap: &'a WrapState,
        rows: Arc<RowIndex>,
    ) -> Self {
        Self {
            text,
            tab_map,
            fold_map,
            wrap,
            rows,
        }
    }

    /// Builds the snapshot from a freshly computed row index.
    pub fn build(
        text: &'a Rope,
        tab_map: TabMap,
        fold_map: &'a FoldMap,
        wrap: &'a WrapState,
    ) -> Self {
        let rows = Arc::new(RowIndex::build(text.line_count(), fold_map, wrap));
        Self::new(text, tab_map, fold_map, wrap, rows)
    }

    /// Total number of visible display rows.
    #[inline]
    pub fn visible_line_count(&self) -> u32 {
        self.rows.total
    }

    /// The line text of a buffer row.
    #[inline]
    fn line_str(&self, buffer_row: u32) -> &'a str {
        self.text.line_str(buffer_row as usize)
    }

    /// Converts a byte offset to a display point. Offsets inside folded
    /// content clamp to the fold header row.
    pub fn offset_to_display_point(&self, offset: usize) -> DisplayPoint {
        let (row, column) = self.text.offset_to_point(offset);
        let buffer_row = row;
        let start_row = self.rows.starts[buffer_row];
        let line = self.line_str(buffer_row as u32);

        let wrap_index = self.wrap.wrap_index_of(buffer_row, column);
        let segment = self.wrap.row_range(buffer_row, wrap_index, 0..line.len());
        let column_in_segment = column.saturating_sub(segment.start);
        let display_column = self.tab_map.char_column_to_display_column(
            &line[segment.start..segment.end],
            column_in_segment as u32,
        );

        DisplayPoint::new(start_row + wrap_index as u32, display_column)
    }

    /// Converts a display point back to a byte offset, clamping to the
    /// visual row's extent.
    pub fn display_point_to_offset(&self, point: DisplayPoint) -> usize {
        let buffer_row = self.rows.buffer_row_at(point.row);
        let start_row = self.rows.starts[buffer_row as usize];
        let wrap_index = point.row.saturating_sub(start_row);

        let line_start = self.text.line_start(buffer_row as usize);
        let line = self.line_str(buffer_row);
        let segment = self
            .wrap
            .row_range(buffer_row as usize, wrap_index as usize, 0..line.len());

        let column_in_segment = self
            .tab_map
            .display_column_to_char_column(&line[segment.start..segment.end], point.column)
            as usize;
        line_start + segment.start + column_in_segment.min(segment.len())
    }

    /// The display row range a buffer row occupies.
    #[inline]
    pub fn display_row_of_buffer_row(&self, buffer_row: u32) -> u32 {
        self.rows.starts[buffer_row as usize]
    }
}
