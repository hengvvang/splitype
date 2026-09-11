//! Gutter line numbers and fold indicators layout.
//!
//! Aligns with Zed's editor buffer layout:
//! [padding_left] -> [line_numbers (right aligned)] -> [fold_area (centered)] -> [margin] -> [code text]

/// Gutter dimensions and layout calculations.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GutterLayout {
    pub total_lines: usize,
    pub font_size: f32,
    pub padding_left: f32,
    pub min_digits: usize,
    pub fold_area_width: f32,
    pub margin: f32,
}

impl Default for GutterLayout {
    fn default() -> Self {
        Self {
            total_lines: 1,
            font_size: 13.0,
            padding_left: 10.0,
            min_digits: 3,
            fold_area_width: 18.0,
            margin: 8.0,
        }
    }
}

impl GutterLayout {
    pub fn new(total_lines: usize, font_size: f32) -> Self {
        Self {
            total_lines: total_lines.max(1),
            font_size: font_size.max(10.0),
            padding_left: 10.0,
            min_digits: 3,
            fold_area_width: 18.0,
            margin: 8.0,
        }
    }

    /// Approximate monospaced character width.
    #[inline]
    pub fn char_width(&self) -> f32 {
        self.font_size * 0.6
    }

    /// Number of digits in total line count, guaranteed to be at least `min_digits`
    /// to avoid visual jumping when line count changes between 1 and 99.
    #[inline]
    pub fn digit_count(&self) -> usize {
        self.total_lines.to_string().len().max(self.min_digits)
    }

    /// Width of the line numbers column.
    #[inline]
    pub fn line_number_width(&self) -> f32 {
        self.digit_count() as f32 * self.char_width()
    }

    /// Left coordinate of the fold area (where line numbers end).
    #[inline]
    pub fn fold_area_left(&self) -> f32 {
        self.padding_left + self.line_number_width()
    }

    /// Right coordinate of the fold area (equivalent to gutter_width).
    #[inline]
    pub fn fold_area_right(&self) -> f32 {
        self.gutter_width()
    }

    /// Computed total width of the gutter column in pixels (excluding text margin).
    #[inline]
    pub fn gutter_width(&self) -> f32 {
        (self.padding_left + self.line_number_width() + self.fold_area_width).max(40.0)
    }

    /// Full offset from editor bounds left to where code text starts (gutter + margin).
    #[inline]
    pub fn text_offset(&self) -> f32 {
        self.gutter_width() + self.margin
    }

    /// X coordinate for drawing a line number right-aligned against the fold area.
    #[inline]
    pub fn line_number_x(&self, shaped_width: f32) -> f32 {
        self.padding_left + self.line_number_width() - shaped_width
    }

    /// X coordinate for horizontally centering a fold icon within the fold area.
    #[inline]
    pub fn fold_icon_x(&self, icon_size: f32) -> f32 {
        self.fold_area_left() + (self.fold_area_width - icon_size) / 2.0
    }

    /// Computed total width of the gutter in pixels (alias for `gutter_width`).
    #[inline]
    pub fn width(&self) -> f32 {
        self.gutter_width()
    }

    /// Formats line number (1-based).
    #[inline]
    pub fn format_line_number(&self, buffer_row: u32) -> String {
        (buffer_row + 1).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gutter_layout_geometry() {
        let layout = GutterLayout::new(10, 13.0);
        assert_eq!(layout.digit_count(), 3); // min_digits is 3

        let char_width = 13.0 * 0.6;
        assert_eq!(layout.line_number_width(), 3.0 * char_width);
        assert_eq!(layout.fold_area_left(), 10.0 + layout.line_number_width());
        assert_eq!(layout.gutter_width(), 10.0 + layout.line_number_width() + 18.0);
        assert_eq!(layout.text_offset(), layout.gutter_width() + 8.0);

        // Right-alignment: line number right edge must equal fold_area_left
        let shaped_width = 14.5;
        let num_x = layout.line_number_x(shaped_width);
        assert!((num_x + shaped_width - layout.fold_area_left()).abs() < 1e-5);

        // Centering: fold icon must be centered in fold area
        let icon_size = 9.0;
        let icon_x = layout.fold_icon_x(icon_size);
        let left_margin = icon_x - layout.fold_area_left();
        let right_margin = layout.fold_area_right() - (icon_x + icon_size);
        assert!((left_margin - right_margin).abs() < 1e-5);
    }

    #[test]
    fn gutter_layout_large_file() {
        let layout = GutterLayout::new(12345, 14.0);
        assert_eq!(layout.digit_count(), 5);
        assert_eq!(layout.format_line_number(41), "42");
    }
}
