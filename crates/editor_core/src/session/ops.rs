//! Editor pane operations of an Editor panel.

use gpui::Context;
use splitter::NodeId;
use splitter::tree::SplitAxis;
use workspace::EditorPanelMode;

use crate::editor::Editor;
use crate::session::{EditorPaneKind, EditorSession};
use editor_model::PaneId;

impl Editor {
    /// This editor's session, mutably (always present).
    pub fn session_mut(&mut self) -> &mut EditorSession {
        &mut self.session
    }

    /// This editor's session.
    pub fn session(&self) -> &EditorSession {
        &self.session
    }

    /// The editor panel's working mode, derived from whether its session holds tabs.
    pub fn panel_mode(&self) -> EditorPanelMode {
        if self.session.has_tabs() {
            EditorPanelMode::Editing
        } else {
            EditorPanelMode::Welcome
        }
    }

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
        if let Some(host) = self.host.clone() {
            host.clear_outer_dropdowns(cx);
        }
    }

    pub fn change_pane_kind(&mut self, pane_id: impl Into<PaneId>, kind: EditorPaneKind) {
        let pane_id = pane_id.into();
        self.session.root.set_kind(pane_id.0, kind);
        self.session.root.activate_leaf(pane_id.0);
        self.session.root.clear_dropdowns();
        self.focused_pane_id = Some(pane_id);
        if let Some(state) = self.pane_state_mut(pane_id) {
            state.ensure_kind(kind);
        }
    }

    pub fn split_pane_with_ratio(&mut self, pane_id: impl Into<PaneId>, axis: SplitAxis, ratio: f32) {
        let pane_id = pane_id.into();
        if let Some(panel) = self.session.root.tree.find_leaf_mut(pane_id.0) {
            panel.maximized = false;
        }
        self.session
            .root
            .split_leaf(pane_id.0, axis, ratio);
    }

    pub fn swap_pane_kinds(&mut self, a: impl Into<PaneId>, b: impl Into<PaneId>) {
        self.session.root.swap_kinds(a.into().0, b.into().0);
    }

    pub fn swap_pane_split_sides(&mut self, split_id: NodeId) {
        self.session.root.swap_split_sides(split_id);
    }

    pub fn toggle_pane_maximize(&mut self, pane_id: impl Into<PaneId>) {
        self.session.root.toggle_maximize(pane_id.into().0);
    }
}
