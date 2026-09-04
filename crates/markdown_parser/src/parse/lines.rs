//! Line-source abstraction of the block parser.
//!
//! Every block-level collector reads its input through [`Lines`] instead of
//! a concrete slice: `&[S]` of lines backs full-text parsing, and a
//! [`RopeLines`] adapter lets the incremental re-parse read directly from
//! the document rope — line `i` resolves through the rope's chunk index in
//! O(log m) without copying text, so no line list is ever materialized on
//! the edit path.

use rope::Rope;

/// Random-access source of document lines. Both backing views mirror
/// `str::lines` exactly: empty text has no lines, and a trailing newline
/// starts no phantom line.
pub trait Lines {
    /// Number of lines.
    fn line_count(&self) -> usize;

    /// The text of line `index` (no trailing newline). Panics when out of
    /// bounds, mirroring slice indexing; every caller bounds-checks with
    /// [`Lines::line_count`] first.
    fn line(&self, index: usize) -> &str;

    /// The text of line `index`, or `None` when out of bounds.
    fn get(&self, index: usize) -> Option<&str> {
        (index < self.line_count()).then(|| self.line(index))
    }

    /// A sub-view over `start..end`, without copying any line.
    fn slice(&self, start: usize, end: usize) -> LinesSlice<'_, Self> {
        LinesSlice {
            lines: self,
            start,
            end,
        }
    }
}

impl<T: AsRef<str>> Lines for [T] {
    fn line_count(&self) -> usize {
        self.len()
    }

    fn line(&self, index: usize) -> &str {
        self[index].as_ref()
    }
}

/// A line view over `start..end` of another [`Lines`].
pub struct LinesSlice<'a, L: Lines + ?Sized> {
    lines: &'a L,
    start: usize,
    end: usize,
}

impl<L: Lines + ?Sized> Lines for LinesSlice<'_, L> {
    fn line_count(&self) -> usize {
        self.end - self.start
    }

    fn line(&self, index: usize) -> &str {
        assert!(index < self.end - self.start, "line index out of bounds");
        self.lines.line(self.start + index)
    }
}

/// Direct line access to a [`Rope`]: each `line` call is an O(log m) chunk
/// lookup and returns a borrow into the chunk, so nothing is allocated or
/// materialized.
pub(crate) struct RopeLines<'a> {
    rope: &'a Rope,
}

impl<'a> RopeLines<'a> {
    pub(crate) fn new(rope: &'a Rope) -> Self {
        Self { rope }
    }
}

impl Lines for RopeLines<'_> {
    fn line_count(&self) -> usize {
        if self.rope.is_empty() {
            0
        } else {
            self.rope.line_count()
        }
    }

    fn line(&self, index: usize) -> &str {
        self.rope.line_str(index)
    }
}
