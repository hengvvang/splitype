//! Unit tests for modern strong-typed IDs, domain errors, and LayoutInteraction.

use crate::error::SplitterError;
use crate::geometry::SplitAxis;
use crate::gesture::{
    AreaDockTarget, BorderMenuState, CornerDragModifier, CornerDragSession, SplitterDragSession,
};
use crate::id::{LeafId, NodeId, NodeIdAllocator, SplitId};
use crate::interaction::LayoutInteraction;
use gpui::{point, px};

#[test]
fn leaf_id_and_split_id_conversions() {
    let leaf = LeafId::new(42);
    assert_eq!(leaf.as_u32(), 42);
    assert_eq!(leaf.as_usize(), 42);
    assert_eq!(LeafId::from_usize(100), LeafId(100));
    assert_eq!(format!("{leaf}"), "Leaf(42)");

    let split = SplitId::new(99);
    assert_eq!(split.as_u32(), 99);
    assert_eq!(split.as_usize(), 99);
    assert_eq!(SplitId::from_usize(200), SplitId(200));
    assert_eq!(format!("{split}"), "Split(99)");

    let node_leaf: NodeId = leaf.into();
    assert!(node_leaf.is_leaf());
    assert!(!node_leaf.is_split());
    assert_eq!(node_leaf.as_leaf(), Some(leaf));
    assert_eq!(node_leaf.as_split(), None);
    assert_eq!(node_leaf.raw_id(), 42);

    let node_split: NodeId = split.into();
    assert!(node_split.is_split());
    assert!(!node_split.is_leaf());
    assert_eq!(node_split.as_split(), Some(split));
    assert_eq!(node_split.as_leaf(), None);
    assert_eq!(node_split.raw_id(), 99);
}

#[test]
fn node_id_allocator_guarantees_collision_free_ids() {
    let mut allocator = NodeIdAllocator::new();
    assert_eq!(allocator.peek_next(), 1);

    let leaf1 = allocator.next_leaf_id();
    let split1 = allocator.next_split_id();
    let leaf2 = allocator.next_leaf_id();

    assert_eq!(leaf1, LeafId(1));
    assert_eq!(split1, SplitId(2));
    assert_eq!(leaf2, LeafId(3));

    // Observing an external ID advances next_id beyond it
    allocator.observe(10);
    assert_eq!(allocator.peek_next(), 11);
    let split2 = allocator.next_split_id();
    assert_eq!(split2, SplitId(11));
}

#[test]
fn splitter_error_formatting() {
    let err_leaf = SplitterError::LeafNotFound(LeafId(5));
    assert_eq!(
        err_leaf.to_string(),
        "Target leaf with id Leaf(5) was not found in layout"
    );

    let err_split = SplitterError::SplitNotFound(SplitId(7));
    assert_eq!(
        err_split.to_string(),
        "Target split divider with id Split(7) was not found in layout"
    );

    let err_sole = SplitterError::CannotRemoveLastLeaf;
    assert_eq!(
        err_sole.to_string(),
        "Cannot remove or collapse the sole remaining leaf in the layout"
    );

    let err_max = SplitterError::LayoutMaximized(LeafId(3));
    assert_eq!(
        err_max.to_string(),
        "Layout is currently maximized by leaf Leaf(3); operation is prohibited"
    );

    let err_adj = SplitterError::NotAdjacent(LeafId(1), LeafId(2));
    assert_eq!(
        err_adj.to_string(),
        "Leaves Leaf(1) and Leaf(2) do not share an immediate split border and cannot be joined"
    );

    let err_ratio = SplitterError::InvalidRatio(1.5);
    assert_eq!(
        err_ratio.to_string(),
        "Invalid ratio 1.5; ratio must be between 0.01 and 0.99"
    );

    let err_same = SplitterError::SameLeaf(LeafId(1));
    assert_eq!(
        err_same.to_string(),
        "Target leaf Leaf(1) is identical to source leaf; cannot perform operation on self"
    );
}

