//! [`SplitterRoot`] — one initialized split region: the tree of panel
//! containers plus the tree-level state.
//!
//! A root is created when a region is initialized as a split container
//! (the window body below the titlebar, or an editor's pane container).
//! Every operation of the split library works on the root: splitting a
//! leaf creates a second [`SplitterContainer`] and both hang on the same
//! [`SplitTree`]. Panel-level state (corner-drag session, dropdown,
//! maximized) lives on each container; root-level state (id pool, splitter
//! drags, activation) lives here.

use gpui::{Pixels, Point, Size};

use crate::container::SplitterContainer;
use crate::sessions::{
    BorderMenuState, CornerDragModifier, CornerDragSession, SplitterDragSession, id_at_point,
};
use crate::tree::{Direction, LeafRect, NodeId, SplitAxis, SplitTree};

/// One initialized split region: the panel tree plus tree-level state.
///
/// Only durable topology is serialized; drag sessions and focus bookkeeping
/// are transient and skipped.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(bound(
    serialize = "T: serde::Serialize",
    deserialize = "T: serde::Deserialize<'de>"
))]
pub struct SplitterRoot<T: Clone + PartialEq> {
    /// The tree of panel containers hanging on this root.
    pub tree: SplitTree<T>,
    /// Id pool shared by every node of this root's tree.
    pub next_node_id: usize,
    /// Splitter-bar drag session (resizing a split divider — tree-level).
    #[serde(skip)]
    pub active_splitter_drag: Option<SplitterDragSession>,
    /// Border context menu state (right-click on a divider — tree-level).
    #[serde(skip)]
    pub active_border_menu: Option<BorderMenuState>,
    /// The most recently activated leaf; hosts route global actions to
    /// this leaf.
    pub active_leaf: Option<NodeId>,
    /// Activation history, ordered oldest to newest. When the active leaf
    /// closes, focus moves to the previous entry in history instead of
    /// picking an arbitrary neighbor.
    pub activation_history: Vec<NodeId>,
    /// The leaf the mouse is currently operating on.
    #[serde(skip)]
    pub focused_leaf: Option<NodeId>,
}

impl<T: Clone + PartialEq> SplitterRoot<T> {
    /// A root with one leaf (one panel container), used to seed editor
    /// pane containers.
    pub fn single_leaf(initial_id: NodeId, kind: T) -> Self {
        Self {
            tree: SplitTree::Leaf(SplitterContainer::new(initial_id, kind)),
            next_node_id: initial_id + 1,
            active_splitter_drag: None,
            active_border_menu: None,
            active_leaf: None,
            activation_history: Vec::new(),
            focused_leaf: None,
        }
    }

    // ------------------------------------------------------------------
    // Split / close / kind / join / swap
    // ------------------------------------------------------------------

    /// Resolves a target node ID (either a Leaf ID or a Split divider ID) to an actionable Leaf ID.
    /// If `target_id` is a Leaf, returns `Some(target_id)`. If it is a Split divider,
    /// returns its second child leaf (or first child leaf).
    pub fn resolve_leaf(&self, target_id: NodeId) -> Option<NodeId> {
        if self.tree.find_leaf_kind(target_id).is_some() {
            Some(target_id)
        } else {
            self.tree
                .find_split_second_leaf_id(target_id)
                .or_else(|| self.tree.find_split_first_leaf_id(target_id))
        }
    }

    /// Split `target_id` (leaf or divider) at `ratio` with a sibling of the SAME kind:
    /// the leaf's container is replaced by a `Split` node holding the
    /// original container and a freshly created one. Returns the new
    /// leaf's id.
    pub fn split_leaf(&mut self, target_id: NodeId, axis: SplitAxis, ratio: f32) -> Option<NodeId> {
        let leaf_id = self.resolve_leaf(target_id)?;
        let kind = self.tree.find_leaf_kind(leaf_id)?;
        let split_id = self.next_node_id;
        self.next_node_id += 1;
        let new_leaf_id = self.next_node_id;
        self.next_node_id += 1;
        self.tree
            .split_leaf_with_ratio(leaf_id, split_id, new_leaf_id, axis, ratio, kind);
        self.active_border_menu = None;
        Some(new_leaf_id)
    }

