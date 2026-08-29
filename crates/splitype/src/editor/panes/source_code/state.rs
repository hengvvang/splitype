use std::ops::Range;

use gpui::{Bounds, FocusHandle, Pixels};

use syntax::highlight::{
    CodeHighlightResult, highlight_code_block,
};

/// Pure-Rust state for a raw Markdown source code editor pane.
#[derive(Clone, Debug, Default)]
pub(crate) struct SourceCodeState {
    pub(crate) text: String,
    pub(crate) line_ranges: Vec<Range<usize>>,
    pub(crate) cursor: usize,
    pub(crate) selection: Option<Range<usize>>,
    pub(crate) marked_range: Option<Range<usize>>,
    pub(crate) last_bounds: Option<Bounds<Pixels>>,
    pub(crate) search_matches: Vec<(Range<usize>, bool)>,
    pub(crate) synced_doc_hash: u64,
    pub(crate) synced_revision: Option<u64>,
    pub(crate) synced_tab_index: Option<usize>,
    pub(crate) is_dragging: bool,
    pub(crate) drag_anchor: Option<usize>,
    pub(crate) focus_handle: Option<FocusHandle>,
    pub(crate) highlight_cache: Option<CodeHighlightResult>,
    pub(crate) highlight_hash: u64,
}

impl SourceCodeState {
    /// Rebuilds cached line byte ranges.
    pub(crate) fn rebuild_lines(&mut self) {
        let mut lines = Vec::new();
        let mut start = 0;
        for part in self.text.split('\n') {
            lines.push(start..start + part.len());
            start += part.len() + 1;
        }
        if lines.is_empty() {
            lines.push(0..0);
        }
        self.line_ranges = lines;
    }

    /// Total number of lines in the buffer.
    #[inline]
    pub(crate) fn line_count(&self) -> usize {
        if self.line_ranges.is_empty() {
            1
        } else {
            self.line_ranges.len()
        }
    }

    /// Returns the byte range of a given 0-indexed line.
    #[inline]
    pub(crate) fn line_range(&self, line_index: usize) -> Range<usize> {
        if line_index < self.line_ranges.len() {
            self.line_ranges[line_index].clone()
        } else if let Some(last) = self.line_ranges.last() {
            last.end..last.end
        } else {
            0..0
        }
    }

    /// Returns start byte offset of a given 0-indexed line.
    #[inline]
    pub(crate) fn line_start_offset(&self, line_index: usize) -> usize {
        self.line_range(line_index).start
    }

    /// Returns end byte offset (before '\n') of a given 0-indexed line.
    #[inline]
    pub(crate) fn line_end_offset(&self, line_index: usize) -> usize {
        self.line_range(line_index).end
    }

    /// Returns string slice of a given 0-indexed line.
    #[inline]
    pub(crate) fn line_str(&self, line_index: usize) -> &str {
        let range = self.line_range(line_index);
        let start = range.start.min(self.text.len());
        let end = range.end.min(self.text.len());
        &self.text[start..end]
    }

