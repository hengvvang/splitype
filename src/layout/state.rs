//! [`WindowLayout`] — the complete tiled layout state and operations.
//!
//! The outer tree (`window_area_tree`) uses [`WindowAreaKind`] for top-level
//! areas. Inner trees (`editor_inner_panel_trees`) use [`EditorInnerPanelKind`]
//! for sub-panels inside Edit areas. All split / join / swap / drag
//! operations live here; rendering lives in the hosts.

use std::collections::HashMap;

use gpui::{Pixels, Point, Size};

use crate::layout::sessions::{
    leaf_id_at_point, BorderMenuState, CornerDragAction, CornerDragModifier, CornerDragPreview,
    CornerDragSession, SplitterDragSession, MODIFIER_THRESHOLD_PX,
};
use crate::layout::tree::{AreaRect, Axis, Direction, SplitTree};
use crate::layout::types::{
    AreaId, EditorInnerPanelKind, InnerPanelLocation, PanelId, SplitId, WindowAreaKind,
};

/// Full state for the tiled area layout manager.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowLayout {
    /// Outer tiled layout tree.
    pub window_area_tree: SplitTree<WindowAreaKind>,
    /// Per-Editor inner panel split trees, keyed by outer area id.
    pub editor_inner_panel_trees: HashMap<AreaId, SplitTree<EditorInnerPanelKind>>,
    /// Global node id pool (leaves and split nodes share it).
    pub next_node_id: usize,
    pub open_window_area_dropdown: Option<AreaId>,
    pub open_editor_inner_panel_dropdown: Option<InnerPanelLocation>,
    pub maximized_window_area: Option<AreaId>,
    pub active_window_area_splitter_drag: Option<SplitterDragSession>,
    pub active_window_area_corner_drag: Option<CornerDragSession>,
    pub active_window_area_border_menu: Option<BorderMenuState>,
    pub active_editor_inner_panel_splitter_drag: Option<(AreaId, SplitterDragSession)>,
    pub active_editor_inner_panel_corner_drag: Option<(AreaId, CornerDragSession)>,
    pub active_editor_inner_panel_border_menu: Option<BorderMenuState>,
    /// Currently focused inner panel — the status-bar action target.
    pub focused_editor_inner_panel: Option<InnerPanelLocation>,
    /// Measured pixel size of the tiled-layout container.
    pub container_size: Option<Size<Pixels>>,
}

/// The id of the root area created by the default layout.
const ROOT_AREA_ID: AreaId = 1;

impl Default for WindowLayout {
    fn default() -> Self {
        Self {
            window_area_tree: SplitTree::Leaf {
                id: ROOT_AREA_ID,
                kind: WindowAreaKind::Editor,
            },
            editor_inner_panel_trees: HashMap::new(),
            next_node_id: 2,
            open_window_area_dropdown: None,
            open_editor_inner_panel_dropdown: None,
            maximized_window_area: None,
            active_window_area_splitter_drag: None,
            active_window_area_corner_drag: None,
            active_window_area_border_menu: None,
            active_editor_inner_panel_splitter_drag: None,
            active_editor_inner_panel_corner_drag: None,
            active_editor_inner_panel_border_menu: None,
            focused_editor_inner_panel: None,
            container_size: None,
        }
    }
}

impl WindowLayout {
    // ------------------------------------------------------------------
    // Split / close / type (outer)
    // ------------------------------------------------------------------

    /// Compute the next outer `WindowAreaKind` when splitting a leaf.
    fn next_window_area_kind(current: WindowAreaKind) -> WindowAreaKind {
        match current {
            WindowAreaKind::Editor => WindowAreaKind::Explorer,
            WindowAreaKind::Explorer => WindowAreaKind::Settings,
            WindowAreaKind::Settings => WindowAreaKind::Editor,
        }
    }

    pub fn split_window_area(&mut self, area_id: AreaId, direction: Axis) {
        let new_id = self.next_node_id;
        self.next_node_id += 1;
        let next_type = self
            .window_area_tree
            .find_leaf_kind(area_id)
            .map(Self::next_window_area_kind)
            .unwrap_or(WindowAreaKind::Editor);
        self.window_area_tree
            .split_leaf(area_id, new_id, direction, next_type);
        self.open_window_area_dropdown = None;
        self.active_window_area_border_menu = None;
    }