    /// Close a leaf or divider; the last leaf is never closable.
    pub fn close_leaf(&mut self, target_id: NodeId) {
        if let Some(leaf_id) = self.resolve_leaf(target_id) {
            if self.tree.count_leaves() > 1 {
                self.tree.remove_leaf(leaf_id);
                self.retire_leaf(leaf_id);
            }
        }
        self.active_border_menu = None;
    }

    /// Mark `leaf_id` as the active leaf (the last leaf that received
    /// focus). Records the activation for fallback ordering.
    pub fn activate_leaf(&mut self, leaf_id: NodeId) {
        self.activation_history.retain(|id| *id != leaf_id);
        self.activation_history.push(leaf_id);
        self.active_leaf = Some(leaf_id);
    }

    /// Returns the active leaf of the given `kind`, resolving through:
    /// 1. Currently active leaf (if of kind `kind`)
    /// 2. MRU activation history fallback
    /// 3. First available leaf of kind `kind` in tree traversal
    pub fn active_leaf_of_kind(&self, kind: T) -> Option<NodeId> {
        if let Some(active) = self.active_leaf {
            if self.tree.find_leaf_kind(active).as_ref() == Some(&kind) {
                return Some(active);
            }
        }
        self.activation_history
            .iter()
            .rev()
            .copied()
            .find(|id| self.tree.find_leaf_kind(*id).as_ref() == Some(&kind))
            .or_else(|| {
                let mut leaves = Vec::new();
                self.tree.leaf_ids(&mut leaves);
                leaves
                    .into_iter()
                    .find(|id| self.tree.find_leaf_kind(*id).as_ref() == Some(&kind))
            })
    }

    /// Recompute the active leaf after the layout changed: the most
    /// recently activated leaf still present, or `None`.
    fn recompute_active_leaf(&mut self) {
        if self
            .active_leaf
            .is_some_and(|id| self.tree.find_leaf_kind(id).is_some())
        {
            return;
        }
        self.active_leaf = self
            .activation_history
            .iter()
            .rev()
            .copied()
            .find(|id| self.tree.find_leaf_kind(*id).is_some());
    }

    /// Drop a leaf from activation tracking and recompute the active leaf.
    fn retire_leaf(&mut self, removed: NodeId) {
        self.activation_history.retain(|id| *id != removed);
        if self.active_leaf == Some(removed) {
            self.active_leaf = None;
        }
        self.recompute_active_leaf();
    }

    /// Change a leaf's kind. A changed kind drops the leaf from the
    /// activation history — the host re-activates it when the new kind
    /// should be the active one. (Kind-agnostic rule: any kind mutation
    /// retires, no matter which kinds are involved.)
    pub fn set_kind(&mut self, leaf_id: NodeId, kind: T) {
        let previous = self.tree.find_leaf_kind(leaf_id);
        self.tree.set_leaf_kind(leaf_id, kind.clone());
        if previous != Some(kind) {
            self.retire_leaf(leaf_id);
        }
        self.active_border_menu = None;
    }

    /// Join `removed` into `into`. The removed leaf is closed and its
    /// space is absorbed by the `into` leaf. The two must be adjacent
    /// (share an edge) in the layout.
    pub fn join_leaves(&mut self, into: NodeId, removed: NodeId) -> bool {
        if into == removed || self.tree.count_leaves() <= 1 {
            return false;
        }
        let ok = self.tree.join_leaf(into, removed);
        if ok {
            self.retire_leaf(removed);
        }
        self.active_border_menu = None;
        ok
    }

    /// Swap the kind of leaf `a` and leaf `b`. Both leaves leave the
    /// activation history (same rule as [`Self::set_kind`]).
    pub fn swap_kinds(&mut self, a: NodeId, b: NodeId) {
        let kind_a = self.tree.find_leaf_kind(a);
        let kind_b = self.tree.find_leaf_kind(b);
        if let (Some(ta), Some(tb)) = (kind_a, kind_b) {
            self.tree.set_leaf_kind(a, tb);
            self.tree.set_leaf_kind(b, ta);
            self.retire_leaf(a);
            self.retire_leaf(b);
        }
    }

