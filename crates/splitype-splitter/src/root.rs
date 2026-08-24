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
pub struct SplitterRoot<T: Copy + PartialEq> {
    /// The tree of panel containers hanging on this root.
    pub tree: SplitTree<T>,
    /// Id pool shared by every node of this root's tree.
    pub next_node_id: usize,
    /// Splitter-bar drag session (resizing a split divider — tree-level).
    pub active_splitter_drag: Option<SplitterDragSession>,
    /// Border context menu state (right-click on a divider — tree-level).
    pub active_border_menu: Option<BorderMenuState>,
    /// The most recently activated leaf; hosts route global actions to
    /// this leaf.
    pub active_leaf: Option<NodeId>,
    /// Activation history, ordered oldest to newest. When the active leaf
    /// closes, focus moves to the previous entry in history instead of
    /// picking an arbitrary neighbor.
    pub activation_history: Vec<NodeId>,
    /// The leaf the mouse is currently operating on.
    pub focused_leaf: Option<NodeId>,
}

impl<T: Copy + PartialEq> SplitterRoot<T> {
    /// A root with one leaf (one panel container), used to seed editor
    /// pane containers.
    pub fn single_leaf(initial_id: usize, kind: T) -> Self {
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
    pub fn resolve_leaf(&self, target_id: usize) -> Option<usize> {
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
    pub fn split_leaf(&mut self, target_id: usize, axis: SplitAxis, ratio: f32) -> Option<usize> {
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
    pub fn close_leaf(&mut self, target_id: usize) {
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
    pub fn activate_leaf(&mut self, leaf_id: usize) {
        self.activation_history.retain(|id| *id != leaf_id);
        self.activation_history.push(leaf_id);
        self.active_leaf = Some(leaf_id);
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
    fn retire_leaf(&mut self, removed: usize) {
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
    pub fn set_kind(&mut self, leaf_id: usize, kind: T) {
        let previous = self.tree.find_leaf_kind(leaf_id);
        self.tree.set_leaf_kind(leaf_id, kind);
        if previous != Some(kind) {
            self.retire_leaf(leaf_id);
        }
        self.active_border_menu = None;
    }

    /// Join `removed` into `into`. The removed leaf is closed and its
    /// space is absorbed by the `into` leaf. The two must be adjacent
    /// (share an edge) in the layout.
    pub fn join_leaves(&mut self, into: usize, removed: usize) -> bool {
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
    pub fn swap_kinds(&mut self, a: usize, b: usize) {
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
        source_id: usize,
        target_id: usize,
        dock_target: crate::sessions::AreaDockTarget,
        ratio: f32,
    ) -> Option<usize> {
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
            crate::sessions::AreaDockTarget::Right => {
                (SplitAxis::Horizontal, (1.0 - ratio).clamp(0.01, 0.99), false)
            }
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

    pub fn toggle_maximize(&mut self, leaf_id: usize) {
        if let Some(panel) = self.tree.find_leaf_mut(leaf_id) {
            panel.maximized = !panel.maximized;
        }
    }

    /// Toggle the dropdown of `leaf_id`; opening one closes any other
    /// open dropdown in this root (only one dropdown at a time).
    pub fn toggle_dropdown(&mut self, leaf_id: usize) {
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
        target_id: usize,
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

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{point, px, size};

    /// A stand-in panel type: the engine must not know any concrete
    /// application kind, so the tests register their own.
    #[derive(Clone, Copy, Debug, PartialEq)]
    enum TestKind {
        A,
        B,
    }

    fn test_root() -> SplitterRoot<TestKind> {
        SplitterRoot::single_leaf(1, TestKind::A)
    }

    #[test]
    fn test_panel_layout_suite() {
        let mut root = test_root();
        assert_eq!(root.tree.count_leaves(), 1);

        let leaf_2 = root.split_leaf(1, SplitAxis::Horizontal, 0.5).unwrap();
        assert_eq!(root.tree.count_leaves(), 2);

        let _leaf_3 = root.split_leaf(leaf_2, SplitAxis::Vertical, 0.5).unwrap();
        assert_eq!(root.tree.count_leaves(), 3);

        root.close_leaf(leaf_2);
        assert_eq!(root.tree.count_leaves(), 2);

        root.set_kind(1, TestKind::B);
        assert_eq!(root.tree.find_leaf_kind(1), Some(TestKind::B));

        root.toggle_maximize(1);
        assert!(root.tree.find_leaf(1).is_some_and(|p| p.maximized));
        root.toggle_maximize(1);
        assert!(root.tree.find_leaf(1).is_some_and(|p| !p.maximized));

        root.active_splitter_drag = Some(SplitterDragSession {
            split_id: 1,
            axis: SplitAxis::Horizontal,
            start_pointer_pos: 100.0,
            start_ratio: 0.5,
            total_span: 1000.0,
        });
        root.update_splitter_drag(200.0);
        root.end_splitter_drag();
        assert_eq!(root.active_splitter_drag, None);
    }

    #[test]
    fn test_split_inherits_source_kind() {
        let mut root = test_root();
        assert_eq!(root.tree.find_leaf_kind(1), Some(TestKind::A));

        // Split leaf 1 → A + A (same kind, not cycled).
        let leaf_2 = root.split_leaf(1, SplitAxis::Horizontal, 0.5).unwrap();
        assert_eq!(root.tree.count_leaves(), 2);
        assert_eq!(root.tree.find_leaf_kind(1), Some(TestKind::A));
        assert_eq!(root.tree.find_leaf_kind(leaf_2), Some(TestKind::A));

        // B splits into B.
        root.set_kind(1, TestKind::B);
        let new_b = root
            .split_leaf(1, SplitAxis::Horizontal, 0.5)
            .expect("second split should succeed");
        assert_eq!(root.tree.count_leaves(), 3);
        assert_eq!(root.tree.find_leaf_kind(new_b), Some(TestKind::B));
    }

    #[test]
    fn test_split_creates_a_second_container_on_the_same_tree() {
        let mut root = test_root();
        // One leaf = one panel container.
        assert!(matches!(&root.tree, SplitTree::Leaf(_)));
        assert_eq!(root.tree.count_leaves(), 1);

        // Splitting replaces the leaf with a Split node holding TWO
        // containers — the original and the freshly created one — both
        // hanging on the same tree.
        let new_id = root.split_leaf(1, SplitAxis::Horizontal, 0.5).unwrap();
        assert_eq!(root.tree.count_leaves(), 2);
        assert!(matches!(&root.tree, SplitTree::Split { .. }));
        assert!(root.tree.find_leaf(1).is_some());
        assert!(root.tree.find_leaf(new_id).is_some());
        assert!(
            root.tree
                .find_leaf(1)
                .is_some_and(|p| p.kind == TestKind::A)
        );
        assert!(
            root.tree
                .find_leaf(new_id)
                .is_some_and(|p| p.kind == TestKind::A)
        );
        // The fresh container starts with no interaction state.
        assert!(
            root.tree
                .find_leaf(new_id)
                .is_some_and(|p| p.active_corner_drag.is_none() && !p.maximized)
        );
    }

    #[test]
    fn test_active_leaf_falls_back_to_last_focused() {
        let mut root = test_root();
        root.activate_leaf(1);
        let a = root.split_leaf(1, SplitAxis::Horizontal, 0.5).unwrap();
        let b = root.split_leaf(1, SplitAxis::Vertical, 0.5).unwrap();
        // Activation order: 1, a, b → active is b.
        root.activate_leaf(a);
        root.activate_leaf(b);
        assert_eq!(root.active_leaf, Some(b));

        // Close the active leaf → falls back to the previous focus (a).
        root.close_leaf(b);
        assert_eq!(root.active_leaf, Some(a));

        // Closing the second-to-last leaf falls back to the remaining
        // root leaf (the last leaf is never closable).
        root.close_leaf(a);
        assert_eq!(root.active_leaf, Some(1));
    }

    #[test]
    fn test_kind_change_retires_activation() {
        let mut root = test_root();
        root.activate_leaf(1);
        let a = root.split_leaf(1, SplitAxis::Horizontal, 0.5).unwrap();
        root.activate_leaf(a);

        // Changing an inactive leaf's kind leaves the active one alone.
        root.set_kind(1, TestKind::B);
        assert_eq!(root.active_leaf, Some(a));

        // Changing the active leaf's kind retires it: no fallback left.
        root.set_kind(a, TestKind::B);
        assert_eq!(root.active_leaf, None);

        // Re-activating restores it.
        root.activate_leaf(a);
        assert_eq!(root.active_leaf, Some(a));

        // A no-op kind change keeps the activation.
        root.set_kind(a, TestKind::B);
        assert_eq!(root.active_leaf, Some(a));
    }

    #[test]
    fn test_join_sibling_leaves() {
        let mut root = test_root();
        let leaf_2 = root.split_leaf(1, SplitAxis::Horizontal, 0.5).unwrap();
        assert_eq!(root.tree.count_leaves(), 2);

        // Join leaf 2 into leaf 1: remove 2, expand 1.
        let ok = root.join_leaves(1, leaf_2);
        assert!(ok);
        assert_eq!(root.tree.count_leaves(), 1);
        assert_eq!(root.tree.find_leaf_kind(1), Some(TestKind::A));
    }

    #[test]
    fn test_join_nested_leaves() {
        let mut root = test_root();
        // Build: Split(H) { Leaf(1), Split(H) { Leaf(leaf_2), Leaf(leaf_3) } }
        let leaf_2 = root.split_leaf(1, SplitAxis::Horizontal, 0.5).unwrap();
        let _leaf_3 = root.split_leaf(leaf_2, SplitAxis::Horizontal, 0.5).unwrap();
        assert_eq!(root.tree.count_leaves(), 3);

        // Join leaf 1 with leaf 2 (different subtrees) → 2 leaves remain.
        let ok = root.join_leaves(1, leaf_2);
        assert!(ok);
        assert_eq!(root.tree.count_leaves(), 2);
    }

    #[test]
    fn test_window_panel_rects() {
        let mut root = test_root();
        root.split_leaf(1, SplitAxis::Horizontal, 0.3); // ids: 1 (30%), 2 (70%)
        let rects = root.leaf_rects(size(px(1000.0), px(800.0)));
        assert_eq!(rects.len(), 2);
        let first = rects[0];
        let second = rects[1];
        assert!((first.width - 300.0).abs() < 1.0);
        assert!((second.width - 700.0).abs() < 1.0);
        assert!((first.height - 800.0).abs() < 1.0);
        assert!((second.height - 800.0).abs() < 1.0);
        assert!((second.x - 300.0).abs() < 1.0);
    }

    #[test]
    fn test_corner_drag_session_lives_on_the_panel() {
        let mut root = test_root();
        root.start_corner_drag(1, point(px(10.0), px(10.0)), CornerDragModifier::None);
        assert!(root.tree.find_leaf(1).unwrap().active_corner_drag.is_some());

        // Update computes the facts on the panel's session.
        assert!(root.update_corner_drag(point(px(60.0), px(20.0)), size(px(100.0), px(100.0))));
        let session = root.tree.find_leaf(1).unwrap().active_corner_drag.unwrap();
        assert_eq!(session.gesture_dir, Some(Direction::Right));
        assert!(session.pointer_pos.is_some());

        // Finish returns the facts and clears the panel's session.
        let facts = root.finish_corner_drag().expect("session present");
        assert_eq!(facts.target_id, 1);
        assert!(root.tree.find_leaf(1).unwrap().active_corner_drag.is_none());
    }

    #[test]
    fn test_split_node_id_uniqueness_and_divider_resolution() {
        let mut root = test_root();
        // Initially leaf 1.
        let leaf_2 = root.split_leaf(1, SplitAxis::Horizontal, 0.5).unwrap();
        assert_ne!(leaf_2, 1);

        // Check that Split node has a distinct ID.
        if let SplitTree::Split {
            id, first, second, ..
        } = &root.tree
        {
            assert_ne!(*id, 1);
            assert_ne!(*id, leaf_2);
            assert!(matches!(&**first, SplitTree::Leaf(c) if c.id == 1));
            assert!(matches!(&**second, SplitTree::Leaf(c) if c.id == leaf_2));

            // Test divider resolution
            assert_eq!(root.resolve_leaf(*id), Some(leaf_2));
            assert_eq!(root.resolve_leaf(1), Some(1));
            assert_eq!(root.resolve_leaf(leaf_2), Some(leaf_2));

            // Split via divider ID
            let split_id = *id;
            let leaf_3 = root
                .split_leaf(split_id, SplitAxis::Vertical, 0.5)
                .unwrap();
            assert_eq!(root.tree.count_leaves(), 3);
            assert!(root.tree.find_leaf(leaf_3).is_some());

            // Close via divider ID
            root.close_leaf(split_id);
            assert_eq!(root.tree.count_leaves(), 2);
        } else {
            panic!("expected split tree");
        }
    }

    #[test]
    fn test_move_and_dock_leaf_rearranges_layout() {
        let mut root = test_root();
        let leaf_2 = root.split_leaf(1, SplitAxis::Horizontal, 0.5).unwrap();
        let leaf_3 = root.split_leaf(leaf_2, SplitAxis::Vertical, 0.5).unwrap();
        assert_eq!(root.tree.count_leaves(), 3);

        // Move leaf 3 and dock onto leaf 1 at Left with ratio 0.4
        let new_leaf = root
            .move_and_dock_leaf(leaf_3, 1, crate::sessions::AreaDockTarget::Left, 0.4)
            .expect("dock should succeed");
        assert_eq!(root.tree.count_leaves(), 3);
        assert!(root.tree.find_leaf(new_leaf).is_some());
    }

    #[test]
    fn test_calc_snapped_ratio_magnetic_half() {
        use crate::sessions::calc_snapped_ratio;
        // Magnetic 0.5 snapping
        assert_eq!(calc_snapped_ratio(0.48, false), 0.5);
        assert_eq!(calc_snapped_ratio(0.50, false), 0.5);
        assert_eq!(calc_snapped_ratio(0.52, false), 0.5);

        // Outside 0.5 range without Ctrl
        assert!((calc_snapped_ratio(0.30, false) - 0.30).abs() < 0.001);

        // With Ctrl (snaps to 1/12 grid)
        assert_eq!(calc_snapped_ratio(0.24, true), 0.25); // 3/12 = 0.25
        assert_eq!(calc_snapped_ratio(0.33, true), 0.33333334); // 4/12 = 1/3
    }

    #[test]
    fn test_calculate_dock_target_quadrants() {
        use crate::sessions::{calculate_dock_target, AreaDockTarget};
        let source = LeafRect {
            id: 1,
            x: 0.0,
            y: 0.0,
            width: 500.0,
            height: 800.0,
        };
        let target = LeafRect {
            id: 2,
            x: 500.0,
            y: 0.0,
            width: 500.0,
            height: 800.0,
        };
        // 1. Direct neighbor near shared border (fac_x = 0.10 <= 0.15) -> Join!
        let (dock, _) = calculate_dock_target(&source, &target, Direction::Right, point(px(550.0), px(400.0)), false);
        assert_eq!(dock, AreaDockTarget::None);

        // 1b. Direct neighbor past Join zone (fac_x = 0.35 -> raw_pos = (0.35 - 0.15)/0.35 = 0.571) -> Dock Left!
        let (dock, ratio) = calculate_dock_target(&source, &target, Direction::Right, point(px(675.0), px(400.0)), false);
        assert_eq!(dock, AreaDockTarget::Left);
        assert!((ratio - (0.20 / 0.35)).abs() < 0.01);

        // 2. Direct neighbor towards Top edge (fac_y = 0.125 -> ratio = 0.25) -> Dock Top!
        let (dock, ratio) = calculate_dock_target(&source, &target, Direction::Right, point(px(750.0), px(100.0)), false);
        assert_eq!(dock, AreaDockTarget::Top);
        assert!((ratio - 0.25).abs() < 0.01);

        // 2b. Direct neighbor at top outer edge (fac_y = 0.0 -> ratio = 0.0) -> Dock Top can reach 0%!
        let (dock, ratio) = calculate_dock_target(&source, &target, Direction::Right, point(px(750.0), px(0.0)), false);
        assert_eq!(dock, AreaDockTarget::Top);
        assert_eq!(ratio, 0.0);

        // 3. Direct neighbor towards Bottom edge (1.0 - fac_y = 0.125 -> ratio = 0.25) -> Dock Bottom!
        let (dock, ratio) = calculate_dock_target(&source, &target, Direction::Right, point(px(750.0), px(700.0)), false);
        assert_eq!(dock, AreaDockTarget::Bottom);
        assert!((ratio - 0.25).abs() < 0.01);

        // 4. Direct neighbor towards Right edge (1.0 - fac_x = 0.10 -> ratio = 0.20) -> Dock Right!
        let (dock, ratio) = calculate_dock_target(&source, &target, Direction::Right, point(px(950.0), px(400.0)), false);
        assert_eq!(dock, AreaDockTarget::Right);
        assert!((ratio - 0.20).abs() < 0.01);

        // 5. Direct neighbor Center -> Swap
        let (dock, _) = calculate_dock_target(&source, &target, Direction::Right, point(px(750.0), px(400.0)), false);
        assert_eq!(dock, AreaDockTarget::Center);

        // 6. Non-neighbor panel -> Dock Top
        let non_neighbor = LeafRect {
            id: 3,
            x: 0.0,
            y: 900.0,
            width: 500.0,
            height: 800.0,
        };
        let (dock, ratio) = calculate_dock_target(&source, &non_neighbor, Direction::Down, point(px(250.0), px(950.0)), false);
        assert_eq!(dock, AreaDockTarget::Top);
        assert!((ratio - 0.125).abs() < 0.01);
    }
}