    /// Split a leaf area at a specific ratio.
    pub fn split_window_area_with_ratio(&mut self, area_id: AreaId, direction: Axis, ratio: f32) {
        let new_id = self.next_node_id;
        self.next_node_id += 1;
        let next_type = self
            .window_area_tree
            .find_leaf_kind(area_id)
            .map(Self::next_window_area_kind)
            .unwrap_or(WindowAreaKind::Editor);
        self.window_area_tree
            .split_leaf_with_ratio(area_id, new_id, direction, ratio, next_type);
        self.open_window_area_dropdown = None;
        self.active_window_area_border_menu = None;
    }

    pub fn close_window_area(&mut self, area_id: AreaId) {
        if self.window_area_tree.count_leaves() > 1 {
            self.window_area_tree.remove_leaf(area_id);
            // Clean up inner trees for the removed Edit area.
            self.editor_inner_panel_trees.remove(&area_id);
            if self.maximized_window_area == Some(area_id) {
                self.maximized_window_area = None;
            }
        }
        self.open_window_area_dropdown = None;
        self.active_window_area_border_menu = None;
    }

    // ------------------------------------------------------------------
    // Inner Edit sub-panel layout
    // ------------------------------------------------------------------

    /// Get or create the inner panel tree for an Edit area.
    /// New Edit areas default to a single `EditorInnerPanelKind::SourceCode` panel.
    pub fn ensure_editor_inner_panel_tree(
        &mut self,
        area_id: AreaId,
    ) -> &mut SplitTree<EditorInnerPanelKind> {
        let next_node_id = &mut self.next_node_id;
        self.editor_inner_panel_trees
            .entry(area_id)
            .or_insert_with(|| {
                let panel_id = *next_node_id;
                *next_node_id += 1;
                SplitTree::Leaf {
                    id: panel_id,
                    kind: EditorInnerPanelKind::SourceCode,
                }
            })
    }

    /// Splits an inner panel via the status-bar buttons. The new panel
    /// inherits the target panel's kind so the split keeps the same view
    /// style; falls back to `SourceCode` if the target is unknown.
    pub fn split_editor_inner_panel(
        &mut self,
        area_id: AreaId,
        panel_id: PanelId,
        direction: Axis,
    ) {
        let new_id = self.next_node_id;
        self.next_node_id += 1;
        let root = self.ensure_editor_inner_panel_tree(area_id);
        let kind = root
            .find_leaf_kind(panel_id)
            .unwrap_or(EditorInnerPanelKind::SourceCode);
        root.split_leaf_with_ratio(panel_id, new_id, direction, 0.5, kind);
    }

    pub fn close_editor_inner_panel(&mut self, area_id: AreaId, panel_id: PanelId) {
        if let Some(root) = self.editor_inner_panel_trees.get_mut(&area_id) {
            if root.count_leaves() > 1 {
                root.remove_leaf(panel_id);
            }
        }
    }

    pub fn toggle_editor_inner_panel_dropdown(&mut self, area_id: AreaId, panel_id: PanelId) {
        let location = InnerPanelLocation { area_id, panel_id };
        if self.open_editor_inner_panel_dropdown == Some(location) {
            self.open_editor_inner_panel_dropdown = None;
        } else {
            self.open_editor_inner_panel_dropdown = Some(location);
            self.open_window_area_dropdown = None;
        }
    }

    pub fn change_editor_inner_panel_kind(
        &mut self,
        area_id: AreaId,
        panel_id: PanelId,
        kind: EditorInnerPanelKind,
    ) {
        let root = self.ensure_editor_inner_panel_tree(area_id);
        root.set_leaf_kind(panel_id, kind);
        self.open_editor_inner_panel_dropdown = None;
    }

    /// Inner split created via corner drag. The new panel inherits the
    /// dragged panel's kind so both sides keep the same view style; falls
    /// back to `SourceCode` if the target is unknown.
    pub fn split_editor_inner_panel_with_ratio(
        &mut self,
        area_id: AreaId,
        panel_id: PanelId,
        direction: Axis,
        ratio: f32,
    ) {
        let new_id = self.next_node_id;
        self.next_node_id += 1;
        if let Some(root) = self.editor_inner_panel_trees.get_mut(&area_id) {
            let kind = root
                .find_leaf_kind(panel_id)
                .unwrap_or(EditorInnerPanelKind::SourceCode);
            root.split_leaf_with_ratio(panel_id, new_id, direction, ratio, kind);
        }
    }

