//! Drag policies — what a finished corner drag means.
//!
//! The engine reports only gesture facts ([`CornerDragSession`]); the
//! policy turns them into operations. Two defaults live here:
//! [`DefaultDragPolicy`] (window level: Shift clones the whole container
//! into a new window, plain drags split with a content seed, Ctrl swaps,
//! Alt does nothing) and [`InnerPanelDragPolicy`] (editor panels: Shift
//! is overridden to a no-op, plain drags split without content).
//!
//! Hosts pick the policy and hold it; the host calls the matching method
//! when a drag finishes (the engine does not own policy state).

use std::collections::HashMap;

use gpui::{App, Pixels, Size};

use crate::sessions::{CornerDragSession, MODIFIER_THRESHOLD_PX};
use crate::state::SplitterContainer;
use crate::tree::SplitTree;
use crate::types::NodeId;

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

/// Strategy for a finished corner drag. `container_size` is the container's
/// pixel size in the same space as the session's pointer facts; `cx`
/// lets the strategy reach host state (the engine depends on gpui only).
pub trait DragPolicy<T: Copy + PartialEq> {
    /// Plain drag finished. Default: split (with content seed) or join.
    fn on_plain_drag(
        &mut self,
        container: &mut SplitterContainer<T>,
        facts: &CornerDragSession,
        container_size: Size<Pixels>,
        cx: &mut App,
    );

    /// Shift drag finished. Default: clone the whole container into a new
    /// window. Shift drags never show the visual indicator.
    fn on_shift_drag(
        &mut self,
        container: &mut SplitterContainer<T>,
        facts: &CornerDragSession,
        container_size: Size<Pixels>,
        cx: &mut App,
    );

    /// Ctrl drag finished. Default: swap the dragged leaf with the hovered
    /// one (only once past the drag threshold).
    fn on_ctrl_drag(
        &mut self,
        container: &mut SplitterContainer<T>,
        facts: &CornerDragSession,
        container_size: Size<Pixels>,
        cx: &mut App,
    );

    /// Alt drag finished. Default: no-op.
    fn on_alt_drag(
        &mut self,
        container: &mut SplitterContainer<T>,
        facts: &CornerDragSession,
        container_size: Size<Pixels>,
        cx: &mut App,
    );
}

/// Window-level default policy: Shift = clone the window, plain drag =
/// split (content seeded through the callback) or join, Ctrl = swap,
/// Alt = nothing.
///
/// The two host-dependent steps are injected as callbacks:
/// - `open_clone_window` receives the whole cloned container (tree + id
///   map + fresh id pool) and opens a new window showing it;
/// - `seed_split_content` seeds the fresh sibling leaf after a split
///   (e.g. the editor deep-copies its tab list and inner panel layout).
pub struct DefaultDragPolicy<OpenWindow, SeedSplit> {
    pub open_clone_window: OpenWindow,
    pub seed_split_content: SeedSplit,
}

impl<T, OpenWindow, SeedSplit> DragPolicy<T> for DefaultDragPolicy<OpenWindow, SeedSplit>
where
    T: Copy + PartialEq,
    OpenWindow: FnMut(ClonedContainer<T>, &mut App),
    SeedSplit: FnMut(&mut SplitterContainer<T>, NodeId, NodeId, &mut App),
{
    fn on_plain_drag(
        &mut self,
        container: &mut SplitterContainer<T>,
        facts: &CornerDragSession,
        container_size: Size<Pixels>,
        cx: &mut App,
    ) {
        if let Some(new_id) = split_or_join(container, facts, container_size) {
            (self.seed_split_content)(container, facts.target_id, new_id, cx);
        }
    }

    fn on_shift_drag(
        &mut self,
        container: &mut SplitterContainer<T>,
        _facts: &CornerDragSession,
        _container_size: Size<Pixels>,
        cx: &mut App,
    ) {
        // Clone the whole container: the tree gets fresh ids, and the
        // host walks the id map to clone the per-leaf content.
        let mut next_id = container.next_node_id;
        let (tree, id_map) = container.tree.clone_with_id_map(&mut next_id);
        (self.open_clone_window)(
            ClonedContainer {
                tree,
                next_node_id: next_id,
                id_map,
            },
            cx,
        );
    }

    fn on_ctrl_drag(
        &mut self,
        container: &mut SplitterContainer<T>,
        facts: &CornerDragSession,
        _container_size: Size<Pixels>,
        _cx: &mut App,
    ) {
        swap_with_hover(container, facts);
    }

    fn on_alt_drag(
        &mut self,
        _container: &mut SplitterContainer<T>,
        _facts: &CornerDragSession,
        _container_size: Size<Pixels>,
        _cx: &mut App,
    ) {
        // No-op by default; hosts may override for their own shortcuts.
    }
}

/// Editor inner-panel policy: Shift is overridden to a no-op, plain drags
/// split/join without any content seed (inner panels have no per-panel
/// content), Ctrl swaps, Alt does nothing.
pub struct InnerPanelDragPolicy;

impl<T: Copy + PartialEq> DragPolicy<T> for InnerPanelDragPolicy {
    fn on_plain_drag(
        &mut self,
        container: &mut SplitterContainer<T>,
        facts: &CornerDragSession,
        container_size: Size<Pixels>,
        _cx: &mut App,
    ) {
        split_or_join(container, facts, container_size);
    }

    fn on_shift_drag(
        &mut self,
        _container: &mut SplitterContainer<T>,
        _facts: &CornerDragSession,
        _container_size: Size<Pixels>,
        _cx: &mut App,
    ) {
        // Shift + drag on an inner panel: explicit no-op override.
    }

    fn on_ctrl_drag(
        &mut self,
        container: &mut SplitterContainer<T>,
        facts: &CornerDragSession,
        _container_size: Size<Pixels>,
        _cx: &mut App,
    ) {
        swap_with_hover(container, facts);
    }

    fn on_alt_drag(
        &mut self,
        _container: &mut SplitterContainer<T>,
        _facts: &CornerDragSession,
        _container_size: Size<Pixels>,
        _cx: &mut App,
    ) {
    }
}

/// Shared plain-drag geometry: join the dragged leaf into the hovered
/// neighbor, or split it (returning the new leaf's id).
fn split_or_join<T: Copy + PartialEq>(
    container: &mut SplitterContainer<T>,
    facts: &CornerDragSession,
    container_size: Size<Pixels>,
) -> Option<NodeId> {
    match facts.hover_leaf {
        Some(hover) if hover != facts.target_id => {
            container.join_leaves(hover, facts.target_id);
            None
        }
        _ => {
            let (direction, ratio) = container.corner_split_facts(facts, container_size)?;
            container.split_leaf(facts.target_id, direction, ratio)
        }
    }
}

/// Shared Ctrl-drag geometry: swap the dragged leaf with the hovered one,
/// but only once the drag is past the modifier threshold (a sub-threshold
/// drag is a mis-click and does nothing).
fn swap_with_hover<T: Copy + PartialEq>(
    container: &mut SplitterContainer<T>,
    facts: &CornerDragSession,
) {
    if drag_distance(facts) < MODIFIER_THRESHOLD_PX {
        return;
    }
    if let Some(hover) = facts.hover_leaf {
        if hover != facts.target_id {
            container.swap_kinds(facts.target_id, hover);
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
