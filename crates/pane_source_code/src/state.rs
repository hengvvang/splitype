//! Pure-Rust state for a raw Markdown source code editor pane.

use std::cell::RefCell;
use std::ops::Range;
use std::sync::Arc;

use core_contracts::OutlineHeading;
use core_contracts::{SearchMatch, SearchQuery};
use gpui::{
    App, Bounds, FocusHandle, InteractiveElement, IntoElement, ParentElement, Pixels,
    StatefulInteractiveElement, Styled, Window,
};
use theme::Theme;

use crate::buffer::{BufferPoint, LineMap};
use crate::display_map::{DisplayPoint, DisplaySnapshot, FoldMap, TabMap, WrapMap};
use crate::gutter::GutterLayout;
use crate::selection::{Selection, SelectionsCollection};
use crate::syntax::{CodeHighlightResult, find_matching_bracket, highlight_code_block};

/// Autonomous state for the SourceCode editor pane.
#[derive(Clone, Debug)]
pub struct SourceCodeState {
    pub text: String,
    pub line_map: LineMap,
    pub selections: SelectionsCollection,
    pub tab_map: TabMap,
    pub fold_map: FoldMap,
    pub wrap_map: WrapMap,
    pub marked_range: Option<Range<usize>>,
    pub last_bounds: RefCell<Option<Bounds<Pixels>>>,
    pub search_matches: Vec<(Range<usize>, bool)>,
    pub synced_doc_hash: u64,
    pub synced_revision: Option<u64>,
    pub synced_tab_index: Option<usize>,
    pub is_dragging: bool,
    pub drag_anchor: Option<usize>,
    pub focus_handle: RefCell<Option<FocusHandle>>,
    pub highlight_cache: Option<CodeHighlightResult>,
    pub highlight_hash: u64,
}

impl Default for SourceCodeState {
    fn default() -> Self {
        Self {
            text: String::new(),
            line_map: LineMap::default(),
            selections: SelectionsCollection::default(),
            tab_map: TabMap::default(),
            fold_map: FoldMap::default(),
            wrap_map: WrapMap::default(),
            marked_range: None,
            last_bounds: RefCell::new(None),
            search_matches: Vec::new(),
            synced_doc_hash: 0,
            synced_revision: None,
            synced_tab_index: None,
            is_dragging: false,
            drag_anchor: None,
            focus_handle: RefCell::new(None),
            highlight_cache: None,
            highlight_hash: 0,
        }
    }
}

impl SourceCodeState {
    /// Creates a new SourceCodeState from initial text.
    pub fn from_text(text: impl Into<String>) -> Self {
        let text = text.into();
        let line_map = LineMap::new(&text);
        let mut state = Self {
            text,
            line_map,
            ..Default::default()
        };
        state.rebuild_lines();
        state
    }

    /// Rebuilds cached line map.
    pub fn rebuild_lines(&mut self) {
        self.line_map = LineMap::new(&self.text);
    }

    /// Total number of lines in the buffer.
    #[inline]
    pub fn line_count(&self) -> usize {
        self.line_map.line_count()
    }

    /// Returns the byte range of a given 0-indexed line.
    #[inline]
    pub fn line_range(&self, line_index: usize) -> Range<usize> {
        self.line_map.line_range(line_index)
    }

    /// Returns start byte offset of a given 0-indexed line.
    #[inline]
    pub fn line_start_offset(&self, line_index: usize) -> usize {
        self.line_map.line_start_offset(line_index)
    }

    /// Returns end byte offset (before '\n') of a given 0-indexed line.
    #[inline]
    pub fn line_end_offset(&self, line_index: usize) -> usize {
        self.line_map.line_end_offset(line_index)
    }

    /// Returns string slice of a given 0-indexed line.
    #[inline]
    pub fn line_str(&self, line_index: usize) -> &str {
        let range = self.line_range(line_index);
        let start = range.start.min(self.text.len());
        let end = range.end.min(self.text.len());
        &self.text[start..end]
    }

    /// Returns (0-indexed line, 0-indexed byte column within that line).
    pub fn line_and_column(&self, offset: usize) -> (usize, usize) {
        let point = self.line_map.offset_to_point(&self.text, offset);
        (point.row as usize, point.column as usize)
    }

    /// Returns the byte offset corresponding to (line, byte-column).
    pub fn offset_at_line_col(&self, line_index: usize, col: usize) -> usize {
        self.line_map
            .point_to_offset(&self.text, BufferPoint::new(line_index as u32, col as u32))
    }

