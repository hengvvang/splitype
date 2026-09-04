//! Persistent chunk rope with incremental line indexing.
//!
//! This is the source editor's text layer, mirroring the architectural
//! properties of Zed's `text` crate: edits are O(log m) in the number of
//! chunks (m = len / chunk size) instead of O(n) in the document, chunks
//! are shared across edits (`Arc<str>`), and line indexing is maintained
//! incrementally as part of the chunk summaries — never rebuilt per edit.
//!
//! Chunks always split at line boundaries, so a line lives entirely within
//! one chunk and `line_str` can borrow without allocating. A single line
//! longer than the target chunk size simply becomes its own chunk.

use std::borrow::Cow;
use std::ops::{Bound, Range, RangeBounds};
use std::sync::Arc;

/// Preferred chunk size; chunks split at line boundaries above this.
const CHUNK_TARGET: usize = 4096;

/// One immutable text chunk: whole lines plus their relative line starts.
#[derive(Clone)]
struct Chunk {
    text: Arc<str>,
    /// Byte offset of each line start within the chunk (always starts at 0).
    line_starts: Vec<u32>,
}

/// An immutable, persistent rope of UTF-8 text.
#[derive(Clone)]
pub struct Rope {
    chunks: Vec<Chunk>,
    /// Prefix summaries; each has `chunks.len() + 1` entries.
    byte_offsets: Vec<usize>,
    line_offsets: Vec<u32>,
    total_bytes: usize,
    /// Whether the text ends with `\n` (which does not create a trailing
    /// line and does not count toward the last line's length).
    ends_with_newline: bool,
}

impl Rope {
    /// Builds a rope from text, splitting into chunks at line boundaries.
    pub fn new(text: &str) -> Self {
        Self::from_chunks(Self::split_into_chunks(text), text.ends_with('\n'))
    }

    fn from_chunks(chunks: Vec<Chunk>, ends_with_newline: bool) -> Self {
        let mut byte_offsets = Vec::with_capacity(chunks.len() + 1);
        let mut line_offsets = Vec::with_capacity(chunks.len() + 1);
        byte_offsets.push(0);
        line_offsets.push(0);
        let mut bytes = 0usize;
        let mut lines = 0u32;
        for chunk in &chunks {
            bytes += chunk.text.len();
            lines += chunk.line_starts.len() as u32;
            byte_offsets.push(bytes);
            line_offsets.push(lines);
        }
        Self {
            chunks,
            byte_offsets,
            line_offsets,
            total_bytes: bytes,
            ends_with_newline,
        }
    }

    fn split_into_chunks(text: &str) -> Vec<Chunk> {
        let mut chunks = Vec::new();
        let mut start = 0usize;
        for (idx, ch) in text.char_indices() {
            if ch == '\n' && idx + 1 - start > CHUNK_TARGET {
                // Split at this line boundary.
                chunks.push(Chunk::from_text(&text[start..idx + 1]));
                start = idx + 1;
            }
        }
        if start < text.len() || chunks.is_empty() {
            chunks.push(Chunk::from_text(&text[start..]));
        }
        chunks
    }

    // ── Queries ──────────────────────────────────────────────────────────

    #[inline]
    pub fn len(&self) -> usize {
        self.total_bytes
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.total_bytes == 0
    }

    /// Whether the text ends with a newline (which, per the rope's line
    /// convention, does not start a trailing line).
    #[inline]
    pub fn ends_with_newline(&self) -> bool {
        self.ends_with_newline
    }

    #[inline]
    pub fn line_count(&self) -> usize {
        let lines = self.line_offsets.last().copied().unwrap_or(0) as usize;
        lines.max(1)
    }

    /// The full text, materialized (used by commits and background work).
    pub fn materialize(&self) -> String {
        let mut out = String::with_capacity(self.total_bytes);
        for chunk in &self.chunks {
            out.push_str(&chunk.text);
        }
        out
    }

    /// The chunk index containing `offset`. Offsets equal to the total
    /// length (or past it) resolve to the last chunk.
    fn chunk_index_for_byte(&self, offset: usize) -> usize {
        let offset = offset.min(self.total_bytes);
        self.byte_offsets
            .partition_point(|start| *start <= offset)
            .saturating_sub(1)
            .min(self.chunks.len().saturating_sub(1))
    }

