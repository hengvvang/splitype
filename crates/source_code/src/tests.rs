//! Unit tests for source_code display maps and selections.
//!
//! The editor entity itself is exercised through the application; these
//! tests cover the pure data structures underneath it. Rope line indexing
//! is covered by the tests in `text.rs`.

use crate::Rope;
use crate::display_map::{DisplayPoint, DisplaySnapshot, FoldMap, FoldRange, TabMap, WrapState};
use crate::selection::Selections;

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
    let rope = Rope::new(text);
    let folds = FoldMap::discover_markdown_folds(&rope);
    assert_eq!(folds.len(), 2);
    assert!(folds.iter().any(|f| f.start_row == 0 && f.end_row == 2));
}

#[test]
fn row_index_flattens_folds_to_visible_rows() {
    let rope = Rope::new("# A\nb\nc\n# B\nd\ne");
    let mut folds = FoldMap::new();
    folds.fold(FoldRange::new(0, 2)); // hides rows 1-2
    let wrap = WrapState::default();

    let snapshot = DisplaySnapshot::build(&rope, TabMap::new(4), &folds, &wrap);
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
    let rope = Rope::new("abcdefghijkl\nshort");
    let mut points = vec![Vec::new(); 2];
    points[0] = vec![10];
    let wrap = WrapState::new(100.0, points);
    let folds = FoldMap::new();

    let snapshot = DisplaySnapshot::build(&rope, TabMap::new(4), &folds, &wrap);
    assert_eq!(snapshot.visible_line_count(), 3);

    // Offset 12 ("l" on the first buffer line) lives on display row 1.
    let dp = snapshot.offset_to_display_point(12);
    assert_eq!(dp, DisplayPoint::new(1, 2));
    assert_eq!(snapshot.display_point_to_offset(dp), 12);

    // Offset 14 is the second buffer line, display row 2.
    let dp = snapshot.offset_to_display_point(14);
    assert_eq!(dp.row, 2);
}

#[cfg(test)]
mod bench {
    use std::time::Instant;

    use crate::Rope;
    use crate::display_map::{DisplaySnapshot, FoldMap, TabMap, WrapState};

    fn frame_cost(name: &str, size_kb: usize) {
        let text = "# Heading\n\nA paragraph of markdown text.\n".repeat(size_kb * 1024 / 40);
        let rope = Rope::new(&text);
        let frames = 60;

        // Per-frame display snapshot build: the row-index walk over all
        // lines. In the editor this is cached per text version; the cost
        // is only paid when the cache is invalidated.
        let folds = FoldMap::new();
        let wrap = WrapState::default();
        let start = Instant::now();
        for _ in 0..frames {
            std::hint::black_box(DisplaySnapshot::build(&rope, TabMap::new(4), &folds, &wrap));
        }
        let snapshot_build = start.elapsed();

        // Per-keystroke rope edit (persistent, O(chunks)) — what a single
        // typed character now costs on the text layer.
        let start = Instant::now();
        for _ in 0..frames {
            std::hint::black_box(rope.edit(0..0, "x"));
        }
        let rope_edit = start.elapsed();

        // Background highlight re-derivation: runs on the background
        // executor after an idle debounce, never on the UI thread.
        let start = Instant::now();
        for _ in 0..frames {
            std::hint::black_box(syntax_highlighter::highlight::highlight_code_block(
                Some("markdown"),
                &text,
            ));
        }
        let highlight = start.elapsed();

        println!(
            "bench_source_code_invalidate[{name}]: {size_kb}KB x{frames} edits: snapshot={:?} ({}us), rope_edit={:?} ({}us), background_highlight={:?} ({}us)",
            snapshot_build,
            snapshot_build.as_micros() / frames as u128,
            rope_edit,
            rope_edit.as_micros() / frames as u128,
            highlight,
            highlight.as_micros() / frames as u128,
        );
    }

    /// Per-invalidation work in `SourceCodeEditor`: the display row-index
    /// rebuild, one rope edit, and the background highlight re-derivation.
    /// The first two are cached per text version / per chunk; the highlight
    /// never runs on the UI thread. This measures the invalidation cost.
    #[test]
    #[ignore = "perf benchmark"]
    fn bench_source_code_frame() {
        frame_cost("64KB", 64);
        frame_cost("1MB", 1024);
    }
}
