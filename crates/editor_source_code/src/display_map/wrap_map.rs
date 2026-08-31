//! Soft line-wrapping calculations.

/// Settings and mapping for line wrapping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WrapMap {
    pub enabled: bool,
    pub wrap_columns: Option<u32>,
}

impl Default for WrapMap {
    fn default() -> Self {
        Self {
            enabled: false,
            wrap_columns: None,
        }
    }
}

impl WrapMap {
    pub fn new(enabled: bool, wrap_columns: Option<u32>) -> Self {
        Self {
            enabled,
            wrap_columns,
        }
    }

    /// Computes how many display rows a line takes up given max_columns.
    pub fn wrap_line_rows(&self, line_len_chars: usize, max_columns: u32) -> u32 {
        if !self.enabled || max_columns == 0 {
            return 1;
        }
        let cols = self.wrap_columns.unwrap_or(max_columns).max(10) as usize;
        if line_len_chars == 0 {
            1
        } else {
            ((line_len_chars + cols - 1) / cols).max(1) as u32
        }
    }
}