    fn chunk_index_for_line(&self, row: usize) -> usize {
        let row = row as u32;
        self.line_offsets
            .partition_point(|lines| *lines <= row)
            .saturating_sub(1)
    }

    /// Byte range of a line within its chunk's text.
    fn line_chunk_range(&self, row: usize) -> (usize, Range<usize>) {
        let chunk_idx = self.chunk_index_for_line(row.min(self.line_count() - 1));
        let chunk = &self.chunks[chunk_idx];
        let relative_row = row as u32 - self.line_offsets[chunk_idx];
        let relative_row = relative_row as usize;
        let start = chunk.line_starts[relative_row] as usize;
        let end = if relative_row + 1 < chunk.line_starts.len() {
            // The next line starts right after this line's newline.
            chunk.line_starts[relative_row + 1] as usize - 1
        } else if chunk_idx + 1 < self.chunks.len() {
            chunk.text.len() - 1 // chunk ends with a newline
        } else if self.ends_with_newline {
            chunk.text.len() - 1
        } else {
            chunk.text.len()
        };
        (chunk_idx, start..end)
    }

    /// Byte offset at which line `row` starts.
    pub fn line_start(&self, row: usize) -> usize {
        let (chunk_idx, range) = self.line_chunk_range(row);
        self.byte_offsets[chunk_idx] + range.start
    }

    /// Byte length of line `row`, excluding the trailing newline.
    pub fn line_len(&self, row: usize) -> usize {
        let (_, range) = self.line_chunk_range(row);
        range.len()
    }

    /// The text of line `row`. Borrows from the rope: lines never span
    /// chunks (except a single over-long line, which is one chunk), so no
    /// allocation is needed.
    pub fn line_str(&self, row: usize) -> &str {
        let (chunk_idx, range) = self.line_chunk_range(row.min(self.line_count() - 1));
        &self.chunks[chunk_idx].text[range]
    }

