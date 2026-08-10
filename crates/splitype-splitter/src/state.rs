//! [`SplitterContainer`] — the generic tiled split layout state and
//! operations.
//!
//! One container model serves both layout levels: the window-level area
//! layout instantiates it with [`WindowAreaKind`] (the default layout
//! seeds Explorer + Editor), and each editor midcontainer instantiates it
//! with `EditorInnerPanelKind`. All split / join / swap / drag operations
//! and the active sessions live here; rendering lives in `interaction`
//! and the hosts.

use gpui::{Pixels, Point, Size};

use crate::sessions::{
    BorderMenuState, CornerDragAction, CornerDragModifier, CornerDragPreview, CornerDragSession,
    MODIFIER_THRESHOLD_PX, SplitterDragSession, id_at_point,
};
use crate::tree::{AreaRect, Axis, Direction, SplitTree};
use crate::types::{AreaSplitMode, NodeId};

/// Kind-dependent behavior of a Shift + corner drag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShiftBehavior {
    /// Fall through to the plain split/join preview; a finished split
    /// either deep-copies the dragged leaf (`fresh: false`, Explorer) or
    /// seeds a blank one (`fresh: true`, Editor).
    Preview { fresh: bool },
    /// Emit a `Duplicate` action immediately (inner editor panels).
    Duplicate,
    /// Emit `Cancel` immediately; the host interprets the shortcut
    /// (outer Settings opens the settings window).
    Cancel,
}

/// Behaviors a leaf kind contributes to the generic container.
pub trait ContainerKind: Copy + PartialEq {
    /// How Shift + corner drag behaves on this kind.
    fn shift_behavior(&self) -> ShiftBehavior;
    /// Whether this kind participates in activation tracking (Editor).
    fn is_editor(&self) -> bool;
}

/// The generic tiled split container: the split tree, its operations, the
/// active drag sessions, and the activation tracking.
///
/// Window-level areas instantiate it with [`WindowAreaKind`]; each editor
/// midcontainer instantiates it with `EditorInnerPanelKind`, so both
/// levels share one interaction model (see `crate::interaction`).
pub struct SplitterContainer<T: ContainerKind> {
    /// The split tree of this container.
    pub tree: SplitTree<T>,
    /// Global id pool shared by leaves and split nodes.
    pub next_node_id: usize,
    pub open_dropdown: Option<usize>,
    pub maximized_area: Option<usize>,
    pub active_splitter_drag: Option<SplitterDragSession>,
    pub active_corner_drag: Option<CornerDragSession>,
    pub active_border_menu: Option<BorderMenuState>,
    /// The most recently activated editor-kind leaf; hosts route global
    /// actions to it. `None` when no editor-kind leaf exists.
    pub active_area: Option<usize>,
    /// Activated leaf ids in activation-recency order (most recent last).
    /// Used to pick the fallback active area when the current one closes.
    pub activation_history: Vec<usize>,
    /// The leaf the mouse is currently operating on.
    pub focused_area: Option<usize>,
}

impl<T: ContainerKind> SplitterContainer<T> {
    /// A container with one leaf, used to seed editor midcontainers.
    pub fn single_leaf(initial_id: usize, kind: T) -> Self {
        Self {
            tree: SplitTree::Leaf {
                id: initial_id,
                kind,
            },
            next_node_id: initial_id + 1,
            open_dropdown: None,
            maximized_area: None,
            active_splitter_drag: None,
            active_corner_drag: None,
            active_border_menu: None,
            active_area: None,
            activation_history: Vec::new(),
            focused_area: None,
        }
    }
}

impl<T: ContainerKind> SplitterContainer<T> {
    // ------------------------------------------------------------------
    // Split / close / kind / join / swap
    // ------------------------------------------------------------------

    /// Split `target_id` at `ratio` with a sibling of the SAME kind.
    /// Returns the new leaf's id.
    pub fn split_leaf(&mut self, target_id: usize, direction: Axis, ratio: f32) -> Option<usize> {
        let kind = self.tree.find_leaf_kind(target_id)?;
        let new_id = self.next_node_id;
        self.next_node_id += 1;
        self.tree
            .split_leaf_with_ratio(target_id, new_id, direction, ratio, kind);
        self.open_dropdown = None;
        self.active_border_menu = None;
        Some(new_id)
    }

