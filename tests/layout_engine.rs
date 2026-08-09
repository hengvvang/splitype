//! Cross-crate contract tests for the tiled layout engine.
//!
//! `splitype-layout` is a pure geometry engine; these tests drive it through
//! its public API as an external consumer would: split areas, close them,
//! manage inner panels, and verify the resulting rectangles.

use gpui::{Size, px};

use splitype_layout::Axis;
use splitype_layout::state::{ROOT_AREA_ID, WindowLayout};
use splitype_layout::tree::AreaRect;
use splitype_layout::types::{AreaSplitMode, EditorAreaMode, WindowAreaKind};

fn layout() -> WindowLayout<()> {
    WindowLayout::<()>::default()
}

fn root_rects(state: &WindowLayout<()>) -> Vec<AreaRect> {
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
        .split_window_area(ROOT_AREA_ID, Axis::Horizontal, 0.5, AreaSplitMode::Copy)
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
        .split_window_area(ROOT_AREA_ID, Axis::Horizontal, 0.5, AreaSplitMode::Fresh)
        .expect("split");

    state.close_window_area(new_id);
    let mut leaves = Vec::new();
    state.window_area_tree.leaf_ids(&mut leaves);
    assert_eq!(leaves, vec![ROOT_AREA_ID]);
}

/// Editor sessions are created lazily per area and keep their panel tree.
#[test]
fn editor_session_panels_split_and_close() {
    let mut state = layout();
    state.enter_editing(ROOT_AREA_ID);

    let session = state.ensure_editor_session(ROOT_AREA_ID);
    let mut panel_ids = Vec::new();
    session.inner_panel_tree.leaf_ids(&mut panel_ids);
    assert_eq!(panel_ids.len(), 1);

    let panel = panel_ids[0];
    state.split_editor_inner_panel(ROOT_AREA_ID, panel, Axis::Vertical);
    let mut panel_ids = Vec::new();
    state
        .ensure_editor_session(ROOT_AREA_ID)
        .inner_panel_tree
        .leaf_ids(&mut panel_ids);
    assert_eq!(panel_ids.len(), 2);

    state.close_editor_inner_panel(ROOT_AREA_ID, panel_ids[1]);
    let mut panel_ids = Vec::new();
    state
        .ensure_editor_session(ROOT_AREA_ID)
        .inner_panel_tree
        .leaf_ids(&mut panel_ids);
    assert_eq!(panel_ids.len(), 1);
}

/// Inner panel rectangles tile the area without overlap.
#[test]
fn inner_panel_rects_tile_the_area() {
    let mut state = layout();
    state.enter_editing(ROOT_AREA_ID);
    let mut panel_ids = Vec::new();
    state
        .ensure_editor_session(ROOT_AREA_ID)
        .inner_panel_tree
        .leaf_ids(&mut panel_ids);
    state.split_editor_inner_panel(ROOT_AREA_ID, panel_ids[0], Axis::Horizontal);

    let rects = state.editor_inner_panel_rects(ROOT_AREA_ID, Size::new(px(100.0), px(80.0)));
    assert_eq!(rects.len(), 2);
    let total_width: f32 = rects.iter().map(|rect| rect.width).sum();
    assert!((total_width - 100.0).abs() < 0.001);
    assert!(rects.iter().all(|rect| rect.height == 80.0));
}

/// Area mode reflects whether an editor session exists.
#[test]
fn area_mode_tracks_editor_sessions() {
    let mut state = layout();
    assert_eq!(
        state.editor_area_mode(ROOT_AREA_ID),
        EditorAreaMode::Welcome
    );

    state
        .ensure_editor_session(ROOT_AREA_ID)
        .tab_list
        .tabs
        .push(());
    assert_eq!(
        state.editor_area_mode(ROOT_AREA_ID),
        EditorAreaMode::Editing
    );
}

/// Area kinds switch without losing the session.
#[test]
fn area_kind_switches_are_tracked() {
    let mut state = layout();
    state.change_window_area_kind(ROOT_AREA_ID, WindowAreaKind::Explorer);
    state.change_window_area_kind(ROOT_AREA_ID, WindowAreaKind::Editor);
    state
        .ensure_editor_session(ROOT_AREA_ID)
        .tab_list
        .tabs
        .push(());
    assert_eq!(
        state.editor_area_mode(ROOT_AREA_ID),
        EditorAreaMode::Editing
    );
}

/// Split ratio changes are reflected in the rectangles.
#[test]
fn split_ratio_affects_rect_widths() {
    let mut state = layout();
    state.split_window_area(ROOT_AREA_ID, Axis::Horizontal, 0.25, AreaSplitMode::Copy);

    let rects = root_rects(&state);
    assert_eq!(rects.len(), 2);
    let left = rects
        .iter()
        .find(|rect| rect.id == ROOT_AREA_ID)
        .expect("left rect");
    assert!((left.width - 25.0).abs() < 0.001);
}

/// Generic payloads pass through the session storage untouched.
#[test]
fn sessions_store_generic_payloads() {
    let mut state: WindowLayout<Vec<u8>> = WindowLayout::default();
    state
        .ensure_editor_session(ROOT_AREA_ID)
        .tab_list
        .tabs
        .push(vec![1, 2, 3]);
    assert_eq!(
        state
            .ensure_editor_session(ROOT_AREA_ID)
            .tab_list
            .tabs
            .first()
            .expect("tab"),
        &vec![1, 2, 3]
    );
}
