//! Code folding regions and coordinate mapping.

use std::collections::BTreeMap;

use crate::buffer::LineMap;

/// A fold region spanning from start_row to end_row (inclusive of folded inner rows).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FoldRange {
    /// The row containing the fold header (e.g. `# Heading` or ```` ```rust ````).
    pub start_row: u32,
    /// The end row hidden inside the fold.
    pub end_row: u32,
}

/// Tracks code folding regions and converts between buffer rows and visible display rows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FoldMap {
    /// Active folded ranges keyed by start_row.
    folds: BTreeMap<u32, FoldRange>,
}

impl FoldMap {
    pub fn new() -> Self {
        Self {
            folds: BTreeMap::new(),
        }
    }

    /// Is the given buffer row folded (i.e. start of a fold)?
    #[inline]
    pub fn is_folded(&self, start_row: u32) -> bool {
        self.folds.contains_key(&start_row)
    }

    /// Is the given buffer row hidden inside an active fold?
    #[inline]
    pub fn is_row_hidden(&self, row: u32) -> bool {
        for fold in self.folds.values() {
            if row > fold.start_row && row <= fold.end_row {
                return true;
            }
        }
        false
    }

    /// Toggles folding for a range.
    pub fn toggle_fold(&mut self, range: FoldRange) {
        if self.folds.contains_key(&range.start_row) {
            self.folds.remove(&range.start_row);
        } else {
            self.folds.insert(range.start_row, range);
        }
    }

    /// Folds a range.
    pub fn fold(&mut self, range: FoldRange) {
        self.folds.insert(range.start_row, range);
    }

    /// Unfolds a range.
    pub fn unfold(&mut self, start_row: u32) {
        self.folds.remove(&start_row);
    }

    /// Unfolds all.
    pub fn unfold_all(&mut self) {
        self.folds.clear();
    }

    /// Automatically scans markdown text to discover all potential foldable ranges.
    pub fn discover_markdown_folds(text: &str, line_map: &LineMap) -> Vec<FoldRange> {
        let mut foldable = Vec::new();
        let total_lines = line_map.line_count() as u32;

        let mut in_code_fence = false;
        let mut fence_start = 0;

        let mut heading_stack: Vec<(u8, u32)> = Vec::new();

        for row in 0..total_lines {
            let r = line_map.line_range(row as usize);
            let s = r.start.min(text.len());
            let e = r.end.min(text.len());
            let line = text[s..e].trim_start();

            // Code fence detection
            if line.starts_with("```") || line.starts_with("~~~") {
                if !in_code_fence {
                    in_code_fence = true;
                    fence_start = row;
                } else {
                    in_code_fence = false;
                    if row > fence_start {
                        foldable.push(FoldRange {
                            start_row: fence_start,
                            end_row: row,
                        });
                    }
                }
                continue;
            }

            if in_code_fence {
                continue;
            }

            // Heading detection
            if line.starts_with('#') {
                let level = line.chars().take_while(|&c| c == '#').count() as u8;
                if level >= 1 && level <= 6 && line[level as usize..].starts_with(' ') {
                    while let Some((prev_lvl, prev_row)) = heading_stack.pop() {
                        if prev_lvl < level {
                            heading_stack.push((prev_lvl, prev_row));
                            break;
                        } else if row > prev_row + 1 {
                            foldable.push(FoldRange {
                                start_row: prev_row,
                                end_row: row - 1,
                            });
                        }
                    }
                    heading_stack.push((level, row));
                }
            }
        }

        if let Some((_, prev_row)) = heading_stack.pop() {
            if total_lines > prev_row + 1 {
                foldable.push(FoldRange {
                    start_row: prev_row,
                    end_row: total_lines - 1,
                });
            }
        }

        foldable
    }

    /// Converts a buffer row to visible row (skipping folded hidden rows).
    pub fn buffer_row_to_visible_row(&self, buffer_row: u32) -> u32 {
        let mut visible = 0;
        let mut r = 0;
        while r <= buffer_row {
            if let Some(fold) = self.folds.get(&r) {
                if r == buffer_row {
                    return visible;
                }
                r = fold.end_row + 1;
                visible += 1;
            } else {
                if r == buffer_row {
                    return visible;
                }
                r += 1;
                visible += 1;
            }
        }
        visible
    }

    /// Converts a visible row back to buffer row.
    pub fn visible_row_to_buffer_row(&self, visible_row: u32, total_buffer_rows: u32) -> u32 {
        let mut current_visible = 0;
        let mut r = 0;
        while r < total_buffer_rows {
            if current_visible == visible_row {
                return r;
            }
            if let Some(fold) = self.folds.get(&r) {
                r = fold.end_row + 1;
            } else {
                r += 1;
            }
            current_visible += 1;
        }
        total_buffer_rows.saturating_sub(1)
    }

    /// Total count of visible lines.
    pub fn visible_line_count(&self, total_buffer_rows: u32) -> u32 {
        let mut count = 0;
        let mut r = 0;
        while r < total_buffer_rows {
            count += 1;
            if let Some(fold) = self.folds.get(&r) {
                r = fold.end_row + 1;
            } else {
                r += 1;
            }
        }
        count.max(1)
    }
}