    /// Join an inner panel into another within the same inner tree.
    pub fn join_editor_inner_panel(
        &mut self,
        area_id: AreaId,
        into: PanelId,
        removed: PanelId,
    ) -> bool {
        if into == removed {
            return false;
        }
        if let Some(root) = self.editor_inner_panel_trees.get_mut(&area_id) {
            if root.count_leaves() <= 1 {
                return false;
            }
            root.join_leaf(into, removed)
        } else {
            false
        }
    }

    /// Swap area types between two inner panels.
    pub fn swap_editor_inner_panel_kinds(&mut self, area_id: AreaId, a: PanelId, b: PanelId) {
        if let Some(root) = self.editor_inner_panel_trees.get_mut(&area_id) {
            let type_a = root.find_leaf_kind(a);
            let type_b = root.find_leaf_kind(b);
            if let (Some(ta), Some(tb)) = (type_a, type_b) {
                root.set_leaf_kind(a, tb);
                root.set_leaf_kind(b, ta);
            }
        }
    }

    // ------------------------------------------------------------------
    // Inner splitter drag
    // ------------------------------------------------------------------

    pub fn update_editor_inner_panel_splitter_drag(
        &mut self,
        area_id: AreaId,
        current_pointer_pos: f32,
    ) {
        if let Some((_area_id, session)) = self.active_editor_inner_panel_splitter_drag {
            if session.total_span > 1.0 {
                let delta = current_pointer_pos - session.start_pointer_pos;
                let ratio_delta = delta / session.total_span;
                let new_ratio = session.start_ratio + ratio_delta;
                if let Some(root) = self.editor_inner_panel_trees.get_mut(&area_id) {
                    root.set_split_ratio(session.split_id, new_ratio);
                }
            }
        }
    }

    pub fn end_editor_inner_panel_splitter_drag(&mut self) {
        self.active_editor_inner_panel_splitter_drag = None;
    }

    // ------------------------------------------------------------------
    // Inner corner drag
    // ------------------------------------------------------------------

    pub fn start_editor_inner_panel_corner_drag(
        &mut self,
        area_id: AreaId,
        panel_id: PanelId,
        pos: Point<Pixels>,
        modifier: CornerDragModifier,
    ) {
        self.active_editor_inner_panel_corner_drag = Some((
            area_id,
            CornerDragSession {
                leaf_id: panel_id,
                start_pos: pos,
                gesture_dir: None,
                modifier,
                preview: CornerDragPreview::Dragging,
            },
        ));
    }

    pub fn update_editor_inner_panel_corner_drag(
        &mut self,
        area_id: AreaId,
        current_pos: Point<Pixels>,
        container_size: Size<Pixels>,
    ) -> Option<CornerDragAction> {
        let (stored_area_id, session) = match self.active_editor_inner_panel_corner_drag {
            Some(ref s) => (s.0, s.1),
            None => return None,
        };
        debug_assert_eq!(
            stored_area_id, area_id,
            "area_id mismatch in inner corner drag"
        );

        let dx = f32::from(current_pos.x - session.start_pos.x);
        let dy = f32::from(current_pos.y - session.start_pos.y);
        let dist = (dx * dx + dy * dy).sqrt();
        let abs_dx = dx.abs();
        let abs_dy = dy.abs();

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

        self.active_editor_inner_panel_corner_drag
            .as_mut()
            .unwrap()
            .1
            .gesture_dir = Some(dir);

        if session.modifier != CornerDragModifier::None {
            if dist < MODIFIER_THRESHOLD_PX {
                return None;
            }
            let leaf_rects = self.editor_inner_panel_rects(area_id, container_size);
            let over_id = leaf_id_at_point(&leaf_rects, current_pos);

            return Some(match session.modifier {
                CornerDragModifier::Swap => {
                    if let Some(target) = over_id {
                        if target != session.leaf_id {
                            CornerDragAction::Swap {
                                from: session.leaf_id,
                                to: target,
                            }
                        } else {
                            CornerDragAction::Cancel
                        }
                    } else {
                        CornerDragAction::Cancel
                    }
                }
                CornerDragModifier::Duplicate => CornerDragAction::Duplicate {
                    leaf_id: session.leaf_id,
                },
                CornerDragModifier::None => unreachable!(),
            });
        }

        let leaf_rects = self.editor_inner_panel_rects(area_id, container_size);
        let over_id = leaf_id_at_point(&leaf_rects, current_pos);

        if over_id == Some(session.leaf_id) || over_id.is_none() {
            // Cursor is still in the same area (or outside).  Potential split.
            // The new panel inherits the dragged panel's kind (applied on finish).
            let split_dir = if dir.is_vertical() {
                Axis::Vertical
            } else {
                Axis::Horizontal
            };
            if let Some(rect) = self.window_area_rect(session.leaf_id, &leaf_rects) {
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
                    self.active_editor_inner_panel_corner_drag
                        .as_mut()
                        .unwrap()
                        .1
                        .preview = CornerDragPreview::SplitPreview {
                        direction: split_dir,
                        ratio,
                    };
                }
            }
        } else if let Some(target_id) = over_id {
            // Cursor is over a different area.  Potential join.
            self.active_editor_inner_panel_corner_drag
                .as_mut()
                .unwrap()
                .1
                .preview = CornerDragPreview::JoinPreview {
                target_leaf_id: target_id,
                direction: dir,
            };
        }

