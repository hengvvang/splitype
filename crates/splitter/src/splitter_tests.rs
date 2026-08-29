//! SplitTree — the pure window/pane split layout tree.

use crate::tree::{SplitAxis, SplitTree};

/// Simple `Copy` payload so the tree can be exercised without entities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pane {
    Editor,
    Explorer,
    Settings,
}

fn new_tree() -> SplitTree<Pane> {
    SplitTree::new(1, Pane::Editor)
}

#[test]
fn single_leaf_tree_starts_with_one_leaf() {
    let tree = new_tree();
    assert_eq!(tree.count_leaves(), 1);
    assert_eq!(tree.find_leaf_kind(1), Some(Pane::Editor));
}

#[test]
fn splitting_creates_two_leaves_sharing_the_split_axis() {
    let mut tree = new_tree();
    let new_id = tree.split_leaf_with_ratio(1, SplitAxis::Horizontal, 0.5);
    assert!(new_id.is_some());
    assert_eq!(tree.count_leaves(), 2);
    let ids = tree.leaf_ids(&mut Vec::new());
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&1));
}

#[test]
fn removing_a_leaf_collapses_the_tree() {
    let mut tree = new_tree();
    let new_id = tree.split_leaf_with_ratio(1, SplitAxis::Horizontal, 0.5).unwrap();
    assert!(tree.remove_leaf(new_id));
    assert_eq!(tree.count_leaves(), 1);
    assert_eq!(tree.find_leaf_kind(1), Some(Pane::Editor));
}

#[test]
fn leaf_kinds_are_updateable() {
    let mut tree = new_tree();
    assert!(tree.set_leaf_kind(1, Pane::Explorer));
    assert_eq!(tree.find_leaf_kind(1), Some(Pane::Explorer));
}

#[test]
fn clone_with_new_ids_produces_a_disjoint_tree() {
    let mut tree = new_tree();
    tree.split_leaf_with_ratio(1, SplitAxis::Horizontal, 0.5);
    let mut next_id = 100;
    let cloned = tree.clone_with_new_ids(&mut next_id);
    assert_eq!(cloned.count_leaves(), 2);
    // The clone's ids must not collide with the original's.
    let original_ids = tree.leaf_ids(&mut Vec::new());
    let cloned_ids = cloned.leaf_ids(&mut Vec::new());
    for id in &cloned_ids {
        assert!(!original_ids.contains(id));
    }
}

#[test]
fn rect_collection_covers_the_area_without_overlap() {
    let mut tree = new_tree();
    tree.split_leaf_with_ratio(1, SplitAxis::Vertical, 0.5);
    tree.split_leaf_with_ratio(2, SplitAxis::Horizontal, 0.5);
    let mut rects = Vec::new();
    tree.collect_leaf_rects(0.0, 0.0, 100.0, 100.0, &mut rects);
    assert_eq!(rects.len(), 3);
    // All rects lie inside the root area.
    for rect in &rects {
        assert!(rect.x >= 0.0 && rect.x + rect.width <= 100.0 + 0.001);
        assert!(rect.y >= 0.0 && rect.y + rect.height <= 100.0 + 0.001);
    }
    // The three rects partition the 100x100 area.
    let total: f32 = rects.iter().map(|r| r.width * r.height).sum();
    assert!((total - 10000.0).abs() < 1.0);
}

#[test]
fn joining_two_leaves_restores_the_parent_split() {
    let mut tree = new_tree();
    let new_id = tree.split_leaf_with_ratio(1, SplitAxis::Horizontal, 0.5).unwrap();
    assert!(tree.join_leaf(1, new_id));
    assert_eq!(tree.count_leaves(), 1);
}
