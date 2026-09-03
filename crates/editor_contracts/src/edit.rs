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
    /// character column). If `offset` falls inside a multi-byte UTF-8
    /// character sequence, it is clamped down to the nearest character boundary.
    pub fn from_offset(text: &str, mut offset: usize) -> Self {
        offset = offset.min(text.len());
        while !text.is_char_boundary(offset) {
            offset -= 1;
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_offset_clamps_to_char_boundary_safely() {
        let text = "a内容b";
        // '内' is at bytes 1..4 (1, 2, 3)
        // '容' is at bytes 4..7 (4, 5, 6)
        assert_eq!(CursorHint::from_offset(text, 0), CursorHint::new(1, 1));
        assert_eq!(CursorHint::from_offset(text, 1), CursorHint::new(1, 2));
        // Offset 2 and 3 are inside '内', should floor to byte 1
        assert_eq!(CursorHint::from_offset(text, 2), CursorHint::new(1, 2));
        assert_eq!(CursorHint::from_offset(text, 3), CursorHint::new(1, 2));
        // Offset 4 is start of '容'
        assert_eq!(CursorHint::from_offset(text, 4), CursorHint::new(1, 3));
        // Offset 5 and 6 are inside '容', should floor to byte 4
        assert_eq!(CursorHint::from_offset(text, 5), CursorHint::new(1, 3));
        assert_eq!(CursorHint::from_offset(text, 6), CursorHint::new(1, 3));
        // Offset 7 is 'b'
        assert_eq!(CursorHint::from_offset(text, 7), CursorHint::new(1, 4));
        // Offset past end
        assert_eq!(CursorHint::from_offset(text, 100), CursorHint::new(1, 5));
    }

    #[test]
    fn cursor_hint_round_trip() {
        let text = "# 第一行内容\n- 第二行列表\n> 第三行引用";
        let offset = CursorHint::new(2, 4).to_offset(text);
        assert!(text.is_char_boundary(offset));
        let hint = CursorHint::from_offset(text, offset);
        assert_eq!(hint, CursorHint::new(2, 4));
    }
}

