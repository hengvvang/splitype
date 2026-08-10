//! Inner panel layout state and operations of an Editor window.
//!
//! The per-area editor sessions (document tab list + inner panel split
//! tree) and all inner-panel operations. Moved out of `splitype-layout`
//! when the layout crate was reduced to the outer window tree; the outer
//! tree lives in `WindowPanels::layout`.

use gpui::*;

use crate::editor::controller::Editor;
use crate::editor::panels::panel_types::{
    EditingPanelKind, EditorInnerPanelDragAction, EditorInnerPanelKind, EditorSession,
    EditorTabList, InnerPanelLocation, WelcomePanelKind,
};
use splitype_layout::sessions::{
    CornerDragModifier, CornerDragPreview, CornerDragSession, MODIFIER_THRESHOLD_PX, id_at_point,
};
use splitype_layout::tree::{AreaRect, Axis, Direction, SplitTree};
use splitype_layout::types::EditorAreaMode;
use splitype_layout::types::{AreaId, AreaSplitMode, PanelId, SplitId, WindowAreaKind};

impl Editor {
    /// Split `area_id` at `ratio` with a sibling of the SAME kind, and seed
    /// the new Editor area's session per `mode`: [`AreaSplitMode::Copy`]
    /// clones the source inner panel layout (the host then deep-copies the
    /// tab list); [`AreaSplitMode::Fresh`] starts blank. Returns the new
    /// area's id.
    pub fn split_window_area(
        &mut self,
        area_id: AreaId,
        direction: Axis,
        ratio: f32,
        mode: AreaSplitMode,
    ) -> Option<AreaId> {
        let new_id = self
            .panels
            .layout
            .split_window_area(area_id, direction, ratio)?;
        let kind = self.panels.layout.window_area_tree.find_leaf_kind(area_id);
        if kind == Some(WindowAreaKind::Editor) {
            match mode {
                AreaSplitMode::Copy => {
                    if let Some(source) = self.editor_sessions.get(&area_id) {
                        let inner_panel_tree = source
                            .inner_panel_tree
                            .clone_with_new_ids(&mut self.panels.layout.next_node_id);
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
                }
                AreaSplitMode::Fresh => {
                    self.ensure_editor_session(new_id);
                }
            }
        }
        Some(new_id)
    }

    /// Close an area and clean up its editor session.
    pub fn close_window_area(&mut self, area_id: AreaId) {
        self.panels.layout.close_window_area(area_id);
        self.editor_sessions.remove(&area_id);
        self.clear_inner_panel_focus(area_id);
    }

    /// Join `removed` into `into`, cleaning up the removed area's session.
    pub fn join_window_area(&mut self, into: AreaId, removed: AreaId) -> bool {
        let ok = self.panels.layout.join_window_area(into, removed);
        if ok {
            self.editor_sessions.remove(&removed);
            self.clear_inner_panel_focus(removed);
        }
        ok
    }

    /// Swap the area kind of area `a` and area `b`. Editor sessions move
    /// along with the Editor kind so the new Editor area inherits the
    /// swapped-in tabs and panel layout.
    pub fn swap_window_area_kinds(&mut self, a: AreaId, b: AreaId) {
        let type_a = self.panels.layout.window_area_tree.find_leaf_kind(a);
        let type_b = self.panels.layout.window_area_tree.find_leaf_kind(b);
        self.panels.layout.swap_window_area_kinds(a, b);
        if let (Some(_), Some(_)) = (type_a, type_b) {
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
        }
    }

    /// Change an area's kind. Leaving Editor keeps the session while it
    /// still holds tabs (background editing — switching back restores it)
    /// and drops it once empty.
    pub fn change_window_area_kind(&mut self, area_id: AreaId, kind: WindowAreaKind) {
        let previous = self.panels.layout.window_area_tree.find_leaf_kind(area_id);
        self.panels.layout.change_window_area_kind(area_id, kind);
        if previous == Some(WindowAreaKind::Editor) && kind != WindowAreaKind::Editor {
            let has_tabs = self
                .editor_sessions
                .get(&area_id)
                .is_some_and(|session| !session.tab_list.tabs.is_empty());
            if !has_tabs {
                self.editor_sessions.remove(&area_id);
            }
            self.clear_inner_panel_focus(area_id);
        } else if kind == WindowAreaKind::Editor && previous != Some(WindowAreaKind::Editor) {
            // Entering Editor: an existing background session (tabs) is
            // restored; a fresh area stays blank until its first use.
            // Either way the switch is an explicit interaction, so the
            // area becomes the active editor.
            self.panels.layout.activate_editor_area(area_id);
        }
    }

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
        let next_node_id = &mut self.panels.layout.next_node_id;
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
        self.panels
            .layout
            .active_editor_area
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
        let new_id = self.panels.layout.next_node_id;
        self.panels.layout.next_node_id += 1;
        let root = &mut self.ensure_editor_session(area_id).inner_panel_tree;
        let kind = root
            .find_leaf_kind(panel_id)
            .unwrap_or(EditorInnerPanelKind::Welcome(WelcomePanelKind::Welcome(
                None,
            )));
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
            self.panels.layout.open_window_area_dropdown = None;
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
        let new_id = self.panels.layout.next_node_id;
        self.panels.layout.next_node_id += 1;
        if let Some(session) = self.editor_sessions.get_mut(&area_id) {
            let root = &mut session.inner_panel_tree;
            let kind = root
                .find_leaf_kind(panel_id)
                .unwrap_or(EditorInnerPanelKind::Welcome(WelcomePanelKind::Welcome(
                    None,
                )));
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

    /// Swap the two sides of an inner split node (border-menu action).
    pub fn swap_editor_inner_panel_split_sides(&mut self, area_id: AreaId, split_id: SplitId) {
        if let Some(session) = self.editor_sessions.get_mut(&area_id) {
            session.inner_panel_tree.swap_sibling_leaves(split_id);
        }
        self.active_editor_inner_panel_border_menu = None;
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
            if let Some(rect) = self
                .panels
                .layout
                .window_area_rect(session.target_id, &leaf_rects)
            {
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
            CornerDragPreview::JoinPreview {
                target_id,
                direction: _,
            } => Some(EditorInnerPanelDragAction::Join {
                from_panel: session.target_id,
                into_panel: target_id,
            }),
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
}
