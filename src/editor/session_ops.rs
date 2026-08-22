//! Editor pane operations of an Editor panel.
//!
//! The per-area editor sessions (document tab list +  pane split
//! root) and all pane operations. The split root is the same
//! [`SplitterRoot`] the outer window tree uses, so pane splits,
//! joins, swaps, and drags go through the shared root API instead of a
//! copied state machine.

use crate::app::window_panels::EditorPanelMode;
use crate::editor::controller::Editor;
use crate::editor::session::{EditorPaneKind, EditorSession};
use crate::splitter::NodeId;
use gpui::Context;
use splitype_splitter::tree::SplitAxis;

impl Editor {
    /// This editor's session, mutably (always present).
    pub fn session_mut(&mut self) -> &mut EditorSession {
        &mut self.session
    }

    /// This editor's session.
    pub fn session(&self) -> &EditorSession {
        &self.session
    }

    /// The editor panel's working mode, derived from whether its session
    /// holds tabs. Renderers and editor-internal operations only consult
    /// this dimension.
    pub fn panel_mode(&self) -> EditorPanelMode {
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

    /// Splits a pane via the status-bar buttons. The new pane inherits the
    /// target pane's kind so the split keeps the same view style.
    pub fn split_pane(&mut self, pane_id: NodeId, axis: SplitAxis) {
        self.split_pane_with_ratio(pane_id, axis, 0.5);
    }

    pub fn close_pane(&mut self, pane_id: NodeId) {
        self.session.root.close_leaf_or_divider(pane_id);
    }

    pub fn toggle_pane_dropdown(&mut self, pane_id: NodeId, cx: &mut Context<Self>) {
        self.session.root.toggle_dropdown(pane_id);
        // Opening an inner dropdown closes any outer dropdown.
        if let Some(shell) = self.shell.clone() {
            let _ = shell.update(cx, |shell, _cx| shell.panels.layout.clear_dropdowns());
        }
    }

    pub fn change_pane_kind(&mut self, pane_id: NodeId, kind: EditorPaneKind) {
        self.session.root.set_kind(pane_id, kind);
        self.session.root.activate_leaf(pane_id);
        self.session.root.clear_dropdowns();
        self.focused_pane_id = Some(pane_id);
    }

    /// Inner split created via corner drag or divider border menu. The new pane inherits the
    /// dragged/target pane's kind so both sides keep the same view style.
    pub fn split_pane_with_ratio(&mut self, pane_id: NodeId, axis: SplitAxis, ratio: f32) {
        self.session
            .root
            .split_leaf_or_divider(pane_id, axis, ratio);
    }

    /// Swap pane kinds between two panes.
    pub fn swap_pane_kinds(&mut self, a: NodeId, b: NodeId) {
        self.session.root.swap_kinds(a, b);
    }

    /// Swap the two sides of a pane split node (border-menu action).
    pub fn swap_pane_split_sides(&mut self, split_id: NodeId) {
        self.session.root.swap_split_sides(split_id);
    }
}
