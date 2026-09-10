//! SplitterRoot — the pure window/pane split layout.

use crate::error::SplitterError;
use crate::geometry::SplitAxis;
use crate::gesture::CornerDragModifier;
use crate::id::{LeafId, SplitId};
use crate::root::SplitterRoot;

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
    let new_id = root.split_leaf(1, SplitAxis::Horizontal, 0.5).unwrap();
    assert_eq!(root.tree.count_leaves(), 2);
    let mut ids = Vec::new();
    root.tree.leaf_ids(&mut ids);
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&LeafId(1)));
    assert!(ids.contains(&new_id));
}

#[test]
fn closing_a_leaf_collapses_the_tree() {
    let mut root = new_root();
    let new_id = root.split_leaf(1, SplitAxis::Horizontal, 0.5).unwrap();
    root.close_leaf(new_id).unwrap();
    assert_eq!(root.tree.count_leaves(), 1);
    assert_eq!(root.tree.find_leaf_kind(1), Some(Pane::Editor));
}

#[test]
fn kinds_are_updateable_and_activation_is_tracked() {
    let mut root = new_root();
    let new_id = root.split_leaf(1, SplitAxis::Horizontal, 0.5).unwrap();
    root.set_kind(new_id, Pane::Explorer).unwrap();
    assert_eq!(root.tree.find_leaf_kind(new_id), Some(Pane::Explorer));
    root.activate_leaf(new_id);
    assert_eq!(root.active_leaf, Some(new_id));
}

#[test]
fn joining_leaves_restores_a_single_leaf() {
    let mut root = new_root();
    let new_id = root.split_leaf(1, SplitAxis::Horizontal, 0.5).unwrap();
    assert!(root.join_leaves(1, new_id).is_ok());
    assert_eq!(root.tree.count_leaves(), 1);
}

#[test]
fn rect_collection_partitions_the_area() {
    let mut root = new_root();
    root.split_leaf(1, SplitAxis::Vertical, 0.5).unwrap();
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
    root.interaction.toggle_maximize(1);
    assert!(root.find_maximized_leaf().is_some());

    // Attempt to split while maximized - must fail with LayoutMaximized error
    let split_attempt = root.split_leaf(1, SplitAxis::Vertical, 0.5);
    assert!(matches!(split_attempt, Err(SplitterError::LayoutMaximized(_))));
    assert_eq!(root.tree.count_leaves(), 2);

    // Attempt to split leaf 2 while leaf 1 is maximized - must fail
    let split_attempt2 = root.split_leaf(new_id, SplitAxis::Vertical, 0.5);
    assert!(matches!(split_attempt2, Err(SplitterError::LayoutMaximized(_))));
    assert_eq!(root.tree.count_leaves(), 2);

    // Start corner drag while maximized - must be a no-op
    root.start_corner_drag(
        1,
        gpui::point(gpui::px(10.0), gpui::px(10.0)),
        CornerDragModifier::None,
    );
    assert!(root.interaction.active_corner_drag().is_none());

    // Unmaximize and verify split works again
    root.interaction.toggle_maximize(1);
    assert!(root.find_maximized_leaf().is_none());
    let split_success = root.split_leaf(1, SplitAxis::Vertical, 0.5);
    assert!(split_success.is_ok());
    assert_eq!(root.tree.count_leaves(), 3);
}

