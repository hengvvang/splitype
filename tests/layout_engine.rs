//! Cross-crate contract tests for the tiled layout engine.
//!
//! `splitype-splitter` owns only the outer window tree; the inner pane
//! layout moved to the editor layer and is covered by module tests there.
//! These tests drive the outer tree through its public API as an external
//! consumer would: split panels, close them, switch kinds, and verify the
//! resulting rectangles.

use gpui::{Size, px};

use splitype::app::window_panels::WindowPanelKind;
use splitype::app::window_panels::{DEFAULT_EDITOR_PANEL_ID, ROOT_PANEL_ID, WindowLayout};
use splitype_splitter::Axis;
use splitype_splitter::tree::LeafRect;

fn layout() -> WindowLayout {
    splitype::app::window_panels::default_layout()
}

fn root_rects(state: &WindowLayout) -> Vec<LeafRect> {
    let mut rects = Vec::new();
    state
        .tree
        .collect_leaf_rects(0.0, 0.0, 100.0, 100.0, &mut rects);
    rects
}

/// The default layout is Explorer (left, 30%) + Editor (right, 70%).
#[test]
fn default_layout_is_explorer_editor_split() {
    let state = layout();
    let mut leaves = Vec::new();
    state.tree.leaf_ids(&mut leaves);
    assert_eq!(leaves, vec![ROOT_PANEL_ID, DEFAULT_EDITOR_PANEL_ID]);
    assert_eq!(
        state.tree.find_leaf_kind(ROOT_PANEL_ID),
        Some(WindowPanelKind::Explorer)
    );
    assert_eq!(
        state.tree.find_leaf_kind(DEFAULT_EDITOR_PANEL_ID),
        Some(WindowPanelKind::Editor)
    );
    let rects = root_rects(&state);
    assert_eq!(rects.len(), 2);
    assert!((rects[0].width - 30.0).abs() < 0.001);
    assert!((rects[1].width - 70.0).abs() < 0.001);
}

/// Splitting an area adds a leaf and halves that side's rects.
#[test]
fn split_window_panel_creates_two_leaves() {
    let mut state = layout();
    let new_id = state
        .split_leaf(DEFAULT_EDITOR_PANEL_ID, Axis::Horizontal, 0.5)
        .expect("split");

    let mut leaves = Vec::new();
    state.tree.leaf_ids(&mut leaves);
    assert_eq!(leaves.len(), 3);

    let rects = root_rects(&state);
    assert_eq!(rects.len(), 3);
    // Explorer keeps its 30%; the Editor side halves into two 35% panes.
    assert!((rects[0].width - 30.0).abs() < 0.001);
    assert!((rects[1].width - 35.0).abs() < 0.001);
    let right = rects
        .iter()
        .find(|rect| rect.id == new_id)
        .expect("new area");
    assert!((right.x - 65.0).abs() < 0.001);
}

/// Closing an area joins the tree back to the default split.
#[test]
fn closing_area_joins_back_to_default_split() {
    let mut state = layout();
    let new_id = state
        .split_leaf(DEFAULT_EDITOR_PANEL_ID, Axis::Horizontal, 0.5)
        .expect("split");

    state.close_leaf(new_id);
    let mut leaves = Vec::new();
    state.tree.leaf_ids(&mut leaves);
    assert_eq!(leaves, vec![ROOT_PANEL_ID, DEFAULT_EDITOR_PANEL_ID]);
}

/// Area kinds switch on the tree leaves.
#[test]
fn area_kinds_switch_on_leaves() {
    let mut state = layout();
    state.set_kind(DEFAULT_EDITOR_PANEL_ID, WindowPanelKind::Settings);
    assert_eq!(
        state.tree.find_leaf_kind(DEFAULT_EDITOR_PANEL_ID),
        Some(WindowPanelKind::Settings)
    );
    state.set_kind(DEFAULT_EDITOR_PANEL_ID, WindowPanelKind::Editor);
    assert_eq!(
        state.tree.find_leaf_kind(DEFAULT_EDITOR_PANEL_ID),
        Some(WindowPanelKind::Editor)
    );
}

/// Split ratio changes are reflected in the rectangles.
#[test]
fn split_ratio_affects_rect_widths() {
    let mut state = layout();
    state.split_leaf(DEFAULT_EDITOR_PANEL_ID, Axis::Horizontal, 0.25);

    let rects = root_rects(&state);
    assert_eq!(rects.len(), 3);
    // Editor side splits at 25%: the first Editor pane is 70% * 25% = 17.5.
    let left = rects
        .iter()
        .find(|rect| rect.id == DEFAULT_EDITOR_PANEL_ID)
        .expect("left editor rect");
    assert!((left.width - 17.5).abs() < 0.001);
}

/// Window panel rects scale to the container size.
#[test]
fn window_panel_rects_scale_to_container() {
    let mut state = layout();
    state.split_leaf(DEFAULT_EDITOR_PANEL_ID, Axis::Horizontal, 0.5);

    let rects = state.leaf_rects(Size::new(px(1000.0), px(800.0)));
    assert_eq!(rects.len(), 3);
    assert_eq!(rects[0].width, 300.0);
    assert_eq!(rects[2].x, 650.0);
    assert_eq!(rects[2].height, 800.0);
    assert_eq!(rects[2].x + rects[2].width, 1000.0);
}

/// Maximizing tracks a single area and can be toggled off.
#[test]
fn maximize_toggles_single_area() {
    let mut state = layout();
    let new_id = state
        .split_leaf(DEFAULT_EDITOR_PANEL_ID, Axis::Horizontal, 0.5)
        .expect("split");

    state.toggle_maximize(new_id);
    assert!(state.tree.find_leaf(new_id).is_some_and(|p| p.maximized));
    state.toggle_maximize(new_id);
    assert!(state.tree.find_leaf(new_id).is_some_and(|p| !p.maximized));
}

/// Activation history records the most recent editor area.
#[test]
fn activation_history_tracks_editors() {
    let mut state = layout();
    state.activate_leaf(DEFAULT_EDITOR_PANEL_ID);
    assert_eq!(state.active_leaf, Some(DEFAULT_EDITOR_PANEL_ID));
    assert_eq!(state.activation_history, vec![DEFAULT_EDITOR_PANEL_ID]);
}