#[test]
fn interaction_manager_maximization_lifecycle() {
    let mut interaction = LayoutInteraction::new();
    assert!(!interaction.is_maximized());
    assert_eq!(interaction.maximized_leaf(), None);

    let leaf1 = LeafId(1);
    let leaf2 = LeafId(2);

    interaction.toggle_maximize(leaf1);
    assert!(interaction.is_maximized());
    assert_eq!(interaction.maximized_leaf(), Some(leaf1));

    // Toggling another leaf switches maximization to that leaf
    interaction.toggle_maximize(leaf2);
    assert!(interaction.is_maximized());
    assert_eq!(interaction.maximized_leaf(), Some(leaf2));

    // Toggling same leaf restores normal tiled state
    interaction.toggle_maximize(leaf2);
    assert!(!interaction.is_maximized());
    assert_eq!(interaction.maximized_leaf(), None);
}

#[test]
fn interaction_manager_dropdown_is_exclusive() {
    let mut interaction = LayoutInteraction::new();
    let leaf1 = LeafId(1);
    let leaf2 = LeafId(2);

    assert!(!interaction.has_any_dropdown_open());

    interaction.toggle_dropdown(leaf1);
    assert!(interaction.is_dropdown_open(leaf1));
    assert!(!interaction.is_dropdown_open(leaf2));
    assert_eq!(interaction.open_dropdown_leaf(), Some(leaf1));

    // Opening leaf2 closes leaf1
    interaction.toggle_dropdown(leaf2);
    assert!(!interaction.is_dropdown_open(leaf1));
    assert!(interaction.is_dropdown_open(leaf2));

    // Clear dropdowns
    assert!(interaction.clear_dropdowns());
    assert!(!interaction.has_any_dropdown_open());
    assert!(!interaction.clear_dropdowns());
}

#[test]
fn interaction_manager_drag_sessions_and_overlays() {
    let mut interaction = LayoutInteraction::new();

    // Splitter drag
    let split_session = SplitterDragSession {
        split_id: SplitId(10),
        axis: SplitAxis::Horizontal,
        start_pointer_pos: 100.0,
        start_ratio: 0.5,
        total_span: 800.0,
    };
    interaction.start_splitter_drag(split_session);
    assert!(interaction.is_splitter_dragging());
    assert_eq!(interaction.active_splitter_drag(), Some(&split_session));
    let finished = interaction.finish_splitter_drag();
    assert_eq!(finished, Some(split_session));
    assert!(!interaction.is_splitter_dragging());

    // Corner drag
    let corner_session = CornerDragSession {
        target_id: LeafId(5),
        start_pos: point(px(10.0), px(10.0)),
        gesture_dir: None,
        modifier: CornerDragModifier::None,
        pointer_pos: Some(point(px(20.0), px(20.0))),
        hover_leaf: None,
        dock_target: AreaDockTarget::None,
        dock_ratio: 0.5,
    };
    interaction.start_corner_drag(corner_session);
    assert!(interaction.is_corner_dragging());
    assert!(interaction.cancel_corner_drag());
    assert!(!interaction.is_corner_dragging());

    // Border menu
    let menu = BorderMenuState {
        split_id: SplitId(10),
        position: point(px(50.0), px(50.0)),
    };
    interaction.open_border_menu(menu);
    assert_eq!(interaction.active_border_menu(), Some(&menu));
    assert!(interaction.clear_border_menu());
    assert_eq!(interaction.active_border_menu(), None);
}

#[test]
fn interaction_on_leaf_removed_cleans_up_state() {
    let mut interaction = LayoutInteraction::new();
    let leaf1 = LeafId(1);

    interaction.toggle_maximize(leaf1);
    interaction.toggle_dropdown(leaf1);
    interaction.start_corner_drag(CornerDragSession {
        target_id: LeafId(1),
        start_pos: point(px(0.0), px(0.0)),
        gesture_dir: None,
        modifier: CornerDragModifier::None,
        pointer_pos: None,
        hover_leaf: None,
        dock_target: AreaDockTarget::None,
        dock_ratio: 0.5,
    });

    assert!(interaction.is_maximized());
    assert!(interaction.has_any_dropdown_open());
    assert!(interaction.is_corner_dragging());

    interaction.on_leaf_removed(leaf1);

    assert!(!interaction.is_maximized());
    assert!(!interaction.has_any_dropdown_open());
    assert!(!interaction.is_corner_dragging());
}
