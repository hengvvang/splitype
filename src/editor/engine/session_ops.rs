//! Editor pane operations of an Editor panel.
//!
//! The per-area editor sessions (document tab list +  pane split
//! root) and all pane operations. The split root is the same
//! [`SplitterRoot`] the outer window tree uses, so pane splits,
//! joins, swaps, and drags go through the shared root API instead of a
//! copied state machine.

use crate::app::window::panels::EditorPanelMode;
use crate::editor::engine::controller::{Editor, PaneId};
use crate::editor::engine::session::{EditorPaneKind, EditorSession};
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
        if self.session.has_tabs() {
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
    pub fn split_pane(&mut self, pane_id: impl Into<PaneId>, axis: SplitAxis) {
        self.split_pane_with_ratio(pane_id, axis, 0.5);
    }

    pub fn close_pane(&mut self, pane_id: impl Into<PaneId>) {
        self.session.root.close_leaf(pane_id.into().0);
    }

    pub fn toggle_pane_dropdown(&mut self, pane_id: impl Into<PaneId>, cx: &mut Context<Self>) {
        self.session.root.toggle_dropdown(pane_id.into().0);
        // Opening an inner dropdown closes any outer dropdown.
        if let Some(shell) = self.shell.clone() {
            let _ = shell.update(cx, |shell, _cx| shell.panels.layout.clear_dropdowns());
        }
    }

    pub fn change_pane_kind(&mut self, pane_id: impl Into<PaneId>, kind: EditorPaneKind) {
        let pane_id = pane_id.into();
        self.session.root.set_kind(pane_id.0, kind);
        self.session.root.activate_leaf(pane_id.0);
        self.session.root.clear_dropdowns();
        self.focused_pane_id = Some(pane_id);
    }

    /// Inner split created via corner drag or divider border menu. The new pane inherits the
    /// dragged/target pane's kind so both sides keep the same view style.
    pub fn split_pane_with_ratio(&mut self, pane_id: impl Into<PaneId>, axis: SplitAxis, ratio: f32) {
        let pane_id = pane_id.into();
        if let Some(panel) = self.session.root.tree.find_leaf_mut(pane_id.0) {
            panel.maximized = false;
        }
        self.session
            .root
            .split_leaf(pane_id.0, axis, ratio);
    }

    /// Swap pane kinds between two panes.
    pub fn swap_pane_kinds(&mut self, a: impl Into<PaneId>, b: impl Into<PaneId>) {
        self.session.root.swap_kinds(a.into().0, b.into().0);
    }

    /// Swap the two sides of a pane split node (border-menu action).
    pub fn swap_pane_split_sides(&mut self, split_id: NodeId) {
        self.session.root.swap_split_sides(split_id);
    }

    /// Toggle the maximized state of an inner editor pane.
    pub fn toggle_pane_maximize(&mut self, pane_id: impl Into<PaneId>) {
        self.session.root.toggle_maximize(pane_id.into().0);
    }
}
