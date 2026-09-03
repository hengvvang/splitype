//! Text editing operations on the editor's local document projection.
//!
//! Every local edit applies a rope edit (O(chunks), with the line index
//! maintained incrementally by the rope) and then commits through the pane
//! host as one [`EditTransaction`]. Typing runs (consecutive
//! single-character insertions at the same position) are merged into one
//! buffer-level undo transaction via [`EditTransaction::merge`].

use std::ops::Range;

use gpui::Context;

use crate::editor::SourceCodeEditor;
use crate::selection::Selections;
use editor_contracts::EditTransaction;

impl SourceCodeEditor {
    // ── Typing-path operations (mutate + commit through the host) ─────────

    /// Inserts text at every cursor (replacing selections) and commits.
    pub fn insert_text_commit(&mut self, inserted: &str, cx: &mut Context<Self>) {
        let cursor_before = self.cursor_hint();
        let merge = self.merge_for_insert(self.cursor(), inserted);
        let insert_pos = self.insert_text_local(inserted);
        self.record_edit_run(merge, cursor_before, insert_pos);
        self.after_text_change();
        self.schedule_highlight(cx);
        self.commit_local_edit(merge, cursor_before, cx);
    }

    /// Inserts a newline preserving the current line's leading indentation.
    pub fn insert_newline_with_auto_indent(&mut self, cx: &mut Context<Self>) {
        let cursor = self.cursor();
        let (row, _) = self.point_of(cursor);
        let indent: String = self
            .line_str(row)
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect();
        self.insert_text_commit(&format!("\n{indent}"), cx);
    }

    /// Indents the current line(s) or selection.
    pub fn indent(&mut self, cx: &mut Context<Self>) {
        let indent_unit = self.settings.indent_unit();
        let Some(range) = self.selections.primary_range() else {
            self.insert_text_commit(&indent_unit, cx);
            return;
        };
        let (start_row, _) = self.point_of(range.start);
        let (end_row, end_col) = self.point_of(range.end);
        let actual_end_row = if end_col == 0 && end_row > start_row {
            end_row - 1
        } else {
            end_row
        };

        let cursor_before = self.cursor_hint();
        for row in (start_row..=actual_end_row).rev() {
            let offset = self.line_start_offset(row);
            self.replace_local(offset..offset, &indent_unit);
        }
        self.record_edit_run(false, cursor_before, None);
        self.after_text_change();
        self.schedule_highlight(cx);
        let new_start = self.line_start_offset(start_row);
        let new_end = self.line_end_offset(actual_end_row);
        self.selections.set_single_range(new_start, new_end);
        self.commit_local_edit(false, cursor_before, cx);
    }

    /// Outdents the current line(s) or selection.
    pub fn outdent(&mut self, cx: &mut Context<Self>) {
        let (start_row, end_row) = if let Some(range) = self.selections.primary_range() {
            let (sr, _) = self.point_of(range.start);
            let (er, ec) = self.point_of(range.end);
            let actual_er = if ec == 0 && er > sr { er - 1 } else { er };
            (sr, actual_er)
        } else {
            let (row, _) = self.point_of(self.cursor());
            (row, row)
        };

        let cursor_before = self.cursor_hint();
        for row in (start_row..=end_row).rev() {
            let start = self.line_start_offset(row);
            let line = self.line_str(row);
            let spaces_to_remove = if line.starts_with("    ") {
                4
            } else if line.starts_with('\t') {
                1
            } else {
                line.chars().take_while(|&c| c == ' ').count().min(4)
            };
            if spaces_to_remove > 0 {
                self.replace_local(start..start + spaces_to_remove, "");
            }
        }
        self.record_edit_run(false, cursor_before, None);
        self.after_text_change();
        self.schedule_highlight(cx);
        let new_start = self.line_start_offset(start_row);
        let new_end = self.line_end_offset(end_row);
        if self.selections.has_selection() {
            self.selections.set_single_range(new_start, new_end);
        } else {
            self.selections
                .set_single_point(new_start.min(self.text.len()));
        }
        self.commit_local_edit(false, cursor_before, cx);
    }

    /// Duplicates the current line, inserting the copy below it.
    pub fn duplicate_line(&mut self, cx: &mut Context<Self>) {
        let cursor = self.cursor();
        let (row, _) = self.point_of(cursor);
        let line = self.line_str(row).to_string();
        let end = self.line_end_offset(row);

        let cursor_before = self.cursor_hint();
        self.replace_local(end..end, &format!("\n{line}"));
        self.record_edit_run(false, cursor_before, None);
        self.after_text_change();
        self.schedule_highlight(cx);
        self.selections.set_single_point(end + 1 + line.len());
        self.commit_local_edit(false, cursor_before, cx);
    }

