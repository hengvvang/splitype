//! [`WindowLayout`] — the complete tiled layout state and operations.
//!
//! The outer tree (`window_area_tree`) uses [`WindowAreaKind`] for top-level
//! list. All split / join / swap / drag operations live here; rendering
//! lives in the hosts.

use gpui::{Pixels, Point, Size};

use crate::sessions::{
    BorderMenuState, CornerDragModifier, CornerDragPreview, CornerDragSession,
    MODIFIER_THRESHOLD_PX, SplitterDragSession, WindowAreaDragAction, id_at_point,
};
use crate::tree::{AreaRect, Axis, Direction, SplitTree};
use crate::types::{AreaId, AreaSplitMode, SplitId, WindowAreaKind};

/// Full state for the tiled area layout manager.
pub struct WindowLayout {
    /// Outer tiled layout tree.
    pub window_area_tree: SplitTree<WindowAreaKind>,
    /// Per-Editor-area sessions (tab list + inner panel layout), keyed by
    /// outer area id. Retained for areas that left Editor with tabs.
    pub next_node_id: usize,
    pub open_window_area_dropdown: Option<AreaId>,
    pub maximized_window_area: Option<AreaId>,
    pub active_window_area_splitter_drag: Option<SplitterDragSession>,
    pub active_window_area_corner_drag: Option<CornerDragSession>,
    pub active_window_area_border_menu: Option<BorderMenuState>,
    /// The active editor area: the last Editor area that received focus.
    /// Explorer interactions target its tab list. `None` when no Editor
    /// area exists.
    pub active_editor_area: Option<AreaId>,
    /// Editor area ids in activation-recency order (most recent last). Used
    /// to pick the fallback active editor when the current one is closed.
    pub editor_activation_history: Vec<AreaId>,
    /// The area the mouse is currently operating on.
    pub focused_window_area: Option<AreaId>,
}

/// The id of the root area created by the default layout.
pub const ROOT_AREA_ID: AreaId = 1;

/// The Editor area id of the default layout: the initial split is
/// Explorer (left) + Editor (right), and the split node shares the Editor
/// leaf's id by the tree's split-id convention.
pub const DEFAULT_EDITOR_AREA_ID: AreaId = 2;

impl Default for WindowLayout {
    fn default() -> Self {
        Self {
            window_area_tree: SplitTree::Split {
                id: DEFAULT_EDITOR_AREA_ID,
                direction: Axis::Horizontal,
                ratio: 0.3,
                first: Box::new(SplitTree::Leaf {
                    id: ROOT_AREA_ID,
                    kind: WindowAreaKind::Explorer,
                }),
                second: Box::new(SplitTree::Leaf {
                    id: DEFAULT_EDITOR_AREA_ID,
                    kind: WindowAreaKind::Editor,
                }),
            },
            next_node_id: 3,
            open_window_area_dropdown: None,
            maximized_window_area: None,
            active_window_area_splitter_drag: None,
            active_window_area_corner_drag: None,
            active_window_area_border_menu: None,
            active_editor_area: None,
            editor_activation_history: Vec::new(),
            focused_window_area: None,
        }
    }
}

impl WindowLayout {
    // ------------------------------------------------------------------
    // Split / close / type (outer)
    // ------------------------------------------------------------------

    /// Split `area_id` at `ratio` with a sibling of the SAME kind.
    ///
    /// Tree operation only: the host (WindowPanels) seeds the new Editor
    /// area's session — `AreaSplitMode::Copy` clones the inner panel
    /// layout (the host then deep-copies the tab list), `Fresh` produces a
    /// blank initial-state area. Returns the new area's id.
    pub fn split_window_area(
        &mut self,
        area_id: AreaId,
        direction: Axis,
        ratio: f32,
    ) -> Option<AreaId> {
        let new_id = self.next_node_id;
        self.next_node_id += 1;
        let kind = self
            .window_area_tree
            .find_leaf_kind(area_id)
            .unwrap_or(WindowAreaKind::Editor);
        self.window_area_tree
            .split_leaf_with_ratio(area_id, new_id, direction, ratio, kind);
        self.open_window_area_dropdown = None;
        self.active_window_area_border_menu = None;
        Some(new_id)
    }

    pub fn close_window_area(&mut self, area_id: AreaId) {
        if self.window_area_tree.count_leaves() > 1 {
            self.window_area_tree.remove_leaf(area_id);
            // Clean up the editor session of the removed area.
            if self.maximized_window_area == Some(area_id) {
                self.maximized_window_area = None;
            }
            self.retire_editor_area(area_id);
        }
        self.open_window_area_dropdown = None;
        self.active_window_area_border_menu = None;
    }

