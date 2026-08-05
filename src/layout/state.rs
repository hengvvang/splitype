//! [`WindowLayout`] — the complete tiled layout state and operations.
//!
//! The outer tree (`root`) uses [`WindowAreaKind`] for top-level areas. Inner
//! trees (`editor_inner_panel_layouts`) use [`EditorInnerPanelKind`] for sub-panels inside
//! Edit areas. Layout-preset construction and all split / join / swap /
//! drag operations live here; rendering lives in the hosts.

use std::collections::HashMap;

use gpui::{Pixels, Point, Size};

use crate::layout::sessions::{
    BorderMenuState, CornerDragAction, CornerDragModifier, CornerDragPreview, CornerDragSession,
    MODIFIER_THRESHOLD_PX, SplitterDragSession, area_id_at_point,
};
use crate::layout::tree::{AreaRect, Axis, Direction, SplitTree};
use crate::layout::types::{EditorInnerPanelKind, WindowAreaKind};

/// Presets for explorer layout arrangements.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutPreset {
    DefaultEditor,
    DualView,
    ExplorerStateEditor,
    TripleView,
    OutlineView,
}

impl LayoutPreset {
    #[allow(dead_code)]
    pub fn name(&self) -> &'static str {
        match self {
            Self::DefaultEditor => "Single Editor",
            Self::DualView => "Dual Edit",
            Self::ExplorerStateEditor => "Explorer + Edit",
            Self::TripleView => "Explorer + Dual Edit",
            Self::OutlineView => "Explorer + Edit + Explorer",
        }
    }

    #[allow(dead_code)]
    pub fn all() -> &'static [LayoutPreset] {
        &[
            Self::DefaultEditor,
            Self::DualView,
            Self::ExplorerStateEditor,
            Self::TripleView,
            Self::OutlineView,
        ]
    }
}

/// Full state for the tiled area layout manager.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowLayout {
    /// Outer tiled layout tree.
    pub root: SplitTree<WindowAreaKind>,
    /// Per-Edit inner sub-panel layouts, keyed by area_id (= outer leaf id).
    pub editor_inner_panel_layouts: HashMap<usize, SplitTree<EditorInnerPanelKind>>,
    pub next_leaf_id: usize,
    pub active_window_area_dropdown: Option<usize>,
    pub active_editor_inner_panel_dropdown: Option<(usize, usize)>,
    pub maximized_window_area: Option<usize>,
    pub active_window_area_splitter_drag: Option<SplitterDragSession>,
    pub active_window_area_corner_drag: Option<CornerDragSession>,
    pub active_window_area_border_menu: Option<BorderMenuState>,
    pub active_editor_inner_panel_splitter_drag: Option<(usize, SplitterDragSession)>,
    pub active_editor_inner_panel_corner_drag: Option<(usize, CornerDragSession)>,
    pub active_editor_inner_panel_border_menu: Option<BorderMenuState>,
    /// Currently focused inner panel (area_id, panel_id) for status bar actions.
    pub focused_editor_inner_panel: Option<(usize, usize)>,
    /// Measured pixel size of the tiled-layout container.
    pub container_size: Option<Size<Pixels>>,
}