    /// Deletes the current line including its trailing newline.
    pub fn delete_line(&mut self, cx: &mut Context<Self>) {
        let cursor = self.cursor();
        let (row, _) = self.point_of(cursor);
        let start = self.line_start_offset(row);
        let end = if row + 1 < self.line_count() {
            self.line_start_offset(row + 1)
        } else {
            self.line_end_offset(row)
        };

        let cursor_before = self.cursor_hint();
        if start < end && end <= self.text.len() {
            self.replace_local(start..end, "");
        }
        self.record_edit_run(false, cursor_before, None);
        self.after_text_change();
        self.schedule_highlight(cx);
        self.selections.set_single_point(start.min(self.text.len()));
        self.commit_local_edit(false, cursor_before, cx);
    }

    /// Deletes backward (Backspace): the selection, or the previous char.
    pub fn delete_backward(&mut self, cx: &mut Context<Self>) {
        let cursor_before = self.cursor_hint();
        if self.selections.has_selection() {
            self.delete_selection_local();
        } else {
            let mut selections: Vec<_> = self.selections.iter().copied().collect();
            selections.sort_by_key(|s| std::cmp::Reverse(s.head));
            for s in &mut selections {
                if s.head > 0 && s.head <= self.text.len() {
                    let prev_char_len = self
                        .text
                        .char_before(s.head)
                        .map(|c| c.len_utf8())
                        .unwrap_or(1);
                    let remove_start = s.head - prev_char_len;
                    self.replace_local(remove_start..s.head, "");
                    *s = crate::selection::Selection::point(s.id, remove_start);
                }
            }
            self.selections =
                Self::selections_from_points(selections.iter().map(|s| s.head).collect());
        }
        self.record_edit_run(false, cursor_before, None);
        self.after_text_change();
        self.schedule_highlight(cx);
        self.commit_local_edit(false, cursor_before, cx);
    }

