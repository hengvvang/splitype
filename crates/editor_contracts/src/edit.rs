//! Edit transaction vocabulary shared by panes and the document buffer.
//!
//! Every pane-produced text change travels as an [`EditTransaction`]: the
//! full new document text plus the bookkeeping the shared buffer needs to
//! maintain a single, document-level undo history. Undo/redo therefore
//! lives in the document buffer (the single source of truth), never in an
//! individual pane view — switching pane kinds or editors never loses
//! history, and an undo performed anywhere is observed everywhere.

/// A 1-based cursor position in the document, used to restore the caret
/// after undo/redo. Mirrors the [`crate::PaneView::cursor_position`]
/// convention.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CursorHint {
    /// 1-based line index.
    pub line: u32,
    /// 1-based column index (character count within the line).
    pub column: u32,
}

impl CursorHint {
    pub fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }

    /// Computes the hint of a byte offset in `text` (1-based line and
    /// character column).
    pub fn from_offset(text: &str, offset: usize) -> Self {
        let offset = offset.min(text.len());
        let before = &text[..offset];
        let line = before.matches('\n').count() as u32 + 1;
        let column = before
            .rsplit('\n')
            .next()
            .map(|line| line.chars().count())
            .unwrap_or(0) as u32
            + 1;
        Self { line, column }
    }

    /// Converts the hint back to a byte offset in `text`, clamping to the
    /// document end and to the line's extent.
    pub fn to_offset(&self, text: &str) -> usize {
        let target_line = self.line.saturating_sub(1) as usize;
        let mut line_start = 0usize;
        let mut current_line = 0usize;
        if target_line > 0 {
            for (idx, ch) in text.char_indices() {
                if ch == '\n' {
                    current_line += 1;
                    if current_line == target_line {
                        line_start = idx + 1;
                        break;
                    }
                }
            }
            if current_line < target_line {
                return text.len();
            }
        }
        let line = &text[line_start..];
        let line_len = line.find('\n').unwrap_or(line.len());
        let column_chars = self.column.saturating_sub(1) as usize;
        let mut byte_col = 0usize;
        for (chars_seen, (idx, ch)) in line.char_indices().enumerate() {
            if idx >= line_len || chars_seen >= column_chars {
                break;
            }
            byte_col = idx + ch.len_utf8();
        }
        (line_start + byte_col).min(text.len())
    }
}

/// One pane-produced document edit.
///
/// `text` is the complete document after the edit. `merge` marks
/// continuation of the previous undo transaction (e.g. subsequent
/// characters of one typing run), so the shared buffer can group them into
/// a single undo step. The cursor hints anchor the caret for undo (back to
/// `cursor_before`) and redo (forward to `cursor_after`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditTransaction {
    pub text: String,
    pub merge: bool,
    pub cursor_before: CursorHint,
    pub cursor_after: CursorHint,
}

impl EditTransaction {
    pub fn new(
        text: impl Into<String>,
        merge: bool,
        cursor_before: CursorHint,
        cursor_after: CursorHint,
    ) -> Self {
        Self {
            text: text.into(),
            merge,
            cursor_before,
            cursor_after,
        }
    }

    /// A single, self-contained edit at `cursor` (for command-driven
    /// edits like paste, cut, or a one-shot replacement).
    pub fn one_shot(text: impl Into<String>, cursor: CursorHint) -> Self {
        Self::new(text, false, cursor, cursor)
    }
}
