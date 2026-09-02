//! Drag policies and unified corner drag evaluation.
//!
//! The engine reports gesture facts ([`CornerDragSession`]) and translates them
//! into tree topology operations and a structured [`CornerDragResult`].

use std::collections::HashMap;

use gpui::{Pixels, Size};

use crate::container::SplitterContainer;
use crate::root::SplitterRoot;
use crate::sessions::{
    AreaDockTarget, CornerDragModifier, CornerDragSession, past_shortcut_threshold,
};
use crate::tree::{NodeId, SplitTree};

/// A whole container cloned for a Shift-drag "open clone window" gesture:
/// the new tree, the id pool it was assigned from, and the mapping from
/// each old node id to its new id.
#[derive(Clone, Debug, PartialEq)]
pub struct ClonedContainer<T: Clone + PartialEq> {
    pub tree: SplitTree<T>,
    pub next_node_id: NodeId,
    /// Old node id → new node id.
    pub id_map: HashMap<NodeId, NodeId>,
}

/// Result of evaluating and applying a finished corner drag gesture on a split root.
#[derive(Clone, Debug, PartialEq)]
pub enum CornerDragResult<T: Clone + PartialEq> {
    /// Same-area split performed, creating a new sibling leaf.
    Split { new_leaf_id: NodeId },
    /// Adjacent join performed: removed_id's leaf was absorbed.
    Join { removed_id: NodeId },
    /// Move and dock performed: source_id moved and docked onto target_id.
    MoveAndDock {
        source_id: NodeId,
        target_id: NodeId,
        new_leaf_id: NodeId,
        dock_target: AreaDockTarget,
    },
    /// Area swap performed between a and b.
    Swap { a: NodeId, b: NodeId },
    /// Shift-drag new window clone created.
    CloneWindow { container: ClonedContainer<T> },
    /// Gesture ended with no topology change.
    None,
}

/// Evaluate and apply a finished corner drag session on a root, modifying the tree
/// topology accordingly and returning the structured high-level result.
pub fn apply_corner_drag_session<T: Clone + PartialEq>(
    root: &mut SplitterRoot<T>,
    facts: &CornerDragSession,
    container_size: Size<Pixels>,
) -> CornerDragResult<T> {
    match facts.modifier {
        CornerDragModifier::Shift => {
            if !past_shortcut_threshold(facts) {
                return CornerDragResult::None;
            }
            let Some(kind) = root.tree.find_leaf_kind(facts.target_id) else {
                return CornerDragResult::None;
            };
            let new_id = root.next_node_id;
            root.next_node_id += 1;
            let mut id_map = HashMap::new();
            id_map.insert(facts.target_id, new_id);
            CornerDragResult::CloneWindow {
                container: ClonedContainer {
                    tree: SplitTree::Leaf(SplitterContainer::new(new_id, kind)),
                    next_node_id: new_id + 1,
                    id_map,
                },
            }
        }
        CornerDragModifier::Ctrl => {
            if !past_shortcut_threshold(facts) {
                return CornerDragResult::None;
            }
            if let Some(hover) = facts.hover_leaf {
                if hover != facts.target_id {
                    root.swap_kinds(facts.target_id, hover);
                    return CornerDragResult::Swap {
                        a: facts.target_id,
                        b: hover,
                    };
                }
            }
            CornerDragResult::None
        }
        CornerDragModifier::None => {
            if let Some(hover) = facts.hover_leaf {
                if hover != facts.target_id {
                    if facts.dock_target == AreaDockTarget::Center {
                        root.swap_kinds(facts.target_id, hover);
                        CornerDragResult::Swap {
                            a: facts.target_id,
                            b: hover,
                        }
                    } else if facts.dock_target != AreaDockTarget::None {
                        if let Some(new_leaf_id) = root.move_and_dock_leaf(
                            facts.target_id,
                            hover,
                            facts.dock_target,
                            facts.dock_ratio,
                        ) {
                            CornerDragResult::MoveAndDock {
                                source_id: facts.target_id,
                                target_id: hover,
                                new_leaf_id,
                                dock_target: facts.dock_target,
                            }
                        } else {
                            CornerDragResult::None
                        }
                    } else if root.join_leaves(hover, facts.target_id) {
                        CornerDragResult::Join {
                            removed_id: facts.target_id,
                        }
                    } else {
                        CornerDragResult::None
                    }
                } else if let Some((axis, ratio)) = root.corner_split_facts(facts, container_size) {
                    if let Some(new_leaf_id) = root.split_leaf(facts.target_id, axis, ratio) {
                        CornerDragResult::Split { new_leaf_id }
                    } else {
                        CornerDragResult::None
                    }
                } else {
                    CornerDragResult::None
                }
            } else if let Some((axis, ratio)) = root.corner_split_facts(facts, container_size) {
                if let Some(new_leaf_id) = root.split_leaf(facts.target_id, axis, ratio) {
                    CornerDragResult::Split { new_leaf_id }
                } else {
                    CornerDragResult::None
                }
            } else {
                CornerDragResult::None
            }
        }
    }
}
