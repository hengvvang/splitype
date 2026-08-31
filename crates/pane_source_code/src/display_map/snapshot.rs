//! Frame-stable snapshot of the display transformation pipeline.

use crate::buffer::{BufferPoint, LineMap};
use crate::display_map::display_point::DisplayPoint;
use crate::display_map::fold_map::FoldMap;
use crate::display_map::tab_map::TabMap;
use crate::display_map::wrap_map::WrapMap;

/// An immutable, frame-stable snapshot of all display transformation maps.
#[derive(Clone, Debug)]
pub struct DisplaySnapshot<'a> {
    pub text: &'a str,
    pub line_map: &'a LineMap,
    pub tab_map: TabMap,
    pub fold_map: &'a FoldMap,
    pub wrap_map: WrapMap,
}

impl<'a> DisplaySnapshot<'a> {
    pub fn new(
        text: &'a str,
        line_map: &'a LineMap,
        tab_map: TabMap,
        fold_map: &'a FoldMap,
        wrap_map: WrapMap,
    ) -> Self {
        Self {
            text,
            line_map,
            tab_map,
            fold_map,
            wrap_map,
        }
    }

    /// Total number of visible display lines.
    pub fn visible_line_count(&self) -> u32 {
        self.fold_map.visible_line_count(self.line_map.line_count() as u32)
    }

    /// Converts a BufferPoint to a DisplayPoint.
    pub fn buffer_point_to_display_point(&self, point: BufferPoint) -> DisplayPoint {
        let display_row = self.fold_map.buffer_row_to_visible_row(point.row);
        let range = self.line_map.line_range(point.row as usize);
        let s = range.start.min(self.text.len());
        let e = range.end.min(self.text.len());
        let line_str = &self.text[s..e];

        let display_col = self.tab_map.char_column_to_display_column(line_str, point.column);
        DisplayPoint::new(display_row, display_col)
    }

    /// Converts a DisplayPoint to a BufferPoint.
    pub fn display_point_to_buffer_point(&self, point: DisplayPoint) -> BufferPoint {
        let total_rows = self.line_map.line_count() as u32;
        let buffer_row = self.fold_map.visible_row_to_buffer_row(point.row, total_rows);

        let range = self.line_map.line_range(buffer_row as usize);
        let s = range.start.min(self.text.len());
        let e = range.end.min(self.text.len());
        let line_str = &self.text[s..e];

        let buffer_col = self.tab_map.display_column_to_char_column(line_str, point.column);
        BufferPoint::new(buffer_row, buffer_col)
    }

    /// Converts a byte offset to a DisplayPoint.
    pub fn offset_to_display_point(&self, offset: usize) -> DisplayPoint {
        let buffer_point = self.line_map.offset_to_point(self.text, offset);
        self.buffer_point_to_display_point(buffer_point)
    }

    /// Converts a DisplayPoint to a byte offset.
    pub fn display_point_to_offset(&self, point: DisplayPoint) -> usize {
        let buffer_point = self.display_point_to_buffer_point(point);
        self.line_map.point_to_offset(self.text, buffer_point)
    }
}