    /// Current cursor offset of the primary selection.
    #[inline]
    pub fn cursor(&self) -> usize {
        self.selections.primary().head
    }

    /// Returns (1-based line, 1-based col) cursor position for the status bar.
    pub fn cursor_position_1based(&self) -> (usize, usize) {
        let (line, byte_col) = self.line_and_column(self.cursor());
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

    /// Returns the GutterLayout helper.
    pub fn gutter_layout(&self, font_size: f32) -> GutterLayout {
        GutterLayout::new(self.line_count(), font_size)
    }

    /// Produces an immutable DisplaySnapshot.
    pub fn display_snapshot(&self) -> DisplaySnapshot<'_> {
        DisplaySnapshot::new(
            &self.text,
            &self.line_map,
            self.tab_map,
            &self.fold_map,
            self.wrap_map,
        )
    }

    /// Replaces the current text buffer.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.rebuild_lines();
        self.selections.clamp_to_len(self.text.len());
        self.marked_range = None;
        self.highlight_cache = None;
    }

    /// Inserts text at all selection cursors (or replaces selections).
    pub fn insert_text(&mut self, inserted: &str) {
        if self.selections.count() == 1 {
            let s = *self.selections.primary();
            if !s.is_empty() {
                let range = s.range_bounds();
                let start = range.start.min(self.text.len());
                let end = range.end.min(self.text.len());
                self.text.replace_range(start..end, inserted);
                self.selections.set_single_point(start + inserted.len());
            } else {
                let pos = s.head.min(self.text.len());
                self.text.insert_str(pos, inserted);
                self.selections.set_single_point(pos + inserted.len());
            }
        } else {
            // Multi-cursor insertion from back to front to preserve offsets
            let mut selections: Vec<Selection> = self.selections.all().to_vec();
            selections.sort_by_key(|s| std::cmp::Reverse(s.start()));

            for s in &mut selections {
                let range = s.range_bounds();
                let start = range.start.min(self.text.len());
                let end = range.end.min(self.text.len());
                self.text.replace_range(start..end, inserted);
                *s = Selection::point(s.id, start + inserted.len());
            }
            self.selections = SelectionsCollection::new();
            for s in selections.into_iter().rev() {
                self.selections.add_selection(s.anchor, s.head);
            }
        }
        self.rebuild_lines();
        self.highlight_cache = None;
    }

    /// Inserts a newline preserving current line's leading indentation.
    pub fn insert_newline_with_auto_indent(&mut self) {
        let cursor = self.cursor();
        let (row, _) = self.line_and_column(cursor);
        let line = self.line_str(row);
        let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();

        let mut newline_text = String::from("\n");
        newline_text.push_str(&indent);
        self.insert_text(&newline_text);
    }

    /// Indents current line(s) or selection.
    pub fn indent(&mut self) {
        if let Some(range) = self.selections.primary_selection_range() {
            let (start_row, _) = self.line_and_column(range.start);
            let (end_row, end_col) = self.line_and_column(range.end);
            let actual_end_row = if end_col == 0 && end_row > start_row {
                end_row - 1
            } else {
                end_row
            };

            for r in (start_row..=actual_end_row).rev() {
                let offset = self.line_start_offset(r);
                self.text.insert_str(offset, "    ");
            }
            self.rebuild_lines();
            let new_start = self.line_start_offset(start_row);
            let new_end = self.line_end_offset(actual_end_row);
            self.selections.set_single_range(new_start, new_end);
        } else {
            self.insert_text("    ");
        }
    }

    /// Outdents current line(s) or selection.
    pub fn outdent(&mut self) {
        let (start_row, end_row) = if let Some(range) = self.selections.primary_selection_range() {
            let (sr, _) = self.line_and_column(range.start);
            let (er, ec) = self.line_and_column(range.end);
            let actual_er = if ec == 0 && er > sr { er - 1 } else { er };
            (sr, actual_er)
        } else {
            let (r, _) = self.line_and_column(self.cursor());
            (r, r)
        };

        for r in (start_row..=end_row).rev() {
            let start = self.line_start_offset(r);
            let line = self.line_str(r);
            let spaces_to_remove = if line.starts_with("    ") {
                4
            } else if line.starts_with('\t') {
                1
            } else {
                line.chars().take_while(|&c| c == ' ').count().min(4)
            };
            if spaces_to_remove > 0 {
                self.text.drain(start..start + spaces_to_remove);
            }
        }
        self.rebuild_lines();
        let new_start = self.line_start_offset(start_row);
        let new_end = self.line_end_offset(end_row);
        if self.selections.has_any_selection() {
            self.selections.set_single_range(new_start, new_end);
        } else {
            self.selections
                .set_single_point(new_start.min(self.text.len()));
        }
    }

    /// Duplicates the current line or selection.
    pub fn duplicate_line(&mut self) {
        let cursor = self.cursor();
        let (row, _) = self.line_and_column(cursor);
        let line = self.line_str(row).to_string();
        let end = self.line_end_offset(row);
        self.text.insert_str(end, &format!("\n{}", line));
        self.rebuild_lines();
        self.selections.set_single_point(end + 1 + line.len());
    }

    /// Deletes the current line.
    pub fn delete_line(&mut self) {
        let cursor = self.cursor();
        let (row, _) = self.line_and_column(cursor);
        let start = self.line_start_offset(row);
        let end = if row + 1 < self.line_count() {
            self.line_start_offset(row + 1)
        } else {
            self.line_end_offset(row)
        };
        if start < end && end <= self.text.len() {
            self.text.drain(start..end);
        }
        self.rebuild_lines();
        self.selections.set_single_point(start.min(self.text.len()));
    }

    /// Deletes backward (Backspace).
    pub fn delete_backward(&mut self) {
        if self.selections.has_any_selection() {
            self.insert_text("");
            return;
        }

        let mut selections = self.selections.all().to_vec();
        selections.sort_by_key(|s| std::cmp::Reverse(s.head));

        for s in &mut selections {
            if s.head > 0 && s.head <= self.text.len() {
                let prev_char_len = self.text[..s.head]
                    .chars()
                    .last()
                    .map(|c| c.len_utf8())
                    .unwrap_or(1);
                let remove_start = s.head - prev_char_len;
                self.text.drain(remove_start..s.head);
                *s = Selection::point(s.id, remove_start);
            }
        }
        self.selections = SelectionsCollection::new();
        for s in selections.into_iter().rev() {
            self.selections.add_selection(s.anchor, s.head);
        }
        self.rebuild_lines();
        self.highlight_cache = None;
    }

    /// Deletes word backward (Ctrl+Backspace).
    pub fn delete_word_backward(&mut self) {
        if self.selections.has_any_selection() {
            self.insert_text("");
            return;
        }
        let cursor = self.cursor();
        if cursor == 0 {
            return;
        }
        let before = &self.text[..cursor];
        let mut remove_start = cursor;
        let mut seen_non_ws = false;
        for (idx, ch) in before.char_indices().rev() {
            if ch.is_whitespace() {
                if seen_non_ws {
                    remove_start = idx + ch.len_utf8();
                    break;
                }
            } else if ch.is_alphanumeric() || ch == '_' {
                seen_non_ws = true;
            } else {
                if seen_non_ws {
                    remove_start = idx + ch.len_utf8();
                    break;
                }
                remove_start = idx;
                break;
            }
            remove_start = idx;
        }
        if remove_start < cursor {
            self.text.drain(remove_start..cursor);
            self.selections.set_single_point(remove_start);
            self.rebuild_lines();
            self.highlight_cache = None;
        }
    }

    /// Deletes forward (Delete key).
    pub fn delete_forward(&mut self) {
        if self.selections.has_any_selection() {
            self.insert_text("");
            return;
        }
        let mut selections = self.selections.all().to_vec();
        selections.sort_by_key(|s| std::cmp::Reverse(s.head));

        for s in &mut selections {
            if s.head < self.text.len() {
                let next_char_len = self.text[s.head..]
                    .chars()
                    .next()
                    .map(|c| c.len_utf8())
                    .unwrap_or(1);
                self.text.drain(s.head..s.head + next_char_len);
                *s = Selection::point(s.id, s.head);
            }
        }
        self.selections = SelectionsCollection::new();
        for s in selections.into_iter().rev() {
            self.selections.add_selection(s.anchor, s.head);
        }
        self.rebuild_lines();
        self.highlight_cache = None;
    }

    /// Deletes word forward (Ctrl+Delete).
    pub fn delete_word_forward(&mut self) {
        if self.selections.has_any_selection() {
            self.insert_text("");
            return;
        }
        let cursor = self.cursor();
        if cursor >= self.text.len() {
            return;
        }
        let after = &self.text[cursor..];
        let mut remove_end = self.text.len();
        let mut seen_non_ws = false;
        for (idx, ch) in after.char_indices() {
            if ch.is_whitespace() {
                if seen_non_ws {
                    remove_end = cursor + idx;
                    break;
                }
            } else if ch.is_alphanumeric() || ch == '_' {
                seen_non_ws = true;
            } else {
                if seen_non_ws {
                    remove_end = cursor + idx;
                    break;
                }
                remove_end = cursor + idx + ch.len_utf8();
                break;
            }
        }
        if remove_end > cursor {
            self.text.drain(cursor..remove_end);
            self.rebuild_lines();
            self.highlight_cache = None;
        }
    }

    /// Selects all text in the buffer.
    pub fn select_all(&mut self) {
        self.selections.set_single_range(0, self.text.len());
    }

    /// The text covered by the primary selection, if any.
    pub fn selected_text(&self) -> Option<&str> {
        let range = self.selections.primary_selection_range()?;
        let start = range.start.min(self.text.len());
        let end = range.end.min(self.text.len());
        if start < end {
            Some(&self.text[start..end])
        } else {
            None
        }
    }

    /// Moves cursor to offset, optionally extending selection.
    pub fn move_to(&mut self, offset: usize, extend: bool) {
        let offset = offset.min(self.text.len());
        if extend {
            let anchor = self.selections.primary().anchor;
            self.selections.set_single_range(anchor, offset);
        } else {
            self.selections.set_single_point(offset);
        }
    }

    /// Adds an extra cursor at `offset` (Alt+Click).
    pub fn add_cursor_at(&mut self, offset: usize) {
        let offset = offset.min(self.text.len());
        self.selections.add_selection(offset, offset);
    }

    /// Adds a cursor on the line above (Ctrl+Alt+Up).
    pub fn add_cursor_above(&mut self) {
        let cursor = self.cursor();
        let (row, col) = self.line_and_column(cursor);
        if row > 0 {
            let target_offset = self.offset_at_line_col(row - 1, col);
            self.selections.add_selection(target_offset, target_offset);
        }
    }

    /// Adds a cursor on the line below (Ctrl+Alt+Down).
    pub fn add_cursor_below(&mut self) {
        let cursor = self.cursor();
        let (row, col) = self.line_and_column(cursor);
        if row + 1 < self.line_count() {
            let target_offset = self.offset_at_line_col(row + 1, col);
            self.selections.add_selection(target_offset, target_offset);
        }
    }

    /// Move left by one character (or by word if word=true).
    pub fn move_left(&mut self, extend: bool, word: bool) {
        for s in self.selections.all_mut() {
            if s.head > 0 {
                let target = if word {
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
                s.head = target;
                if !extend {
                    s.anchor = target;
                }
                s.goal_column = None;
            }
        }
        self.selections.normalize();
    }

    /// Move right by one character (or by word if word=true).
    pub fn move_right(&mut self, extend: bool, word: bool) {
        let text_len = self.text.len();
        for s in self.selections.all_mut() {
            if s.head < text_len {
                let target = if word {
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
                s.head = target;
                if !extend {
                    s.anchor = target;
                }
                s.goal_column = None;
            }
        }
        self.selections.normalize();
    }

    /// Move up one visual line.
    pub fn move_up(&mut self, extend: bool) {
        let new_offsets: Vec<(usize, usize, Option<u32>)> = {
            let snapshot = self.display_snapshot();
            self.selections
                .all()
                .iter()
                .map(|s| {
                    let dp = snapshot.offset_to_display_point(s.head);
                    let gc = s.goal_column.unwrap_or(dp.column);
                    if dp.row > 0 {
                        let target_dp = DisplayPoint::new(dp.row - 1, gc);
                        (s.id, snapshot.display_point_to_offset(target_dp), Some(gc))
                    } else {
                        (s.id, s.head, s.goal_column)
                    }
                })
                .collect()
        };

        for (id, offset, goal_col) in new_offsets {
            if let Some(s) = self
                .selections
                .all_mut()
                .iter_mut()
                .find(|sel| sel.id == id)
            {
                s.head = offset;
                if !extend {
                    s.anchor = offset;
                }
                s.goal_column = goal_col;
            }
        }
        self.selections.normalize();
    }

    /// Move down one visual line.
    pub fn move_down(&mut self, extend: bool) {
        let new_offsets: Vec<(usize, usize, Option<u32>)> = {
            let snapshot = self.display_snapshot();
            let total_visible = snapshot.visible_line_count();
            self.selections
                .all()
                .iter()
                .map(|s| {
                    let dp = snapshot.offset_to_display_point(s.head);
                    let gc = s.goal_column.unwrap_or(dp.column);
                    if dp.row + 1 < total_visible {
                        let target_dp = DisplayPoint::new(dp.row + 1, gc);
                        (s.id, snapshot.display_point_to_offset(target_dp), Some(gc))
                    } else {
                        (s.id, s.head, s.goal_column)
                    }
                })
                .collect()
        };

        for (id, offset, goal_col) in new_offsets {
            if let Some(s) = self
                .selections
                .all_mut()
                .iter_mut()
                .find(|sel| sel.id == id)
            {
                s.head = offset;
                if !extend {
                    s.anchor = offset;
                }
                s.goal_column = goal_col;
            }
        }
        self.selections.normalize();
    }

    /// Move to start of line.
    pub fn move_to_line_start(&mut self, extend: bool) {
        let starts: Vec<(usize, usize)> = self
            .selections
            .all()
            .iter()
            .map(|s| {
                let (row, _) = self.line_and_column(s.head);
                (s.id, self.line_start_offset(row))
            })
            .collect();

        for (id, offset) in starts {
            if let Some(s) = self
                .selections
                .all_mut()
                .iter_mut()
                .find(|sel| sel.id == id)
            {
                s.head = offset;
                if !extend {
                    s.anchor = offset;
                }
                s.goal_column = None;
            }
        }
        self.selections.normalize();
    }

    /// Move to end of line.
    pub fn move_to_line_end(&mut self, extend: bool) {
        let ends: Vec<(usize, usize)> = self
            .selections
            .all()
            .iter()
            .map(|s| {
                let (row, _) = self.line_and_column(s.head);
                (s.id, self.line_end_offset(row))
            })
            .collect();

        for (id, offset) in ends {
            if let Some(s) = self
                .selections
                .all_mut()
                .iter_mut()
                .find(|sel| sel.id == id)
            {
                s.head = offset;
                if !extend {
                    s.anchor = offset;
                }
                s.goal_column = None;
            }
        }
        self.selections.normalize();
    }

    /// Selects the word at `offset`.
    pub fn select_word_at(&mut self, offset: usize) {
        let offset = offset.min(self.text.len());
        let (row, _) = self.line_and_column(offset);
        let line = self.line_str(row);
        let line_start = self.line_start_offset(row);
        let col = offset - line_start;

        let mut word_start = col;
        let mut word_end = col;

        let chars: Vec<(usize, char)> = line.char_indices().collect();
        for (i, &(byte_idx, ch)) in chars.iter().enumerate() {
            let next_byte = chars.get(i + 1).map(|(b, _)| *b).unwrap_or(line.len());
            if byte_idx <= col && col < next_byte {
                if ch.is_alphanumeric() || ch == '_' {
                    // Expand left
                    for &(b_idx, c) in chars[..=i].iter().rev() {
                        if c.is_alphanumeric() || c == '_' {
                            word_start = b_idx;
                        } else {
                            break;
                        }
                    }
                    // Expand right
                    for &(b_idx, c) in chars[i..].iter() {
                        if c.is_alphanumeric() || c == '_' {
                            word_end = b_idx + c.len_utf8();
                        } else {
                            break;
                        }
                    }
                }
                break;
            }
        }

        self.selections
            .set_single_range(line_start + word_start, line_start + word_end);
    }

    /// Selects the entire line at `line_index`.
    pub fn select_line_at(&mut self, line_index: usize) {
        let start = self.line_start_offset(line_index);
        let end = if line_index + 1 < self.line_count() {
            self.line_start_offset(line_index + 1)
        } else {
            self.line_end_offset(line_index)
        };
        self.selections.set_single_range(start, end);
    }

    /// Start a drag selection.
    pub fn start_drag(&mut self, offset: usize) {
        let offset = offset.min(self.text.len());
        self.is_dragging = true;
        self.drag_anchor = Some(offset);
        self.selections.set_single_point(offset);
    }

    /// Update drag selection.
    pub fn update_drag(&mut self, offset: usize) {
        if let Some(anchor) = self.drag_anchor {
            let offset = offset.min(self.text.len());
            self.selections.set_single_range(anchor, offset);
        }
    }

    /// End drag selection.
    pub fn end_drag(&mut self) {
        self.is_dragging = false;
        self.drag_anchor = None;
    }

    /// Finds matching bracket for the primary cursor.
    pub fn matching_bracket(&self) -> Option<usize> {
        find_matching_bracket(&self.text, self.cursor())
    }
}

impl core_contracts::PaneView for SourceCodeState {
    fn kind(&self) -> core_contracts::PaneKind {
        core_contracts::PaneKind::new("source_code")
    }

    fn capabilities(&self) -> core_contracts::PaneCapabilities {
        core_contracts::PaneCapabilities {
            editable: true,
            searchable: true,
            replaceable: true,
            outline: true,
            navigable: true,
        }
    }

    fn focus_handle(&self, cx: &App) -> Option<FocusHandle> {
        let mut handle = self.focus_handle.borrow_mut();
        if handle.is_none() {
            *handle = Some(cx.focus_handle());
        }
        handle.clone()
    }

    fn cursor_position(&self, _cx: &App) -> Option<(usize, usize)> {
        Some(self.cursor_position_1based())
    }

    fn sync_document(&mut self, document: &core_contracts::DocumentSnapshot, _cx: &mut App) {
        let text = document.text.as_ref();
        let revision = document.revision;
        if self.synced_revision == Some(revision) && self.text == text {
            return;
        }
        let hash = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            text.hash(&mut h);
            h.finish()
        };
        if self.synced_doc_hash != hash || self.text != text {
            self.set_text(text);
            self.synced_doc_hash = hash;
        }
        self.synced_revision = Some(revision);
    }

    fn serialize_text(&self, _cx: &App) -> Option<String> {
        Some(self.text.clone())
    }

    fn outline_headings(&self, _cx: &App) -> Vec<OutlineHeading> {
        crate::outline::extract_source_headings(&self.text)
    }

    fn navigate_to_outline(&mut self, index: usize, _theme: &Theme, cx: &mut App) {
        let headings = self.outline_headings(cx);
        if let Some(h) = headings.get(index) {
            let offset = self.line_start_offset(h.block_index);
            self.selections.set_single_point(offset);
        }
    }

    fn search_matches(&self, query: &SearchQuery, _cx: &App) -> Vec<SearchMatch> {
        crate::search::search_in_source(&self.text, query)
    }

    fn navigate_to_search_match(&mut self, match_item: &SearchMatch, _cx: &mut App) {
        let start = match_item.byte_range.start.min(self.text.len());
        let end = match_item.byte_range.end.min(self.text.len());
        self.selections.set_single_range(start, end);
    }

    fn replace_match(
        &mut self,
        match_item: &SearchMatch,
        replace_with: &str,
        _cx: &mut App,
    ) -> Option<String> {
        crate::search::replace_source_match(self, match_item, replace_with);
        Some(self.text.clone())
    }

    fn replace_all_matches(
        &mut self,
        query: &SearchQuery,
        replace_with: &str,
        _cx: &mut App,
    ) -> Option<String> {
        let matches = self.search_matches(query, _cx);
        for m in matches.into_iter().rev() {
            crate::search::replace_source_match(self, &m, replace_with);
        }
        Some(self.text.clone())
    }

    fn apply_line_prefix(&mut self, prefix: &str, _cx: &mut App) -> Option<String> {
        let cursor = self.cursor();
        let (row, _) = self.line_and_column(cursor);
        let start = self.line_start_offset(row);
        self.text.insert_str(start, prefix);
        self.rebuild_lines();
        self.selections.set_single_point(cursor + prefix.len());
        Some(self.text.clone())
    }

    fn apply_snippet(
        &mut self,
        snippet: &str,
        caret_offset: usize,
        _cx: &mut App,
    ) -> Option<String> {
        let start_pos = self.selections.primary().start();
        self.insert_text(snippet);
        self.selections.set_single_point(start_pos + caret_offset);
        Some(self.text.clone())
    }

    fn apply_wrapped_or_template(
        &mut self,
        empty_template: &str,
        caret_offset_in_empty: usize,
        wrap_prefix: &str,
        wrap_suffix: &str,
        _cx: &mut App,
    ) -> Option<String> {
        if let Some(selected) = self.selected_text() {
            let wrapped = format!("{}{}{}", wrap_prefix, selected, wrap_suffix);
            self.insert_text(&wrapped);
        } else {
            let _ = self.apply_snippet(empty_template, caret_offset_in_empty, _cx);
        }
        Some(self.text.clone())
    }

    fn apply_clear_format(&mut self, _cx: &mut App) -> Option<String> {
        None
    }

    fn handle_key_down(
        &mut self,
        pane_id: core_contracts::PaneId,
        event: &gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut App,
        host: &dyn core_contracts::PaneHost,
    ) -> bool {
        crate::input::handle_key_down(self, pane_id, event, window, cx, host)
    }

    fn handle_mouse_down(
        &mut self,
        _pane_id: core_contracts::PaneId,
        event: &gpui::MouseDownEvent,
        window: &mut Window,
        cx: &mut App,
    ) {
        crate::input::handle_mouse_down(self, event, window, cx);
    }

    fn handle_mouse_move(
        &mut self,
        _pane_id: core_contracts::PaneId,
        event: &gpui::MouseMoveEvent,
        window: &mut Window,
        cx: &mut App,
    ) {
        crate::input::handle_mouse_move(self, event, window, cx);
    }

    fn handle_mouse_up(
        &mut self,
        _pane_id: core_contracts::PaneId,
        _event: &gpui::MouseUpEvent,
        _window: &mut Window,
        _cx: &mut App,
    ) {
        crate::input::handle_mouse_up(self);
    }

    fn render(
        &mut self,
        ctx: &core_contracts::PaneRenderContext,
        _window: &mut Window,
        cx: &mut App,
    ) -> gpui::AnyElement {
        let theme = cx.global::<theme::ThemeManager>().current_arc();

        let code_hash = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            self.text.hash(&mut h);
            theme.name.hash(&mut h);
            h.finish()
        };

        if self.highlight_hash != code_hash || self.highlight_cache.is_none() {
            self.highlight_cache = highlight_code_block(Some("markdown"), &self.text);
            self.highlight_hash = code_hash;
        }

        let element = crate::element::EditorElement::new(
            self.clone(),
            ctx.pane_id,
            ctx.is_focused,
            ctx.scroll.clone(),
            ctx.host.clone(),
        );

        let outline_host: Arc<dyn core_contracts::OutlineHost> =
            Arc::new(core_contracts::PaneOutlineHost {
                pane_id: ctx.pane_id,
                host: ctx.host.clone(),
            });
        let outline_hud = core_contracts::render_floating_outline_hud(
            ctx.pane_id.0,
            &self.outline_headings(cx),
            None,
            false,
            &theme,
            &outline_host,
        );

        let mut outer = gpui::div()
            .id(gpui::ElementId::Name(
                format!("tiled-source-editor-{}", ctx.pane_id.0).into(),
            ))
            .key_context("SourceCode")
            .w_full()
            .h_full()
            .relative()
            .bg(theme.colors.editor_background);

        if let Some(focus_handle) = self.focus_handle(cx) {
            outer = outer.track_focus(&focus_handle);
        }

        let pane_id = ctx.pane_id;
        let host = ctx.host.clone();
        let host_key = host.clone();
        let host_down = host.clone();
        let host_move = host.clone();
        let host_up = host.clone();

        outer = outer.on_key_down(move |event, window, cx| {
            let handled = host_key.handle_pane_key_down(pane_id, event, window, cx);
            if handled {
                cx.stop_propagation();
            }
        });

        outer
            .child(
                gpui::div()
                    .id(gpui::ElementId::Name(
                        format!("tiled-source-scroll-{}", pane_id.0).into(),
                    ))
                    .w_full()
                    .h_full()
                    .overflow_y_scroll()
                    .track_scroll(ctx.scroll)
                    .on_mouse_down(gpui::MouseButton::Left, move |event, window, cx| {
                        host_down.handle_pane_mouse_down(pane_id, event, window, cx);
                    })
                    .on_mouse_move(move |event, window, cx| {
                        host_move.handle_pane_mouse_move(pane_id, event, window, cx);
                    })
                    .on_mouse_up(gpui::MouseButton::Left, move |event, window, cx| {
                        host_up.handle_pane_mouse_up(pane_id, event, window, cx);
                    })
                    .child(element),
            )
            .child(outline_hud)
            .into_any_element()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