    /// Returns (0-indexed line, 0-indexed byte column within that line).
    pub(crate) fn line_and_column(&self, offset: usize) -> (usize, usize) {
        let clamped = offset.min(self.text.len());
        if self.line_ranges.is_empty() {
            return (0, clamped);
        }
        let line_idx = match self.line_ranges.binary_search_by(|r| {
            if clamped < r.start {
                std::cmp::Ordering::Greater
            } else if clamped > r.end {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        }) {
            Ok(idx) => idx,
            Err(idx) => idx.saturating_sub(1).min(self.line_ranges.len() - 1),
        };
        let line_start = self.line_ranges[line_idx].start;
        let col = clamped.saturating_sub(line_start);
        (line_idx, col)
    }

    /// Returns the byte offset corresponding to a given (0-indexed line, 0-indexed byte column).
    pub(crate) fn offset_at_line_col(&self, line_index: usize, col: usize) -> usize {
        let range = self.line_range(line_index);
        let line_len = range.end.saturating_sub(range.start);
        let clamped_col = col.min(line_len);
        let target = range.start + clamped_col;
        clamp_to_char_boundary(&self.text, target)
    }

    /// Start a mouse drag selection session.
    pub(crate) fn start_drag(&mut self, offset: usize) {
        let clamped = offset.min(self.text.len());
        self.cursor = clamped;
        self.selection = None;
        self.is_dragging = true;
        self.drag_anchor = Some(clamped);
    }

    /// Update mouse drag selection session with a new target offset.
    pub(crate) fn update_drag(&mut self, offset: usize) {
        let Some(anchor) = self.drag_anchor else {
            self.cursor = offset.min(self.text.len());
            return;
        };
        let target = offset.min(self.text.len());
        self.cursor = target;
        if anchor == target {
            self.selection = None;
        } else {
            let start = anchor.min(target);
            let end = anchor.max(target);
            self.selection = Some(start..end);
        }
    }

    /// End mouse drag selection session.
    pub(crate) fn end_drag(&mut self) {
        self.is_dragging = false;
        self.drag_anchor = None;
    }

    /// Select word around a given byte offset.
    pub(crate) fn select_word_at(&mut self, offset: usize) {
        if self.text.is_empty() {
            return;
        }
        let pos = offset.min(self.text.len());
        let s = self.text.as_str();

        let mut start = pos;
        while start > 0 {
            let prev = prev_char_boundary(s, start);
            let ch = s[prev..start].chars().next().unwrap_or(' ');
            if ch.is_alphanumeric() || ch == '_' {
                start = prev;
            } else {
                break;
            }
        }

        let mut end = pos;
        while end < s.len() {
            let next = next_char_boundary(s, end);
            let ch = s[end..next].chars().next().unwrap_or(' ');
            if ch.is_alphanumeric() || ch == '_' {
                end = next;
            } else {
                break;
            }
        }

        if start < end {
            self.selection = Some(start..end);
            self.cursor = end;
        } else {
            self.move_to(pos, false);
        }
    }

    /// Select entire line at given line index.
    pub(crate) fn select_line_at(&mut self, line_index: usize) {
        let range = self.line_range(line_index);
        self.selection = Some(range.clone());
        self.cursor = range.end;
    }

    /// Update the buffer's full text from an external sync.
    pub(crate) fn set_text(&mut self, text: String) {
        self.text = text;
        self.rebuild_lines();
        self.cursor = self.cursor.min(self.text.len());
        if let Some(sel) = self.selection.as_mut() {
            sel.start = sel.start.min(self.text.len());
            sel.end = sel.end.min(self.text.len());
            if sel.start == sel.end {
                self.selection = None;
            }
        }
        self.refresh_highlight();
    }

    /// Refresh syntax highlighting cache if text has changed.
    pub(crate) fn refresh_highlight(&mut self) {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.text.hash(&mut h);
        let hash = h.finish();
        if hash != self.highlight_hash || self.highlight_cache.is_none() {
            self.highlight_cache = highlight_code_block(Some("markdown"), &self.text);
            self.highlight_hash = hash;
        }
    }

    /// Returns the currently selected text slice, if any.
    pub(crate) fn selected_text(&self) -> Option<&str> {
        let sel = self.selection.as_ref()?;
        if sel.start < sel.end && sel.end <= self.text.len() {
            Some(&self.text[sel.start..sel.end])
        } else {
            None
        }
    }

    /// Inserts text at the current cursor position, replacing selection if any.
    pub(crate) fn insert_text(&mut self, inserted: &str) {
        if let Some(sel) = self.selection.take() {
            let start = sel.start.min(self.text.len());
            let end = sel.end.min(self.text.len());
            self.text.replace_range(start..end, inserted);
            self.cursor = start + inserted.len();
        } else {
            let pos = self.cursor.min(self.text.len());
            self.text.insert_str(pos, inserted);
            self.cursor = pos + inserted.len();
        }
        self.rebuild_lines();
        self.refresh_highlight();
    }

    /// Deletes text backward (Backspace).
    pub(crate) fn delete_backward(&mut self) {
        if let Some(sel) = self.selection.take() {
            let start = sel.start.min(self.text.len());
            let end = sel.end.min(self.text.len());
            self.text.replace_range(start..end, "");
            self.cursor = start;
        } else if self.cursor > 0 {
            let prev = prev_char_boundary(&self.text, self.cursor);
            self.text.replace_range(prev..self.cursor, "");
            self.cursor = prev;
        }
        self.rebuild_lines();
        self.refresh_highlight();
    }

    /// Deletes text forward (Delete).
    pub(crate) fn delete_forward(&mut self) {
        if let Some(sel) = self.selection.take() {
            let start = sel.start.min(self.text.len());
            let end = sel.end.min(self.text.len());
            self.text.replace_range(start..end, "");
            self.cursor = start;
        } else if self.cursor < self.text.len() {
            let next = next_char_boundary(&self.text, self.cursor);
            self.text.replace_range(self.cursor..next, "");
        }
        self.rebuild_lines();
        self.refresh_highlight();
    }

    /// Moves cursor to a specific byte offset.
    pub(crate) fn move_to(&mut self, offset: usize, extend_selection: bool) {
        let target = offset.min(self.text.len());
        if extend_selection {
            let anchor = match self.selection.as_ref() {
                Some(sel) => {
                    if self.cursor == sel.end {
                        sel.start
                    } else {
                        sel.end
                    }
                }
                None => self.cursor,
            };
            let (start, end) = if target < anchor {
                (target, anchor)
            } else {
                (anchor, target)
            };
            self.selection = if start == end { None } else { Some(start..end) };
        } else {
            self.selection = None;
        }
        self.cursor = target;
    }

    pub(crate) fn move_left(&mut self, extend_selection: bool) {
        if !extend_selection && self.selection.is_some() {
            let start = self.selection.take().unwrap().start;
            self.cursor = start;
            return;
        }
        if self.cursor > 0 {
            let prev = prev_char_boundary(&self.text, self.cursor);
            self.move_to(prev, extend_selection);
        }
    }

    pub(crate) fn move_right(&mut self, extend_selection: bool) {
        if !extend_selection && self.selection.is_some() {
            let end = self.selection.take().unwrap().end;
            self.cursor = end;
            return;
        }
        if self.cursor < self.text.len() {
            let next = next_char_boundary(&self.text, self.cursor);
            self.move_to(next, extend_selection);
        }
    }

    pub(crate) fn move_up(&mut self, extend_selection: bool) {
        let (cur_line, col) = self.line_and_column(self.cursor);
        if cur_line > 0 {
            let target_line = cur_line - 1;
            let target_offset = self.offset_at_line_col(target_line, col);
            self.move_to(target_offset, extend_selection);
        } else {
            self.move_to(0, extend_selection);
        }
    }

    pub(crate) fn move_down(&mut self, extend_selection: bool) {
        let (cur_line, col) = self.line_and_column(self.cursor);
        let total_lines = self.line_count();
        if cur_line + 1 < total_lines {
            let target_line = cur_line + 1;
            let target_offset = self.offset_at_line_col(target_line, col);
            self.move_to(target_offset, extend_selection);
        } else {
            self.move_to(self.text.len(), extend_selection);
        }
    }

    pub(crate) fn move_to_line_start(&mut self, extend_selection: bool) {
        let (cur_line, _) = self.line_and_column(self.cursor);
        let range = self.line_range(cur_line);
        self.move_to(range.start, extend_selection);
    }

    pub(crate) fn move_to_line_end(&mut self, extend_selection: bool) {
        let (cur_line, _) = self.line_and_column(self.cursor);
        let range = self.line_range(cur_line);
        self.move_to(range.end, extend_selection);
    }

    pub(crate) fn select_all(&mut self) {
        if !self.text.is_empty() {
            self.selection = Some(0..self.text.len());
            self.cursor = self.text.len();
        }
    }
}

fn prev_char_boundary(s: &str, mut idx: usize) -> usize {
    if idx == 0 {
        return 0;
    }
    idx -= 1;
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

fn next_char_boundary(s: &str, mut idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    idx += 1;
    while idx < s.len() && !s.is_char_boundary(idx) {
        idx += 1;
    }
    idx
}

fn clamp_to_char_boundary(s: &str, mut idx: usize) -> usize {
    idx = idx.min(s.len());
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

// ── Pane plugin contract ─────────────────────────────────────────────────

use gpui::App;

use crate::editor::engine::controller::Editor;
use crate::editor::engine::session::EditorPaneKind;
use crate::editor::panes::outline::build_outline_headings_from_markdown;
use crate::editor::panes::pane::{OutlineNode, Pane};

impl Pane for SourceCodeState {
    fn kind(&self) -> EditorPaneKind {
        EditorPaneKind::SourceCode
    }

    fn document_source(&self, _editor: &Editor, _cx: &App) -> String {
        self.text.clone()
    }

    fn set_search_matches(&mut self, matches: &[(std::ops::Range<usize>, bool)]) {
        self.search_matches = matches.to_vec();
    }

    fn outline_items(&self, editor: &Editor, cx: &App) -> Vec<OutlineNode> {
        // The source buffer is authoritative while it holds text; fall
        // back to the shared document when the pane was never synced.
        let text = if self.text.is_empty() {
            editor.doc().serialize_markdown(cx)
        } else {
            self.text.clone()
        };
        build_outline_headings_from_markdown(&text)
    }
}
