//! Cursor movement, multi-cursor management, and drag selection.
//!
//! Vertical movement goes through the display snapshot so soft wraps and
//! folds are navigated by visual row with a memorized goal column. Any
//! cursor movement ends the current typing run, which keeps undo grouping
//! scoped to consecutive typing.

use crate::display_map::DisplayPoint;
use crate::editor::SourceCodeEditor;
use crate::syntax::find_matching_bracket;

impl SourceCodeEditor {
    // ── Point/offset geometry ─────────────────────────────────────────────

    /// (0-based line, byte column within the line) of a byte offset.
    #[inline]
    pub fn point_of(&self, offset: usize) -> (usize, usize) {
        let point = self.line_map.offset_to_point(offset);
        (point.row as usize, point.column as usize)
    }

    /// Byte offset of a (line, byte-column) point.
    #[inline]
    pub fn offset_at_line_col(&self, line_index: usize, col: usize) -> usize {
        self.line_map
            .point_to_offset(crate::buffer::BufferPoint::new(
                line_index as u32,
                col as u32,
            ))
    }

    #[inline]
    pub fn line_start_offset(&self, row: usize) -> usize {
        self.line_map.line_start(row)
    }

    #[inline]
    pub fn line_end_offset(&self, row: usize) -> usize {
        self.line_start_offset(row) + self.line_map.line_len(row)
    }

    /// Returns (1-based line, 1-based char column) cursor position for the
    /// status bar.
    pub fn cursor_position_1based(&self) -> (usize, usize) {
        let (line, byte_col) = self.point_of(self.cursor());
        let line_str = self.line_str(line);
        let mut char_count = 0;
        for (b_idx, _) in line_str.char_indices() {
            if b_idx >= byte_col {
                break;
            }
            char_count += 1;
        }
        (line + 1, char_count + 1)
    }

    /// Finds the matching bracket offset for the primary cursor.
    pub fn matching_bracket(&self) -> Option<usize> {
        find_matching_bracket(&self.text, self.cursor())
    }

    // ── Movement ──────────────────────────────────────────────────────────

    /// Moves the primary cursor to `offset`, optionally extending the
    /// selection from its current anchor.
    pub fn move_to(&mut self, offset: usize, extend: bool) {
        self.reset_typing_run();
        let offset = offset.min(self.text.len());
        if extend {
            let anchor = self.selections.primary().anchor;
            self.selections.set_single_range(anchor, offset);
        } else {
            self.selections.set_single_point(offset);
        }
        self.start_cursor_blink();
    }

    /// Adds an extra cursor at `offset` (Alt+Click).
    pub fn add_cursor_at(&mut self, offset: usize) {
        self.reset_typing_run();
        self.selections.add_point(offset.min(self.text.len()));
        self.selections.clamp_and_sort(self.text.len());
        self.start_cursor_blink();
    }

    /// Adds a cursor on the line above (Ctrl+Alt+Up).
    pub fn add_cursor_above(&mut self) {
        let cursor = self.cursor();
        let (row, col) = self.point_of(cursor);
        if row > 0 {
            let target = self.offset_at_line_col(row - 1, col);
            self.add_cursor_at(target);
        }
    }

    /// Adds a cursor on the line below (Ctrl+Alt+Down).
    pub fn add_cursor_below(&mut self) {
        let cursor = self.cursor();
        let (row, col) = self.point_of(cursor);
        if row + 1 < self.line_count() {
            let target = self.offset_at_line_col(row + 1, col);
            self.add_cursor_at(target);
        }
    }