    /// Close a leaf; the last leaf is never closable.
    pub fn close_leaf(&mut self, target_id: usize) {
        if self.tree.count_leaves() > 1 {
            self.tree.remove_leaf(target_id);
            if self.maximized_area == Some(target_id) {
                self.maximized_area = None;
            }
            self.retire_area(target_id);
        }
        self.open_dropdown = None;
        self.active_border_menu = None;
    }

    /// Mark `area_id` as the active area (the last editor-kind leaf that
    /// received focus). Records the activation for fallback ordering.
    pub fn activate_area(&mut self, area_id: usize) {
        self.activation_history.retain(|id| *id != area_id);
        self.activation_history.push(area_id);
        self.active_area = Some(area_id);
    }

    /// Whether the leaf's current kind is an editor kind.
    pub fn is_editor_area(&self, area_id: usize) -> bool {
        self.tree
            .find_leaf_kind(area_id)
            .is_some_and(|kind| kind.is_editor())
    }

    /// Recompute the active area after the layout changed: the most
    /// recently activated editor-kind leaf still present, or `None`.
    fn recompute_active_area(&mut self) {
        if let Some(active) = self.active_area {
            if self.is_editor_area(active) {
                return;
            }
        }
        self.active_area = self
            .activation_history
            .iter()
            .rev()
            .copied()
            .find(|id| self.is_editor_area(*id));
    }

    /// Drop a leaf from activation tracking and recompute the active area.
    fn retire_area(&mut self, removed: usize) {
        self.activation_history.retain(|id| *id != removed);
        self.recompute_active_area();
    }

