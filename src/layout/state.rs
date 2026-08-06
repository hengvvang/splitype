//! [`WindowLayout`] — the complete tiled layout state and operations.
//!
//! The outer tree (`window_area_tree`) uses [`WindowAreaKind`] for top-level
//! areas. Inner trees (`editor_inner_panel_trees`) use [`EditorInnerPanelKind`]
//! for sub-panels inside Edit areas. All split / join / swap / drag
//! operations live here; rendering lives in the hosts.

use std::collections::HashMap;

use gpui::{Pixels, Point, Size};

use crate::editor::controller::DocumentTab;
use crate::layout::sessions::{
    BorderMenuState, CornerDragModifier, CornerDragPreview, CornerDragSession,
    EditorInnerPanelDragAction, MODIFIER_THRESHOLD_PX, SplitterDragSession, WindowAreaDragAction,
    id_at_point,
};
use crate::layout::tree::{AreaRect, Axis, Direction, SplitTree};
use crate::layout::types::{
    AreaId, AreaSplitMode, EditorInnerPanelKind, InnerPanelLocation, PanelId, SplitId,
    WindowAreaKind,
};

/// The document tabs owned by one Editor area.
///
/// Every Editor area keeps its own ordered tab list; tabs are deep-copied
/// when an Editor area is split (normal drag) and start empty for fresh
/// editors (Shift-drag).
pub struct EditorTabList {
    pub tabs: Vec<DocumentTab>,
    pub active_tab: usize,
}

impl EditorTabList {
    pub fn empty() -> Self {
        Self {
            tabs: Vec::new(),
            active_tab: 0,
        }
    }
}

