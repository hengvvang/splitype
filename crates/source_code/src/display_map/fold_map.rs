//! Code folding regions and their row mapping.
//!
//! A fold hides every buffer row strictly between (and including) its
//! `start_row` and `end_row` except the header row itself. The display
//! snapshot flattens folds into the row index; this map only stores the
//! folded regions.

use std::collections::BTreeMap;

use crate::Rope;

/// A fold region from `start_row` (visible header) to `end_row` (last
/// hidden row), inclusive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FoldRange {
    pub start_row: u32,
    pub end_row: u32,
}

impl FoldRange {
    pub fn new(start_row: u32, end_row: u32) -> Self {
        Self {
            start_row,
            end_row: end_row.max(start_row),
        }
    }
}

/// Tracks active fold regions keyed by their header row.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FoldMap {
    folds: BTreeMap<u32, FoldRange>,
}

impl FoldMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// The fold whose header is `row`, if any.
    #[inline]
    pub fn fold_at(&self, row: u32) -> Option<FoldRange> {
        self.folds.get(&row).copied()
    }

    /// Whether the header row of some fold is `row`.
    #[inline]
    pub fn is_folded(&self, row: u32) -> bool {
        self.folds.contains_key(&row)
    }

    /// Whether `row` is hidden inside an active fold.
    #[inline]
    pub fn is_row_hidden(&self, row: u32) -> bool {
        self.folds
            .values()
            .any(|fold| row > fold.start_row && row <= fold.end_row)
    }

    /// Folds a region.
    pub fn fold(&mut self, range: FoldRange) {
        if range.end_row > range.start_row {
            self.folds.insert(range.start_row, range);
        }
    }

    /// Unfolds the region headed by `start_row`.
    pub fn unfold(&mut self, start_row: u32) {
        self.folds.remove(&start_row);
    }

    /// Unfolds everything.
    pub fn unfold_all(&mut self) {
        self.folds.clear();
    }

    /// The header row of the fold hiding `row`, if any.
    pub fn header_of_hidden(&self, row: u32) -> Option<u32> {
        self.folds
            .iter()
            .find(|(_, fold)| row > fold.start_row && row <= fold.end_row)
            .map(|(start, _)| *start)
    }

    /// Drops folds whose header rows no longer exist. Returns whether any
    /// fold was removed.
    pub fn prune_to_line_count(&mut self, line_count: u32) -> bool {
        let before = self.folds.len();
        self.folds.retain(|start, _| *start < line_count);
        self.folds.len() != before
    }

    /// Scans Markdown text for all foldable regions: fenced code blocks and
    /// heading sections.
    pub fn discover_markdown_folds(text: &Rope) -> Vec<FoldRange> {
        let mut foldable = Vec::new();
        let total_lines = text.line_count() as u32;

        let mut in_code_fence = false;
        let mut fence_start = 0u32;
        let mut heading_stack: Vec<(u8, u32)> = Vec::new();

        for row in 0..total_lines {
            let line = text.line_str(row as usize).trim_start();

            // Code fence detection
            if line.starts_with("```") || line.starts_with("~~~") {
                if !in_code_fence {
                    in_code_fence = true;
                    fence_start = row;
                } else {
                    in_code_fence = false;
                    if row > fence_start + 1 {
                        foldable.push(FoldRange::new(fence_start, row - 1));
                    }
                }
                continue;
            }

            if in_code_fence {
                continue;
            }

            // Heading detection
            if let Some(level) = heading_level(line) {
                while let Some((prev_level, prev_row)) = heading_stack.pop() {
                    if prev_level < level {
                        heading_stack.push((prev_level, prev_row));
                        break;
                    } else if row > prev_row + 1 {
                        foldable.push(FoldRange::new(prev_row, row - 1));
                    }
                }
                heading_stack.push((level, row));
            }
        }

        // Every remaining heading folds its section to the end of the
        // document (outer sections include nested subsections).
        while let Some((_, prev_row)) = heading_stack.pop() {
            if total_lines > prev_row + 1 {
                foldable.push(FoldRange::new(prev_row, total_lines - 1));
            }
        }

        foldable
    }
}

fn heading_level(line: &str) -> Option<u8> {
    let level = line.chars().take_while(|&c| c == '#').count() as u8;
    if (1..=6).contains(&level) && line.as_bytes().get(level as usize) == Some(&b' ') {
        Some(level)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_heading_folds() {
        let text = "# A\n\npara\n\n## B\npara2\n";
        let rope = Rope::new(text);
        let folds = FoldMap::discover_markdown_folds(&rope);
        // "# A" spans the whole document (rows 0-5, including the
        // "## B" subsection at row 4); "## B" folds its own section.
        assert!(folds.iter().any(|f| f.start_row == 0 && f.end_row == 5));
        assert!(folds.iter().any(|f| f.start_row == 4 && f.end_row == 5));
    }

    #[test]
    fn fold_and_hide() {
        let mut map = FoldMap::new();
        map.fold(FoldRange::new(0, 2));
        assert!(map.is_folded(0));
        assert!(map.is_row_hidden(1));
        assert!(map.is_row_hidden(2));
        assert!(!map.is_row_hidden(3));
        map.unfold(0);
        assert!(!map.is_folded(0));
    }
}
