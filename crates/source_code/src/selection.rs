//! Multi-cursor selection model.
//!
//! Selections live in byte offsets of the pane's local text copy. Local
//! edits adjust every selection through [`Selections::apply_edit`]; edits
//! arriving from outside (document sync) rebuild offsets from scratch, so
//! no anchor indirection is needed. Undo history itself lives in the shared
//! document buffer, not here.

/// A single cursor or selection range, identified for stable multi-cursor
/// updates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    pub id: usize,
    /// The fixed anchor point of the selection.
    pub anchor: usize,
    /// The active head / caret position.
    pub head: usize,
    /// Memorized target display column for vertical navigation.
    pub goal_column: Option<u32>,
}

impl Selection {
    pub fn point(id: usize, offset: usize) -> Self {
        Self {
            id,
            anchor: offset,
            head: offset,
            goal_column: None,
        }
    }

    pub fn range(id: usize, anchor: usize, head: usize) -> Self {
        Self {
            id,
            anchor,
            head,
            goal_column: None,
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    #[inline]
    pub fn start(&self) -> usize {
        self.anchor.min(self.head)
    }

    #[inline]
    pub fn end(&self) -> usize {
        self.anchor.max(self.head)
    }

    #[inline]
    pub fn is_reversed(&self) -> bool {
        self.head < self.anchor
    }

    /// Collapses to a caret at `head`.
    #[inline]
    pub fn collapse(&mut self) {
        self.anchor = self.head;
        self.goal_column = None;
    }
}

/// Ordered, non-overlapping-in-practice multi-cursor collection.
///
/// The list is kept sorted by start offset; the first selection is the
/// primary one reported to the status bar. Exact duplicates are dropped,
/// but overlapping selections are kept as-is (matching Zed's behavior).
#[derive(Clone, Debug, Default)]
pub struct Selections {
    selections: Vec<Selection>,
    next_id: usize,
}

impl Selections {
    pub fn new(offset: usize) -> Self {
        Self {
            selections: vec![Selection::point(0, offset)],
            next_id: 1,
        }
    }

    #[inline]
    pub fn count(&self) -> usize {
        self.selections.len()
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &Selection> {
        self.selections.iter()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Selection> {
        self.selections.iter_mut()
    }

    /// The primary selection (first in sorted order).
    #[inline]
    pub fn primary(&self) -> &Selection {
        &self.selections[0]
    }

    #[inline]
    pub fn primary_mut(&mut self) -> &mut Selection {
        &mut self.selections[0]
    }

    /// The primary selection's head offset.
    #[inline]
    pub fn cursor(&self) -> usize {
        self.primary().head
    }

    /// Replaces everything with a single caret.
    pub fn set_single_point(&mut self, offset: usize) {
        let id = self.alloc_id();
        self.selections = vec![Selection::point(id, offset)];
    }

    /// Replaces everything with a single range.
    pub fn set_single_range(&mut self, anchor: usize, head: usize) {
        let id = self.alloc_id();
        self.selections = vec![Selection::range(id, anchor, head)];
    }

    /// Adds a cursor at `offset` (e.g. Alt+Click).
    pub fn add_point(&mut self, offset: usize) {
        let id = self.alloc_id();
        self.selections.push(Selection::point(id, offset));
    }

    /// Adds a range selection.
    pub fn add_range(&mut self, anchor: usize, head: usize) {
        let id = self.alloc_id();
        self.selections.push(Selection::range(id, anchor, head));
    }

    /// Whether any selection covers a non-empty range.
    pub fn has_selection(&self) -> bool {
        self.selections.iter().any(|s| !s.is_empty())
    }

    /// The primary selection as an ordered byte range when non-empty.
    pub fn primary_range(&self) -> Option<std::ops::Range<usize>> {
        let primary = self.primary();
        (!primary.is_empty()).then(|| primary.start()..primary.end())
    }

    /// Adjusts every selection for a local text edit replacing
    /// `range` with text of `new_len` bytes. Anchors keep left bias and
    /// heads keep right bias around insertions, so typing at the caret
    /// keeps anchors fixed and moves heads past the inserted text.
    pub fn apply_edit(&mut self, range: std::ops::Range<usize>, new_len: usize) {
        let old_len = range.len();
        let delta = new_len as isize - old_len as isize;
        for selection in &mut self.selections {
            selection.anchor = adjust(selection.anchor, range.clone(), new_len, delta, Bias::Left);
            selection.head = adjust(selection.head, range.clone(), new_len, delta, Bias::Right);
        }
    }

    /// Clamps offsets to the text length, sorts by start, and drops exact
    /// duplicates.
    pub fn clamp_and_sort(&mut self, text_len: usize) {
        for selection in &mut self.selections {
            selection.anchor = selection.anchor.min(text_len);
            selection.head = selection.head.min(text_len);
        }
        self.selections.sort_by_key(|s| (s.start(), s.end(), s.id));
        self.selections
            .dedup_by(|a, b| a.anchor == b.anchor && a.head == b.head);
    }

    /// Collapses every selection to a caret at its head.
    pub fn collapse_all(&mut self) {
        for selection in &mut self.selections {
            selection.collapse();
        }
    }

    fn alloc_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

#[derive(Clone, Copy)]
enum Bias {
    Left,
    Right,
}

fn adjust(
    offset: usize,
    range: std::ops::Range<usize>,
    new_len: usize,
    delta: isize,
    bias: Bias,
) -> usize {
    let in_range = offset >= range.start && offset <= range.end;
    if in_range {
        match bias {
            // Left-biased positions inside the replaced region fall back to
            // its start; right-biased ones move to its end.
            Bias::Left => range.start,
            Bias::Right => range.start + new_len,
        }
    } else if offset > range.end {
        (offset as isize + delta) as usize
    } else {
        offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insertion_keeps_anchor_and_moves_head() {
        let mut selections = Selections::new(2);
        selections.apply_edit(2..2, 1);
        assert_eq!(selections.cursor(), 3);
        assert_eq!(selections.primary().anchor, 2);
    }

    #[test]
    fn deletion_replaces_range() {
        let mut selections = Selections::new(4);
        selections.set_single_range(1, 3);
        selections.apply_edit(1..3, 0);
        assert_eq!(selections.primary().anchor, 1);
        assert_eq!(selections.primary().head, 1);
    }

    #[test]
    fn offsets_after_edit_shift() {
        let mut selections = Selections::new(10);
        selections.apply_edit(0..0, 2);
        assert_eq!(selections.cursor(), 12);
    }
}