        None
    }

    pub fn finish_editor_inner_panel_corner_drag(&mut self) -> Option<(AreaId, CornerDragAction)> {
        let (area_id, session) = self.active_editor_inner_panel_corner_drag?;
        let action = match session.preview {
            CornerDragPreview::SplitPreview { direction, ratio } => Some(CornerDragAction::Split {
                leaf_id: session.leaf_id,
                direction,
                ratio,
            }),
            CornerDragPreview::JoinPreview {
                target_leaf_id,
                direction: _,
            } => Some(CornerDragAction::Join {
                from: session.leaf_id,
                into: target_leaf_id,
            }),
            CornerDragPreview::Dragging => Some(CornerDragAction::Cancel),
        };
        self.end_editor_inner_panel_corner_drag();
        action.map(|a| (area_id, a))
    }

    pub fn end_editor_inner_panel_corner_drag(&mut self) {
        self.active_editor_inner_panel_corner_drag = None;
    }

    // ------------------------------------------------------------------
    // Inner layout helpers
    // ------------------------------------------------------------------

    /// Collect all inner panel rectangles in pixel coordinates for a given Edit area.
    pub fn editor_inner_panel_rects(
        &self,
        area_id: AreaId,
        container_size: Size<Pixels>,
    ) -> Vec<AreaRect> {
        let w = f32::from(container_size.width);
        let h = f32::from(container_size.height);
        let mut rects = Vec::new();
        if w > 0.0 && h > 0.0 {
            if let Some(root) = self.editor_inner_panel_trees.get(&area_id) {
                let mut norm = Vec::new();
                root.collect_leaf_rects(0.0, 0.0, 1.0, 1.0, &mut norm);
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
        }
        rects
    }

    /// Calculate the pixel span of an inner split container.
    pub fn editor_inner_panel_split_pixel_span(
        &self,
        area_id: AreaId,
        split_id: SplitId,
        container_size: Size<Pixels>,
    ) -> Option<f32> {
        let w = f32::from(container_size.width);
        let h = f32::from(container_size.height);
        if w > 0.0 && h > 0.0 {
            if let Some(root) = self.editor_inner_panel_trees.get(&area_id) {
                if let Some((dir, span_norm)) = root.find_split_span(split_id, 0.0, 0.0, 1.0, 1.0) {
                    let pixel_span = match dir {
                        Axis::Horizontal => span_norm * w,
                        Axis::Vertical => span_norm * h,
                    };
                    return Some(pixel_span);
                }
            }
        }
        None
    }

    // ------------------------------------------------------------------
    // Outer area type changes
    // ------------------------------------------------------------------

    pub fn change_window_area_kind(&mut self, area_id: AreaId, kind: WindowAreaKind) {
        self.window_area_tree.set_leaf_kind(area_id, kind);
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
            // Clean up inner trees for the removed Edit area.
            self.editor_inner_panel_trees.remove(&removed);
            if self.maximized_window_area == Some(removed) {
                self.maximized_window_area = None;
            }
        }
        self.open_window_area_dropdown = None;
        self.active_window_area_border_menu = None;
        ok
    }

    // ------------------------------------------------------------------
    // Swap two area types
    // ------------------------------------------------------------------

    /// Swap the area type of area `a` and area `b`.
    pub fn swap_window_area_kinds(&mut self, a: AreaId, b: AreaId) {
        let type_a = self.window_area_tree.find_leaf_kind(a);
        let type_b = self.window_area_tree.find_leaf_kind(b);
        if let (Some(ta), Some(tb)) = (type_a, type_b) {
            self.window_area_tree.set_leaf_kind(a, tb);
            self.window_area_tree.set_leaf_kind(b, ta);
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
            leaf_id: area_id,
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
    ) -> Option<CornerDragAction> {
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
        // These remain immediate: once the threshold is crossed the action
        // is returned so the caller can execute it straight away.
        if session.modifier != CornerDragModifier::None {
            if dist < MODIFIER_THRESHOLD_PX {
                return None;
            }
            let leaf_rects = self.window_area_rects(container_size);
            let over_id = leaf_id_at_point(&leaf_rects, current_pos);

            return Some(match session.modifier {
                CornerDragModifier::Swap => {
                    if let Some(target) = over_id {
                        if target != session.leaf_id {
                            CornerDragAction::Swap {
                                from: session.leaf_id,
                                to: target,
                            }
                        } else {
                            CornerDragAction::Cancel
                        }
                    } else {
                        CornerDragAction::Cancel
                    }
                }
                CornerDragModifier::Duplicate => CornerDragAction::Duplicate {
                    leaf_id: session.leaf_id,
                },
                CornerDragModifier::None => unreachable!(),
            });
        }

        // --- No modifier: split vs join preview ---
        let leaf_rects = self.window_area_rects(container_size);
        let over_id = leaf_id_at_point(&leaf_rects, current_pos);

        if over_id == Some(session.leaf_id) || over_id.is_none() {
            // Cursor is still in the same area (or outside).  Potential split.
            let split_dir = if dir.is_vertical() {
                Axis::Vertical
            } else {
                Axis::Horizontal
            };
            // Calculate split ratio from cursor position within the leaf.
            if let Some(rect) = self.window_area_rect(session.leaf_id, &leaf_rects) {
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
                target_leaf_id: target_id,
                direction: dir,
            };
        }

        None
    }

    /// Finish the corner-drag gesture on mouse release.
    ///
    /// Reads the active session's preview state and returns the appropriate
    /// action, then clears the session.
    pub fn finish_window_area_corner_drag(&mut self) -> Option<CornerDragAction> {
        let session = self.active_window_area_corner_drag?;
        let action = match session.preview {
            CornerDragPreview::SplitPreview { direction, ratio } => Some(CornerDragAction::Split {
                leaf_id: session.leaf_id,
                direction,
                ratio,
            }),
            CornerDragPreview::JoinPreview {
                target_leaf_id,
                direction: _,
            } => Some(CornerDragAction::Join {
                from: session.leaf_id,
                into: target_leaf_id,
            }),
            CornerDragPreview::Dragging => Some(CornerDragAction::Cancel),
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

    #[test]
    fn test_area_layout_suite() {
        let mut layout = WindowLayout::default();
        assert_eq!(layout.window_area_tree.count_leaves(), 1);

        layout.split_window_area(1, Axis::Horizontal);
        assert_eq!(layout.window_area_tree.count_leaves(), 2);

        layout.split_window_area(2, Axis::Vertical);
        assert_eq!(layout.window_area_tree.count_leaves(), 3);

        layout.close_window_area(2);
        assert_eq!(layout.window_area_tree.count_leaves(), 2);

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
    fn test_split_cycles_outer_types() {
        let mut layout = WindowLayout::default();
        // Default: single Edit leaf.
        assert_eq!(
            layout.window_area_tree.find_leaf_kind(1),
            Some(WindowAreaKind::Editor)
        );

        // Split → Edit + Explorer
        layout.split_window_area(1, Axis::Horizontal);
        assert_eq!(layout.window_area_tree.count_leaves(), 2);
        // The original leaf (Edit) stays; new side gets Explorer.
        assert_eq!(
            layout.window_area_tree.find_leaf_kind(1),
            Some(WindowAreaKind::Editor)
        );

        // Split the Explorer → Explorer + Settings
        // The Explorer leaf is the one with id 3 (the new one from the split).
        // Actually, split_leaf_with_ratio creates the split with id=new_id, the
        // original leaf keeps its id and type, and the new leaf gets next_type.
        // So leaf 1 is Edit, leaf 3 is Explorer.
        layout.split_window_area(3, Axis::Vertical);
        assert_eq!(layout.window_area_tree.count_leaves(), 3);
        // Leaf 3 (Explorer) stays, new leaf gets Settings.
    }

    #[test]
    fn test_join_sibling_leaves() {
        let mut layout = WindowLayout::default();
        // Create a simple horizontal split: [Edit, Explorer]
        layout.split_window_area(1, Axis::Horizontal);
        assert_eq!(layout.window_area_tree.count_leaves(), 2);

        // Join the Explorer into Edit: remove Explorer, expand Edit.
        // Leaf 1 = Edit, leaf 3 = Explorer.
        let ok = layout.join_window_area(1, 3);
        assert!(ok);
        assert_eq!(layout.window_area_tree.count_leaves(), 1);
        assert_eq!(
            layout.window_area_tree.find_leaf_kind(1),
            Some(WindowAreaKind::Editor)
        );
    }

    #[test]
    fn test_join_nested_leaves() {
        let mut layout = WindowLayout::default();
        // Build: Split(H) { Leaf(1, Edit), Split(V) { Leaf(3, Explorer), Leaf(4, Settings) } }
        layout.split_window_area(1, Axis::Horizontal); // ids: 1 (Edit), 3 (Explorer)
        layout.split_window_area(3, Axis::Vertical); // ids: 1, 4, 5 (Explorer → Explorer + Settings)
        assert_eq!(layout.window_area_tree.count_leaves(), 3);

        // Join leaf 1 (Edit) with leaf 4 (was Explorer, now from second split).
        // After join we should have 2 leaves: 1 (Edit, expanded) and 5 (Settings).
        let ok = layout.join_window_area(1, 4);
        assert!(ok);
        assert_eq!(layout.window_area_tree.count_leaves(), 2);
    }

    #[test]
    fn test_window_area_rects() {
        let mut layout = WindowLayout::default();
        layout.split_window_area(1, Axis::Horizontal);
        let rects = layout.window_area_rects(size(px(1000.0), px(800.0)));
        assert_eq!(rects.len(), 2);
        // First leaf: left half, second leaf: right half.
        let first = rects[0];
        let second = rects[1];
        assert!((first.width - 500.0).abs() < 1.0);
        assert!((second.width - 500.0).abs() < 1.0);
        assert!((first.height - 800.0).abs() < 1.0);
        assert!((second.height - 800.0).abs() < 1.0);
        assert!((second.x - 500.0).abs() < 1.0);
    }

    #[test]
    fn test_inner_layout_defaults_to_source() {
        let mut layout = WindowLayout::default();
        let inner = layout.ensure_editor_inner_panel_tree(1);
        assert_eq!(inner.count_leaves(), 1);
        assert_eq!(
            inner.find_leaf_kind(1),
            Some(EditorInnerPanelKind::SourceCode)
        );
    }

    #[test]
    fn test_inner_split_inherits_target_kind() {
        let mut layout = WindowLayout::default();
        // Set up inner: Wysiwyg panel (id 1).
        let _ = layout.ensure_editor_inner_panel_tree(1);
        layout.change_editor_inner_panel_kind(1, 1, EditorInnerPanelKind::Wysiwyg);
        // Split it via the status-bar path; the new panel inherits Wysiwyg.
        layout.split_editor_inner_panel(1, 1, Axis::Horizontal);
        let inner = layout.editor_inner_panel_trees.get(&1).unwrap();
        assert_eq!(inner.count_leaves(), 2);
        assert_eq!(inner.find_leaf_kind(1), Some(EditorInnerPanelKind::Wysiwyg));
        // The new leaf (id 2) is also Wysiwyg.
        assert_eq!(inner.find_leaf_kind(2), Some(EditorInnerPanelKind::Wysiwyg));
    }

    #[test]
    fn test_corner_drag_split_inherits_dragged_kind() {
        let mut layout = WindowLayout::default();
        let _ = layout.ensure_editor_inner_panel_tree(1);
        layout.change_editor_inner_panel_kind(1, 1, EditorInnerPanelKind::Preview);
        // Corner-drag split; the new panel inherits Preview.
        layout.split_editor_inner_panel_with_ratio(1, 1, Axis::Vertical, 0.4);
        let inner = layout.editor_inner_panel_trees.get(&1).unwrap();
        assert_eq!(inner.count_leaves(), 2);
        assert_eq!(inner.find_leaf_kind(2), Some(EditorInnerPanelKind::Preview));
    }
}