/// Full state for the tiled area layout manager.
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
    /// Per-Editor-area tab lists (independent tab bars).
    pub editor_tab_lists: HashMap<AreaId, EditorTabList>,
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
pub(crate) const ROOT_AREA_ID: AreaId = 1;

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
            editor_tab_lists: HashMap::new(),
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
    /// `mode` controls how the sibling is seeded: [`AreaSplitMode::Copy`]
    /// inherits the source kind and — for Editor areas — clones the inner
    /// panel layout (the host then deep-copies the tab list);
    /// [`AreaSplitMode::Fresh`] produces a blank initial-state area of the
    /// same kind. Returns the new area's id.
    pub fn split_window_area(
        &mut self,
        area_id: AreaId,
        direction: Axis,
        ratio: f32,
        mode: AreaSplitMode,
    ) -> Option<AreaId> {
        let new_id = self.next_node_id;
        self.next_node_id += 1;
        let kind = self
            .window_area_tree
            .find_leaf_kind(area_id)
            .unwrap_or(WindowAreaKind::Editor);
        self.window_area_tree
            .split_leaf_with_ratio(area_id, new_id, direction, ratio, kind);
        if kind == WindowAreaKind::Editor && mode == AreaSplitMode::Copy {
            // Clone the source area's inner panel layout (fresh ids).
            if let Some(source_tree) = self.editor_inner_panel_trees.get(&area_id) {
                let cloned = source_tree.clone_with_new_ids(&mut self.next_node_id);
                self.editor_inner_panel_trees.insert(new_id, cloned);
            }
        } else if kind == WindowAreaKind::Editor && mode == AreaSplitMode::Fresh {
            // A fresh editor starts with an empty tab list.
            self.ensure_editor_tab_list(new_id);
        }
        self.open_window_area_dropdown = None;
        self.active_window_area_border_menu = None;
        Some(new_id)
    }

    pub fn close_window_area(&mut self, area_id: AreaId) {
        if self.window_area_tree.count_leaves() > 1 {
            self.window_area_tree.remove_leaf(area_id);
            // Clean up inner trees and tab lists for the removed Editor area.
            self.editor_inner_panel_trees.remove(&area_id);
            self.editor_tab_lists.remove(&area_id);
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

    /// Get or create the tab list for an Editor area.
    pub fn ensure_editor_tab_list(&mut self, area_id: AreaId) -> &mut EditorTabList {
        self.editor_tab_lists
            .entry(area_id)
            .or_insert_with(EditorTabList::empty)
    }

    pub fn editor_tab_list(&self, area_id: AreaId) -> Option<&EditorTabList> {
        self.editor_tab_lists.get(&area_id)
    }

    /// The active editor area's tab list, if an active editor exists.
    pub fn active_editor_tab_list(&self) -> Option<&EditorTabList> {
        self.active_editor_area
            .and_then(|area| self.editor_tab_lists.get(&area))
    }

    /// Recompute the active editor after the layout changed: the most
    /// recently focused Editor area still present, or `None`.
    fn recompute_active_editor(&mut self) {
        if let Some(active) = self.active_editor_area {
            if self.window_area_tree.find_leaf_kind(active) == Some(WindowAreaKind::Editor) {
                return;
            }
        }
        self.active_editor_area =
            self.editor_activation_history.iter().rev().copied().find(|id| {
                self.window_area_tree.find_leaf_kind(*id) == Some(WindowAreaKind::Editor)
            });
    }

    /// Drop an area from activation tracking and recompute the active editor.
    fn retire_editor_area(&mut self, removed: AreaId) {
        self.editor_activation_history.retain(|id| *id != removed);
        self.recompute_active_editor();
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
                target_id: panel_id,
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
    ) -> Option<EditorInnerPanelDragAction> {
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
            let over_id = id_at_point(&leaf_rects, current_pos);

            return Some(match session.modifier {
                CornerDragModifier::Swap => {
                    if let Some(target) = over_id {
                        if target != session.target_id {
                            EditorInnerPanelDragAction::Swap {
                                from: session.target_id,
                                to: target,
                            }
                        } else {
                            EditorInnerPanelDragAction::Cancel
                        }
                    } else {
                        EditorInnerPanelDragAction::Cancel
                    }
                }
                CornerDragModifier::Duplicate => EditorInnerPanelDragAction::Duplicate {
                    panel_id: session.target_id,
                },
                CornerDragModifier::None => unreachable!(),
            });
        }

        let leaf_rects = self.editor_inner_panel_rects(area_id, container_size);
        let over_id = id_at_point(&leaf_rects, current_pos);

        if over_id == Some(session.target_id) || over_id.is_none() {
            // Cursor is still in the same area (or outside).  Potential split.
            // The new panel inherits the dragged panel's kind (applied on finish).
            let split_dir = if dir.is_vertical() {
                Axis::Vertical
            } else {
                Axis::Horizontal
            };
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
                target_id,
                direction: dir,
            };
        }

        None
    }

    pub fn finish_editor_inner_panel_corner_drag(
        &mut self,
    ) -> Option<(AreaId, EditorInnerPanelDragAction)> {
        let (area_id, session) = self.active_editor_inner_panel_corner_drag?;
        let action = match session.preview {
            CornerDragPreview::SplitPreview { direction, ratio } => {
                Some(EditorInnerPanelDragAction::Split {
                    panel_id: session.target_id,
                    direction,
                    ratio,
                })
            }
            CornerDragPreview::JoinPreview { target_id, direction: _ } => {
                Some(EditorInnerPanelDragAction::Join {
                    from_panel: session.target_id,
                    into_panel: target_id,
                })
            }
            CornerDragPreview::Dragging => Some(EditorInnerPanelDragAction::Cancel),
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
        let previous = self.window_area_tree.find_leaf_kind(area_id);
        self.window_area_tree.set_leaf_kind(area_id, kind);
        if previous == Some(WindowAreaKind::Editor) && kind != WindowAreaKind::Editor {
            // Leaving Editor: drop the area's document state.
            self.editor_inner_panel_trees.remove(&area_id);
            self.editor_tab_lists.remove(&area_id);
            self.retire_editor_area(area_id);
        } else if kind == WindowAreaKind::Editor && previous != Some(WindowAreaKind::Editor) {
            // Entering Editor: an empty tab list, and the area becomes the
            // active editor (the type switch is an explicit interaction).
            self.ensure_editor_tab_list(area_id);
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
            // Clean up inner trees and tab lists for the removed Editor area.
            self.editor_inner_panel_trees.remove(&removed);
            self.editor_tab_lists.remove(&removed);
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

    /// Swap the area type of area `a` and area `b`. Per-area Editor state
    /// (inner trees, tab lists) moves along with the Editor kind.
    pub fn swap_window_area_kinds(&mut self, a: AreaId, b: AreaId) {
        let type_a = self.window_area_tree.find_leaf_kind(a);
        let type_b = self.window_area_tree.find_leaf_kind(b);
        if let (Some(ta), Some(tb)) = (type_a, type_b) {
            self.window_area_tree.set_leaf_kind(a, tb);
            self.window_area_tree.set_leaf_kind(b, ta);
            if let (Some(tree_a), Some(tree_b)) = (
                self.editor_inner_panel_trees.remove(&a),
                self.editor_inner_panel_trees.remove(&b),
            ) {
                self.editor_inner_panel_trees.insert(a, tree_b);
                self.editor_inner_panel_trees.insert(b, tree_a);
            }
            if let (Some(tabs_a), Some(tabs_b)) = (
                self.editor_tab_lists.remove(&a),
                self.editor_tab_lists.remove(&b),
            ) {
                self.editor_tab_lists.insert(a, tabs_b);
                self.editor_tab_lists.insert(b, tabs_a);
            }
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
            CornerDragPreview::JoinPreview { target_id, direction: _ } => {
                Some(WindowAreaDragAction::Join {
                    from_area: session.target_id,
                    into_area: target_id,
                })
            }
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

    #[test]
    fn test_area_layout_suite() {
        let mut layout = WindowLayout::default();
        assert_eq!(layout.window_area_tree.count_leaves(), 1);

        layout.split_window_area(1, Axis::Horizontal, 0.5, AreaSplitMode::Copy);
        assert_eq!(layout.window_area_tree.count_leaves(), 2);

        layout.split_window_area(2, Axis::Vertical, 0.5, AreaSplitMode::Copy);
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
    fn test_split_inherits_source_kind() {
        let mut layout = WindowLayout::default();
        // Default: single Editor leaf.
        assert_eq!(
            layout.window_area_tree.find_leaf_kind(1),
            Some(WindowAreaKind::Editor)
        );

        // Split → Editor + Editor (same kind, not cycled).
        layout.split_window_area(1, Axis::Horizontal, 0.5, AreaSplitMode::Copy);
        assert_eq!(layout.window_area_tree.count_leaves(), 2);
        assert_eq!(
            layout.window_area_tree.find_leaf_kind(1),
            Some(WindowAreaKind::Editor)
        );
        assert_eq!(
            layout.window_area_tree.find_leaf_kind(2),
            Some(WindowAreaKind::Editor)
        );

        // Turn leaf 1 into Explorer, then split it → Explorer + Explorer.
        layout.change_window_area_kind(1, WindowAreaKind::Explorer);
        layout.split_window_area(1, Axis::Vertical, 0.5, AreaSplitMode::Copy);
        assert_eq!(layout.window_area_tree.count_leaves(), 3);
        assert_eq!(
            layout.window_area_tree.find_leaf_kind(1),
            Some(WindowAreaKind::Explorer)
        );
        // The new leaf from the second split (id 3) is also Explorer.
        assert_eq!(
            layout.window_area_tree.find_leaf_kind(3),
            Some(WindowAreaKind::Explorer)
        );

        // Settings splits into Settings.
        layout.change_window_area_kind(2, WindowAreaKind::Settings);
        layout.split_window_area(2, Axis::Horizontal, 0.5, AreaSplitMode::Copy);
        assert_eq!(layout.window_area_tree.count_leaves(), 4);
        assert_eq!(
            layout.window_area_tree.find_leaf_kind(4),
            Some(WindowAreaKind::Settings)
        );
    }

    #[test]
    fn test_editor_split_clones_inner_layout_and_tab_list() {
        let mut layout = WindowLayout::default();
        // Source Editor area (id 1) with a two-panel inner layout.
        let inner = layout.ensure_editor_inner_panel_tree(1);
        inner.split_leaf_with_ratio(2, 20, Axis::Horizontal, 0.5, EditorInnerPanelKind::Wysiwyg);
        layout.editor_tab_lists.insert(1, EditorTabList::empty());
        layout.activate_editor_area(1);

        let new_id = layout
            .split_window_area(1, Axis::Vertical, 0.5, AreaSplitMode::Copy)
            .unwrap();
        assert_eq!(
            layout.window_area_tree.find_leaf_kind(new_id),
            Some(WindowAreaKind::Editor)
        );
        // Inner layout cloned with fresh ids (the cloned tree must not
        // equal the source tree, whose ids it shares the pool with).
        let cloned = layout.editor_inner_panel_trees.get(&new_id).unwrap();
        assert_eq!(cloned.count_leaves(), 2);
        assert_ne!(cloned, layout.editor_inner_panel_trees.get(&1).unwrap());

        // The new area's tab list is created by the host (deep-copying the
        // documents needs a Context); the layout only guarantees the new
        // area id so the host can seed it.
        assert!(layout.editor_tab_lists.get(&new_id).is_none());
    }

    #[test]
    fn test_fresh_editor_split_gets_empty_tab_list() {
        let mut layout = WindowLayout::default();
        let inner = layout.ensure_editor_inner_panel_tree(1);
        inner.split_leaf_with_ratio(2, 20, Axis::Horizontal, 0.5, EditorInnerPanelKind::Wysiwyg);
        layout.editor_tab_lists.insert(1, EditorTabList::empty());
        layout.activate_editor_area(1);

        let new_id = layout
            .split_window_area(1, Axis::Vertical, 0.5, AreaSplitMode::Fresh)
            .unwrap();
        assert_eq!(
            layout.window_area_tree.find_leaf_kind(new_id),
            Some(WindowAreaKind::Editor)
        );
        // No cloned inner layout.
        assert!(layout.editor_inner_panel_trees.get(&new_id).is_none());
        // Empty tab list.
        assert_eq!(layout.editor_tab_list(new_id).unwrap().tabs.len(), 0);
    }

    #[test]
    fn test_active_editor_falls_back_to_last_focused() {
        let mut layout = WindowLayout::default();
        layout.activate_editor_area(1);
        let a = layout
            .split_window_area(1, Axis::Horizontal, 0.5, AreaSplitMode::Copy)
            .unwrap();
        let b = layout
            .split_window_area(1, Axis::Vertical, 0.5, AreaSplitMode::Copy)
            .unwrap();
        // Activation order: 1, a, b → active is b.
        layout.activate_editor_area(a);
        layout.activate_editor_area(b);
        assert_eq!(layout.active_editor_area, Some(b));

        // Close the active editor → falls back to the previous focus (a).
        layout.close_window_area(b);
        assert_eq!(layout.active_editor_area, Some(a));

        // Closing the last editor → no active editor.
        layout.close_window_area(a);
        layout.close_window_area(1);
        assert_eq!(layout.active_editor_area, None);
    }

    #[test]
    fn test_join_sibling_leaves() {
        let mut layout = WindowLayout::default();
        // Create a simple horizontal split: [Editor, Editor]
        layout.split_window_area(1, Axis::Horizontal, 0.5, AreaSplitMode::Copy);
        assert_eq!(layout.window_area_tree.count_leaves(), 2);

        // Join leaf 2 into leaf 1: remove 2, expand 1.
        let ok = layout.join_window_area(1, 2);
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
        // Build: Split(H) { Leaf(1), Split(V) { Leaf(2), Leaf(3) } }
        layout.split_window_area(1, Axis::Horizontal, 0.5, AreaSplitMode::Copy); // ids: 1, 2
        layout.split_window_area(2, Axis::Vertical, 0.5, AreaSplitMode::Copy); // ids: 1, 2, 3
        assert_eq!(layout.window_area_tree.count_leaves(), 3);

        // Join leaf 1 with leaf 2 → 2 leaves remain.
        let ok = layout.join_window_area(1, 2);
        assert!(ok);
        assert_eq!(layout.window_area_tree.count_leaves(), 2);
    }

    #[test]
    fn test_window_area_rects() {
        let mut layout = WindowLayout::default();
        layout.split_window_area(1, Axis::Horizontal, 0.5, AreaSplitMode::Copy);
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
            inner.find_leaf_kind(2),
            Some(EditorInnerPanelKind::SourceCode)
        );
    }

    #[test]
    fn test_inner_split_inherits_target_kind() {
        let mut layout = WindowLayout::default();
        // Set up inner: Wysiwyg panel (id 2, first allocated panel id).
        let _ = layout.ensure_editor_inner_panel_tree(1);
        layout.change_editor_inner_panel_kind(1, 2, EditorInnerPanelKind::Wysiwyg);
        // Split it via the status-bar path; the new panel inherits Wysiwyg.
        layout.split_editor_inner_panel(1, 2, Axis::Horizontal);
        let inner = layout.editor_inner_panel_trees.get(&1).unwrap();
        assert_eq!(inner.count_leaves(), 2);
        assert_eq!(inner.find_leaf_kind(2), Some(EditorInnerPanelKind::Wysiwyg));
        // The new leaf (id 3) is also Wysiwyg.
        assert_eq!(inner.find_leaf_kind(3), Some(EditorInnerPanelKind::Wysiwyg));
    }

    #[test]
    fn test_corner_drag_split_inherits_dragged_kind() {
        let mut layout = WindowLayout::default();
        let _ = layout.ensure_editor_inner_panel_tree(1);
        layout.change_editor_inner_panel_kind(1, 2, EditorInnerPanelKind::Preview);
        // Corner-drag split; the new panel inherits Preview.
        layout.split_editor_inner_panel_with_ratio(1, 2, Axis::Vertical, 0.4);
        let inner = layout.editor_inner_panel_trees.get(&1).unwrap();
        assert_eq!(inner.count_leaves(), 2);
        assert_eq!(inner.find_leaf_kind(3), Some(EditorInnerPanelKind::Preview));
    }
}