    /// Moves every cursor left by one character (or one word when
    /// `word` is set), optionally extending selections.
    pub fn move_left(&mut self, extend: bool, word: bool) {
        self.reset_typing_run();
        let offsets: Vec<(usize, usize)> = self
            .selections
            .iter()
            .map(|s| {
                let target = if s.head == 0 {
                    s.head
                } else if word {
                    let before = &self.text[..s.head];
                    let mut prev = 0;
                    let mut seen_non_ws = false;
                    for (idx, ch) in before.char_indices().rev() {
                        if ch.is_whitespace() {
                            if seen_non_ws {
                                prev = idx + ch.len_utf8();
                                break;
                            }
                        } else {
                            seen_non_ws = true;
                        }
                        prev = idx;
                    }
                    prev
                } else {
                    let prev_char_len = self.text[..s.head]
                        .chars()
                        .last()
                        .map(|c| c.len_utf8())
                        .unwrap_or(1);
                    s.head.saturating_sub(prev_char_len)
                };
                (s.id, target)
            })
            .collect();
        for (id, target) in offsets {
            if let Some(s) = self.selections.iter_mut().find(|s| s.id == id) {
                s.head = target;
                if !extend {
                    s.anchor = target;
                }
                s.goal_column = None;
            }
        }
        self.selections.clamp_and_sort(self.text.len());
        self.start_cursor_blink();
    }

    /// Moves every cursor right by one character (or one word when
    /// `word` is set), optionally extending selections.
    pub fn move_right(&mut self, extend: bool, word: bool) {
        self.reset_typing_run();
        let text_len = self.text.len();
        let offsets: Vec<(usize, usize)> = self
            .selections
            .iter()
            .map(|s| {
                let target = if s.head >= text_len {
                    s.head
                } else if word {
                    let after = &self.text[s.head..];
                    let mut next = text_len;
                    let mut seen_non_ws = false;
                    for (idx, ch) in after.char_indices() {
                        if ch.is_whitespace() {
                            if seen_non_ws {
                                next = s.head + idx;
                                break;
                            }
                        } else {
                            seen_non_ws = true;
                        }
                    }
                    next
                } else {
                    let next_char_len = self.text[s.head..]
                        .chars()
                        .next()
                        .map(|c| c.len_utf8())
                        .unwrap_or(1);
                    (s.head + next_char_len).min(text_len)
                };
                (s.id, target)
            })
            .collect();
        for (id, target) in offsets {
            if let Some(s) = self.selections.iter_mut().find(|s| s.id == id) {
                s.head = target;
                if !extend {
                    s.anchor = target;
                }
                s.goal_column = None;
            }
        }
        self.selections.clamp_and_sort(self.text.len());
        self.start_cursor_blink();
    }

    /// Moves every cursor up one visual row.
    pub fn move_up(&mut self, extend: bool) {
        self.move_vertical(-1, extend);
    }

    /// Moves every cursor down one visual row.
    pub fn move_down(&mut self, extend: bool) {
        self.move_vertical(1, extend);
    }

    fn move_vertical(&mut self, delta: i32, extend: bool) {
        self.reset_typing_run();
        let snapshot = self.snapshot();
        let targets: Vec<(usize, usize, Option<u32>)> = self
            .selections
            .iter()
            .map(|s| {
                let dp = snapshot.offset_to_display_point(s.head);
                let goal = s.goal_column.unwrap_or(dp.column);
                let target_row = (dp.row as i64 + delta as i64).max(0) as u32;
                if target_row < snapshot.visible_line_count() {
                    let target_dp = DisplayPoint::new(target_row, goal);
                    (
                        s.id,
                        snapshot.display_point_to_offset(target_dp),
                        Some(goal),
                    )
                } else if delta < 0 {
                    (
                        s.id,
                        snapshot.display_point_to_offset(DisplayPoint::ZERO),
                        Some(goal),
                    )
                } else {
                    (s.id, s.head, s.goal_column)
                }
            })
            .collect();
        for (id, offset, goal) in targets {
            if let Some(s) = self.selections.iter_mut().find(|s| s.id == id) {
                s.head = offset;
                if !extend {
                    s.anchor = offset;
                }
                s.goal_column = goal;
            }
        }
        self.selections.clamp_and_sort(self.text.len());
        self.start_cursor_blink();
    }

