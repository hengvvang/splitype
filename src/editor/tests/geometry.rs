//! Pure layout geometry: centered-column ratio, scrollbar
//! mapping, and the visible row band.

use crate::editor::controller::Editor;
use crate::infra::theme::Theme;

use super::*;

#[test]
fn centered_column_ratio_stays_full_before_shrink_start() {
    let theme = Theme::default_theme();
    assert_eq!(Editor::centered_column_ratio(900.0, &theme.dimensions), 1.0);
    assert_eq!(
        Editor::centered_column_ratio(theme.dimensions.centered_shrink_start, &theme.dimensions),
        1.0
    );
}

#[test]
fn centered_column_ratio_reaches_new_minimum() {
    let theme = Theme::default_theme();
    let ratio =
        Editor::centered_column_ratio(theme.dimensions.centered_shrink_end, &theme.dimensions);
    assert!((ratio - 0.58).abs() < f32::EPSILON);
}

#[test]
fn scrollbar_geometry_and_inverse_mapping_stay_aligned() {
    let geometry = Editor::scrollbar_geometry(400.0, 600.0, 300.0);
    assert_eq!(geometry.track_height, 400.0);
    assert!(geometry.thumb_height >= 28.0);
    assert!((geometry.thumb_top - (400.0 - geometry.thumb_height) * 0.5).abs() < 0.001);

    let scroll_y = Editor::scroll_offset_for_thumb_top(
        geometry.thumb_top,
        geometry.track_height,
        geometry.thumb_height,
        geometry.max_scroll_y,
    );
    assert!((scroll_y - 300.0).abs() < 0.001);
}

#[test]
fn scrollbar_offset_mapping_clamps_to_track_bounds() {
    let geometry = Editor::scrollbar_geometry(300.0, 450.0, 0.0);
    assert_eq!(
        Editor::scroll_offset_for_thumb_top(
            -25.0,
            geometry.track_height,
            geometry.thumb_height,
            geometry.max_scroll_y,
        ),
        0.0
    );
    assert_eq!(
        Editor::scroll_offset_for_thumb_top(
            999.0,
            geometry.track_height,
            geometry.thumb_height,
            geometry.max_scroll_y,
        ),
        geometry.max_scroll_y
    );
}

/// Equal-height rows as per-row footprints, the input `visible_row_band` takes.

#[test]
fn rendered_window_culls_offscreen_rows() {
    // 100 rows of 50px (total 5000). Scroll 2000, viewport 400 -> band [2000, 2400].
    let strides = uniform_strides(100, 50.0);
    let band = Editor::visible_row_band(&strides, 2000.0, 400.0, 0.0, None);

    // Row i spans [50i, 50i+50). bottom>=2000 -> i>=39; top<=2400 -> i<=48.
    assert_eq!(band.run_start, 39);
    assert_eq!(band.run_end, 49);
    assert!((band.top_h - 1950.0).abs() < 0.01);
    assert!((band.bottom_h - 2550.0).abs() < 0.01);
}

#[test]
fn rendered_window_keeps_focus_row_mounted() {
    let strides = uniform_strides(100, 50.0);
    // Viewport at the top, caret parked far below at row 80.
    let band = Editor::visible_row_band(&strides, 0.0, 400.0, 0.0, Some(80));

    assert_eq!(band.run_start, 0);
    assert_eq!(band.run_end, 81);
}

#[test]
fn rendered_window_tracks_current_scroll_offset() {
    // Scrolling by one row's height shifts the mounted run by exactly one row.
    let strides = uniform_strides(100, 50.0);

    let low = Editor::visible_row_band(&strides, 2000.0, 400.0, 0.0, None);
    let high = Editor::visible_row_band(&strides, 2050.0, 400.0, 0.0, None);

    assert_eq!(low.run_start, 39);
    assert_eq!(low.run_end, 49);
    assert_eq!(high.run_start, low.run_start + 1);
    assert_eq!(high.run_end, low.run_end + 1);
}

#[test]
fn rendered_window_has_no_spacer_at_document_edges() {
    let strides = uniform_strides(50, 40.0); // total 2000

    let at_top = Editor::visible_row_band(&strides, 0.0, 400.0, 0.0, None);
    assert_eq!(at_top.run_start, 0);
    assert_eq!(at_top.top_h, 0.0);
    assert!(at_top.bottom_h > 0.0);

    let at_bottom = Editor::visible_row_band(&strides, 1600.0, 400.0, 0.0, None);
    assert_eq!(at_bottom.run_end, 50);
    assert_eq!(at_bottom.bottom_h, 0.0);
    assert!(at_bottom.top_h > 0.0);
}

#[test]
fn rendered_window_preserves_total_height() {
    let strides = uniform_strides(200, 37.0);
    let total: f32 = strides.iter().sum();

    for &(scroll_y, viewport_height, focus) in &[
        (0.0f32, 500.0f32, None),
        (3000.0, 500.0, None),
        (37.0 * 150.0, 37.0 * 5.0, Some(10usize)),
    ] {
        let band = Editor::visible_row_band(&strides, scroll_y, viewport_height, 200.0, focus);
        let rendered: f32 = strides[band.run_start..band.run_end].iter().sum();
        assert!(
            (band.top_h + rendered + band.bottom_h - total).abs() < 0.01,
            "height invariant broken at scroll {scroll_y}"
        );
    }
}

#[test]
fn rendered_window_estimated_row_keeps_culling_active() {
    // Row 60 is an estimated (unmeasured) row; it must not disable culling.
    let mut strides = uniform_strides(100, 50.0);
    strides[60] = 20.0;

    let band = Editor::visible_row_band(&strides, 0.0, 400.0, 0.0, None);
    assert_eq!(band.run_start, 0);
    assert!(
        band.run_end < strides.len(),
        "a single estimated row must not disable culling"
    );
}

#[test]
fn rendered_window_all_estimated_windows_near_top() {
    // Cold start: all rows estimated. At the top the window still covers the
    // first rows, so the viewport is never blank while heights are learned.
    let strides = uniform_strides(500, 20.0);

    let band = Editor::visible_row_band(&strides, 0.0, 400.0, 0.0, None);
    assert_eq!(band.run_start, 0);
    assert!(band.run_end < strides.len());
    // A viewport-plus-band worth of rows, not the whole document.
    assert!(band.run_end >= 20);
}

#[test]
fn rendered_window_scrolled_past_bottom_maintains_stable_bottom_cluster() {
    // 50 rows of 40px (total 2000px). Viewport 600px, overdraw 200px.
    // User scrolls into bottom padding: scroll_y = 2500px.
    let strides = uniform_strides(50, 40.0);
    let total: f32 = strides.iter().sum();
    let band = Editor::visible_row_band(&strides, 2500.0, 600.0, 200.0, None);

    assert_eq!(band.run_end, 50);
    // It should mount a full viewport of rows ending at 50, not just 1 row!
    assert!(band.run_end - band.run_start >= 15);
    assert_eq!(band.bottom_h, 0.0);
    let rendered: f32 = strides[band.run_start..band.run_end].iter().sum();
    assert!(
        (band.top_h + rendered + band.bottom_h - total).abs() < 0.01,
        "height invariant broken at bottom overscroll"
    );
}