    // ------------------------------------------------------------------
    // Editor area activation / tab lists
    // ------------------------------------------------------------------

    /// Mark `area_id` as the active editor: the last Editor area that
    /// received focus. Records the activation for fallback ordering.
    pub fn activate_editor_area(&mut self, area_id: AreaId) {
        self.editor_activation_history.retain(|id| *id != area_id);
        self.editor_activation_history.push(area_id);
        self.active_editor_area = Some(area_id);
    }

    /// Whether the area's current kind is Editor (a foreground editor).
    ///
    /// The foreground/background dimension exists for exactly one reason:
    /// the active-editor rule — only foreground editors can be active, so
    /// explorer file opens never land in a background (retained) session.
    pub fn is_foreground_editor(&self, area_id: AreaId) -> bool {
        self.window_area_tree.find_leaf_kind(area_id) == Some(WindowAreaKind::Editor)
    }

    /// Recompute the active editor after the layout changed: the most
    /// recently focused Editor area still present, or `None`.
    fn recompute_active_editor(&mut self) {
        if let Some(active) = self.active_editor_area {
            if self.is_foreground_editor(active) {
                return;
            }
        }
        self.active_editor_area = self
            .editor_activation_history
            .iter()
            .rev()
            .copied()
            .find(|id| self.is_foreground_editor(*id));
    }

    /// Drop an area from activation tracking and recompute the active editor.
    fn retire_editor_area(&mut self, removed: AreaId) {
        self.editor_activation_history.retain(|id| *id != removed);
        self.recompute_active_editor();
    }

    // ------------------------------------------------------------------
    // Outer area type changes
    // ------------------------------------------------------------------

    pub fn change_window_area_kind(&mut self, area_id: AreaId, kind: WindowAreaKind) {
        let previous = self.window_area_tree.find_leaf_kind(area_id);
        self.window_area_tree.set_leaf_kind(area_id, kind);
        if previous == Some(WindowAreaKind::Editor) && kind != WindowAreaKind::Editor {
            // Leaving Editor: the host (WindowPanels) keeps the session
            // while it still holds tabs and drops it once empty.
            self.retire_editor_area(area_id);
        } else if kind == WindowAreaKind::Editor && previous != Some(WindowAreaKind::Editor) {
            // Entering Editor: an existing background session (tabs) is
            // restored; a fresh area stays blank until its first use.
            // Either way the switch is an explicit interaction, so the
            // area becomes the active editor.
            self.activate_editor_area(area_id);
        }
        self.open_window_area_dropdown = None;
    }

    // ------------------------------------------------------------------
    // Join two adjacent areas
    // ------------------------------------------------------------------

    /// Join `removed` into `into`. The removed area is closed and its
    /// space is absorbed by the `into` area. The two must be adjacent
    /// (share an edge) in the layout.
    pub fn join_window_area(&mut self, into: AreaId, removed: AreaId) -> bool {
        if into == removed {
            return false;
        }
        if self.window_area_tree.count_leaves() <= 1 {
            return false;
        }
        let ok = self.window_area_tree.join_leaf(into, removed);
        if ok {
            // Clean up the editor session of the removed area.
            if self.maximized_window_area == Some(removed) {
                self.maximized_window_area = None;
            }
            self.retire_editor_area(removed);
        }
        self.open_window_area_dropdown = None;
        self.active_window_area_border_menu = None;
        ok
    }

    // ------------------------------------------------------------------
    // Swap two area types
    // ------------------------------------------------------------------

    /// Swap the area kind of area `a` and area `b`.
    pub fn swap_window_area_kinds(&mut self, a: AreaId, b: AreaId) {
        let type_a = self.window_area_tree.find_leaf_kind(a);
        let type_b = self.window_area_tree.find_leaf_kind(b);
        if let (Some(ta), Some(tb)) = (type_a, type_b) {
            self.window_area_tree.set_leaf_kind(a, tb);
            self.window_area_tree.set_leaf_kind(b, ta);
            self.recompute_active_editor();
        }
    }

    // ------------------------------------------------------------------
    // Maximise / dropdown
    // ------------------------------------------------------------------

    pub fn toggle_window_area_maximize(&mut self, area_id: AreaId) {
        if self.maximized_window_area == Some(area_id) {
            self.maximized_window_area = None;
        } else {
            self.maximized_window_area = Some(area_id);
        }
    }