    /// Moves every cursor to its line start.
    pub fn move_to_line_start(&mut self, extend: bool) {
        self.reset_typing_run();
        let starts: Vec<(usize, usize)> = self
            .selections
            .iter()
            .map(|s| (s.id, self.line_start_offset(self.point_of(s.head).0)))
            .collect();
        for (id, offset) in starts {
            if let Some(s) = self.selections.iter_mut().find(|s| s.id == id) {
                s.head = offset;
                if !extend {
                    s.anchor = offset;
                }
                s.goal_column = None;
            }
        }
        self.selections.clamp_and_sort(self.text.len());
        self.start_cursor_blink();
    }

    /// Moves every cursor to its line end.
    pub fn move_to_line_end(&mut self, extend: bool) {
        self.reset_typing_run();
        let ends: Vec<(usize, usize)> = self
            .selections
            .iter()
            .map(|s| (s.id, self.line_end_offset(self.point_of(s.head).0)))
            .collect();
        for (id, offset) in ends {
            if let Some(s) = self.selections.iter_mut().find(|s| s.id == id) {
                s.head = offset;
                if !extend {
                    s.anchor = offset;
                }
                s.goal_column = None;
            }
        }
        self.selections.clamp_and_sort(self.text.len());
        self.start_cursor_blink();
    }

    /// Selects the word at `offset`.
    pub fn select_word_at(&mut self, offset: usize) {
        self.reset_typing_run();
        let offset = offset.min(self.text.len());
        let (row, _) = self.point_of(offset);
        let line_start = self.line_start_offset(row);
        let line = self.line_str(row);
        let col = offset - line_start;

        let mut word_start = col;
        let mut word_end = col;

        let chars: Vec<(usize, char)> = line.char_indices().collect();
        for (i, &(byte_idx, ch)) in chars.iter().enumerate() {
            let next_byte = chars.get(i + 1).map(|(b, _)| *b).unwrap_or(line.len());
            if byte_idx <= col && col < next_byte && (ch.is_alphanumeric() || ch == '_') {
                for &(b_idx, c) in chars[..=i].iter().rev() {
                    if c.is_alphanumeric() || c == '_' {
                        word_start = b_idx;
                    } else {
                        break;
                    }
                }
                for &(b_idx, c) in chars[i..].iter() {
                    if c.is_alphanumeric() || c == '_' {
                        word_end = b_idx + c.len_utf8();
                    } else {
                        break;
                    }
                }
                break;
            }
        }

        self.selections
            .set_single_range(line_start + word_start, line_start + word_end);
        self.start_cursor_blink();
    }

    /// Selects the entire line at `line_index` (including the newline when
    /// it is not the last line).
    pub fn select_line_at(&mut self, line_index: usize) {
        self.reset_typing_run();
        let start = self.line_start_offset(line_index);
        let end = if line_index + 1 < self.line_count() {
            self.line_start_offset(line_index + 1)
        } else {
            self.line_end_offset(line_index)
        };
        self.selections.set_single_range(start, end);
        self.start_cursor_blink();
    }

    // ── Drag selection ────────────────────────────────────────────────────

    /// Starts a drag selection anchored at `offset`.
    pub fn start_drag(&mut self, offset: usize) {
        let offset = offset.min(self.text.len());
        self.is_dragging = true;
        self.drag_anchor = Some(offset);
        self.selections.set_single_point(offset);
        self.start_cursor_blink();
    }

    /// Extends the drag selection to `offset`.
    pub fn update_drag(&mut self, offset: usize) {
        if let Some(anchor) = self.drag_anchor {
            let offset = offset.min(self.text.len());
            self.selections.set_single_range(anchor, offset);
            self.start_cursor_blink();
        }
    }

    /// Ends the drag session.
    pub fn end_drag(&mut self) {
        self.is_dragging = false;
        self.drag_anchor = None;
    }
}
