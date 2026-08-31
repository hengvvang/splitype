//! Tab character expansion to visual spaces.

/// Tab expansion settings and calculations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TabMap {
    pub tab_size: u32,
}

impl Default for TabMap {
    fn default() -> Self {
        Self { tab_size: 4 }
    }
}

impl TabMap {
    pub fn new(tab_size: u32) -> Self {
        Self {
            tab_size: tab_size.max(1),
        }
    }

    /// Expands tab characters in `line` into visual spaces.
    pub fn expand_tabs(&self, line: &str) -> String {
        let mut result = String::with_capacity(line.len());
        let mut col = 0;
        for ch in line.chars() {
            if ch == '\t' {
                let spaces = self.tab_size - (col % self.tab_size);
                for _ in 0..spaces {
                    result.push(' ');
                }
                col += spaces;
            } else {
                result.push(ch);
                col += 1;
            }
        }
        result
    }

    /// Converts a byte column in raw text to a visual column after tab expansion.
    pub fn char_column_to_display_column(&self, line: &str, target_byte_col: u32) -> u32 {
        let mut visual_col = 0;
        for (b_idx, ch) in line.char_indices() {
            if b_idx as u32 >= target_byte_col {
                break;
            }
            if ch == '\t' {
                let spaces = self.tab_size - (visual_col % self.tab_size);
                visual_col += spaces;
            } else {
                visual_col += 1;
            }
        }
        visual_col
    }

    /// Converts a visual display column back to a byte column in raw text.
    pub fn display_column_to_char_column(&self, line: &str, target_display_col: u32) -> u32 {
        let mut visual_col = 0;
        for (b_idx, ch) in line.char_indices() {
            if visual_col >= target_display_col {
                return b_idx as u32;
            }
            if ch == '\t' {
                let spaces = self.tab_size - (visual_col % self.tab_size);
                if visual_col + spaces > target_display_col {
                    return b_idx as u32;
                }
                visual_col += spaces;
            } else {
                visual_col += 1;
            }
        }
        line.len() as u32
    }
}

