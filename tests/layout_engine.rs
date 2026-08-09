//! Cross-crate contract tests for the tiled layout engine.
//!
//! `splitype-layout` owns only the outer window tree; the inner panel
//! layout moved to the editor layer and is covered by module tests there.
//! These tests drive the outer tree through its public API as an external
//! consumer would: split areas, close them, switch kinds, and verify the
//! resulting rectangles.

use gpui::{Size, px};

use splitype_layout::Axis;
use splitype_layout::state::{ROOT_AREA_ID, WindowLayout};
use splitype_layout::tree::AreaRect;
use splitype_layout::types::WindowAreaKind;

fn layout() -> WindowLayout {
    WindowLayout::default()
}

fn root_rects(state: &WindowLayout) -> Vec<AreaRect> {
    let mut rects = Vec::new();
    state
        .window_area_tree
        .collect_leaf_rects(0.0, 0.0, 100.0, 100.0, &mut rects);
    rects
}

/// The default layout has exactly one root editor area.
#[test]
fn default_layout_has_single_root_area() {
    let state = layout();
    let mut out = Vec::new();
    state.window_area_tree.leaf_ids(&mut out);
    assert_eq!(out, vec![ROOT_AREA_ID]);
}

/// Splitting an area doubles the leaf count and halves the rects.
#[test]
fn split_window_area_creates_two_leaves() {
    let mut state = layout();
    let new_id = state
        .split_window_area(ROOT_AREA_ID, Axis::Horizontal, 0.5)
        .expect("split");

    let mut leaves = Vec::new();
    state.window_area_tree.leaf_ids(&mut leaves);
    assert_eq!(leaves.len(), 2);

    let rects = root_rects(&state);
    assert_eq!(rects.len(), 2);
    assert_eq!(rects[0].width, rects[1].width);
    assert_eq!(rects[0].x, 0.0);
    let right = rects
        .iter()
        .find(|rect| rect.id == new_id)
        .expect("new area");
    assert_eq!(right.x, 50.0);
}

/// Closing an area joins the tree back to a single leaf.
#[test]
fn closing_area_joins_back_to_single_leaf() {
    let mut state = layout();
    let new_id = state
        .split_window_area(ROOT_AREA_ID, Axis::Horizontal, 0.5)
        .expect("split");

    state.close_window_area(new_id);
    let mut leaves = Vec::new();
    state.window_area_tree.leaf_ids(&mut leaves);
    assert_eq!(leaves, vec![ROOT_AREA_ID]);
}

/// Area kinds switch on the tree leaves.
#[test]
fn area_kinds_switch_on_leaves() {
    let mut state = layout();
    state.change_window_area_kind(ROOT_AREA_ID, WindowAreaKind::Explorer);
    assert_eq!(
        state.window_area_tree.find_leaf_kind(ROOT_AREA_ID),
        Some(WindowAreaKind::Explorer)
    );
    state.change_window_area_kind(ROOT_AREA_ID, WindowAreaKind::Editor);
    assert_eq!(
        state.window_area_tree.find_leaf_kind(ROOT_AREA_ID),
        Some(WindowAreaKind::Editor)
    );
}

/// Split ratio changes are reflected in the rectangles.
#[test]
fn split_ratio_affects_rect_widths() {
    let mut state = layout();
    state.split_window_area(ROOT_AREA_ID, Axis::Horizontal, 0.25);

    let rects = root_rects(&state);
    assert_eq!(rects.len(), 2);
    let left = rects
        .iter()
        .find(|rect| rect.id == ROOT_AREA_ID)
        .expect("left rect");
    assert!((left.width - 25.0).abs() < 0.001);
}

/// Window area rects scale to the container size.
#[test]
fn window_area_rects_scale_to_container() {
    let mut state = layout();
    state.split_window_area(ROOT_AREA_ID, Axis::Horizontal, 0.5);

    let rects = state.window_area_rects(Size::new(px(1000.0), px(800.0)));
    assert_eq!(rects.len(), 2);
    assert_eq!(rects[0].width, 500.0);
    assert_eq!(rects[1].x, 500.0);
    assert_eq!(rects[1].height, 800.0);
}

/// Maximizing tracks a single area and can be toggled off.
#[test]
fn maximize_toggles_single_area() {
    let mut state = layout();
    let new_id = state
        .split_window_area(ROOT_AREA_ID, Axis::Horizontal, 0.5)
        .expect("split");

    state.toggle_window_area_maximize(new_id);
    assert_eq!(state.maximized_window_area, Some(new_id));
    state.toggle_window_area_maximize(new_id);
    assert_eq!(state.maximized_window_area, None);
}

/// Activation history records the most recent editor area.
#[test]
fn activation_history_tracks_editors() {
    let mut state = layout();
    let new_id = state
        .split_window_area(ROOT_AREA_ID, Axis::Horizontal, 0.5)
        .expect("split");

    state.activate_editor_area(new_id);
    assert_eq!(state.active_editor_area, Some(new_id));
    assert_eq!(state.editor_activation_history, vec![new_id]);
}