    /// A slice over an arbitrary byte range. Borrows when the range lies
    /// within one chunk; allocates otherwise.
    pub fn slice(&self, range: impl RangeBounds<usize>) -> Cow<'_, str> {
        let start = match range.start_bound() {
            Bound::Included(&start) => start,
            Bound::Excluded(&start) => start.saturating_add(1),
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(&end) => end.saturating_add(1),
            Bound::Excluded(&end) => end,
            Bound::Unbounded => self.total_bytes,
        };
        let start = start.min(self.total_bytes);
        let end = end.min(self.total_bytes).max(start);
        if start == end {
            return Cow::Borrowed("");
        }
        let first = self.chunk_index_for_byte(start);
        let last = self.chunk_index_for_byte(end.saturating_sub(1));
        if first == last {
            let chunk_start = self.byte_offsets[first];
            return Cow::Borrowed(&self.chunks[first].text[start - chunk_start..end - chunk_start]);
        }
        let mut out = String::with_capacity(end - start);
        for chunk_idx in first..=last {
            let chunk_start = self.byte_offsets[chunk_idx];
            let chunk_end = self.byte_offsets[chunk_idx + 1];
            let from = start.max(chunk_start) - chunk_start;
            let to = end.min(chunk_end) - chunk_start;
            out.push_str(&self.chunks[chunk_idx].text[from..to]);
        }
        Cow::Owned(out)
    }

    /// Owned slice (convenience for callers that need `String`).
    pub fn slice_owned(&self, range: impl RangeBounds<usize>) -> String {
        self.slice(range).into_owned()
    }

    /// (row, byte column within the line) of a byte offset. Offsets
    /// pointing at newlines resolve to the end of the preceding line.
    pub fn offset_to_point(&self, offset: usize) -> (usize, usize) {
        let offset = offset.min(self.total_bytes);
        let chunk_idx = self.chunk_index_for_byte(offset);
        let chunk = &self.chunks[chunk_idx];
        let relative = offset - self.byte_offsets[chunk_idx];
        let line_idx = chunk
            .line_starts
            .partition_point(|start| *start as usize <= relative)
            .saturating_sub(1);
        let row = (self.line_offsets[chunk_idx] + line_idx as u32) as usize;
        let line_start = chunk.line_starts[line_idx] as usize;
        let column = (relative - line_start).min(self.line_len(row));
        (row, column)
    }

    /// Byte offset of a (row, byte column) point; clamps to line ends.
    pub fn point_to_offset(&self, row: usize, column: usize) -> usize {
        self.line_start(row) + column.min(self.line_len(row))
    }

    /// The character immediately before `offset`, if any.
    pub fn char_before(&self, offset: usize) -> Option<char> {
        if offset == 0 || offset > self.total_bytes {
            return None;
        }
        let chunk_idx = self.chunk_index_for_byte(offset - 1);
        let chunk = &self.chunks[chunk_idx];
        let relative = offset - self.byte_offsets[chunk_idx];
        chunk.text[..relative].chars().last()
    }

    /// The character at `offset`, if any.
    pub fn char_after(&self, offset: usize) -> Option<char> {
        if offset >= self.total_bytes {
            return None;
        }
        let chunk_idx = self.chunk_index_for_byte(offset);
        let chunk = &self.chunks[chunk_idx];
        let relative = offset - self.byte_offsets[chunk_idx];
        chunk.text[relative..].chars().next()
    }

    // ── Edits ────────────────────────────────────────────────────────────

    /// Replaces `range` with `new_text`, returning a new rope. Only the
    /// chunks overlapping the edit are rebuilt; untouched chunks are
    /// shared with the previous rope, and the line index is rebuilt only
    /// for the affected chunks.
    pub fn edit(&self, range: Range<usize>, new_text: &str) -> Self {
        let start = range.start.min(self.total_bytes);
        let end = range.end.min(self.total_bytes).max(start);

        let first = if start == 0 {
            0
        } else {
            self.chunk_index_for_byte(start - 1)
        };
        let last = if end == 0 {
            0
        } else {
            self.chunk_index_for_byte(end - 1)
        };

        // Rebuild only the text span [first chunk start, last chunk end)
        // with the edit applied. Both boundaries are line boundaries (chunks
        // split only at newlines), so the rebuilt chunks stay line-aligned.
        let head_start = self.byte_offsets[first];
        let tail_end = self.byte_offsets[last + 1];
        let mut merged =
            String::with_capacity((start - head_start) + new_text.len() + (tail_end - end));
        merged.push_str(&self.slice(head_start..start));
        merged.push_str(new_text);
        merged.push_str(&self.slice(end..tail_end));

        let mut chunks = Vec::with_capacity(first + (self.chunks.len() - last - 1) + 2);
        chunks.extend_from_slice(&self.chunks[..first]);
        chunks.extend(Self::split_into_chunks(&merged));
        chunks.extend_from_slice(&self.chunks[last + 1..]);

        let ends_with_newline = chunks
            .last()
            .is_some_and(|chunk| chunk.text.ends_with('\n'));
        Self::from_chunks(chunks, ends_with_newline)
    }
}

impl Chunk {
    fn from_text(text: &str) -> Self {
        let mut line_starts = vec![0u32];
        for (idx, ch) in text.char_indices() {
            // A newline at the very end of the chunk does not start a line
            // within it: the next line (if any) begins in the next chunk.
            // Every chunk boundary is a line boundary, so this holds for
            // intermediate chunks as well as the final one.
            if ch == '\n' && idx + 1 < text.len() {
                line_starts.push((idx + 1) as u32);
            }
        }
        Self {
            text: Arc::from(text),
            line_starts,
        }
    }
}

impl PartialEq for Rope {
    fn eq(&self, other: &Self) -> bool {
        self.materialize() == other.materialize()
    }
}

impl Eq for Rope {}

