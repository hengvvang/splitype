//! Text buffer operations for inline filename editing.

use std::ops::Range;

use crate::state::state::ExplorerFilenameEditor;

// ── UTF-8 / UTF-16 offset conversion (IME bridge) ────────────────────────
// Local implementation — the explorer must not depend on the markdown
// world, and this conversion is generic text math.

fn utf16_to_utf8_in(text: &str, utf16_offset: usize) -> usize {
    let mut utf16_count = 0;
    let mut utf8_offset = 0;
    for ch in text.chars() {
        if utf16_count >= utf16_offset {
            break;
        }
        utf16_count += ch.len_utf16();
        utf8_offset += ch.len_utf8();
    }
    utf8_offset
}

fn utf8_to_utf16_in(text: &str, utf8_offset: usize) -> usize {
    let mut utf16_offset = 0;
    let mut utf8_count = 0;
    for ch in text.chars() {
        if utf8_count >= utf8_offset {
            break;
        }
        utf8_count += ch.len_utf8();
        utf16_offset += ch.len_utf16();
    }
    utf16_offset
}

pub fn utf16_range_to_utf8_in(text: &str, range_utf16: &Range<usize>) -> Range<usize> {
    utf16_to_utf8_in(text, range_utf16.start)..utf16_to_utf8_in(text, range_utf16.end)
}

pub fn utf8_range_to_utf16_in(text: &str, range: &Range<usize>) -> Range<usize> {
    utf8_to_utf16_in(text, range.start)..utf8_to_utf16_in(text, range.end)
}

pub fn utf8_to_utf16_in_single(text: &str, utf8_offset: usize) -> usize {
    utf8_to_utf16_in(text, utf8_offset)
}

// ── Text buffer operations ──────────────────────────────────────────────
impl ExplorerFilenameEditor {
    /// Replace the whole buffer, optionally preselecting a byte range.
    pub(crate) fn set_text(&mut self, text: String, select: Option<Range<usize>>) {
        self.text = text;
        let end = self.text.len();
        let selection = select.unwrap_or(end..end);
        self.selection = selection.start.min(end)..selection.end.min(end);
        self.reversed = false;
        self.marked_range = None;
    }

    /// The selected range in forward order.
    pub(crate) fn selection_range(&self) -> Range<usize> {
        let (start, end) = if self.reversed {
            (self.selection.end, self.selection.start)
        } else {
            (self.selection.start, self.selection.end)
        };
        start..end
    }

    pub(crate) fn selected_text(&self) -> &str {
        &self.text[self.selection_range()]
    }

    pub(crate) fn cursor(&self) -> usize {
        if self.reversed {
            self.selection.start
        } else {
            self.selection.end
        }
    }

    /// Replace `range` with `new_text`, placing the caret after it.
    pub(crate) fn replace_range(&mut self, range: Range<usize>, new_text: &str) {
        let start = range.start.min(self.text.len());
        let end = range.end.min(self.text.len());
        if start > end {
            return;
        }
        self.text.replace_range(start..end, new_text);
        let cursor = start + new_text.len();
        self.selection = cursor..cursor;
        self.reversed = false;
        self.marked_range = None;
    }

    pub(crate) fn insert_at_selection(&mut self, new_text: &str) {
        let range = self.selection_range();
        self.replace_range(range, new_text);
    }

    pub(crate) fn delete_backward(&mut self) {
        let range = self.selection_range();
        if !range.is_empty() {
            self.replace_range(range, "");
            return;
        }
        let cursor = self.cursor();
        if cursor > 0 {
            let start = self.text.floor_char_boundary(cursor - 1);
            self.replace_range(start..cursor, "");
        }
    }

    pub(crate) fn delete_forward(&mut self) {
        let range = self.selection_range();
        if !range.is_empty() {
            self.replace_range(range, "");
            return;
        }
        let cursor = self.cursor();
        if cursor < self.text.len() {
            let end = self.text.ceil_char_boundary(cursor + 1);
            self.replace_range(cursor..end, "");
        }
    }

    pub(crate) fn move_left(&mut self, extend: bool) {
        let cursor = self.cursor();
        let anchor = self.selection_anchor();
        if !extend && !self.selection_range().is_empty() {
            let new_cursor = if self.reversed {
                self.selection.start
            } else {
                self.selection.end
            };
            self.selection = new_cursor..new_cursor;
            self.reversed = false;
            return;
        }
        let target = self.text.floor_char_boundary(cursor.saturating_sub(1));
        self.set_cursor(target, anchor, extend);
    }

    pub(crate) fn move_right(&mut self, extend: bool) {
        let cursor = self.cursor();
        let anchor = self.selection_anchor();
        if !extend && !self.selection_range().is_empty() {
            let new_cursor = if self.reversed {
                self.selection.end
            } else {
                self.selection.start
            };
            self.selection = new_cursor..new_cursor;
            self.reversed = false;
            return;
        }
        let target = self
            .text
            .ceil_char_boundary(cursor + 1)
            .min(self.text.len());
        self.set_cursor(target, anchor, extend);
    }

    pub(crate) fn move_home(&mut self, extend: bool) {
        let anchor = self.selection_anchor();
        self.set_cursor(0, anchor, extend);
    }

    pub(crate) fn move_end(&mut self, extend: bool) {
        let anchor = self.selection_anchor();
        self.set_cursor(self.text.len(), anchor, extend);
    }

    fn selection_anchor(&self) -> usize {
        if self.reversed {
            self.selection.end
        } else {
            self.selection.start
        }
    }

    fn set_cursor(&mut self, cursor: usize, anchor: usize, extend: bool) {
        if extend {
            self.selection = anchor..cursor;
            self.reversed = cursor < anchor;
        } else {
            self.selection = cursor..cursor;
            self.reversed = false;
        }
    }
}

// ── Editor-side handlers: validation, confirm/cancel, keyboard, clipboard ─

