//! Drag policies — what a finished corner drag means.
//!
//! The engine reports only gesture facts ([`CornerDragSession`]); the
//! policy turns them into operations. The default implementations live on
//! the [`DragPolicy`] trait, which is implemented per panel type on
//! [`SplitterContainer`] — so every panel shares the same defaults and a
//! specific panel type can override one behavior (the editor's inner
//! panels override Shift to a no-op).
//!
//! Defaults: plain drag splits with a content seed (or joins), Shift
//! drags clone the whole container into a new window, Ctrl swaps, Alt
//! does nothing. The host-dependent content steps run through the root's
//! injected hooks ([`SplitterRoot::seed_split`],
//! [`SplitterRoot::open_clone_window`]).

use std::collections::HashMap;

use gpui::{App, Pixels, Size};

use crate::root::SplitterRoot;
use crate::sessions::{CornerDragSession, MODIFIER_THRESHOLD_PX};
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
    /// Plain drag finished. Default: split (with the root's content seed
    /// hook) or join the dragged panel with the hovered one.
    fn on_plain_drag(
        root: &mut SplitterRoot<T>,
        facts: &CornerDragSession,
        container_size: Size<Pixels>,
        cx: &mut App,
    ) {
        if let Some(new_id) = split_or_join(root, facts, container_size) {
            if let Some(mut seed) = root.seed_split.take() {
                seed(root, facts.target_id, new_id, cx);
                root.seed_split = Some(seed);
            }
        }
    }

    /// Shift drag finished. Default: clone the whole container into a new
    /// window (via the root's open-clone-window hook). Shift drags never
    /// show the visual indicator.
    fn on_shift_drag(
        root: &mut SplitterRoot<T>,
        _facts: &CornerDragSession,
        _container_size: Size<Pixels>,
        cx: &mut App,
    ) {
        let mut next_id = root.next_node_id;
        let (tree, id_map) = root.tree.clone_with_id_map(&mut next_id);
        if let Some(mut open) = root.open_clone_window.take() {
            open(
                ClonedContainer {
                    tree,
                    next_node_id: next_id,
                    id_map,
                },
                cx,
            );
            root.open_clone_window = Some(open);
        }
    }

    /// Ctrl drag finished. Default: swap the dragged panel's kind with the
    /// hovered one (only once past the drag threshold).
    fn on_ctrl_drag(
        root: &mut SplitterRoot<T>,
        facts: &CornerDragSession,
        _container_size: Size<Pixels>,
        _cx: &mut App,
    ) {
        if drag_distance(facts) < MODIFIER_THRESHOLD_PX {
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
        _cx: &mut App,
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

fn drag_distance(facts: &CornerDragSession) -> f32 {
    let Some(pos) = facts.pointer_pos else {
        return 0.0;
    };
    let dx = f32::from(pos.x - facts.start_pos.x);
    let dy = f32::from(pos.y - facts.start_pos.y);
    (dx * dx + dy * dy).sqrt()
}
