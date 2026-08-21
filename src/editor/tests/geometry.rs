//! Pure layout geometry: centered-column ratio, scrollbar
//! mapping, and the visible row band.

use crate::editor::controller::Editor;
use crate::infra::theme::Theme;

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

#[test]
fn centered_column_width_calculation() {
    let theme = Theme::default_theme();
    let width_small = Editor::centered_column_width(600.0, &theme.dimensions);
    assert_eq!(width_small, 600.0 - theme.dimensions.editor_padding * 2.0);

    let width_large = Editor::centered_column_width(1200.0, &theme.dimensions);
    assert!(width_large < 1200.0);
    assert!(width_large > 0.0);
}
