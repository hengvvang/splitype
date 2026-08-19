//! [`SplitterRoot`] — one initialized split region: the tree of panel
//! containers plus the tree-level state.
//!
//! A root is created when a region is initialized as a split container
//! (the window body below the titlebar, or an editor's inner body).
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
use crate::tree::{LeafRect, Axis, Direction, NodeId, SplitTree};

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
    /// it. `None` when no leaf was activated yet.
    pub active_leaf: Option<NodeId>,
    /// Activated leaf ids in activation-recency order (most recent last).
    /// A leaf whose kind changed is dropped from the history.
    pub activation_history: Vec<NodeId>,
    /// The leaf the mouse is currently operating on.
    pub focused_leaf: Option<NodeId>,
}

impl<T: Copy + PartialEq> SplitterRoot<T> {
    /// A root with one leaf (one panel container), used to seed editor
    /// inner editor bodies.
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

    /// Split `target_id` at `ratio` with a sibling of the SAME kind:
    /// the leaf's container is replaced by a `Split` node holding the
    /// original container and a freshly created one. Returns the new
    /// leaf's id.
    pub fn split_leaf(&mut self, target_id: usize, axis: Axis, ratio: f32) -> Option<usize> {
        let kind = self.tree.find_leaf_kind(target_id)?;
        let new_id = self.next_node_id;
        self.next_node_id += 1;
        self.tree
            .split_leaf_with_ratio(target_id, new_id, axis, ratio, kind);
        self.active_border_menu = None;
        Some(new_id)
    }