    /// Move a source leaf and dock it onto a target leaf at the specified edge,
    /// closing the source leaf and rearranging its former neighbors, while splitting
    /// the target leaf to accommodate the moved leaf at the specified ratio.
    /// If `dock_target` is `Center`, it swaps kinds between source and target instead.
    pub fn move_and_dock_leaf(
        &mut self,
        source_id: NodeId,
        target_id: NodeId,
        dock_target: crate::sessions::AreaDockTarget,
        ratio: f32,
    ) -> Option<NodeId> {
        if source_id == target_id {
            return None;
        }
        if dock_target == crate::sessions::AreaDockTarget::Center {
            self.swap_kinds(source_id, target_id);
            return None;
        }
        let source_kind = self.tree.find_leaf_kind(source_id)?;
        let target_kind = self.tree.find_leaf_kind(target_id)?;

        // 1. Remove source leaf (collapsing source parent Split node and expanding neighbors)
        self.tree.remove_leaf(source_id);
        self.retire_leaf(source_id);

        // 2. Allocate new IDs
        let split_id = self.next_node_id;
        self.next_node_id += 1;
        let new_leaf_id = self.next_node_id;
        self.next_node_id += 1;

        // 3. Split target leaf based on dock_target
        let (axis, split_ratio, source_first) = match dock_target {
            crate::sessions::AreaDockTarget::Left => {
                (SplitAxis::Horizontal, ratio.clamp(0.01, 0.99), true)
            }
            crate::sessions::AreaDockTarget::Right => (
                SplitAxis::Horizontal,
                (1.0 - ratio).clamp(0.01, 0.99),
                false,
            ),
            crate::sessions::AreaDockTarget::Top => {
                (SplitAxis::Vertical, ratio.clamp(0.01, 0.99), true)
            }
            crate::sessions::AreaDockTarget::Bottom => {
                (SplitAxis::Vertical, (1.0 - ratio).clamp(0.01, 0.99), false)
            }
            _ => (SplitAxis::Horizontal, 0.5, true),
        };

        if source_first {
            self.tree.split_leaf_with_ratio(
                target_id,
                split_id,
                new_leaf_id,
                axis,
                split_ratio,
                target_kind,
            );
            self.tree.set_leaf_kind(target_id, source_kind);
        } else {
            self.tree.split_leaf_with_ratio(
                target_id,
                split_id,
                new_leaf_id,
                axis,
                split_ratio,
                source_kind,
            );
        }
        self.active_border_menu = None;
        Some(new_leaf_id)
    }

    // ------------------------------------------------------------------
    // Maximise / dropdown (panel-level flags)
    // ------------------------------------------------------------------

    pub fn toggle_maximize(&mut self, leaf_id: NodeId) {
        if let Some(panel) = self.tree.find_leaf_mut(leaf_id) {
            panel.maximized = !panel.maximized;
        }
    }

    /// Toggle the dropdown of `leaf_id`; opening one closes any other
    /// open dropdown in this root (only one dropdown at a time).
    pub fn toggle_dropdown(&mut self, leaf_id: NodeId) {
        let mut ids = Vec::new();
        self.tree.leaf_ids(&mut ids);
        let mut target_state = None;
        for id in ids {
            if let Some(panel) = self.tree.find_leaf_mut(id) {
                if id == leaf_id {
                    target_state = Some(!panel.open_dropdown);
                } else if panel.open_dropdown {
                    panel.open_dropdown = false;
                }
            }
        }
        if let Some(state) = target_state {
            if let Some(panel) = self.tree.find_leaf_mut(leaf_id) {
                panel.open_dropdown = state;
            }
        }
    }

    /// Close every panel's dropdown in this root.
    pub fn clear_dropdowns(&mut self) {
        let mut ids = Vec::new();
        self.tree.leaf_ids(&mut ids);
        for id in ids {
            if let Some(panel) = self.tree.find_leaf_mut(id) {
                panel.open_dropdown = false;
            }
        }
    }