    pub fn toggle_window_area_dropdown(&mut self, area_id: AreaId) {
        if self.open_window_area_dropdown == Some(area_id) {
            self.open_window_area_dropdown = None;
        } else {
            self.open_window_area_dropdown = Some(area_id);
        }
    }

    // ------------------------------------------------------------------
    // Splitter drag
    // ------------------------------------------------------------------

    pub fn update_window_area_splitter_drag(&mut self, current_pointer_pos: f32) {
        if let Some(session) = self.active_window_area_splitter_drag {
            if session.total_span > 1.0 {
                let delta = current_pointer_pos - session.start_pointer_pos;
                let ratio_delta = delta / session.total_span;
                let new_ratio = session.start_ratio + ratio_delta;
                self.window_area_tree
                    .set_split_ratio(session.split_id, new_ratio);
            }
        }
    }

    pub fn end_window_area_splitter_drag(&mut self) {
        self.active_window_area_splitter_drag = None;
    }

    // ------------------------------------------------------------------
    // Corner drag — split / join / swap / duplicate
    // ------------------------------------------------------------------

    /// Begin a corner-drag gesture from `area_id` at `pos` with optional
    /// modifier key.
    pub fn start_window_area_corner_drag(
        &mut self,
        area_id: AreaId,
        pos: Point<Pixels>,
        modifier: CornerDragModifier,
    ) {
        self.active_window_area_corner_drag = Some(CornerDragSession {
            target_id: area_id,
            start_pos: pos,
            gesture_dir: None,
            modifier,
            preview: CornerDragPreview::Dragging,
        });
    }