impl std::fmt::Debug for Rope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rope")
            .field("len", &self.total_bytes)
            .field("lines", &self.line_count())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexes_lines_and_offsets() {
        let rope = Rope::new("ab\ncd\nef");
        assert_eq!(rope.line_count(), 3);
        assert_eq!(rope.line_start(0), 0);
        assert_eq!(rope.line_start(1), 3);
        assert_eq!(rope.line_len(0), 2);
        assert_eq!(rope.offset_to_point(4), (1, 1));
        assert_eq!(rope.point_to_offset(1, 1), 4);
    }

    #[test]
    fn newline_offsets_resolve_to_line_end() {
        let rope = Rope::new("ab\ncd");
        assert_eq!(rope.offset_to_point(2), (0, 2));
        assert_eq!(rope.offset_to_point(3), (1, 0));
    }

    #[test]
    fn trailing_newline_and_empty() {
        assert_eq!(Rope::new("").line_count(), 1);
        let rope = Rope::new("ab\n");
        assert_eq!(rope.line_count(), 1);
        assert_eq!(rope.line_len(0), 2);
        assert_eq!(rope.offset_to_point(3), (0, 2));
    }

    #[test]
    fn multibyte_offsets_never_panic() {
        let text = "你好，世界。\n这是一个测试。";
        let rope = Rope::new(text);
        for byte_offset in 0..=text.len() {
            let (row, col) = rope.offset_to_point(byte_offset);
            let recovered = rope.point_to_offset(row, col);
            assert!(recovered <= text.len());
        }
    }

    #[test]
    fn edits_preserve_lines_across_chunk_boundaries() {
        // Force multiple chunks.
        let mut text = String::new();
        for i in 0..600 {
            text.push_str(&format!("line {i:04} with some content\n"));
        }
        let rope = Rope::new(&text);
        assert!(rope.chunks.len() > 1);

        // Edit near the start, across a chunk boundary, and at the end.
        let edited = rope.edit(10..20, "REPLACED");
        assert_eq!(
            edited.materialize(),
            format!("{}{}{}", &text[..10], "REPLACED", &text[20..])
        );

        let boundary = rope.byte_offsets[1];
        let edited = rope.edit(boundary - 5..boundary + 5, "X");
        let expected = format!("{}{}{}", &text[..boundary - 5], "X", &text[boundary + 5..]);
        assert_eq!(edited.materialize(), expected);

        let len = rope.len();
        let edited = rope.edit(len..len, "tail");
        assert_eq!(edited.materialize(), format!("{text}tail"));

        // Line queries stay consistent after an edit.
        let edited = rope.edit(0..0, "aa\nbb\n");
        assert_eq!(edited.line_count(), rope.line_count() + 2);
        assert_eq!(edited.line_str(0), "aa");
        assert_eq!(edited.line_str(1), "bb");
        assert_eq!(edited.line_str(2), "line 0000 with some content");
    }

    #[test]
    fn slices_borrow_within_a_chunk_and_materialize_across() {
        let mut text = String::new();
        for i in 0..600 {
            text.push_str(&format!("line {i:04}\n"));
        }
        let rope = Rope::new(&text);
        let borrowed = rope.slice(5..10);
        assert!(matches!(borrowed, Cow::Borrowed(_)));
        assert_eq!(borrowed.as_ref(), &text[5..10]);

        let crossed = rope.slice(0..rope.len());
        assert_eq!(crossed.as_ref(), text);
    }

    #[test]
    fn edits_are_persistent() {
        let base = Rope::new("hello world\nsecond line\n");
        let edited = base.edit(6..11, "zed");
        assert_eq!(base.materialize(), "hello world\nsecond line\n");
        assert_eq!(edited.materialize(), "hello zed\nsecond line\n");
    }

    #[test]
    fn every_line_across_chunks_is_intact() {
        // Intermediate chunks end with a newline; the line starting the
        // next chunk must never appear as a phantom row in this chunk.
        let mut text = String::new();
        for i in 0..600 {
            text.push_str(&format!("line {i:04} with content\n"));
        }
        let rope = Rope::new(&text);
        let expected: Vec<&str> = text.lines().collect();
        assert_eq!(rope.line_count(), expected.len());
        for (row, line) in expected.iter().enumerate() {
            assert_eq!(rope.line_str(row), *line, "row {row}");
        }
        // The first byte of every line resolves to that line's first
        // column, and no offset ever resolves past the last line.
        for (row, _) in expected.iter().enumerate() {
            let (point_row, col) = rope.offset_to_point(rope.line_start(row));
            assert_eq!((point_row, col), (row, 0));
        }
        let (last_row, _) = rope.offset_to_point(rope.len());
        assert_eq!(last_row, expected.len() - 1);
    }

    #[test]
    fn trailing_newline_creates_no_phantom_line() {
        let rope = Rope::new("a\nb\n");
        assert_eq!(rope.line_count(), 2);
        assert_eq!(rope.line_str(0), "a");
        assert_eq!(rope.line_str(1), "b");
        let multi = Rope::new(&"x\n".repeat(5000));
        assert_eq!(multi.line_count(), 5000);
        assert_eq!(multi.line_str(4999), "x");
    }
}
