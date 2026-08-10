//! Drag policies — what a finished corner drag means.
//!
//! The engine reports only gesture facts ([`CornerDragSession`]); the
//! policy turns them into tree operations. The default implementations
//! live on the [`DragPolicy`] trait, which is implemented per panel type
//! on [`SplitterContainer`] — so every panel shares the same defaults and
//! a specific panel type can override one behavior (the editor's inner
//! panels override Shift to a no-op).
//!
//! Defaults: plain drag splits (or joins), Shift drags clone the dragged
//! panel's container into a new window, Ctrl swaps, Alt does nothing.
//! Policies only perform the content-free tree geometry and return what
//! happened; the host performs the content steps itself (seeding a new
//! leaf, opening the cloned window). Policies are always invoked from
//! inside the host's own entity update, so they never re-enter the host.

use std::collections::HashMap;

use gpui::{Pixels, Size};

use crate::container::SplitterContainer;
use crate::root::SplitterRoot;
use crate::sessions::{CornerDragSession, past_shortcut_threshold};
use crate::tree::{NodeId, SplitTree};

/// A whole container cloned for a Shift-drag "open clone window" gesture:
/// the new tree, the id pool it was assigned from, and the mapping from
/// each old node id to its new id (so the host can clone the per-leaf
/// content of the matching leaves).
#[derive(Clone, Debug)]
pub struct ClonedContainer<T: Copy + PartialEq> {
    pub tree: SplitTree<T>,
    pub next_node_id: usize,
    /// Old node id → new node id.
    pub id_map: HashMap<NodeId, NodeId>,
}

/// Strategy for a finished corner drag, implemented per panel type on
/// [`SplitterContainer`]. The default implementations are shared by every
/// panel type; a specific type overrides just what it needs (e.g. the
/// editor's inner panels override Shift to a no-op).
///
/// Methods take the root plus the gesture facts (the dragged panel is
/// identified by `facts.target_id`), so the tree-level operations — split,
/// join, swap, clone — stay borrow-clean.
pub trait DragPolicy<T: Copy + PartialEq> {
    /// Plain drag finished. Default: split or join the dragged panel with
    /// the hovered one. Returns the id of the leaf a split created, so the
    /// host can seed its content (`None` = joined, or nothing happened).
    fn on_plain_drag(
        root: &mut SplitterRoot<T>,
        facts: &CornerDragSession,
        container_size: Size<Pixels>,
    ) -> Option<NodeId> {
        split_or_join(root, facts, container_size)
    }

    /// Shift drag finished. Default: clone the DRAGGED panel's container
    /// into a fresh single-leaf tree, returning it for the host to open
    /// in a new window — not the whole window layout. Shift drags never
    /// show the visual indicator. `None` = the gesture is a no-op (inner
    /// panels).
    fn on_shift_drag(
        root: &mut SplitterRoot<T>,
        facts: &CornerDragSession,
        _container_size: Size<Pixels>,
    ) -> Option<ClonedContainer<T>> {
        let kind = root.tree.find_leaf_kind(facts.target_id)?;
        let new_id = root.next_node_id;
        let mut id_map = HashMap::new();
        id_map.insert(facts.target_id, new_id);
        Some(ClonedContainer {
            tree: SplitTree::Leaf(SplitterContainer::new(new_id, kind)),
            next_node_id: new_id + 1,
            id_map,
        })
    }

    /// Ctrl drag finished. Default: swap the dragged panel's kind with the
    /// hovered one (only once past the drag threshold).
    fn on_ctrl_drag(
        root: &mut SplitterRoot<T>,
        facts: &CornerDragSession,
        _container_size: Size<Pixels>,
    ) {
        if !past_shortcut_threshold(facts) {
            return;
        }
        if let Some(hover) = facts.hover_leaf {
            if hover != facts.target_id {
                root.swap_kinds(facts.target_id, hover);
            }
        }
    }

    /// Alt drag finished. Default: no-op.
    fn on_alt_drag(
        _root: &mut SplitterRoot<T>,
        _facts: &CornerDragSession,
        _container_size: Size<Pixels>,
    ) {
    }
}

/// Shared plain-drag geometry: join the dragged panel into the hovered
/// neighbor, or split it (returning the new leaf's id).
fn split_or_join<T: Copy + PartialEq>(
    root: &mut SplitterRoot<T>,
    facts: &CornerDragSession,
    container_size: Size<Pixels>,
) -> Option<NodeId> {
    match facts.hover_leaf {
        Some(hover) if hover != facts.target_id => {
            root.join_leaves(hover, facts.target_id);
            None
        }
        _ => {
            let (direction, ratio) = root.corner_split_facts(facts, container_size)?;
            root.split_leaf(facts.target_id, direction, ratio)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sessions::CornerDragModifier;
    use gpui::{point, px, size};

    /// A stand-in panel type: the policy must not know any concrete
    /// application kind.
    #[derive(Clone, Copy, Debug, PartialEq)]
    enum TestKind {
        A,
    }

    /// Policies are implemented per panel type; the test registers its own
    /// kind with the shared defaults.
    impl DragPolicy<TestKind> for SplitterContainer<TestKind> {}

    #[test]
    fn test_shift_drag_clones_only_the_dragged_leaf() {
        let mut root = SplitterRoot::single_leaf(1, TestKind::A);
        let split_id = root.split_leaf(1, crate::tree::Axis::Horizontal, 0.5).unwrap();
        let pool = root.next_node_id;
        let facts = CornerDragSession {
            target_id: split_id,
            start_pos: point(px(0.0), px(0.0)),
            gesture_dir: None,
            modifier: CornerDragModifier::Shift,
            pointer_pos: None,
            hover_leaf: None,
        };
        let cloned = <SplitterContainer<TestKind> as DragPolicy<TestKind>>::on_shift_drag(
            &mut root,
            &facts,
            size(px(100.0), px(100.0)),
        )
        .expect("shift drag should clone the dragged leaf");

        // The clone is a single-leaf tree of the dragged panel's kind
        // with a fresh id from the root's pool.
        let mut ids = Vec::new();
        cloned.tree.leaf_ids(&mut ids);
        assert_eq!(ids, vec![pool]);
        assert_eq!(cloned.tree.find_leaf_kind(pool), Some(TestKind::A));
        assert_eq!(cloned.id_map.len(), 1);
        assert_eq!(cloned.id_map.get(&split_id), Some(&pool));
        assert_eq!(cloned.next_node_id, pool + 1);

        // The source root is untouched by the gesture.
        assert_eq!(root.tree.count_leaves(), 2);
        assert_eq!(root.next_node_id, pool);
    }
}