impl Default for WindowLayout {
    fn default() -> Self {
        Self {
            root: SplitTree::Leaf {
                id: 1,
                kind: WindowAreaKind::Editor,
            },
            editor_inner_panel_layouts: HashMap::new(),
            next_leaf_id: 2,
            active_window_area_dropdown: None,
            active_editor_inner_panel_dropdown: None,
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
    // Preset
    // ------------------------------------------------------------------

    #[allow(dead_code)]
    pub fn apply_preset(&mut self, preset: LayoutPreset) {
        self.maximized_window_area = None;
        self.active_window_area_dropdown = None;
        self.active_window_area_border_menu = None;
        match preset {
            LayoutPreset::DefaultEditor => {
                self.root = SplitTree::Leaf {
                    id: 1,
                    kind: WindowAreaKind::Editor,
                };
                self.next_leaf_id = 2;
            }
            LayoutPreset::DualView => {
                self.root = SplitTree::Split {
                    id: 1,
                    direction: Axis::Horizontal,
                    ratio: 0.5,
                    first: Box::new(SplitTree::Leaf {
                        id: 2,
                        kind: WindowAreaKind::Editor,
                    }),
                    second: Box::new(SplitTree::Leaf {
                        id: 3,
                        kind: WindowAreaKind::Editor,
                    }),
                };
                self.next_leaf_id = 4;
            }
            LayoutPreset::ExplorerStateEditor => {
                self.root = SplitTree::Split {
                    id: 1,
                    direction: Axis::Horizontal,
                    ratio: 0.22,
                    first: Box::new(SplitTree::Leaf {
                        id: 2,
                        kind: WindowAreaKind::Explorer,
                    }),
                    second: Box::new(SplitTree::Leaf {
                        id: 3,
                        kind: WindowAreaKind::Editor,
                    }),
                };
                self.next_leaf_id = 4;
            }
            LayoutPreset::TripleView => {
                self.root = SplitTree::Split {
                    id: 1,
                    direction: Axis::Horizontal,
                    ratio: 0.2,
                    first: Box::new(SplitTree::Leaf {
                        id: 2,
                        kind: WindowAreaKind::Explorer,
                    }),
                    second: Box::new(SplitTree::Split {
                        id: 3,
                        direction: Axis::Horizontal,
                        ratio: 0.5,
                        first: Box::new(SplitTree::Leaf {
                            id: 4,
                            kind: WindowAreaKind::Editor,
                        }),
                        second: Box::new(SplitTree::Leaf {
                            id: 5,
                            kind: WindowAreaKind::Editor,
                        }),
                    }),
                };
                self.next_leaf_id = 6;
            }
            LayoutPreset::OutlineView => {
                self.root = SplitTree::Split {
                    id: 1,
                    direction: Axis::Horizontal,
                    ratio: 0.2,
                    first: Box::new(SplitTree::Leaf {
                        id: 2,
                        kind: WindowAreaKind::Explorer,
                    }),
                    second: Box::new(SplitTree::Split {
                        id: 3,
                        direction: Axis::Horizontal,
                        ratio: 0.7,
                        first: Box::new(SplitTree::Leaf {
                            id: 4,
                            kind: WindowAreaKind::Editor,
                        }),
                        second: Box::new(SplitTree::Leaf {
                            id: 5,
                            kind: WindowAreaKind::Explorer,
                        }),
                    }),
                };
                self.next_leaf_id = 6;
            }
        }
    }

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

    pub fn split_window_area(&mut self, target_leaf_id: usize, direction: Axis) {
        let new_id = self.next_leaf_id;
        self.next_leaf_id += 1;
        let next_type = self
            .root
            .find_leaf_kind(target_leaf_id)
            .map(Self::next_window_area_kind)
            .unwrap_or(WindowAreaKind::Editor);
        self.root
            .split_leaf(target_leaf_id, new_id, direction, next_type);
        self.active_window_area_dropdown = None;
        self.active_window_area_border_menu = None;
    }

    /// Split a leaf area at a specific ratio.
    pub fn split_window_area_with_ratio(
        &mut self,
        target_leaf_id: usize,
        direction: Axis,
        ratio: f32,
    ) {
        let new_id = self.next_leaf_id;
        self.next_leaf_id += 1;
        let next_type = self
            .root
            .find_leaf_kind(target_leaf_id)
            .map(Self::next_window_area_kind)
            .unwrap_or(WindowAreaKind::Editor);
        self.root
            .split_leaf_with_ratio(target_leaf_id, new_id, direction, ratio, next_type);
        self.active_window_area_dropdown = None;
        self.active_window_area_border_menu = None;
    }

    pub fn close_window_area(&mut self, target_leaf_id: usize) {
        if self.root.count_leaves() > 1 {
            self.root.remove_leaf(target_leaf_id);
            // Clean up inner layout for the removed Edit area.
            self.editor_inner_panel_layouts.remove(&target_leaf_id);
            if self.maximized_window_area == Some(target_leaf_id) {
                self.maximized_window_area = None;
            }
        }
        self.active_window_area_dropdown = None;
        self.active_window_area_border_menu = None;
    }

    // ------------------------------------------------------------------
    // Inner Edit sub-panel layout
    // ------------------------------------------------------------------

    /// Get or create the inner layout for an Edit area.
    /// New Edit areas default to a single `EditorInnerPanelKind::SourceCode` panel.
    pub fn ensure_editor_inner_panel_layout(
        &mut self,
        area_id: usize,
    ) -> &mut SplitTree<EditorInnerPanelKind> {
        let next_leaf_id = &mut self.next_leaf_id;
        self.editor_inner_panel_layouts
            .entry(area_id)
            .or_insert_with(|| {
                let panel_id = *next_leaf_id;
                *next_leaf_id += 1;
                SplitTree::Leaf {
                    id: panel_id,
                    kind: EditorInnerPanelKind::SourceCode,
                }
            })
    }

    pub fn split_editor_inner_panel(&mut self, area_id: usize, target_id: usize, direction: Axis) {
        let new_id = self.next_leaf_id;
        self.next_leaf_id += 1;
        let root = self.ensure_editor_inner_panel_layout(area_id);
        // Inner splits always create a Source panel on the new side.
        root.split_leaf_with_ratio(
            target_id,
            new_id,
            direction,
            0.5,
            EditorInnerPanelKind::SourceCode,
        );
    }

    pub fn close_editor_inner_panel(&mut self, area_id: usize, panel_id: usize) {
        if let Some(root) = self.editor_inner_panel_layouts.get_mut(&area_id) {
            if root.count_leaves() > 1 {
                root.remove_leaf(panel_id);
            }
        }
    }

    pub fn toggle_editor_inner_panel_dropdown(&mut self, area_id: usize, panel_id: usize) {
        if self.active_editor_inner_panel_dropdown == Some((area_id, panel_id)) {
            self.active_editor_inner_panel_dropdown = None;
        } else {
            self.active_editor_inner_panel_dropdown = Some((area_id, panel_id));
            self.active_window_area_dropdown = None;
        }
    }

    pub fn change_editor_inner_panel_kind(
        &mut self,
        area_id: usize,
        inner_leaf_id: usize,
        new_type: EditorInnerPanelKind,
    ) {
        let root = self.ensure_editor_inner_panel_layout(area_id);
        root.set_leaf_kind(inner_leaf_id, new_type);
        self.active_editor_inner_panel_dropdown = None;
    }

    /// Inner split created via corner drag.  The new panel is always `Source`.
    pub fn split_editor_inner_panel_with_ratio(
        &mut self,
        area_id: usize,
        target_leaf_id: usize,
        direction: Axis,
        ratio: f32,
    ) {
        let new_id = self.next_leaf_id;
        self.next_leaf_id += 1;
        if let Some(root) = self.editor_inner_panel_layouts.get_mut(&area_id) {
            root.split_leaf_with_ratio(
                target_leaf_id,
                new_id,
                direction,
                ratio,
                EditorInnerPanelKind::SourceCode,
            );
        }
    }

    /// Join an inner leaf into another within the same inner layout.
    pub fn join_editor_inner_panel(
        &mut self,
        area_id: usize,
        into_id: usize,
        target_id: usize,
    ) -> bool {
        if into_id == target_id {
            return false;
        }
        if let Some(root) = self.editor_inner_panel_layouts.get_mut(&area_id) {
            if root.count_leaves() <= 1 {
                return false;
            }
            root.join_leaf(into_id, target_id)
        } else {
            false
        }
    }

    /// Swap area types between two inner leaves.
    pub fn swap_editor_inner_panel_kinds(&mut self, area_id: usize, a: usize, b: usize) {
        if let Some(root) = self.editor_inner_panel_layouts.get_mut(&area_id) {
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
        area_id: usize,
        current_pointer_pos: f32,
    ) {
        if let Some((_area_id, session)) = self.active_editor_inner_panel_splitter_drag {
            if session.total_span > 1.0 {
                let delta = current_pointer_pos - session.start_pointer_pos;
                let ratio_delta = delta / session.total_span;
                let new_ratio = session.start_ratio + ratio_delta;
                if let Some(root) = self.editor_inner_panel_layouts.get_mut(&area_id) {
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
        area_id: usize,
        leaf_id: usize,
        pos: Point<Pixels>,
        modifier: CornerDragModifier,
    ) {
        self.active_editor_inner_panel_corner_drag = Some((
            area_id,
            CornerDragSession {
                leaf_id,
                start_pos: pos,
                gesture_dir: None,
                modifier,
                preview: CornerDragPreview::Dragging,
            },
        ));
    }

    pub fn update_editor_inner_panel_corner_drag(
        &mut self,
        area_id: usize,
        current_pos: Point<Pixels>,
        container_size: Size<Pixels>,
    ) -> Option<CornerDragAction> {
        let (cid, session) = match self.active_editor_inner_panel_corner_drag {
            Some(ref s) => (s.0, s.1),
            None => return None,
        };
        debug_assert_eq!(cid, area_id, "area_id mismatch in inner corner drag");

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
            let over_id = area_id_at_point(&leaf_rects, current_pos);

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
        let over_id = area_id_at_point(&leaf_rects, current_pos);

        if over_id == Some(session.leaf_id) || over_id.is_none() {
            // Cursor is still in the same area (or outside).  Potential split.
            // Inner splits always create a Source panel.
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

    pub fn finish_editor_inner_panel_corner_drag(&mut self) -> Option<(usize, CornerDragAction)> {
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

    /// Collect all inner leaf rectangles in pixel coordinates for a given Edit container.
    pub fn editor_inner_panel_rects(
        &self,
        area_id: usize,
        container_size: Size<Pixels>,
    ) -> Vec<AreaRect> {
        let w = f32::from(container_size.width);
        let h = f32::from(container_size.height);
        let mut rects = Vec::new();
        if w > 0.0 && h > 0.0 {
            if let Some(root) = self.editor_inner_panel_layouts.get(&area_id) {
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
        area_id: usize,
        split_id: usize,
        container_size: Size<Pixels>,
    ) -> Option<f32> {
        let w = f32::from(container_size.width);
        let h = f32::from(container_size.height);
        if w > 0.0 && h > 0.0 {
            if let Some(root) = self.editor_inner_panel_layouts.get(&area_id) {
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

    pub fn change_window_area_kind(&mut self, leaf_id: usize, new_type: WindowAreaKind) {
        self.root.set_leaf_kind(leaf_id, new_type);
        self.active_window_area_dropdown = None;
    }

    // ------------------------------------------------------------------
    // Join two adjacent areas
    // ------------------------------------------------------------------

    /// Join `target_id` into `into_id`. The target leaf is removed and its
    /// space is absorbed by the `into_id` leaf. The two must be adjacent
    /// (share an edge) in the layout.
    pub fn join_window_area(&mut self, into_id: usize, target_id: usize) -> bool {
        if into_id == target_id {
            return false;
        }
        if self.root.count_leaves() <= 1 {
            return false;
        }
        let ok = self.root.join_leaf(into_id, target_id);
        if ok {
            // Clean up inner layout for the removed Edit area.
            self.editor_inner_panel_layouts.remove(&target_id);
            if self.maximized_window_area == Some(target_id) {
                self.maximized_window_area = None;
            }
        }
        self.active_window_area_dropdown = None;
        self.active_window_area_border_menu = None;
        ok
    }

    // ------------------------------------------------------------------
    // Swap two area types
    // ------------------------------------------------------------------

    /// Swap the area type of leaf `a` and leaf `b`.
    pub fn swap_window_area_kinds(&mut self, a: usize, b: usize) {
        let type_a = self.root.find_leaf_kind(a);
        let type_b = self.root.find_leaf_kind(b);
        if let (Some(ta), Some(tb)) = (type_a, type_b) {
            self.root.set_leaf_kind(a, tb);
            self.root.set_leaf_kind(b, ta);
        }
    }

    // ------------------------------------------------------------------
    // Maximise / dropdown
    // ------------------------------------------------------------------

    pub fn toggle_window_area_maximize(&mut self, leaf_id: usize) {
        if self.maximized_window_area == Some(leaf_id) {
            self.maximized_window_area = None;
        } else {
            self.maximized_window_area = Some(leaf_id);
        }
    }

    pub fn toggle_window_area_dropdown(&mut self, leaf_id: usize) {
        if self.active_window_area_dropdown == Some(leaf_id) {
            self.active_window_area_dropdown = None;
        } else {
            self.active_window_area_dropdown = Some(leaf_id);
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
                self.root.set_split_ratio(session.split_id, new_ratio);
            }
        }
    }

    pub fn end_window_area_splitter_drag(&mut self) {
        self.active_window_area_splitter_drag = None;
    }

    // ------------------------------------------------------------------
    // Corner drag — split / join / swap / duplicate
    // ------------------------------------------------------------------

    /// Begin a corner-drag gesture from `leaf_id` at `pos` with optional
    /// modifier key.
    pub fn start_window_area_corner_drag(
        &mut self,
        leaf_id: usize,
        pos: Point<Pixels>,
        modifier: CornerDragModifier,
    ) {
        self.active_window_area_corner_drag = Some(CornerDragSession {
            leaf_id,
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
            let over_id = area_id_at_point(&leaf_rects, current_pos);

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
        let over_id = area_id_at_point(&leaf_rects, current_pos);

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
            self.root.collect_leaf_rects(0.0, 0.0, 1.0, 1.0, &mut norm);
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

    /// Get the pixel-space rectangle for a specific leaf, given pre-computed
    /// leaf rects from `window_area_rects`.
    pub fn window_area_rect(&self, leaf_id: usize, rects: &[AreaRect]) -> Option<AreaRect> {
        rects.iter().find(|rect| rect.id == leaf_id).copied()
    }

    /// Calculate the pixel span (width or height) of a split container.
    pub fn window_area_split_pixel_span(
        &self,
        split_id: usize,
        container_size: Size<Pixels>,
    ) -> Option<f32> {
        let w = f32::from(container_size.width);
        let h = f32::from(container_size.height);
        if w > 0.0 && h > 0.0 {
            if let Some((dir, span_norm)) = self.root.find_split_span(split_id, 0.0, 0.0, 1.0, 1.0)
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

    pub fn swap_window_area_split_sides(&mut self, split_id: usize) {
        self.root.swap_sibling_leaves(split_id);
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
        assert_eq!(layout.root.count_leaves(), 1);

        layout.split_window_area(1, Axis::Horizontal);
        assert_eq!(layout.root.count_leaves(), 2);

        layout.split_window_area(2, Axis::Vertical);
        assert_eq!(layout.root.count_leaves(), 3);

        layout.close_window_area(2);
        assert_eq!(layout.root.count_leaves(), 2);

        layout.change_window_area_kind(1, WindowAreaKind::Explorer);
        assert_eq!(
            layout.root.find_leaf_kind(1),
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
        assert_eq!(layout.root.find_leaf_kind(1), Some(WindowAreaKind::Editor));

        // Split → Edit + Explorer
        layout.split_window_area(1, Axis::Horizontal);
        assert_eq!(layout.root.count_leaves(), 2);
        // The original leaf (Edit) stays; new side gets Explorer.
        assert_eq!(layout.root.find_leaf_kind(1), Some(WindowAreaKind::Editor));

        // Split the Explorer → Explorer + Settings
        // The Explorer leaf is the one with id 3 (the new one from the split).
        // Actually, split_leaf_with_ratio creates the split with id=new_id, the
        // original leaf keeps its id and type, and the new leaf gets next_type.
        // So leaf 1 is Edit, leaf 3 is Explorer.
        layout.split_window_area(3, Axis::Vertical);
        assert_eq!(layout.root.count_leaves(), 3);
        // Leaf 3 (Explorer) stays, new leaf gets Settings.
    }

    #[test]
    fn test_join_sibling_leaves() {
        let mut layout = WindowLayout::default();
        // Create a simple horizontal split: [Edit, Explorer]
        layout.split_window_area(1, Axis::Horizontal);
        assert_eq!(layout.root.count_leaves(), 2);

        // Join the Explorer into Edit: remove Explorer, expand Edit.
        // Leaf 1 = Edit, leaf 3 = Explorer.
        let ok = layout.join_window_area(1, 3);
        assert!(ok);
        assert_eq!(layout.root.count_leaves(), 1);
        assert_eq!(layout.root.find_leaf_kind(1), Some(WindowAreaKind::Editor));
    }

    #[test]
    fn test_join_nested_leaves() {
        let mut layout = WindowLayout::default();
        // Build: Split(H) { Leaf(1, Edit), Split(V) { Leaf(3, Explorer), Leaf(4, Settings) } }
        layout.split_window_area(1, Axis::Horizontal); // ids: 1 (Edit), 3 (Explorer)
        layout.split_window_area(3, Axis::Vertical); // ids: 1, 4, 5 (Explorer → Explorer + Settings)
        assert_eq!(layout.root.count_leaves(), 3);

        // Join leaf 1 (Edit) with leaf 4 (was Explorer, now from second split).
        // After join we should have 2 leaves: 1 (Edit, expanded) and 5 (Settings).
        let ok = layout.join_window_area(1, 4);
        assert!(ok);
        assert_eq!(layout.root.count_leaves(), 2);
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
        let inner = layout.ensure_editor_inner_panel_layout(1);
        assert_eq!(inner.count_leaves(), 1);
        assert_eq!(
            inner.find_leaf_kind(1),
            Some(EditorInnerPanelKind::SourceCode)
        );
    }

    #[test]
    fn test_inner_split_creates_source() {
        let mut layout = WindowLayout::default();
        // Set up inner: Source panel.
        let _ = layout.ensure_editor_inner_panel_layout(1);
        // Split it. The new panel should be Source.
        layout.split_editor_inner_panel(1, 1, Axis::Horizontal); // panel_id 1
        // Now we have 2 inner leaves.
        let inner = layout.editor_inner_panel_layouts.get(&1).unwrap();
        assert_eq!(inner.count_leaves(), 2);
    }
}