    // ------------------------------------------------------------------
    // Splitter drag (resizing dividers — tree-level)
    // ------------------------------------------------------------------

    pub fn update_splitter_drag(&mut self, current_pointer_pos: f32) {
        if let Some(session) = self.active_splitter_drag {
            if session.total_span > 1.0 {
                let new_ratio = session.ratio_at(current_pointer_pos);
                self.tree.set_split_ratio(session.split_id, new_ratio);
            }
        }
    }

    pub fn end_splitter_drag(&mut self) {
        self.active_splitter_drag = None;
    }

    // ------------------------------------------------------------------
    // Corner drag — sessions live on each panel container
    // ------------------------------------------------------------------

    /// Begin a corner-drag gesture from `target_id`'s panel at `pos` with
    /// an optional modifier key. The session is recorded on the panel
    /// itself (panels stay self-contained).
    pub fn start_corner_drag(
        &mut self,
        target_id: NodeId,
        pos: Point<Pixels>,
        modifier: CornerDragModifier,
    ) {
        if let Some(panel) = self.tree.find_leaf_mut(target_id) {
            panel.start_corner_drag(pos, modifier);
            self.focused_leaf = Some(target_id);
        }
    }

    /// The panel currently dragging a corner, if any.
    pub fn corner_drag_panel(&self) -> Option<NodeId> {
        let mut ids = Vec::new();
        self.tree.leaf_ids(&mut ids);
        ids.into_iter().find(|id| {
            self.tree
                .find_leaf(*id)
                .is_some_and(|p| p.active_corner_drag.is_some())
        })
    }

    /// Process a mouse-move event during a corner drag.
    ///
    /// Only updates the raw facts of the dragging panel's session —
    /// cardinal gesture direction, pointer position, and the hovered
    /// leaf. The engine never interprets them: the host decides what the
    /// gesture means, when a shortcut fires, and whether to render an
    /// indicator.
    ///
    /// `current_pos` and the container size must share a coordinate system
    /// (window coords for the outer layout, the pane layout's local space
    /// for an editor). Returns whether a session was updated.
    pub fn update_corner_drag(
        &mut self,
        current_pos: Point<Pixels>,
        container_size: Size<Pixels>,
    ) -> bool {
        let Some(target_id) = self.corner_drag_panel() else {
            return false;
        };
        let Some(session) = self
            .tree
            .find_leaf(target_id)
            .and_then(|p| p.active_corner_drag)
        else {
            return false;
        };

        let dx = f32::from(current_pos.x - session.start_pos.x);
        let dy = f32::from(current_pos.y - session.start_pos.y);
        let abs_dx = dx.abs();
        let abs_dy = dy.abs();

        // Cardinal direction from the mouse delta so far.
        let dir = if abs_dy > abs_dx {
            if dy > 0.0 {
                Direction::Down
            } else {
                Direction::Up
            }
        } else if dx > 0.0 {
            Direction::Right
        } else {
            Direction::Left
        };

        // The leaf under the pointer, if any.
        let leaf_rects = self.leaf_rects(container_size);
        let over_id = id_at_point(&leaf_rects, current_pos);

        let (dock_target, dock_ratio) = if let Some(over) = over_id {
            if over != target_id {
                let target_rect = leaf_rects.iter().find(|r| r.id == over);
                let source_rect = leaf_rects.iter().find(|r| r.id == target_id);
                if let (Some(target_rect), Some(source_rect)) = (target_rect, source_rect) {
                    crate::sessions::calculate_dock_target(
                        source_rect,
                        target_rect,
                        dir,
                        current_pos,
                        session.modifier == CornerDragModifier::Ctrl,
                    )
                } else {
                    (crate::sessions::AreaDockTarget::None, 0.5)
                }
            } else {
                (crate::sessions::AreaDockTarget::None, 0.5)
            }
        } else {
            (crate::sessions::AreaDockTarget::None, 0.5)
        };

        if let Some(panel) = self.tree.find_leaf_mut(target_id) {
            panel.active_corner_drag = Some(CornerDragSession {
                target_id,
                start_pos: session.start_pos,
                gesture_dir: Some(dir),
                modifier: session.modifier,
                pointer_pos: Some(current_pos),
                hover_leaf: over_id,
                dock_target,
                dock_ratio,
            });
            true
        } else {
            false
        }
    }

