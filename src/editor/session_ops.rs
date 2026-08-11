//! Inner panel layout state and operations of an Editor window.
//!
//! The per-area editor sessions (document tab list + inner panel split
//! root) and all inner-panel operations. The split root is the same
//! [`SplitterRoot`] the outer window tree uses, so inner-panel splits,
//! joins, swaps, and drags go through the shared root API instead of a
//! copied state machine.

use crate::app::window_area::EditorAreaMode;
use crate::editor::controller::Editor;
use crate::editor::session::{
    EditingPanelKind, EditorInnerPanelKind, EditorSession, WelcomePanelKind,
};
use crate::splitter::NodeId;
use gpui::Context;
use splitype_splitter::tree::Axis;

impl Editor {
    pub fn ensure_editor_session(&mut self, _area_id: NodeId) -> &mut EditorSession {
        &mut self.session
    }

    /// The editor session for `area_id`, if one exists.
    pub fn editor_session(&self, _area_id: NodeId) -> Option<&EditorSession> {
        Some(&self.session)
    }

    /// The active editor area's session, if an active editor exists.
    pub fn active_editor_session(&self) -> Option<&EditorSession> {
        Some(&self.session)
    }

    /// The editor area's working mode, derived from whether its session
    /// holds tabs. Renderers and editor-internal operations always run on
    /// a foreground area and only consult this dimension.
    pub fn editor_area_mode(&self, _area_id: NodeId) -> EditorAreaMode {
        let has_tabs = !self.session.tab_list.tabs.is_empty();
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
        let session = self.ensure_editor_session(area_id);
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

    /// Editing → Welcome: every panel becomes a welcome panel that
    /// remembers its editing panel type, so entering editing again
    /// restores the previous layout. The split structure is preserved.
    /// Idempotent.
    pub fn exit_editing(&mut self, area_id: NodeId) {
        let session = self.ensure_editor_session(area_id);
        let mut rects = Vec::new();
        session
            .root
            .tree
            .collect_leaf_rects(0.0, 0.0, 1.0, 1.0, &mut rects);
        let ids: Vec<usize> = rects.iter().map(|rect| rect.id).collect();
        for id in ids {
            let Some(EditorInnerPanelKind::Editing(panel)) = session.root.tree.find_leaf_kind(id)
            else {
                continue;
            };
            session.root.tree.set_leaf_kind(
                id,
                EditorInnerPanelKind::Welcome(WelcomePanelKind::Welcome(Some(panel))),
            );
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
        let session = self.ensure_editor_session(area_id);
        session.root.close_leaf(panel_id);
    }

    pub fn toggle_editor_inner_panel_dropdown(
        &mut self,
        area_id: NodeId,
        panel_id: NodeId,
        cx: &mut Context<Self>,
    ) {
        let session = self.ensure_editor_session(area_id);
        session.root.toggle_dropdown(panel_id);
        // Opening an inner dropdown closes any outer dropdown.
        if let Some(shell) = self.shell.clone() {
            let _ = shell.update(cx, |shell, _cx| shell.panels.layout.clear_dropdowns());
        }
    }

    pub fn change_editor_inner_panel_kind(
        &mut self,
        area_id: NodeId,
        panel_id: NodeId,
        kind: EditingPanelKind,
    ) {
        let session = self.ensure_editor_session(area_id);
        session
            .root
            .set_kind(panel_id, EditorInnerPanelKind::Editing(kind));
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
        let session = self.ensure_editor_session(area_id);
        session.root.split_leaf(panel_id, direction, ratio);
    }

    /// Swap area types between two inner panels.
    pub fn swap_editor_inner_panel_kinds(&mut self, area_id: NodeId, a: NodeId, b: NodeId) {
        let session = self.ensure_editor_session(area_id);
        session.root.swap_kinds(a, b);
    }

    /// Swap the two sides of an inner split node (border-menu action).
    pub fn swap_editor_inner_panel_split_sides(&mut self, area_id: NodeId, split_id: NodeId) {
        let session = self.ensure_editor_session(area_id);
        session.root.swap_split_sides(split_id);
    }
}