    /// Deletes the word before the cursor (Ctrl+Backspace).
    pub fn delete_word_backward(&mut self, cx: &mut Context<Self>) {
        let cursor_before = self.cursor_hint();
        if self.selections.has_selection() {
            self.delete_selection_local();
        } else {
            let cursor = self.cursor();
            if cursor > 0 {
                let before = self.text.slice_owned(..cursor);
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
                    } else if seen_non_ws {
                        remove_start = idx + ch.len_utf8();
                        break;
                    } else {
                        remove_start = idx;
                        break;
                    }
                    remove_start = idx;
                }
                if remove_start < cursor {
                    self.replace_local(remove_start..cursor, "");
                    self.selections.set_single_point(remove_start);
                }
            }
        }
        self.record_edit_run(false, cursor_before, None);
        self.after_text_change();
        self.schedule_highlight(cx);
        self.commit_local_edit(false, cursor_before, cx);
    }

    /// Deletes forward (Delete key): the selection, or the next char.
    pub fn delete_forward(&mut self, cx: &mut Context<Self>) {
        let cursor_before = self.cursor_hint();
        if self.selections.has_selection() {
            self.delete_selection_local();
        } else {
            let mut selections: Vec<_> = self.selections.iter().copied().collect();
            selections.sort_by_key(|s| std::cmp::Reverse(s.head));
            for s in &mut selections {
                if s.head < self.text.len() {
                    let next_char_len = self
                        .text
                        .char_after(s.head)
                        .map(|c| c.len_utf8())
                        .unwrap_or(1);
                    self.replace_local(s.head..s.head + next_char_len, "");
                    *s = crate::selection::Selection::point(s.id, s.head);
                }
            }
            self.selections =
                Self::selections_from_points(selections.iter().map(|s| s.head).collect());
        }
        self.record_edit_run(false, cursor_before, None);
        self.after_text_change();
        self.schedule_highlight(cx);
        self.commit_local_edit(false, cursor_before, cx);
    }

    /// Deletes the word after the cursor (Ctrl+Delete).
    pub fn delete_word_forward(&mut self, cx: &mut Context<Self>) {
        let cursor_before = self.cursor_hint();
        if self.selections.has_selection() {
            self.delete_selection_local();
        } else {
            let cursor = self.cursor();
            if cursor < self.text.len() {
                let after = self.text.slice_owned(cursor..);
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
                    } else if seen_non_ws {
                        remove_end = cursor + idx;
                        break;
                    } else {
                        remove_end = cursor + idx + ch.len_utf8();
                        break;
                    }
                }
                if remove_end > cursor {
                    self.replace_local(cursor..remove_end, "");
                    self.selections.set_single_point(cursor);
                }
            }
        }
        self.record_edit_run(false, cursor_before, None);
        self.after_text_change();
        self.schedule_highlight(cx);
        self.commit_local_edit(false, cursor_before, cx);
    }

    // ── Contract-path operations (mutate + hand back the transaction) ─────

    /// Inserts text at every cursor and returns the resulting transaction
    /// without committing (the caller commits it).
    pub fn insert_text(
        &mut self,
        inserted: &str,
        cx: &mut Context<Self>,
    ) -> Option<EditTransaction> {
        let cursor_before = self.cursor_hint();
        let merge = self.merge_for_insert(self.cursor(), inserted);
        let insert_pos = self.insert_text_local(inserted);
        self.record_edit_run(merge, cursor_before, insert_pos);
        self.after_text_change();
        self.schedule_highlight(cx);
        cx.notify();
        self.take_transaction(merge, cursor_before)
    }

    /// Deletes the current selection(s) and returns the resulting
    /// transaction, or `None` when there is nothing to delete.
    pub fn delete_selection(&mut self, cx: &mut Context<Self>) -> Option<EditTransaction> {
        if !self.selections.has_selection() {
            return None;
        }
        let cursor_before = self.cursor_hint();
        self.delete_selection_local();
        self.record_edit_run(false, cursor_before, None);
        self.after_text_change();
        self.schedule_highlight(cx);
        cx.notify();
        self.take_transaction(false, cursor_before)
    }

    /// Selects the whole document; no text change, so no transaction.
    pub fn select_all(&mut self, cx: &mut Context<Self>) {
        self.reset_typing_run();
        self.selections.set_single_range(0, self.text.len());
        self.start_cursor_blink();
        cx.notify();
    }

    /// Replaces one byte range and returns the resulting transaction.
    pub fn replace_range(
        &mut self,
        range: Range<usize>,
        replacement: &str,
        cx: &mut Context<Self>,
    ) -> Option<EditTransaction> {
        if range.start > self.text.len() || range.end > self.text.len() {
            return None;
        }
        let cursor_before = self.cursor_hint();
        self.replace_local(range.clone(), replacement);
        self.record_edit_run(false, cursor_before, None);
        self.after_text_change();
        self.schedule_highlight(cx);
        self.selections
            .set_single_point(range.start + replacement.len());
        cx.notify();
        self.take_transaction(false, cursor_before)
    }

    /// Replaces several byte ranges at once (back to front) and returns the
    /// resulting transaction.
    pub fn replace_all_ranges(
        &mut self,
        mut replacements: Vec<(Range<usize>, String)>,
        cx: &mut Context<Self>,
    ) -> Option<EditTransaction> {
        let cursor_before = self.cursor_hint();
        replacements.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
        for (range, replacement) in &replacements {
            if range.start <= self.text.len() && range.end <= self.text.len() {
                self.replace_local(range.clone(), replacement);
            }
        }
        let caret = replacements
            .iter()
            .map(|(range, replacement)| range.start + replacement.len())
            .min()
            .unwrap_or(0);
        self.record_edit_run(false, cursor_before, None);
        self.after_text_change();
        self.schedule_highlight(cx);
        self.selections.set_single_point(caret.min(self.text.len()));
        cx.notify();
        self.take_transaction(false, cursor_before)
    }

    // ── Local mutation primitives (no commit, no derived-state rebuild) ───

    /// Inserts `inserted` at every cursor in place, collapsing selections.
    /// Returns the new caret offset for a single-cursor edit, or `None`
    /// when multiple cursors were collapsed.
    fn insert_text_local(&mut self, inserted: &str) -> Option<usize> {
        if self.selections.count() == 1 {
            let s = *self.selections.primary();
            let range = s.start().min(self.text.len())..s.end().min(self.text.len());
            self.replace_local(range.clone(), inserted);
            self.selections
                .set_single_point(range.start + inserted.len());
            Some(range.start + inserted.len())
        } else {
            let mut ordered: Vec<(usize, Range<usize>)> = self
                .selections
                .iter()
                .map(|s| {
                    (
                        s.id,
                        s.start().min(self.text.len())..s.end().min(self.text.len()),
                    )
                })
                .collect();
            ordered.sort_by_key(|(_, range)| std::cmp::Reverse(range.start));
            let mut heads: Vec<usize> = Vec::with_capacity(ordered.len());
            for (_, range) in &ordered {
                self.replace_local(range.clone(), inserted);
                heads.push(range.start + inserted.len());
            }
            self.selections = Self::selections_from_points(heads);
            None
        }
    }

    /// Deletes every non-empty selection in place, collapsing them to
    /// carets at their starts.
    pub(crate) fn delete_selection_local(&mut self) {
        let mut ordered: Vec<Range<usize>> = self
            .selections
            .iter()
            .filter(|s| !s.is_empty())
            .map(|s| s.start().min(self.text.len())..s.end().min(self.text.len()))
            .collect();
        ordered.sort_by_key(|range| std::cmp::Reverse(range.start));
        let mut heads: Vec<usize> = Vec::with_capacity(ordered.len());
        for range in &ordered {
            self.replace_local(range.clone(), "");
            heads.push(range.start);
        }
        self.selections = Self::selections_from_points(heads);
    }

    /// Rebuilds a selections collection from caret offsets, ascending.
    fn selections_from_points(mut points: Vec<usize>) -> Selections {
        points.sort_unstable();
        points.dedup();
        let mut selections = Selections::new(points[0]);
        for point in &points[1..] {
            selections.add_point(*point);
        }
        selections
    }
}