    /// Close a leaf; the last leaf is never closable.
    pub fn close_leaf(&mut self, target_id: usize) {
        if self.tree.count_leaves() > 1 {
            self.tree.remove_leaf(target_id);
            self.retire_leaf(target_id);
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
    /// (window coords for the outer layout, the container's local space
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

        if let Some(panel) = self.tree.find_leaf_mut(target_id) {
            panel.active_corner_drag = Some(CornerDragSession {
                target_id,
                start_pos: session.start_pos,
                gesture_dir: Some(dir),
                modifier: session.modifier,
                pointer_pos: Some(current_pos),
                hover_leaf: over_id,
            });
            true
        } else {
            false
        }
    }

    /// Finish the corner-drag gesture on mouse release: returns the final
    /// raw facts of the dragging panel and clears its session. The host
    /// decides what to do with them (split / join / shortcut / nothing).
    pub fn finish_corner_drag(&mut self) -> Option<CornerDragSession> {
        let target_id = self.corner_drag_panel()?;
        self.tree.find_leaf_mut(target_id)?.finish_corner_drag()
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

    /// Get the pixel-space rectangle for a specific leaf, given
    /// pre-computed rects from [`Self::leaf_rects`].
    pub fn leaf_rect(&self, leaf_id: usize, rects: &[LeafRect]) -> Option<LeafRect> {
        rects.iter().find(|rect| rect.id == leaf_id).copied()
    }

    /// Calculate the pixel span (width or height) of a split container.
    pub fn split_pixel_span(&self, split_id: NodeId, container_size: Size<Pixels>) -> Option<f32> {
        let w = f32::from(container_size.width);
        let h = f32::from(container_size.height);
        let (_, span) = self.tree.find_split_span(split_id, 0.0, 0.0, w, h)?;
        Some(span)
    }

    /// Compute the split axis and ratio for a finished corner drag from
    /// the session facts (host-independent geometry). Returns `None` until
    /// the gesture has a direction and a pointer position.
    pub fn corner_split_facts(
        &self,
        facts: &CornerDragSession,
        container_size: Size<Pixels>,
    ) -> Option<(Axis, f32)> {
        let dir = facts.gesture_dir?;
        let pos = facts.pointer_pos?;
        let mut rects = Vec::new();
        self.tree.collect_leaf_rects(0.0, 0.0, 1.0, 1.0, &mut rects);
        let target = rects.iter().find(|rect| rect.id == facts.target_id)?;
        let axis = if dir.is_vertical() {
            Axis::Vertical
        } else {
            Axis::Horizontal
        };
        let norm_x = f32::from(pos.x) / f32::from(container_size.width);
        let norm_y = f32::from(pos.y) / f32::from(container_size.height);
        let ratio = match axis {
            Axis::Horizontal => ((norm_x - target.x) / target.width).clamp(0.15, 0.85),
            Axis::Vertical => ((norm_y - target.y) / target.height).clamp(0.15, 0.85),
        };
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
    fn test_area_layout_suite() {
        let mut root = test_root();
        assert_eq!(root.tree.count_leaves(), 1);

        root.split_leaf(1, Axis::Horizontal, 0.5);
        assert_eq!(root.tree.count_leaves(), 2);

        root.split_leaf(2, Axis::Vertical, 0.5);
        assert_eq!(root.tree.count_leaves(), 3);

        root.close_leaf(2);
        assert_eq!(root.tree.count_leaves(), 2);

        root.set_kind(1, TestKind::B);
        assert_eq!(root.tree.find_leaf_kind(1), Some(TestKind::B));

        root.toggle_maximize(1);
        assert!(root.tree.find_leaf(1).is_some_and(|p| p.maximized));
        root.toggle_maximize(1);
        assert!(root.tree.find_leaf(1).is_some_and(|p| !p.maximized));

        root.active_splitter_drag = Some(SplitterDragSession {
            split_id: 1,
            axis: Axis::Horizontal,
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
        root.split_leaf(1, Axis::Horizontal, 0.5);
        assert_eq!(root.tree.count_leaves(), 2);
        assert_eq!(root.tree.find_leaf_kind(1), Some(TestKind::A));
        assert_eq!(root.tree.find_leaf_kind(2), Some(TestKind::A));

        // B splits into B.
        root.set_kind(1, TestKind::B);
        let new_b = root
            .split_leaf(1, Axis::Horizontal, 0.5)
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
        let new_id = root.split_leaf(1, Axis::Horizontal, 0.5).unwrap();
        assert_eq!(root.tree.count_leaves(), 2);
        assert!(matches!(&root.tree, SplitTree::Split { .. }));
        assert!(root.tree.find_leaf(1).is_some());
        assert!(root.tree.find_leaf(new_id).is_some());
        assert!(root.tree.find_leaf(1).is_some_and(|p| p.kind == TestKind::A));
        assert!(root.tree.find_leaf(new_id).is_some_and(|p| p.kind == TestKind::A));
        // The fresh container starts with no interaction state.
        assert!(root
            .tree
            .find_leaf(new_id)
            .is_some_and(|p| p.active_corner_drag.is_none() && !p.maximized));
    }

    #[test]
    fn test_active_leaf_falls_back_to_last_focused() {
        let mut root = test_root();
        root.activate_leaf(1);
        let a = root.split_leaf(1, Axis::Horizontal, 0.5).unwrap();
        let b = root.split_leaf(1, Axis::Vertical, 0.5).unwrap();
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
        let a = root.split_leaf(1, Axis::Horizontal, 0.5).unwrap();
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
        root.split_leaf(1, Axis::Horizontal, 0.5); // ids: 1, 2
        assert_eq!(root.tree.count_leaves(), 2);

        // Join leaf 2 into leaf 1: remove 2, expand 1.
        let ok = root.join_leaves(1, 2);
        assert!(ok);
        assert_eq!(root.tree.count_leaves(), 1);
        assert_eq!(root.tree.find_leaf_kind(1), Some(TestKind::A));
    }

    #[test]
    fn test_join_nested_leaves() {
        let mut root = test_root();
        // Build: Split(H) { Leaf(1), Split(H) { Leaf(2), Leaf(3) } }
        root.split_leaf(1, Axis::Horizontal, 0.5); // ids: 1, 2
        root.split_leaf(2, Axis::Horizontal, 0.5); // ids: 1, 2, 3
        assert_eq!(root.tree.count_leaves(), 3);

        // Join leaf 1 with leaf 2 (different subtrees) → 2 leaves remain.
        let ok = root.join_leaves(1, 2);
        assert!(ok);
        assert_eq!(root.tree.count_leaves(), 2);
    }

    #[test]
    fn test_window_panel_rects() {
        let mut root = test_root();
        root.split_leaf(1, Axis::Horizontal, 0.3); // ids: 1 (30%), 2 (70%)
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
}
