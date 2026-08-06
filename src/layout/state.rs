//! [`WindowLayout`] — the complete tiled layout state and operations.
//!
//! The outer tree (`window_area_tree`) uses [`WindowAreaKind`] for top-level
//! areas. Per-area sessions (`editor_sessions`) bundle each area's inner
//! panel tree (`[`EditorInnerPanelKind`] sub-panels) with its document tab
//! list. All split / join / swap / drag operations live here; rendering
//! lives in the hosts.

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
    AreaId, AreaSplitMode, EditingPanelKind, EditorAreaMode, EditorInnerPanelKind,
    InnerPanelLocation, PanelId, SplitId, WelcomePanelKind, WindowAreaKind,
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

/// The complete per-area editor state: the document tabs plus the inner
/// panel split tree.
///
/// Aggregating both under one key guarantees they can never drift apart —
/// an area always has exactly one tab list and one panel layout. Sessions
/// are created lazily and survive a switch away from Editor (background
/// editing) so the tabs are restored when the area becomes Editor again.
/// A retained session is a pure cache: it never participates in explorer
/// or activation logic until its area is back in the foreground.
pub struct EditorSession {
    pub tab_list: EditorTabList,
    pub inner_panel_tree: SplitTree<EditorInnerPanelKind>,
}

/// Full state for the tiled area layout manager.
pub struct WindowLayout {
    /// Outer tiled layout tree.
    pub window_area_tree: SplitTree<WindowAreaKind>,
    /// Per-Editor-area sessions (tab list + inner panel layout), keyed by
    /// outer area id. Retained for areas that left Editor with tabs.
    pub editor_sessions: HashMap<AreaId, EditorSession>,
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
            editor_sessions: HashMap::new(),
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
            // Clone the source area's session: the inner panel layout gets
            // fresh ids, and an empty tab list the host fills by
            // deep-copying the documents.
            if let Some(source) = self.editor_sessions.get(&area_id) {
                let inner_panel_tree =
                    source.inner_panel_tree.clone_with_new_ids(&mut self.next_node_id);
                self.editor_sessions.insert(
                    new_id,
                    EditorSession {
                        tab_list: EditorTabList::empty(),
                        inner_panel_tree,
                    },
                );
            } else {
                self.ensure_editor_session(new_id);
            }
        } else if kind == WindowAreaKind::Editor && mode == AreaSplitMode::Fresh {
            // A fresh editor starts as a blank session.
            self.ensure_editor_session(new_id);
        }
        self.open_window_area_dropdown = None;
        self.active_window_area_border_menu = None;
        Some(new_id)
    }

    pub fn close_window_area(&mut self, area_id: AreaId) {
        if self.window_area_tree.count_leaves() > 1 {
            self.window_area_tree.remove_leaf(area_id);
            // Clean up the editor session of the removed area.
            self.editor_sessions.remove(&area_id);
            self.clear_inner_panel_focus(area_id);
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

    /// Drop inner-panel focus and dropdown state that points into `area_id`.
    fn clear_inner_panel_focus(&mut self, area_id: AreaId) {
        if self
            .focused_editor_inner_panel
            .is_some_and(|loc| loc.area_id == area_id)
        {
            self.focused_editor_inner_panel = None;
        }
        if self
            .open_editor_inner_panel_dropdown
            .is_some_and(|loc| loc.area_id == area_id)
        {
            self.open_editor_inner_panel_dropdown = None;
        }
    }

    /// Get or create the editor session for an area. New sessions start
    /// with no tabs and a single default `Welcome` panel.
    pub fn ensure_editor_session(&mut self, area_id: AreaId) -> &mut EditorSession {
        let next_node_id = &mut self.next_node_id;
        self.editor_sessions.entry(area_id).or_insert_with(|| {
            let panel_id = *next_node_id;
            *next_node_id += 1;
            EditorSession {
                tab_list: EditorTabList::empty(),
                inner_panel_tree: SplitTree::Leaf {
                    id: panel_id,
                    kind: EditorInnerPanelKind::Welcome(WelcomePanelKind::Welcome(None)),
                },
            }
        })
    }

    /// The editor session for `area_id`, if one exists.
    pub fn editor_session(&self, area_id: AreaId) -> Option<&EditorSession> {
        self.editor_sessions.get(&area_id)
    }

    /// The active editor area's session, if an active editor exists.
    pub fn active_editor_session(&self) -> Option<&EditorSession> {
        self.active_editor_area
            .and_then(|area| self.editor_sessions.get(&area))
    }

    /// The editor area's working mode, derived from whether its session
    /// holds tabs. Renderers and editor-internal operations always run on
    /// a foreground area and only consult this dimension.
    pub fn editor_area_mode(&self, area_id: AreaId) -> EditorAreaMode {
        let has_tabs = self
            .editor_sessions
            .get(&area_id)
            .is_some_and(|session| !session.tab_list.tabs.is_empty());
        if has_tabs {
            EditorAreaMode::Editing
        } else {
            EditorAreaMode::Welcome
        }
    }

    /// Whether the area's current kind is Editor (a foreground editor).
    ///
    /// The foreground/background dimension exists for exactly one reason:
    /// the active-editor rule — only foreground editors can be active, so
    /// explorer file opens never land in a background (retained) session.
    pub fn is_foreground_editor(&self, area_id: AreaId) -> bool {
        self.window_area_tree.find_leaf_kind(area_id) == Some(WindowAreaKind::Editor)
    }

    /// Welcome → Editing: every welcome panel migrates to the editing
    /// panel it remembers (or `SourceCode` if it never edited before).
    /// The split structure is preserved. Idempotent.
    pub fn enter_editing(&mut self, area_id: AreaId) {
        if let Some(session) = self.editor_sessions.get_mut(&area_id) {
            let mut rects = Vec::new();
            session
                .inner_panel_tree
                .collect_leaf_rects(0.0, 0.0, 1.0, 1.0, &mut rects);
            let ids: Vec<usize> = rects.iter().map(|rect| rect.id).collect();
            for id in ids {
                let Some(EditorInnerPanelKind::Welcome(WelcomePanelKind::Welcome(previous))) =
                    session.inner_panel_tree.find_leaf_kind(id)
                else {
                    continue;
                };
                session.inner_panel_tree.set_leaf_kind(
                    id,
                    EditorInnerPanelKind::Editing(previous.unwrap_or(EditingPanelKind::SourceCode)),
                );
            }
        }
    }

    /// Editing → Welcome: every panel becomes a welcome panel that
    /// remembers its editing panel type, so entering editing again
    /// restores the previous layout. The split structure is preserved.
    /// Idempotent.
    pub fn exit_editing(&mut self, area_id: AreaId) {
        if let Some(session) = self.editor_sessions.get_mut(&area_id) {
            let mut rects = Vec::new();
            session
                .inner_panel_tree
                .collect_leaf_rects(0.0, 0.0, 1.0, 1.0, &mut rects);
            let ids: Vec<usize> = rects.iter().map(|rect| rect.id).collect();
            for id in ids {
                let Some(EditorInnerPanelKind::Editing(panel)) =
                    session.inner_panel_tree.find_leaf_kind(id)
                else {
                    continue;
                };
                session.inner_panel_tree.set_leaf_kind(
                    id,
                    EditorInnerPanelKind::Welcome(WelcomePanelKind::Welcome(Some(panel))),
                );
            }
        }
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
    // Inner Edit sub-panel layout
    // ------------------------------------------------------------------

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
        let root = &mut self.ensure_editor_session(area_id).inner_panel_tree;
        let kind = root
            .find_leaf_kind(panel_id)
            .unwrap_or(EditorInnerPanelKind::Welcome(WelcomePanelKind::Welcome(None)));
        root.split_leaf_with_ratio(panel_id, new_id, direction, 0.5, kind);
    }

    pub fn close_editor_inner_panel(&mut self, area_id: AreaId, panel_id: PanelId) {
        if let Some(session) = self.editor_sessions.get_mut(&area_id) {
            if session.inner_panel_tree.count_leaves() > 1 {
                session.inner_panel_tree.remove_leaf(panel_id);
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
        kind: EditingPanelKind,
    ) {
        let root = &mut self.ensure_editor_session(area_id).inner_panel_tree;
        root.set_leaf_kind(panel_id, EditorInnerPanelKind::Editing(kind));
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
        if let Some(session) = self.editor_sessions.get_mut(&area_id) {
            let root = &mut session.inner_panel_tree;
            let kind = root
                .find_leaf_kind(panel_id)
                .unwrap_or(EditorInnerPanelKind::Welcome(WelcomePanelKind::Welcome(None)));
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
        if let Some(session) = self.editor_sessions.get_mut(&area_id) {
            let root = &mut session.inner_panel_tree;
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
        if let Some(session) = self.editor_sessions.get_mut(&area_id) {
            let root = &mut session.inner_panel_tree;
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
                if let Some(editor_session) = self.editor_sessions.get_mut(&area_id) {
                    editor_session
                        .inner_panel_tree
                        .set_split_ratio(session.split_id, new_ratio);
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
            if let Some(session) = self.editor_sessions.get(&area_id) {
                let mut norm = Vec::new();
                session
                    .inner_panel_tree
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
            if let Some(session) = self.editor_sessions.get(&area_id) {
                if let Some((dir, span_norm)) = session
                    .inner_panel_tree
                    .find_split_span(split_id, 0.0, 0.0, 1.0, 1.0)
                {
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
            // Leaving Editor: keep the session while it still holds tabs
            // (background editing — switching back restores it) and drop
            // it once empty.
            let has_tabs = self
                .editor_sessions
                .get(&area_id)
                .is_some_and(|session| !session.tab_list.tabs.is_empty());
            if !has_tabs {
                self.editor_sessions.remove(&area_id);
            }
            self.clear_inner_panel_focus(area_id);
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
            self.editor_sessions.remove(&removed);
            self.clear_inner_panel_focus(removed);
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

    /// Swap the area kind of area `a` and area `b`. Editor sessions move
    /// along with the Editor kind so the new Editor area inherits the
    /// swapped-in tabs and panel layout.
    pub fn swap_window_area_kinds(&mut self, a: AreaId, b: AreaId) {
        let type_a = self.window_area_tree.find_leaf_kind(a);
        let type_b = self.window_area_tree.find_leaf_kind(b);
        if let (Some(ta), Some(tb)) = (type_a, type_b) {
            self.window_area_tree.set_leaf_kind(a, tb);
            self.window_area_tree.set_leaf_kind(b, ta);
            let session_a = self.editor_sessions.remove(&a);
            let session_b = self.editor_sessions.remove(&b);
            match (session_a, session_b) {
                (Some(sa), Some(sb)) => {
                    self.editor_sessions.insert(a, sb);
                    self.editor_sessions.insert(b, sa);
                }
                (Some(sa), None) => {
                    // Only `a` had editor state: it follows the Editor
                    // kind over to `b`.
                    self.editor_sessions.insert(b, sa);
                }
                (None, Some(sb)) => {
                    self.editor_sessions.insert(a, sb);
                }
                (None, None) => {}
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
        layout
            .ensure_editor_session(1)
            .inner_panel_tree
            .split_leaf_with_ratio(
                2,
                20,
                Axis::Horizontal,
                0.5,
                EditorInnerPanelKind::Editing(EditingPanelKind::Wysiwyg),
            );
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
        let cloned = layout.editor_sessions.get(&new_id).unwrap();
        assert_eq!(cloned.inner_panel_tree.count_leaves(), 2);
        assert_ne!(
            cloned.inner_panel_tree,
            layout.editor_sessions.get(&1).unwrap().inner_panel_tree
        );

        // The new session's tab list is seeded by the host (deep-copying
        // the documents needs a Context); the layout reserves the entry.
        assert_eq!(cloned.tab_list.tabs.len(), 0);
    }

    #[test]
    fn test_fresh_editor_split_gets_empty_tab_list() {
        let mut layout = WindowLayout::default();
        layout
            .ensure_editor_session(1)
            .inner_panel_tree
            .split_leaf_with_ratio(
                2,
                20,
                Axis::Horizontal,
                0.5,
                EditorInnerPanelKind::Editing(EditingPanelKind::Wysiwyg),
            );
        layout.activate_editor_area(1);

        let new_id = layout
            .split_window_area(1, Axis::Vertical, 0.5, AreaSplitMode::Fresh)
            .unwrap();
        assert_eq!(
            layout.window_area_tree.find_leaf_kind(new_id),
            Some(WindowAreaKind::Editor)
        );
        // A fresh editor starts as a blank session: no cloned inner
        // layout, no tabs. The single panel is the initial welcome panel.
        let session = layout.editor_sessions.get(&new_id).unwrap();
        assert_eq!(session.tab_list.tabs.len(), 0);
        assert_eq!(session.inner_panel_tree.count_leaves(), 1);
        assert_eq!(
            session.inner_panel_tree.find_leaf_kind(new_id + 1),
            Some(EditorInnerPanelKind::Welcome(WelcomePanelKind::Welcome(
                None
            )))
        );
    }

    #[test]
    fn test_editing_mode_transitions_restore_panel_kinds() {
        let mut layout = WindowLayout::default();
        // Enter editing: the initial welcome panel becomes SourceCode.
        layout.ensure_editor_session(1);
        layout.enter_editing(1);
        // Split and customize the panel kinds (status-bar path).
        layout.split_editor_inner_panel(1, 2, Axis::Horizontal);
        layout.change_editor_inner_panel_kind(1, 2, EditingPanelKind::Preview);
        layout.change_editor_inner_panel_kind(1, 3, EditingPanelKind::Wysiwyg);
        // Exit editing: every panel becomes a welcome panel that
        // remembers its previous editing kind; structure is preserved.
        layout.exit_editing(1);
        let inner = &layout.editor_sessions.get(&1).unwrap().inner_panel_tree;
        assert_eq!(inner.count_leaves(), 2);
        assert_eq!(
            inner.find_leaf_kind(2),
            Some(EditorInnerPanelKind::Welcome(WelcomePanelKind::Welcome(
                Some(EditingPanelKind::Preview)
            )))
        );
        assert_eq!(
            inner.find_leaf_kind(3),
            Some(EditorInnerPanelKind::Welcome(WelcomePanelKind::Welcome(
                Some(EditingPanelKind::Wysiwyg)
            )))
        );
        // Enter editing again: the previous kinds are restored.
        layout.enter_editing(1);
        let inner = &layout.editor_sessions.get(&1).unwrap().inner_panel_tree;
        assert_eq!(inner.count_leaves(), 2);
        assert_eq!(
            inner.find_leaf_kind(2),
            Some(EditorInnerPanelKind::Editing(EditingPanelKind::Preview))
        );
        assert_eq!(
            inner.find_leaf_kind(3),
            Some(EditorInnerPanelKind::Editing(EditingPanelKind::Wysiwyg))
        );
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
    fn test_area_mode_transitions() {
        let mut layout = WindowLayout::default();
        // A default root editor that never opened a document: welcome.
        assert_eq!(layout.editor_area_mode(1), EditorAreaMode::Welcome);
        // Fresh-split editors are welcome too.
        let fresh = layout
            .split_window_area(1, Axis::Horizontal, 0.5, AreaSplitMode::Fresh)
            .unwrap();
        assert_eq!(layout.editor_area_mode(fresh), EditorAreaMode::Welcome);
        // Switching a welcome editor to another kind drops its empty
        // session: the area has no editor state left.
        layout.change_window_area_kind(fresh, WindowAreaKind::Explorer);
        assert_eq!(layout.editor_area_mode(fresh), EditorAreaMode::Welcome);
        assert!(layout.editor_sessions.get(&fresh).is_none());
        assert!(!layout.is_foreground_editor(fresh));
        // Switching back to Editor: still welcome (the session is
        // re-created lazily on first use), and foreground again.
        layout.change_window_area_kind(fresh, WindowAreaKind::Editor);
        assert_eq!(layout.editor_area_mode(fresh), EditorAreaMode::Welcome);
        assert!(layout.is_foreground_editor(fresh));
        assert_eq!(layout.active_editor_area, Some(fresh));
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
    fn test_inner_layout_defaults_to_welcome() {
        let mut layout = WindowLayout::default();
        let inner = &layout.ensure_editor_session(1).inner_panel_tree;
        assert_eq!(inner.count_leaves(), 1);
        assert_eq!(
            inner.find_leaf_kind(2),
            Some(EditorInnerPanelKind::Welcome(WelcomePanelKind::Welcome(
                None
            )))
        );
    }

    #[test]
    fn test_inner_split_inherits_target_kind() {
        let mut layout = WindowLayout::default();
        // Set up inner: Wysiwyg panel (id 2, first allocated panel id).
        let _ = layout.ensure_editor_session(1);
        layout.change_editor_inner_panel_kind(1, 2, EditingPanelKind::Wysiwyg);
        // Split it via the status-bar path; the new panel inherits Wysiwyg.
        layout.split_editor_inner_panel(1, 2, Axis::Horizontal);
        let inner = &layout.editor_sessions.get(&1).unwrap().inner_panel_tree;
        assert_eq!(inner.count_leaves(), 2);
        assert_eq!(
            inner.find_leaf_kind(2),
            Some(EditorInnerPanelKind::Editing(EditingPanelKind::Wysiwyg))
        );
        // The new leaf (id 3) is also Wysiwyg.
        assert_eq!(
            inner.find_leaf_kind(3),
            Some(EditorInnerPanelKind::Editing(EditingPanelKind::Wysiwyg))
        );
    }

    #[test]
    fn test_corner_drag_split_inherits_dragged_kind() {
        let mut layout = WindowLayout::default();
        let _ = layout.ensure_editor_session(1);
        layout.change_editor_inner_panel_kind(1, 2, EditingPanelKind::Preview);
        // Corner-drag split; the new panel inherits Preview.
        layout.split_editor_inner_panel_with_ratio(1, 2, Axis::Vertical, 0.4);
        let inner = &layout.editor_sessions.get(&1).unwrap().inner_panel_tree;
        assert_eq!(inner.count_leaves(), 2);
        assert_eq!(
            inner.find_leaf_kind(3),
            Some(EditorInnerPanelKind::Editing(EditingPanelKind::Preview))
        );
    }
}