    /// Change a leaf's kind. Leaving an editor kind retires the leaf from
    /// activation tracking; entering one activates it.
    pub fn set_kind(&mut self, area_id: usize, kind: T) {
        let previous = self.tree.find_leaf_kind(area_id);
        self.tree.set_leaf_kind(area_id, kind);
        if previous.is_some_and(|k| k.is_editor()) && !kind.is_editor() {
            self.retire_area(area_id);
        } else if kind.is_editor() && !previous.is_some_and(|k| k.is_editor()) {
            self.activate_area(area_id);
        }
        self.open_dropdown = None;
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
            if self.maximized_area == Some(removed) {
                self.maximized_area = None;
            }
            self.retire_area(removed);
        }
        self.open_dropdown = None;
        self.active_border_menu = None;
        ok
    }

    /// Swap the kind of leaf `a` and leaf `b`.
    pub fn swap_kinds(&mut self, a: usize, b: usize) {
        let type_a = self.tree.find_leaf_kind(a);
        let type_b = self.tree.find_leaf_kind(b);
        if let (Some(ta), Some(tb)) = (type_a, type_b) {
            self.tree.set_leaf_kind(a, tb);
            self.tree.set_leaf_kind(b, ta);
            self.recompute_active_area();
        }
    }

    // ------------------------------------------------------------------
    // Maximise / dropdown
    // ------------------------------------------------------------------

    pub fn toggle_maximize(&mut self, area_id: usize) {
        if self.maximized_area == Some(area_id) {
            self.maximized_area = None;
        } else {
            self.maximized_area = Some(area_id);
        }
    }

    pub fn toggle_dropdown(&mut self, area_id: usize) {
        if self.open_dropdown == Some(area_id) {
            self.open_dropdown = None;
        } else {
            self.open_dropdown = Some(area_id);
        }
    }

    // ------------------------------------------------------------------
    // Splitter drag
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
    // Corner drag — split / join / swap / duplicate
    // ------------------------------------------------------------------

    /// Begin a corner-drag gesture from `target_id` at `pos` with an
    /// optional modifier key.
    pub fn start_corner_drag(
        &mut self,
        target_id: usize,
        pos: Point<Pixels>,
        modifier: CornerDragModifier,
    ) {
        self.active_corner_drag = Some(CornerDragSession {
            target_id,
            start_pos: pos,
            gesture_dir: None,
            modifier,
            preview: CornerDragPreview::Dragging,
        });
    }

    /// Process a mouse-move event during a corner drag.
    ///
    /// `current_pos` and the container size must share a coordinate system
    /// (the host passes the pointer and size in the container's own space —
    /// window coords for the outer layout, the midcontainer's local space
    /// for an editor). Modifier-based actions (Ctrl / Shift) fire
    /// immediately when they cross their threshold; the no-modifier
    /// split/join path only updates the preview and returns `None` so the
    /// caller can paint the live overlay.
    pub fn update_corner_drag(
        &mut self,
        current_pos: Point<Pixels>,
        container_size: Size<Pixels>,
    ) -> Option<CornerDragAction> {
        let session = match self.active_corner_drag {
            Some(ref s) => *s,
            None => return None,
        };

        let dx = f32::from(current_pos.x - session.start_pos.x);
        let dy = f32::from(current_pos.y - session.start_pos.y);
        let dist = (dx * dx + dy * dy).sqrt();
        let abs_dx = dx.abs();
        let abs_dy = dy.abs();

        // Determine cardinal direction.
        let dir = if abs_dy > abs_dx {
            if dy > 0.0 {
                Direction::Down
            } else {
                Direction::Up
            }
        } else {
            if dx > 0.0 {
                Direction::Right
            } else {
                Direction::Left
            }
        };

        // Update gesture direction for cursor feedback.
        self.active_corner_drag.as_mut().unwrap().gesture_dir = Some(dir);

        // --- Modifier-based actions (Ctrl / Shift) ---
        // Swap (Ctrl) stays immediate: once the threshold is crossed the
        // action is returned so the caller can execute it straight away.
        // Duplicate (Shift) is kind-dependent: inner panels emit a
        // Duplicate action, outer Settings cancels (the host opens the
        // settings window), and outer Editor/Explorer fall through to the
        // plain split/join preview.
        if session.modifier != CornerDragModifier::None {
            if dist < MODIFIER_THRESHOLD_PX {
                return None;
            }
            match session.modifier {
                CornerDragModifier::Swap => {
                    let leaf_rects = self.leaf_rects(container_size);
                    let over_id = id_at_point(&leaf_rects, current_pos);
                    return Some(if let Some(target) = over_id {
                        if target != session.target_id {
                            CornerDragAction::Swap {
                                from: session.target_id,
                                to: target,
                            }
                        } else {
                            CornerDragAction::Cancel
                        }
                    } else {
                        CornerDragAction::Cancel
                    });
                }
                CornerDragModifier::Duplicate => {
                    match self
                        .tree
                        .find_leaf_kind(session.target_id)
                        .map(|kind| kind.shift_behavior())
                    {
                        Some(ShiftBehavior::Duplicate) => {
                            return Some(CornerDragAction::Duplicate {
                                target_id: session.target_id,
                            });
                        }
                        Some(ShiftBehavior::Cancel) => {
                            return Some(CornerDragAction::Cancel);
                        }
                        Some(ShiftBehavior::Preview { .. }) => {
                            // Fall through to the preview path below.
                        }
                        None => return Some(CornerDragAction::Cancel),
                    }
                }
                CornerDragModifier::None => unreachable!(),
            }
        }

        // --- No modifier: split vs join preview ---
        let leaf_rects = self.leaf_rects(container_size);
        let over_id = id_at_point(&leaf_rects, current_pos);

        if over_id == Some(session.target_id) || over_id.is_none() {
            // Cursor is still in the same leaf (or outside). Potential split.
            let split_dir = if dir.is_vertical() {
                Axis::Vertical
            } else {
                Axis::Horizontal
            };
            // Calculate split ratio from cursor position within the leaf.
            if let Some(rect) = self.leaf_rect(session.target_id, &leaf_rects) {
                if rect.width > 1.0 && rect.height > 1.0 {
                    let ratio = match split_dir {
                        Axis::Horizontal => {
                            let r = (f32::from(current_pos.x) - rect.x) / rect.width;
                            r.clamp(0.15, 0.85)
                        }
                        Axis::Vertical => {
                            let r = (f32::from(current_pos.y) - rect.y) / rect.height;
                            r.clamp(0.15, 0.85)
                        }
                    };
                    self.active_corner_drag.as_mut().unwrap().preview =
                        CornerDragPreview::SplitPreview {
                            direction: split_dir,
                            ratio,
                        };
                }
            }
        } else if let Some(target_id) = over_id {
            // Cursor is over a different leaf. Potential join.
            self.active_corner_drag.as_mut().unwrap().preview = CornerDragPreview::JoinPreview {
                target_id,
                direction: dir,
            };
        }

        None
    }

    /// Finish the corner-drag gesture on mouse release.
    ///
    /// Reads the active session's preview state and returns the appropriate
    /// action, then clears the session.
    pub fn finish_corner_drag(&mut self) -> Option<CornerDragAction> {
        let session = self.active_corner_drag?;
        // Shift + drag on a `Preview { fresh: true }` kind (outer Editor)
        // creates a fresh initial-state leaf instead of a deep copy.
        let fresh = session.modifier == CornerDragModifier::Duplicate
            && self
                .tree
                .find_leaf_kind(session.target_id)
                .is_some_and(|kind| {
                    matches!(
                        kind.shift_behavior(),
                        ShiftBehavior::Preview { fresh: true }
                    )
                });
        let action = match session.preview {
            CornerDragPreview::SplitPreview { direction, ratio } => {
                let mode = if fresh {
                    AreaSplitMode::Fresh
                } else {
                    AreaSplitMode::Copy
                };
                CornerDragAction::Split {
                    target_id: session.target_id,
                    direction,
                    ratio,
                    mode,
                }
            }
            CornerDragPreview::JoinPreview { target_id, .. } => CornerDragAction::Join {
                from: session.target_id,
                into: target_id,
            },
            CornerDragPreview::Dragging => CornerDragAction::Cancel,
        };
        self.end_corner_drag();
        Some(action)
    }

    /// End the corner-drag session, clearing state.
    pub fn end_corner_drag(&mut self) {
        self.active_corner_drag = None;
    }

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    /// Collect all leaf rectangles in pixel coordinates.
    pub fn leaf_rects(&self, container_size: Size<Pixels>) -> Vec<AreaRect> {
        let w = f32::from(container_size.width);
        let h = f32::from(container_size.height);
        let mut rects = Vec::new();
        if w > 0.0 && h > 0.0 {
            // Use normalised layout coords, then scale to pixels.
            let mut norm = Vec::new();
            self.tree.collect_leaf_rects(0.0, 0.0, 1.0, 1.0, &mut norm);
            for rect in norm {
                rects.push(AreaRect {
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
    pub fn leaf_rect(&self, area_id: usize, rects: &[AreaRect]) -> Option<AreaRect> {
        rects.iter().find(|rect| rect.id == area_id).copied()
    }

    /// Calculate the pixel span (width or height) of a split container.
    pub fn split_pixel_span(&self, split_id: NodeId, container_size: Size<Pixels>) -> Option<f32> {
        let w = f32::from(container_size.width);
        let h = f32::from(container_size.height);
        if w > 0.0 && h > 0.0 {
            if let Some((dir, span_norm)) = self.tree.find_split_span(split_id, 0.0, 0.0, 1.0, 1.0)
            {
                let pixel_span = match dir {
                    Axis::Horizontal => span_norm * w,
                    Axis::Vertical => span_norm * h,
                };
                return Some(pixel_span);
            }
        }
        None
    }

    // ------------------------------------------------------------------
    // Border context menu
    // ------------------------------------------------------------------

    pub fn swap_split_sides(&mut self, split_id: NodeId) {
        self.tree.swap_sibling_leaves(split_id);
        self.active_border_menu = None;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{px, size};

    /// A stand-in kind: the engine must not know any concrete application
    /// kind, so the tests register their own.
    #[derive(Clone, Copy, Debug, PartialEq)]
    enum TestKind {
        A,
        B,
    }

    impl ContainerKind for TestKind {
        fn shift_behavior(&self) -> ShiftBehavior {
            match self {
                Self::A => ShiftBehavior::Preview { fresh: true },
                Self::B => ShiftBehavior::Duplicate,
            }
        }

        fn is_editor(&self) -> bool {
            matches!(self, Self::A)
        }
    }

    fn test_layout() -> SplitterContainer<TestKind> {
        SplitterContainer::single_leaf(1, TestKind::A)
    }

    #[test]
    fn test_area_layout_suite() {
        let mut layout = test_layout();
        assert_eq!(layout.tree.count_leaves(), 1);

        layout.split_leaf(1, Axis::Horizontal, 0.5);
        assert_eq!(layout.tree.count_leaves(), 2);

        layout.split_leaf(2, Axis::Vertical, 0.5);
        assert_eq!(layout.tree.count_leaves(), 3);

        layout.close_leaf(2);
        assert_eq!(layout.tree.count_leaves(), 2);

        layout.set_kind(1, TestKind::B);
        assert_eq!(layout.tree.find_leaf_kind(1), Some(TestKind::B));

        layout.toggle_maximize(1);
        assert_eq!(layout.maximized_area, Some(1));
        layout.toggle_maximize(1);
        assert_eq!(layout.maximized_area, None);

        layout.active_splitter_drag = Some(SplitterDragSession {
            split_id: 1,
            direction: Axis::Horizontal,
            start_pointer_pos: 100.0,
            start_ratio: 0.5,
            total_span: 1000.0,
        });
        layout.update_splitter_drag(200.0);
        layout.end_splitter_drag();
        assert_eq!(layout.active_splitter_drag, None);
    }

    #[test]
    fn test_split_inherits_source_kind() {
        let mut layout = test_layout();
        assert_eq!(layout.tree.find_leaf_kind(1), Some(TestKind::A));

        // Split leaf 1 → A + A (same kind, not cycled).
        layout.split_leaf(1, Axis::Horizontal, 0.5);
        assert_eq!(layout.tree.count_leaves(), 2);
        assert_eq!(layout.tree.find_leaf_kind(1), Some(TestKind::A));
        assert_eq!(layout.tree.find_leaf_kind(2), Some(TestKind::A));

        // B splits into B.
        layout.set_kind(1, TestKind::B);
        let new_b = layout
            .split_leaf(1, Axis::Horizontal, 0.5)
            .expect("second split should succeed");
        assert_eq!(layout.tree.count_leaves(), 3);
        assert_eq!(layout.tree.find_leaf_kind(new_b), Some(TestKind::B));
    }

    #[test]
    fn test_active_editor_falls_back_to_last_focused() {
        let mut layout = test_layout();
        // A is the editor-like kind.
        layout.activate_area(1);
        let a = layout.split_leaf(1, Axis::Horizontal, 0.5).unwrap();
        let b = layout.split_leaf(1, Axis::Vertical, 0.5).unwrap();
        // Activation order: 1, a, b → active is b.
        layout.activate_area(a);
        layout.activate_area(b);
        assert_eq!(layout.active_area, Some(b));

        // Close the active editor → falls back to the previous focus (a).
        layout.close_leaf(b);
        assert_eq!(layout.active_area, Some(a));

        // Closing the second-to-last editor falls back to the remaining
        // root area (the last editor is never closable).
        layout.close_leaf(a);
        assert_eq!(layout.active_area, Some(1));
    }

    #[test]
    fn test_join_sibling_leaves() {
        let mut layout = test_layout();
        layout.split_leaf(1, Axis::Horizontal, 0.5); // ids: 1, 2
        assert_eq!(layout.tree.count_leaves(), 2);

        // Join leaf 2 into leaf 1: remove 2, expand 1.
        let ok = layout.join_leaves(1, 2);
        assert!(ok);
        assert_eq!(layout.tree.count_leaves(), 1);
        assert_eq!(layout.tree.find_leaf_kind(1), Some(TestKind::A));
    }

    #[test]
    fn test_join_nested_leaves() {
        let mut layout = test_layout();
        // Build: Split(H) { Leaf(1), Split(H) { Leaf(2), Leaf(3) } }
        layout.split_leaf(1, Axis::Horizontal, 0.5); // ids: 1, 2
        layout.split_leaf(2, Axis::Horizontal, 0.5); // ids: 1, 2, 3
        assert_eq!(layout.tree.count_leaves(), 3);

        // Join leaf 1 with leaf 2 (different subtrees) → 2 leaves remain.
        let ok = layout.join_leaves(1, 2);
        assert!(ok);
        assert_eq!(layout.tree.count_leaves(), 2);
    }

    #[test]
    fn test_window_area_rects() {
        let mut layout = test_layout();
        layout.split_leaf(1, Axis::Horizontal, 0.3); // ids: 1 (30%), 2 (70%)
        let rects = layout.leaf_rects(size(px(1000.0), px(800.0)));
        assert_eq!(rects.len(), 2);
        let first = rects[0];
        let second = rects[1];
        assert!((first.width - 300.0).abs() < 1.0);
        assert!((second.width - 700.0).abs() < 1.0);
        assert!((first.height - 800.0).abs() < 1.0);
        assert!((second.height - 800.0).abs() < 1.0);
        assert!((second.x - 300.0).abs() < 1.0);
    }
}
