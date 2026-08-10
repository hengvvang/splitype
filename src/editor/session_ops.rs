//! Inner panel layout state and operations of an Editor window.
//!
//! The per-area editor sessions (document tab list + inner panel split
//! root) and all inner-panel operations. The split root is the same
//! [`SplitterRoot`] the outer window tree uses, so inner-panel splits,
//! joins, swaps, and drags go through the shared root API instead of a
//! copied state machine.

use crate::app::window_area::EditorAreaMode;
use crate::app::window_area::WindowAreaKind;
use crate::editor::controller::Editor;
use crate::editor::session::{
    EditingPanelKind, EditorInnerPanelKind, EditorSession, EditorTabList, WelcomePanelKind,
};
use crate::splitter::NodeId;
use splitype_splitter::root::SplitterRoot;
use splitype_splitter::tree::Axis;

/// Create a fresh session: one welcome panel. The inner root is fully
/// self-contained — it numbers its own nodes from 1, so nested roots
/// never share state with the outer layout.
fn new_inner_session() -> EditorSession {
    let root = SplitterRoot::single_leaf(
        1,
        EditorInnerPanelKind::Welcome(WelcomePanelKind::Welcome(None)),
    );
    EditorSession {
        tab_list: EditorTabList::empty(),
        root,
    }
}

impl Editor {
    /// Split `area_id` at `ratio` with a sibling of the SAME kind. With
    /// `copy_content = false` the new Editor area starts with a fresh
    /// blank session; with `true` the caller runs the shared content
    /// steps ([`Editor::seed_split_content`]) right after this call.
    /// Returns the new area's id.
    pub fn split_window_area(
        &mut self,
        area_id: NodeId,
        direction: Axis,
        ratio: f32,
        copy_content: bool,
    ) -> Option<NodeId> {
        let new_id = self.panels.layout.split_leaf(area_id, direction, ratio)?;
        if !copy_content
            && self.panels.layout.tree.find_leaf_kind(area_id) == Some(WindowAreaKind::Editor)
        {
            self.ensure_editor_session(new_id);
        }
        Some(new_id)
    }

    /// Close an area and clean up its editor session.
    pub fn close_window_area(&mut self, area_id: NodeId) {
        self.panels.layout.close_leaf(area_id);
        self.editor_sessions.remove(&area_id);
        self.clear_inner_panel_focus(area_id);
    }

    /// Join `removed` into `into`, cleaning up the removed area's session.
    pub fn join_window_area(&mut self, into: NodeId, removed: NodeId) -> bool {
        let ok = self.panels.layout.join_leaves(into, removed);
        if ok {
            self.editor_sessions.remove(&removed);
            self.clear_inner_panel_focus(removed);
        }
        ok
    }

    /// Swap the area kind of area `a` and area `b`. Editor sessions move
    /// along with the Editor kind so the new Editor area inherits the
    /// swapped-in tabs and panel layout.
    pub fn swap_window_area_kinds(&mut self, a: NodeId, b: NodeId) {
        let type_a = self.panels.layout.tree.find_leaf_kind(a);
        let type_b = self.panels.layout.tree.find_leaf_kind(b);
        self.panels.layout.swap_kinds(a, b);
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
    pub fn change_window_area_kind(&mut self, area_id: NodeId, kind: WindowAreaKind) {
        let previous = self.panels.layout.tree.find_leaf_kind(area_id);
        self.panels.layout.set_kind(area_id, kind);
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
            self.panels.layout.activate_area(area_id);
        }
    }

    fn clear_inner_panel_focus(&mut self, area_id: NodeId) {
        if self
            .focused_editor_inner_panel
            .is_some_and(|loc| loc.area_id == area_id)
        {
            self.focused_editor_inner_panel = None;
        }
        if let Some(session) = self.editor_sessions.get_mut(&area_id) {
            session.root.clear_dropdowns();
        }
    }
    /// Get or create the editor session for an area. New sessions start
    /// with no tabs and a single default `Welcome` panel.
    pub fn ensure_editor_session(&mut self, area_id: NodeId) -> &mut EditorSession {
        self.editor_sessions
            .entry(area_id)
            .or_insert_with(new_inner_session)
    }

    /// The editor session for `area_id`, if one exists.
    pub fn editor_session(&self, area_id: NodeId) -> Option<&EditorSession> {
        self.editor_sessions.get(&area_id)
    }

    /// The active editor area's session, if an active editor exists.
    pub fn active_editor_session(&self) -> Option<&EditorSession> {
        self.panels
            .layout
            .active_area
            .and_then(|area| self.editor_sessions.get(&area))
    }

