//! Drag policies and unified corner drag evaluation.
//!
//! Translates reported gesture facts ([`CornerDragSession`]) into tree topology
//! operations and a structured [`CornerDragResult`].

use std::collections::HashMap;

use gpui::{Pixels, Size};

use crate::container::SplitterContainer;
use crate::gesture::{
    AreaDockTarget, CornerDragModifier, CornerDragSession, past_shortcut_threshold,
};
use crate::id::{LeafId, NodeIdAllocator};
use crate::root::SplitterRoot;
use crate::tree::SplitTree;

/// A whole container cloned for a Shift-drag "open clone window" gesture:
/// the new tree, its dedicated ID allocator, and the mapping from
/// each old leaf id to its new id.
#[derive(Clone, Debug, PartialEq)]
pub struct ClonedContainer<T: Clone + PartialEq> {
    pub tree: SplitTree<T>,
    pub allocator: NodeIdAllocator,
    /// Old leaf id → new leaf id.
    pub id_map: HashMap<LeafId, LeafId>,
}

/// Result of evaluating and applying a finished corner drag gesture on a split root.
#[derive(Clone, Debug, PartialEq)]
pub enum CornerDragResult<T: Clone + PartialEq> {
    /// Same-area split performed, creating a new sibling leaf.
    Split { new_leaf_id: LeafId },
    /// Adjacent join performed: removed_id's leaf was absorbed.
    Join { removed_id: LeafId },
    /// Move and dock performed: source_id moved and docked onto target_id.
    MoveAndDock {
        source_id: LeafId,
        target_id: LeafId,
        new_leaf_id: LeafId,
        dock_target: AreaDockTarget,
    },
    /// Area swap performed between a and b.
    Swap { a: LeafId, b: LeafId },
    /// Shift-drag new window clone created.
    CloneWindow { container: ClonedContainer<T> },
    /// Gesture ended with no topology change.
    None,
}

impl<T: Clone + PartialEq> CornerDragResult<T> {
    /// Returns `true` if this result represents no topology modification.
    #[inline]
    pub const fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Returns the newly created leaf ID if this was a Split or MoveAndDock operation.
    #[inline]
    pub const fn new_leaf_id(&self) -> Option<LeafId> {
        match self {
            Self::Split { new_leaf_id } | Self::MoveAndDock { new_leaf_id, .. } => {
                Some(*new_leaf_id)
            }
            _ => None,
        }
    }
}

/// Evaluate and apply a finished corner drag session on a root, modifying the tree
/// topology accordingly and returning the structured high-level result.
pub fn apply_corner_drag_session<T: Clone + PartialEq>(
    root: &mut SplitterRoot<T>,
    facts: &CornerDragSession,
    container_size: Size<Pixels>,
) -> CornerDragResult<T> {
    let target_id = facts.target_id;
    match facts.modifier {
        CornerDragModifier::Shift => {
            if !past_shortcut_threshold(facts) {
                return CornerDragResult::None;
            }
            let Some(kind) = root.tree.find_leaf_kind(facts.target_id) else {
                return CornerDragResult::None;
            };
            let new_leaf_id = root.allocator.next_leaf_id();
            let mut id_map = HashMap::new();
            id_map.insert(target_id, new_leaf_id);
            CornerDragResult::CloneWindow {
                container: ClonedContainer {
                    tree: SplitTree::Leaf(SplitterContainer::new(new_leaf_id, kind)),
                    allocator: NodeIdAllocator::with_start(new_leaf_id.as_u32() + 1),
                    id_map,
                },
            }
        }
        CornerDragModifier::Ctrl => {
            if !past_shortcut_threshold(facts) {
                return CornerDragResult::None;
            }
            if let Some(hover_id) = facts.hover_leaf {
                if hover_id != target_id && root.swap_kinds(target_id, hover_id).is_ok() {
                    return CornerDragResult::Swap {
                        a: target_id,
                        b: hover_id,
                    };
                }
            }
            CornerDragResult::None
        }
        CornerDragModifier::None => {
            if let Some(hover_id) = facts.hover_leaf {
                if hover_id != target_id {
                    if facts.dock_target == AreaDockTarget::Center {
                        if root.swap_kinds(target_id, hover_id).is_ok() {
                            return CornerDragResult::Swap {
                                a: target_id,
                                b: hover_id,
                            };
                        }
                    } else if facts.dock_target != AreaDockTarget::None {
                        if let Ok(Some(new_leaf_id)) = root.move_and_dock_leaf(
                            target_id,
                            hover_id,
                            facts.dock_target,
                            facts.dock_ratio,
                        ) {
                            return CornerDragResult::MoveAndDock {
                                source_id: target_id,
                                target_id: hover_id,
                                new_leaf_id,
                                dock_target: facts.dock_target,
                            };
                        }
                    } else if root.join_leaves(hover_id, target_id).is_ok() {
                        return CornerDragResult::Join {
                            removed_id: target_id,
                        };
                    }
                    return CornerDragResult::None;
                }
            }

            // Same-area split when dragging within the same area or without hovering a neighbor:
            if let Some((axis, ratio)) = root.corner_split_facts(facts, container_size) {
                if let Ok(new_leaf_id) = root.split_leaf(target_id, axis, ratio) {
                    return CornerDragResult::Split { new_leaf_id };
                }
            }

            CornerDragResult::None
        }
    }
}
