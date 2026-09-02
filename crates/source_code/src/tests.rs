//! Unit tests for source_code coordinates, selections, and display maps.
//!
//! The editor entity itself is exercised through the application; these
//! tests cover the pure data structures underneath it.

use crate::buffer::{BufferPoint, LineMap};
use crate::display_map::{DisplayPoint, DisplaySnapshot, FoldMap, FoldRange, TabMap, WrapState};
use crate::selection::Selections;

#[test]
fn line_map_indexes_lines_and_offsets() {
    let map = LineMap::new("ab\ncd\nef");
    assert_eq!(map.line_count(), 3);
    assert_eq!(map.line_start(0), 0);
    assert_eq!(map.line_start(1), 3);
    assert_eq!(map.line_len(0), 2);
    assert_eq!(map.offset_to_point(4), BufferPoint::new(1, 1));
    assert_eq!(map.point_to_offset(BufferPoint::new(1, 1)), 4);
}

#[test]
fn line_map_trailing_newline_and_empty() {
    assert_eq!(LineMap::new("").line_count(), 1);
    let map = LineMap::new("ab\n");
    assert_eq!(map.line_count(), 1);
    assert_eq!(map.line_len(0), 2);
    assert_eq!(map.offset_to_point(3), BufferPoint::new(0, 2));
}

#[test]
fn multibyte_utf8_chinese_offset_conversions() {
    let text = "你好，世界。\n这是一个测试。";
    let map = LineMap::new(text);

    // Any byte offset, even inside a multibyte character, should never panic
    for byte_offset in 0..=text.len() {
        let point = map.offset_to_point(byte_offset);
        let recovered_offset = map.point_to_offset(point);
        assert!(recovered_offset <= text.len());
    }
}

#[test]
fn tab_map_expansion() {
    let tab_map = TabMap::new(4);
    let line = "\tlet x = 1;";
    let expanded = tab_map.expand_tabs(line);
    assert_eq!(expanded, "    let x = 1;");
    assert_eq!(tab_map.char_column_to_display_column(line, 1), 4);
    assert_eq!(tab_map.display_column_to_char_column(line, 4), 1);
}

#[test]
fn selections_apply_edit_biases() {
    let mut selections = Selections::new(2);
    selections.apply_edit(2..2, 1);
    // Anchors keep left bias, heads keep right bias: typing at the caret
    // keeps the anchor fixed and moves the head past the inserted text.
    assert_eq!(selections.cursor(), 3);
    assert_eq!(selections.primary().anchor, 2);
}

#[test]
fn selections_collapse_and_dedupe() {
    let mut selections = Selections::new(0);
    selections.set_single_range(4, 9);
    selections.add_point(9);
    selections.add_point(9); // exact duplicate
    selections.clamp_and_sort(20);
    assert_eq!(selections.count(), 2);
    selections.collapse_all();
    assert!(selections.iter().all(|s| s.is_empty()));
}

#[test]
fn fold_map_discovers_markdown_regions() {
    let text = "# Section 1\nBody 1\nBody 2\n# Section 2\nBody 3";
    let line_map = LineMap::new(text);
    let folds = FoldMap::discover_markdown_folds(text, &line_map);
    assert_eq!(folds.len(), 2);
    assert!(folds.iter().any(|f| f.start_row == 0 && f.end_row == 2));
}

#[test]
fn row_index_flattens_folds_to_visible_rows() {
    let text = "# A\nb\nc\n# B\nd\ne";
    let line_map = LineMap::new(text);
    let mut folds = FoldMap::new();
    folds.fold(FoldRange::new(0, 2)); // hides rows 1-2
    let wrap = WrapState::default();

    let snapshot = DisplaySnapshot::new(text, &line_map, TabMap::new(4), &folds, &wrap);
    assert_eq!(snapshot.visible_line_count(), 4); // header + 3 remaining
    assert_eq!(snapshot.rows.buffer_row_at(0), 0);
    // Display rows 1-3 are the remaining buffer rows 3, 4, 5.
    assert_eq!(snapshot.rows.buffer_row_at(1), 3);
    assert_eq!(snapshot.rows.buffer_row_at(2), 4);
    assert_eq!(snapshot.rows.buffer_row_at(3), 5);
}

#[test]
fn snapshot_maps_offsets_through_wraps() {
    // A 10-column wrap splits the long first line into two visual rows.
    let text = "abcdefghijkl\nshort";
    let line_map = LineMap::new(text);
    let mut points = vec![Vec::new(); 2];
    points[0] = vec![10];
    let wrap = WrapState::new(100.0, points);
    let folds = FoldMap::new();

    let snapshot = DisplaySnapshot::new(text, &line_map, TabMap::new(4), &folds, &wrap);
    assert_eq!(snapshot.visible_line_count(), 3);

    // Offset 12 ("l" on the first buffer line) lives on display row 1.
    let dp = snapshot.offset_to_display_point(12);
    assert_eq!(dp, DisplayPoint::new(1, 2));
    assert_eq!(snapshot.display_point_to_offset(dp), 12);

    // Offset 14 is the second buffer line, display row 2.
    let dp = snapshot.offset_to_display_point(14);
    assert_eq!(dp.row, 2);
}
