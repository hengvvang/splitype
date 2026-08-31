//! Gutter line numbers and fold indicators layout.

/// Gutter dimensions and layout calculations.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GutterLayout {
    pub total_lines: usize,
    pub font_size: f32,
    pub padding_left: f32,
    pub padding_right: f32,
    pub fold_button_width: f32,
}

impl Default for GutterLayout {
    fn default() -> Self {
        Self {
            total_lines: 1,
            font_size: 13.0,
            padding_left: 12.0,
            padding_right: 12.0,
            fold_button_width: 14.0,
        }
    }
}

impl GutterLayout {
    pub fn new(total_lines: usize, font_size: f32) -> Self {
        Self {
            total_lines: total_lines.max(1),
            font_size: font_size.max(10.0),
            padding_left: 12.0,
            padding_right: 12.0,
            fold_button_width: 14.0,
        }
    }

    /// Number of digits in total line count.
    #[inline]
    pub fn digit_count(&self) -> usize {
        self.total_lines.to_string().len()
    }

    /// Computed total width of the gutter in pixels.
    pub fn width(&self) -> f32 {
        let char_width = self.font_size * 0.6;
        let digits_width = self.digit_count() as f32 * char_width;
        (self.padding_left + digits_width + self.fold_button_width + self.padding_right).max(36.0)
    }

    /// Formats line number (1-based) with right-alignment.
    pub fn format_line_number(&self, buffer_row: u32) -> String {
        let line_num = buffer_row + 1;
        format!("{:>width$}", line_num, width = self.digit_count())
    }
}

