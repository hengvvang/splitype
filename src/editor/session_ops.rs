//! Editor pane operations of an Editor window.
//!
//! The per-area editor sessions (document tab list +  pane split
//! root) and all pane operations. The split root is the same
//! [`SplitterRoot`] the outer window tree uses, so pane splits,
//! joins, swaps, and drags go through the shared root API instead of a
//! copied state machine.

use crate::app::window_area::EditorPanelMode;
use crate::editor::controller::Editor;
use crate::editor::session::{EditorPaneKind, EditorSession};
use crate::splitter::NodeId;
use gpui::Context;
use splitype_splitter::tree::Axis;

impl Editor {
    pub fn ensure_editor_session(&mut self, _panel_id: NodeId) -> &mut EditorSession {
        &mut self.session
    }

    /// The editor session for `panel_id`, if one exists.
    pub fn editor_session(&self, _panel_id: NodeId) -> Option<&EditorSession> {
        Some(&self.session)
    }

    /// The active editor area's session, if an active editor exists.
    pub fn active_editor_session(&self) -> Option<&EditorSession> {
        Some(&self.session)
    }

    /// The editor area's working mode, derived from whether its session
    /// holds tabs. Renderers and editor-internal operations always run on
    /// a foreground area and only consult this dimension.
    pub fn panel_mode(&self, _panel_id: NodeId) -> EditorPanelMode {
        let has_tabs = !self.session.tab_list.tabs.is_empty();
        if has_tabs {
            EditorPanelMode::Editing
        } else {
            EditorPanelMode::Welcome
        }
    }
    // ------------------------------------------------------------------
    // Pane layout (via the session's root container)
    // ------------------------------------------------------------------

    /// Splits a pane via the status-bar buttons. The new pane
    /// inherits the target panel's kind so the split keeps the same view
    /// style.
    pub fn split_pane(&mut self, panel_id: NodeId, pane_id: NodeId, direction: Axis) {
        self.split_pane_with_ratio(panel_id, pane_id, direction, 0.5);
    }

    pub fn close_pane(&mut self, panel_id: NodeId, pane_id: NodeId) {
        let session = self.ensure_editor_session(panel_id);
        session.root.close_leaf(pane_id);
    }

    pub fn toggle_pane_dropdown(
        &mut self,
        panel_id: NodeId,
        pane_id: NodeId,
        cx: &mut Context<Self>,
    ) {
        let session = self.ensure_editor_session(panel_id);
        session.root.toggle_dropdown(pane_id);
        // Opening an inner dropdown closes any outer dropdown.
        if let Some(shell) = self.shell.clone() {
            let _ = shell.update(cx, |shell, _cx| shell.panels.layout.clear_dropdowns());
        }
    }

    pub fn change_pane_kind(
        &mut self,
        panel_id: NodeId,
        pane_id: NodeId,
        kind: EditorPaneKind,
    ) {
        let session = self.ensure_editor_session(panel_id);
        session.root.set_kind(pane_id, kind);
    }

    /// Inner split created via corner drag. The new panel inherits the
    /// dragged panel's kind so both sides keep the same view style.
    pub fn split_pane_with_ratio(
        &mut self,
        panel_id: NodeId,
        pane_id: NodeId,
        direction: Axis,
        ratio: f32,
    ) {
        let session = self.ensure_editor_session(panel_id);
        session.root.split_leaf(pane_id, direction, ratio);
    }

    /// Swap pane kinds between two panes.
    pub fn swap_pane_kinds(&mut self, panel_id: NodeId, a: NodeId, b: NodeId) {
        let session = self.ensure_editor_session(panel_id);
        session.root.swap_kinds(a, b);
    }

    /// Swap the two sides of an inner split node (border-menu action).
    pub fn swap_pane_split_sides(&mut self, panel_id: NodeId, split_id: NodeId) {
        let session = self.ensure_editor_session(panel_id);
        session.root.swap_split_sides(split_id);
    }
}