    /// The editor area's working mode, derived from whether its session
    /// holds tabs. Renderers and editor-internal operations always run on
    /// a foreground area and only consult this dimension.
    pub fn editor_area_mode(&self, area_id: NodeId) -> EditorAreaMode {
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
    pub fn enter_editing(&mut self, area_id: NodeId) {
        if let Some(session) = self.editor_sessions.get_mut(&area_id) {
            let mut rects = Vec::new();
            session
                .root
                .tree
                .collect_leaf_rects(0.0, 0.0, 1.0, 1.0, &mut rects);
            let ids: Vec<usize> = rects.iter().map(|rect| rect.id).collect();
            for id in ids {
                let Some(EditorInnerPanelKind::Welcome(WelcomePanelKind::Welcome(previous))) =
                    session.root.tree.find_leaf_kind(id)
                else {
                    continue;
                };
                session.root.tree.set_leaf_kind(
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
    pub fn exit_editing(&mut self, area_id: NodeId) {
        if let Some(session) = self.editor_sessions.get_mut(&area_id) {
            let mut rects = Vec::new();
            session
                .root
                .tree
                .collect_leaf_rects(0.0, 0.0, 1.0, 1.0, &mut rects);
            let ids: Vec<usize> = rects.iter().map(|rect| rect.id).collect();
            for id in ids {
                let Some(EditorInnerPanelKind::Editing(panel)) =
                    session.root.tree.find_leaf_kind(id)
                else {
                    continue;
                };
                session.root.tree.set_leaf_kind(
                    id,
                    EditorInnerPanelKind::Welcome(WelcomePanelKind::Welcome(Some(panel))),
                );
            }
        }
    }
    // ------------------------------------------------------------------
    // Inner panel layout (via the session's root container)
    // ------------------------------------------------------------------

    /// Splits an inner panel via the status-bar buttons. The new panel
    /// inherits the target panel's kind so the split keeps the same view
    /// style.
    pub fn split_editor_inner_panel(&mut self, area_id: NodeId, panel_id: NodeId, direction: Axis) {
        self.split_editor_inner_panel_with_ratio(area_id, panel_id, direction, 0.5);
    }

    pub fn close_editor_inner_panel(&mut self, area_id: NodeId, panel_id: NodeId) {
        if let Some(session) = self.editor_sessions.get_mut(&area_id) {
            session.root.close_leaf(panel_id);
        }
    }

    pub fn toggle_editor_inner_panel_dropdown(&mut self, area_id: NodeId, panel_id: NodeId) {
        if let Some(session) = self.editor_sessions.get_mut(&area_id) {
            session.root.toggle_dropdown(panel_id);
            // Opening an inner dropdown closes any outer dropdown.
            self.panels.layout.clear_dropdowns();
        }
    }

    pub fn change_editor_inner_panel_kind(
        &mut self,
        area_id: NodeId,
        panel_id: NodeId,
        kind: EditingPanelKind,
    ) {
        if let Some(session) = self.editor_sessions.get_mut(&area_id) {
            session
                .root
                .set_kind(panel_id, EditorInnerPanelKind::Editing(kind));
        }
    }

    /// Inner split created via corner drag. The new panel inherits the
    /// dragged panel's kind so both sides keep the same view style.
    pub fn split_editor_inner_panel_with_ratio(
        &mut self,
        area_id: NodeId,
        panel_id: NodeId,
        direction: Axis,
        ratio: f32,
    ) {
        let session = self
            .editor_sessions
            .entry(area_id)
            .or_insert_with(new_inner_session);
        session.root.split_leaf(panel_id, direction, ratio);
    }

    /// Join an inner panel into another within the same inner tree.
    pub fn join_editor_inner_panel(
        &mut self,
        area_id: NodeId,
        into: NodeId,
        removed: NodeId,
    ) -> bool {
        if let Some(session) = self.editor_sessions.get_mut(&area_id) {
            session.root.join_leaves(into, removed)
        } else {
            false
        }
    }

    /// Swap area types between two inner panels.
    pub fn swap_editor_inner_panel_kinds(&mut self, area_id: NodeId, a: NodeId, b: NodeId) {
        if let Some(session) = self.editor_sessions.get_mut(&area_id) {
            session.root.swap_kinds(a, b);
        }
    }

    /// Swap the two sides of an inner split node (border-menu action).
    pub fn swap_editor_inner_panel_split_sides(&mut self, area_id: NodeId, split_id: NodeId) {
        if let Some(session) = self.editor_sessions.get_mut(&area_id) {
            session.root.swap_split_sides(split_id);
        }
    }
}
