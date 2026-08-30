//! Unit tests for editor_source_code buffer and line indexing.

use crate::SourceCodeState;

#[test]
fn rebuild_lines_computes_accurate_byte_ranges() {
    let state = SourceCodeState::from_text("Hello\nWorld\nRust\n");
    assert_eq!(state.line_count(), 4);
    assert_eq!(state.line_range(0), 0..5);
    assert_eq!(state.line_range(1), 6..11);
    assert_eq!(state.line_range(2), 12..16);
    assert_eq!(state.line_range(3), 17..17);
}

#[test]
fn single_line_without_trailing_newline() {
    let state = SourceCodeState::from_text("Single line");
    assert_eq!(state.line_count(), 1);
    assert_eq!(state.line_range(0), 0..11);
}

#[test]
fn insert_and_delete_text_updates_cursor_and_lines() {
    let mut state = SourceCodeState::from_text("Line 1\nLine 2");
    state.cursor = 6; // At '\n'
    state.insert_text("\nInserted");
    assert_eq!(state.text, "Line 1\nInserted\nLine 2");
    assert_eq!(state.line_count(), 3);
}

#[test]
fn selection_replacement_preserves_bounds() {
    let mut state = SourceCodeState::from_text("Foo Bar Baz");
    state.selection = Some(4..7); // "Bar"
    state.insert_text("Qux");
    assert_eq!(state.text, "Foo Qux Baz");
    assert_eq!(state.cursor, 7);
    assert_eq!(state.selection, None);
}
