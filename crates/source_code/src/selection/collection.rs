//! Multi-cursor selection collection and operations.

use std::ops::Range;

use crate::selection::selection::Selection;

/// Collection of multiple active cursors and selections.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionsCollection {
    selections: Vec<Selection>,
    next_id: usize,
    primary_index: usize,
}

impl Default for SelectionsCollection {
    fn default() -> Self {
        Self {
            selections: vec![Selection::point(0, 0)],
            next_id: 1,
            primary_index: 0,
        }
    }
}

impl SelectionsCollection {
    pub fn new() -> Self {
        Self::default()
    }

    /// Total count of active cursors.
    #[inline]
    pub fn count(&self) -> usize {
        self.selections.len()
    }

    /// Access all selections as a slice.
    #[inline]
    pub fn all(&self) -> &[Selection] {
        &self.selections
    }

    /// Access all selections mutably.
    #[inline]
    pub fn all_mut(&mut self) -> &mut [Selection] {
        &mut self.selections
    }

    /// The primary selection / cursor.
    #[inline]
    pub fn primary(&self) -> &Selection {
        &self.selections[self
            .primary_index
            .min(self.selections.len().saturating_sub(1))]
    }

    /// The primary selection / cursor mutably.
    #[inline]
    pub fn primary_mut(&mut self) -> &mut Selection {
        let idx = self
            .primary_index
            .min(self.selections.len().saturating_sub(1));
        &mut self.selections[idx]
    }

    /// Replaces all selections with a single point cursor.
    pub fn set_single_point(&mut self, offset: usize) {
        let id = self.alloc_id();
        self.selections = vec![Selection::point(id, offset)];
        self.primary_index = 0;
    }

    /// Replaces all selections with a single range selection.
    pub fn set_single_range(&mut self, anchor: usize, head: usize) {
        let id = self.alloc_id();
        self.selections = vec![Selection::range(id, anchor, head)];
        self.primary_index = 0;
    }

    /// Adds another selection cursor.
    pub fn add_selection(&mut self, anchor: usize, head: usize) {
        let id = self.alloc_id();
        self.selections.push(Selection::range(id, anchor, head));
        self.normalize();
    }

    /// Normalizes selections: sorts by start offset and merges overlapping/touching ranges.
    pub fn normalize(&mut self) {
        if self.selections.len() <= 1 {
            return;
        }

        self.selections.sort_by_key(|s| s.start());

        let mut merged: Vec<Selection> = Vec::with_capacity(self.selections.len());
        for current in &self.selections {
            if let Some(prev) = merged.last_mut() {
                if current.start() <= prev.end() {
                    // Overlapping or touching: merge
                    let new_start = prev.start().min(current.start());
                    let new_end = prev.end().max(current.end());
                    let head = if current.head >= prev.head {
                        new_end
                    } else {
                        new_start
                    };
                    let anchor = if head == new_end { new_start } else { new_end };
                    *prev = Selection::range(prev.id, anchor, head);
                    continue;
                }
            }
            merged.push(*current);
        }

        self.selections = merged;
        if self.primary_index >= self.selections.len() {
            self.primary_index = self.selections.len().saturating_sub(1);
        }
    }

    /// Helper to clamp all selection offsets to `max_len`.
    pub fn clamp_to_len(&mut self, max_len: usize) {
        for s in &mut self.selections {
            s.anchor = s.anchor.min(max_len);
            s.head = s.head.min(max_len);
        }
        self.normalize();
    }

    /// Check if any selection is non-empty.
    pub fn has_any_selection(&self) -> bool {
        self.selections.iter().any(|s| !s.is_empty())
    }

    /// Returns the primary selection as `Option<Range<usize>>`.
    pub fn primary_selection_range(&self) -> Option<Range<usize>> {
        let p = self.primary();
        if p.is_empty() {
            None
        } else {
            Some(p.range_bounds())
        }
    }

    fn alloc_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}
