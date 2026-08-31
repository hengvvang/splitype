//! Unit tests for editor_source_code buffer, coordinates, selections, and display maps.

use crate::buffer::{BufferPoint, LineMap};
use crate::display_map::{FoldMap, FoldRange, TabMap};
use crate::selection::SelectionsCollection;
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
fn buffer_point_offset_conversions() {
    let text = "Hello\nWorld\nRust";
    let map = LineMap::new(text);

    let p0 = map.offset_to_point(text, 0);
    assert_eq!(p0, BufferPoint::new(0, 0));

    let p_world = map.offset_to_point(text, 6);
    assert_eq!(p_world, BufferPoint::new(1, 0));

    let off_world = map.point_to_offset(text, BufferPoint::new(1, 2));
    assert_eq!(off_world, 8); // 'r' in "World"
}

#[test]
fn multi_cursor_collection_normalization() {
    let mut sel = SelectionsCollection::new();
    sel.set_single_range(10, 15);
    sel.add_selection(20, 25);
    sel.add_selection(12, 18); // overlaps with 10..15

    assert_eq!(sel.count(), 2);
    assert_eq!(sel.all()[0].range_bounds(), 10..18);
    assert_eq!(sel.all()[1].range_bounds(), 20..25);
}

#[test]
fn tab_map_expansion() {
    let tab_map = TabMap::new(4);
    let line = "\tlet x = 1;";
    let expanded = tab_map.expand_tabs(line);
    assert_eq!(expanded, "    let x = 1;");
    assert_eq!(tab_map.char_column_to_display_column(line, 1), 4);
}

#[test]
fn fold_map_markdown_discovery_and_projection() {
    let text = "# Section 1\nBody 1\nBody 2\n# Section 2\nBody 3";
    let line_map = LineMap::new(text);
    let folds = FoldMap::discover_markdown_folds(text, &line_map);
    assert_eq!(folds.len(), 2);

    let mut fold_map = FoldMap::new();
    fold_map.fold(FoldRange { start_row: 0, end_row: 2 });

    assert_eq!(fold_map.visible_line_count(5), 3);
    assert_eq!(fold_map.buffer_row_to_visible_row(0), 0);
    assert_eq!(fold_map.buffer_row_to_visible_row(3), 1);
}

#[test]
fn insert_and_delete_text_updates_cursor_and_lines() {
    let mut state = SourceCodeState::from_text("Line 1\nLine 2");
    state.selections.set_single_point(6); // At '\n'
    state.insert_text("\nInserted");
    assert_eq!(state.text, "Line 1\nInserted\nLine 2");
    assert_eq!(state.line_count(), 3);
}

#[test]
fn selection_replacement_preserves_bounds() {
    let mut state = SourceCodeState::from_text("Foo Bar Baz");
    state.selections.set_single_range(4, 7); // "Bar"
    state.insert_text("Qux");
    assert_eq!(state.text, "Foo Qux Baz");
    assert_eq!(state.cursor(), 7);
    assert_eq!(state.selected_text(), None);
}

#[test]
fn indent_and_outdent() {
    let mut state = SourceCodeState::from_text("fn main() {\nprintln!();\n}");
    state.selections.set_single_point(12); // inside line 1
    state.indent();
    assert_eq!(state.text, "fn main() {\n    println!();\n}");
    state.outdent();
    assert_eq!(state.text, "fn main() {\nprintln!();\n}");
}

#[test]
fn multibyte_utf8_chinese_offset_conversions() {
    let text = "你好，世界。\n这是一个测试。";
    let map = LineMap::new(text);

    // Any byte offset, even inside a multibyte character, should never panic
    for byte_offset in 0..=text.len() {
        let point = map.offset_to_point(text, byte_offset);
        let recovered_offset = map.point_to_offset(text, point);
        assert!(recovered_offset <= text.len());
    }

    let mut state = SourceCodeState::from_text(text);
    // Move cursor and insert text
    state.move_to(6, false); // inside first line
    state.insert_text("Rust");
    assert_eq!(state.text, "你好Rust，世界。\n这是一个测试。");
}