    /// Process a mouse-move event during a corner drag.
    ///
    /// `current_pos`   – current mouse position in window coords.
    /// `container_size` – size of the tiled-layout container (used for
    ///                    normalised hit-test rects and split-ratio calc).
    ///
    /// Updates the active session's preview state. Modifier-based actions
    /// (Ctrl / Shift) still fire immediately when they cross their threshold
    /// by returning `Some(action)`; the no-modifier split/join path only
    /// updates the preview and always returns `None` so the caller can paint
    /// the live overlay.
    pub fn update_window_area_corner_drag(
        &mut self,
        current_pos: Point<Pixels>,
        container_size: Size<Pixels>,
    ) -> Option<WindowAreaDragAction> {
        let session = match self.active_window_area_corner_drag {
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
        self.active_window_area_corner_drag
            .as_mut()
            .unwrap()
            .gesture_dir = Some(dir);

        // --- Modifier-based actions (Ctrl / Shift) ---
        // Swap (Ctrl) stays immediate: once the threshold is crossed the
        // action is returned so the caller can execute it straight away.
        // Duplicate (Shift) differs per area kind: Explorer falls through
        // to the plain split/join preview (same effect as a normal drag),
        // Settings opens the floating settings window immediately, and
        // Editor previews a fresh-editor split (applied on finish).
        if session.modifier != CornerDragModifier::None {
            if dist < MODIFIER_THRESHOLD_PX {
                return None;
            }
            if session.modifier == CornerDragModifier::Duplicate {
                let kind = self.window_area_tree.find_leaf_kind(session.target_id);
                match kind {
                    Some(WindowAreaKind::Settings) => {
                        return Some(WindowAreaDragAction::OpenSettings);
                    }
                    Some(WindowAreaKind::Editor) | Some(WindowAreaKind::Explorer) => {
                        // Fall through to the no-modifier preview path below.
                    }
                    None => return Some(WindowAreaDragAction::Cancel),
                }
            } else {
                let leaf_rects = self.window_area_rects(container_size);
                let over_id = id_at_point(&leaf_rects, current_pos);

                return Some(match session.modifier {
                    CornerDragModifier::Swap => {
                        if let Some(target) = over_id {
                            if target != session.target_id {
                                WindowAreaDragAction::Swap {
                                    from: session.target_id,
                                    to: target,
                                }
                            } else {
                                WindowAreaDragAction::Cancel
                            }
                        } else {
                            WindowAreaDragAction::Cancel
                        }
                    }
                    CornerDragModifier::None | CornerDragModifier::Duplicate => unreachable!(),
                });
            }
        }

        // --- No modifier: split vs join preview ---
        let leaf_rects = self.window_area_rects(container_size);
        let over_id = id_at_point(&leaf_rects, current_pos);

        if over_id == Some(session.target_id) || over_id.is_none() {
            // Cursor is still in the same area (or outside).  Potential split.
            let split_dir = if dir.is_vertical() {
                Axis::Vertical
            } else {
                Axis::Horizontal
            };
            // Calculate split ratio from cursor position within the leaf.
            if let Some(rect) = self.window_area_rect(session.target_id, &leaf_rects) {
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
                    self.active_window_area_corner_drag
                        .as_mut()
                        .unwrap()
                        .preview = CornerDragPreview::SplitPreview {
                        direction: split_dir,
                        ratio,
                    };
                }
            }
        } else if let Some(target_id) = over_id {
            // Cursor is over a different area.  Potential join.
            self.active_window_area_corner_drag
                .as_mut()
                .unwrap()
                .preview = CornerDragPreview::JoinPreview {
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
    pub fn finish_window_area_corner_drag(&mut self) -> Option<WindowAreaDragAction> {
        let session = self.active_window_area_corner_drag?;
        // Shift + drag on an Editor corner creates a fresh initial-state
        // editor instead of a deep copy of the source area.
        let is_shift_editor = session.modifier == CornerDragModifier::Duplicate
            && self.window_area_tree.find_leaf_kind(session.target_id)
                == Some(WindowAreaKind::Editor);
        let action = match session.preview {
            CornerDragPreview::SplitPreview { direction, ratio } => {
                let mode = if is_shift_editor {
                    AreaSplitMode::Fresh
                } else {
                    AreaSplitMode::Copy
                };
                Some(WindowAreaDragAction::Split {
                    area_id: session.target_id,
                    direction,
                    ratio,
                    mode,
                })
            }
            CornerDragPreview::JoinPreview {
                target_id,
                direction: _,
            } => Some(WindowAreaDragAction::Join {
                from_area: session.target_id,
                into_area: target_id,
            }),
            CornerDragPreview::Dragging => Some(WindowAreaDragAction::Cancel),
        };
        self.end_window_area_corner_drag();
        action
    }

    /// End the corner-drag session, clearing state.
    pub fn end_window_area_corner_drag(&mut self) {
        self.active_window_area_corner_drag = None;
    }

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    /// Collect all leaf rectangles in pixel coordinates.
    pub fn window_area_rects(&self, container_size: Size<Pixels>) -> Vec<AreaRect> {
        let w = f32::from(container_size.width);
        let h = f32::from(container_size.height);
        let mut rects = Vec::new();
        if w > 0.0 && h > 0.0 {
            // Use normalised layout coords, then scale to pixels.
            let mut norm = Vec::new();
            self.window_area_tree
                .collect_leaf_rects(0.0, 0.0, 1.0, 1.0, &mut norm);
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

    /// Get the pixel-space rectangle for a specific area, given pre-computed
    /// area rects from `window_area_rects`.
    pub fn window_area_rect(&self, area_id: AreaId, rects: &[AreaRect]) -> Option<AreaRect> {
        rects.iter().find(|rect| rect.id == area_id).copied()
    }

    /// Calculate the pixel span (width or height) of a split container.
    pub fn window_area_split_pixel_span(
        &self,
        split_id: SplitId,
        container_size: Size<Pixels>,
    ) -> Option<f32> {
        let w = f32::from(container_size.width);
        let h = f32::from(container_size.height);
        if w > 0.0 && h > 0.0 {
            if let Some((dir, span_norm)) = self
                .window_area_tree
                .find_split_span(split_id, 0.0, 0.0, 1.0, 1.0)
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

    pub fn swap_window_area_split_sides(&mut self, split_id: SplitId) {
        self.window_area_tree.swap_sibling_leaves(split_id);
        self.active_window_area_border_menu = None;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{px, size};

    #[test]
    fn test_area_layout_suite() {
        let mut layout = WindowLayout::default();
        // Default: Explorer (1) + Editor (2).
        assert_eq!(layout.window_area_tree.count_leaves(), 2);

        layout.split_window_area(1, Axis::Horizontal, 0.5);
        assert_eq!(layout.window_area_tree.count_leaves(), 3);

        layout.split_window_area(2, Axis::Vertical, 0.5);
        assert_eq!(layout.window_area_tree.count_leaves(), 4);

        layout.close_window_area(2);
        assert_eq!(layout.window_area_tree.count_leaves(), 3);

        layout.change_window_area_kind(1, WindowAreaKind::Explorer);
        assert_eq!(
            layout.window_area_tree.find_leaf_kind(1),
            Some(WindowAreaKind::Explorer)
        );

        layout.toggle_window_area_maximize(1);
        assert_eq!(layout.maximized_window_area, Some(1));
        layout.toggle_window_area_maximize(1);
        assert_eq!(layout.maximized_window_area, None);

        layout.active_window_area_splitter_drag = Some(SplitterDragSession {
            split_id: 1,
            direction: Axis::Horizontal,
            start_pointer_pos: 100.0,
            start_ratio: 0.5,
            total_span: 1000.0,
        });
        layout.update_window_area_splitter_drag(200.0);
        layout.end_window_area_splitter_drag();
        assert_eq!(layout.active_window_area_splitter_drag, None);
    }

    #[test]
    fn test_split_inherits_source_kind() {
        let mut layout = WindowLayout::default();
        // Default: Explorer (1) + Editor (2).
        assert_eq!(
            layout.window_area_tree.find_leaf_kind(1),
            Some(WindowAreaKind::Explorer)
        );
        assert_eq!(
            layout.window_area_tree.find_leaf_kind(2),
            Some(WindowAreaKind::Editor)
        );

        // Split the Explorer leaf → Explorer + Explorer (same kind, not cycled).
        layout.split_window_area(1, Axis::Horizontal, 0.5);
        assert_eq!(layout.window_area_tree.count_leaves(), 3);
        assert_eq!(
            layout.window_area_tree.find_leaf_kind(1),
            Some(WindowAreaKind::Explorer)
        );
        assert_eq!(
            layout.window_area_tree.find_leaf_kind(3),
            Some(WindowAreaKind::Explorer)
        );

        // Settings splits into Settings.
        layout.change_window_area_kind(2, WindowAreaKind::Settings);
        let new_settings = layout
            .split_window_area(2, Axis::Horizontal, 0.5)
            .expect("settings split should succeed");
        assert_eq!(layout.window_area_tree.count_leaves(), 4);
        assert_eq!(
            layout.window_area_tree.find_leaf_kind(new_settings),
            Some(WindowAreaKind::Settings)
        );
    }

    #[test]
    fn test_active_editor_falls_back_to_last_focused() {
        let mut layout = WindowLayout::default();
        // Default: Explorer (1) + Editor (2); drive the Editor side.
        layout.activate_editor_area(2);
        let a = layout.split_window_area(2, Axis::Horizontal, 0.5).unwrap();
        let b = layout.split_window_area(2, Axis::Vertical, 0.5).unwrap();
        // Activation order: 2, a, b → active is b.
        layout.activate_editor_area(a);
        layout.activate_editor_area(b);
        assert_eq!(layout.active_editor_area, Some(b));

        // Close the active editor → falls back to the previous focus (a).
        layout.close_window_area(b);
        assert_eq!(layout.active_editor_area, Some(a));

        // Closing the second-to-last editor falls back to the remaining
        // root area (the last editor is never closable).
        layout.close_window_area(a);
        assert_eq!(layout.active_editor_area, Some(2));
    }

    #[test]
    fn test_join_sibling_leaves() {
        let mut layout = WindowLayout::default();
        // Default layout already has two sibling leaves: Explorer (1) + Editor (2).
        assert_eq!(layout.window_area_tree.count_leaves(), 2);

        // Join leaf 2 into leaf 1: remove 2, expand 1.
        let ok = layout.join_window_area(1, 2);
        assert!(ok);
        assert_eq!(layout.window_area_tree.count_leaves(), 1);
        assert_eq!(
            layout.window_area_tree.find_leaf_kind(1),
            Some(WindowAreaKind::Explorer)
        );
    }

    #[test]
    fn test_join_nested_leaves() {
        let mut layout = WindowLayout::default();
        // Build: Split(H) { Split(H) { Leaf(1), Leaf(3) }, Leaf(2) }
        layout.split_window_area(1, Axis::Horizontal, 0.5); // ids: 1, 3, 2
        assert_eq!(layout.window_area_tree.count_leaves(), 3);

        // Join leaf 1 with leaf 2 (different subtrees) → 2 leaves remain.
        let ok = layout.join_window_area(1, 2);
        assert!(ok);
        assert_eq!(layout.window_area_tree.count_leaves(), 2);
    }

    #[test]
    fn test_window_area_rects() {
        let mut layout = WindowLayout::default();
        // Default: Explorer (1) at 30%, Editor (2) at 70%.
        let rects = layout.window_area_rects(size(px(1000.0), px(800.0)));
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
