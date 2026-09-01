//! SplitterRoot — the pure window/pane split layout.

use crate::root::SplitterRoot;
use crate::tree::SplitAxis;

/// Simple `Copy` payload so the layout can be exercised without entities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pane {
    Editor,
    Explorer,
}

fn new_root() -> SplitterRoot<Pane> {
    SplitterRoot::single_leaf(1, Pane::Editor)
}

#[test]
fn single_leaf_root_starts_with_one_leaf() {
    let root = new_root();
    assert_eq!(root.tree.count_leaves(), 1);
    assert_eq!(root.tree.find_leaf_kind(1), Some(Pane::Editor));
}

#[test]
fn splitting_creates_two_leaves() {
    let mut root = new_root();
    let new_id = root.split_leaf(1, SplitAxis::Horizontal, 0.5);
    assert!(new_id.is_some());
    assert_eq!(root.tree.count_leaves(), 2);
    let mut ids = Vec::new();
    root.tree.leaf_ids(&mut ids);
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&1));
    assert!(ids.contains(&new_id.unwrap()));
}

#[test]
fn closing_a_leaf_collapses_the_tree() {
    let mut root = new_root();
    let new_id = root.split_leaf(1, SplitAxis::Horizontal, 0.5).unwrap();
    root.close_leaf(new_id);
    assert_eq!(root.tree.count_leaves(), 1);
    assert_eq!(root.tree.find_leaf_kind(1), Some(Pane::Editor));
}

#[test]
fn kinds_are_updateable_and_activation_is_tracked() {
    let mut root = new_root();
    let new_id = root.split_leaf(1, SplitAxis::Horizontal, 0.5).unwrap();
    root.set_kind(new_id, Pane::Explorer);
    assert_eq!(root.tree.find_leaf_kind(new_id), Some(Pane::Explorer));
    root.activate_leaf(new_id);
    assert_eq!(root.active_leaf, Some(new_id));
}

#[test]
fn joining_leaves_restores_a_single_leaf() {
    let mut root = new_root();
    let new_id = root.split_leaf(1, SplitAxis::Horizontal, 0.5).unwrap();
    assert!(root.join_leaves(1, new_id));
    assert_eq!(root.tree.count_leaves(), 1);
}

#[test]
fn rect_collection_partitions_the_area() {
    let mut root = new_root();
    root.split_leaf(1, SplitAxis::Vertical, 0.5);
    let mut rects = Vec::new();
    root.tree
        .collect_leaf_rects(0.0, 0.0, 100.0, 100.0, &mut rects);
    assert_eq!(rects.len(), 2);
    for rect in &rects {
        assert!(rect.x >= 0.0 && rect.x + rect.width <= 100.0 + 0.001);
        assert!(rect.y >= 0.0 && rect.y + rect.height <= 100.0 + 0.001);
    }
    let total: f32 = rects.iter().map(|r| r.width * r.height).sum();
    assert!((total - 10000.0).abs() < 1.0);
}

#[test]
fn split_is_disallowed_when_maximized() {
    let mut root = new_root();
    let new_id = root.split_leaf(1, SplitAxis::Horizontal, 0.5).unwrap();
    assert_eq!(root.tree.count_leaves(), 2);

    // Maximize leaf 1
    root.toggle_maximize(1);
    assert!(root.tree.find_maximized_leaf().is_some());

    // Attempt to split while maximized - must fail
    let split_attempt = root.split_leaf(1, SplitAxis::Vertical, 0.5);
    assert!(split_attempt.is_none());
    assert_eq!(root.tree.count_leaves(), 2);

    // Attempt to split leaf 2 while leaf 1 is maximized - must fail
    let split_attempt2 = root.split_leaf(new_id, SplitAxis::Vertical, 0.5);
    assert!(split_attempt2.is_none());
    assert_eq!(root.tree.count_leaves(), 2);

    // Start corner drag while maximized - must be a no-op
    root.start_corner_drag(
        1,
        gpui::point(gpui::px(10.0), gpui::px(10.0)),
        crate::sessions::CornerDragModifier::None,
    );
    assert!(root.corner_drag_panel().is_none());

    // Unmaximize and verify split works again
    root.toggle_maximize(1);
    assert!(root.tree.find_maximized_leaf().is_none());
    let split_success = root.split_leaf(1, SplitAxis::Vertical, 0.5);
    assert!(split_success.is_some());
    assert_eq!(root.tree.count_leaves(), 3);
}