    /// Finish the corner-drag gesture on mouse release: returns the final
    /// raw facts of the dragging panel and clears its session.
    pub fn finish_corner_drag(&mut self) -> Option<CornerDragSession> {
        let target_id = self.corner_drag_panel()?;
        self.tree.find_leaf_mut(target_id)?.finish_corner_drag()
    }

    /// Finish and apply the active corner-drag gesture on mouse release:
    /// executes the topological operation and returns the structured result.
    pub fn apply_corner_drag(
        &mut self,
        container_size: Size<Pixels>,
    ) -> crate::policy::CornerDragResult<T> {
        let Some(facts) = self.finish_corner_drag() else {
            return crate::policy::CornerDragResult::None;
        };
        crate::policy::apply_corner_drag_session(self, &facts, container_size)
    }

    /// End the dragging panel's corner-drag session, clearing state.
    pub fn end_corner_drag(&mut self) {
        if let Some(target_id) = self.corner_drag_panel() {
            if let Some(panel) = self.tree.find_leaf_mut(target_id) {
                panel.end_corner_drag();
            }
        }
    }

    // ------------------------------------------------------------------
    // Geometry
    // ------------------------------------------------------------------

    /// Collect all leaf rectangles in pixel coordinates.
    pub fn leaf_rects(&self, container_size: Size<Pixels>) -> Vec<LeafRect> {
        let w = f32::from(container_size.width);
        let h = f32::from(container_size.height);
        let mut rects = Vec::new();
        if w > 0.0 && h > 0.0 {
            let mut norm = Vec::new();
            self.tree.collect_leaf_rects(0.0, 0.0, 1.0, 1.0, &mut norm);
            for rect in norm {
                rects.push(LeafRect {
                    id: rect.id,
                    x: rect.x * w,
                    y: rect.y * h,
                    width: rect.width * w,
                    height: rect.height * h,
                });
            }
        }
        rects
    }

    /// Calculate the pixel span (width or height) of a split container.
    pub fn split_pixel_span(&self, split_id: NodeId, container_size: Size<Pixels>) -> Option<f32> {
        let w = f32::from(container_size.width);
        let h = f32::from(container_size.height);
        let (_, span) = self.tree.find_split_span(split_id, w, h)?;
        Some(span)
    }

    /// Compute the split axis and ratio for a finished corner drag from
    /// the session facts (host-independent geometry). Returns `None` until
    /// the gesture has a direction and a pointer position.
    pub fn corner_split_facts(
        &self,
        facts: &CornerDragSession,
        container_size: Size<Pixels>,
    ) -> Option<(SplitAxis, f32)> {
        let dir = facts.gesture_dir?;
        let pos = facts.pointer_pos?;
        let mut rects = Vec::new();
        self.tree.collect_leaf_rects(0.0, 0.0, 1.0, 1.0, &mut rects);
        let target = rects.iter().find(|rect| rect.id == facts.target_id)?;
        let axis = if dir.is_vertical() {
            SplitAxis::Vertical
        } else {
            SplitAxis::Horizontal
        };
        let norm_x = f32::from(pos.x) / f32::from(container_size.width);
        let norm_y = f32::from(pos.y) / f32::from(container_size.height);
        let raw_ratio = match axis {
            SplitAxis::Horizontal => (norm_x - target.x) / target.width,
            SplitAxis::Vertical => (norm_y - target.y) / target.height,
        };
        let ratio = crate::sessions::calc_snapped_ratio(
            raw_ratio,
            facts.modifier == CornerDragModifier::Ctrl,
        );
        Some((axis, ratio))
    }

    pub fn swap_split_sides(&mut self, split_id: NodeId) {
        self.tree.swap_sibling_leaves(split_id);
        self.active_border_menu = None;
    }
}