#[test]
fn splitter_drag_updates_ratio_and_cancel_restores_it() {
    let mut root = new_root();
    let new_id = root.split_leaf(1, SplitAxis::Horizontal, 0.5).unwrap();
    // The split node id is allocated right before the new leaf id.
    let split_id = new_id.as_usize() - 1;

    root.start_splitter_drag(split_id, SplitAxis::Horizontal, 100.0, 0.5);
    assert!(root.interaction.active_splitter_drag().is_some());

    // Span refreshes from the viewport (1000px wide); +200px on a 1000px
    // span moves the ratio from 0.5 to 0.7.
    let viewport = gpui::size(gpui::px(1000.0), gpui::px(800.0));
    let updated = root.update_drag_gesture(gpui::point(gpui::px(300.0), gpui::px(0.0)), viewport);
    assert!(updated);
    let ratio = match &root.tree {
        crate::tree::SplitTree::Split { ratio, .. } => *ratio,
        _ => panic!("expected a split node"),
    };
    assert!((ratio - 0.7).abs() < 0.001);

    // Escape-style cancel restores the start ratio and clears the session.
    assert!(root.cancel_drag_gesture());
    assert!(root.interaction.active_splitter_drag().is_none());
    let restored = match &root.tree {
        crate::tree::SplitTree::Split { ratio, .. } => *ratio,
        _ => panic!("expected a split node"),
    };
    assert!((restored - 0.5).abs() < 0.001);
}

#[test]
fn corner_drag_gesture_round_trips_through_policy() {
    let mut root = new_root();
    let new_id = root.split_leaf(1, SplitAxis::Horizontal, 0.5).unwrap();

    // Plain drag within the same leaf splits it along the drag direction.
    root.start_corner_drag(
        1,
        gpui::point(gpui::px(10.0), gpui::px(10.0)),
        CornerDragModifier::None,
    );
    assert!(root.interaction.active_corner_drag().is_some());
    let viewport = gpui::size(gpui::px(1000.0), gpui::px(800.0));
    assert!(root.update_drag_gesture(gpui::point(gpui::px(300.0), gpui::px(200.0)), viewport));

    let result = root.finish_drag_gesture(viewport);
    match result {
        Some(crate::policy::CornerDragResult::Split { new_leaf_id, .. }) => {
            assert!(new_leaf_id > new_id);
        }
        other => panic!("expected a split result, got {other:?}"),
    }
    assert!(root.interaction.active_corner_drag().is_none());
}

#[test]
fn finishing_without_an_active_gesture_reports_none() {
    let mut root = new_root();
    let viewport = gpui::size(gpui::px(1000.0), gpui::px(800.0));
    assert!(root.finish_drag_gesture(viewport).is_none());
    assert!(!root.cancel_drag_gesture());
}

#[test]
fn border_menu_state_tracks_split_and_position_only() {
    let mut root = new_root();
    let new_id = root.split_leaf(1, SplitAxis::Horizontal, 0.5).unwrap();
    let split_id = SplitId::from_usize(new_id.as_usize() - 1);
    root.open_border_menu(split_id, gpui::point(gpui::px(320.0), gpui::px(240.0)));
    let menu = *root.interaction.active_border_menu().expect("menu open");
    assert_eq!(menu.split_id, split_id);
    assert_eq!(f32::from(menu.position.x), 320.0);
}

#[test]
fn structured_error_handling_in_topology() {
    let mut root = new_root();

    // Cannot remove or close the sole remaining leaf
    assert_eq!(
        root.close_leaf(1),
        Err(SplitterError::CannotRemoveLastLeaf)
    );

    // Invalid split ratio
    assert_eq!(
        root.split_leaf(1, SplitAxis::Horizontal, 1.5),
        Err(SplitterError::InvalidRatio(1.5))
    );
    assert_eq!(
        root.split_leaf(1, SplitAxis::Horizontal, -0.1),
        Err(SplitterError::InvalidRatio(-0.1))
    );

    // Leaf not found
    assert_eq!(
        root.split_leaf(999, SplitAxis::Horizontal, 0.5),
        Err(SplitterError::LeafNotFound(LeafId(999)))
    );

    // Joining self
    assert_eq!(
        root.join_leaves(1, 1),
        Err(SplitterError::SameLeaf(LeafId(1)))
    );

    // After splitting, divider operations succeed
    let leaf2 = root.split_leaf(1, SplitAxis::Horizontal, 0.5).unwrap();
    let split_id = SplitId::from_usize(leaf2.as_usize() - 1);

    let leaf3 = root.split_divider(split_id, SplitAxis::Vertical, 0.5).unwrap();
    assert_eq!(root.tree.count_leaves(), 3);
    assert!(root.tree.contains_leaf(leaf3));
}
