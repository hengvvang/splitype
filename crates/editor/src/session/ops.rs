//! Editor pane operations of an Editor panel.

use gpui::Context;
use splitter::NodeId;
use splitter::tree::SplitAxis;

use crate::editor::Editor;
use crate::session::{EditorSession, PaneKind};
use editor_contracts::PaneId;

impl Editor {
    /// This editor's session, mutably (always present).
    pub fn session_mut(&mut self) -> &mut EditorSession {
        &mut self.session
    }

    /// This editor's session.
    pub fn session(&self) -> &EditorSession {
        &self.session
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

    pub fn change_pane_kind(&mut self, pane_id: impl Into<PaneId>, kind: PaneKind) {
        let pane_id = pane_id.into();
        self.session.root.set_kind(pane_id.0, kind.clone());
        self.session.root.activate_leaf(pane_id.0);
        self.session.root.clear_dropdowns();
        self.focused_pane_id = Some(pane_id);
        if let Some(state) = self.pane_state_mut(pane_id) {
            state.ensure_kind(kind);
        }
    }

    pub fn split_pane_with_ratio(
        &mut self,
        pane_id: impl Into<PaneId>,
        axis: SplitAxis,
        ratio: f32,
    ) {
        let pane_id = pane_id.into();
        if let Some(panel) = self.session.root.tree.find_leaf_mut(pane_id.0) {
            panel.maximized = false;
        }
        self.session.root.split_leaf(pane_id.0, axis, ratio);
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

    /// Toggles the active pane's maximized state and refreshes the view.
    pub fn toggle_maximize_pane(&mut self, cx: &mut Context<Self>) {
        let active = self.active_pane_id();
        self.toggle_pane_maximize(active);
        cx.notify();
    }

    /// Switches `pane_id` to `kind`, resetting its viewport and syncing the
    /// freshly created pane with the active document.
    pub fn select_pane_kind(&mut self, pane_id: PaneId, kind: PaneKind, cx: &mut Context<Self>) {
        self.change_pane_kind(pane_id, kind);
        {
            let state = self.pane_state(pane_id);
            state.scroll.pending_autoscroll = Some(editor_contracts::AutoscrollStrategy::Fit {
                margin: gpui::px(20.0),
            });
            state.scroll.last_viewport_size = None;
        }
        if let Some(tab) = self.session.active_tab_mut() {
            tab.file.pending_window_title_refresh = true;
            tab.file.close_dialog_restore_focus = None;
        }
        self.sync_panes_with_active_tab(cx);
        cx.notify();
    }

    /// Cycles the active pane through every registered pane kind.
    pub fn toggle_pane_kind(&mut self, cx: &mut Context<Self>) {
        let active_pane = self.active_pane_id();
        let current_kind = self.active_pane_kind();
        let descriptors =
            editor_contracts::PaneRegistry::registered_descriptors().unwrap_or_default();
        let next_kind = if descriptors.is_empty() {
            current_kind
        } else {
            let current_idx = descriptors
                .iter()
                .position(|d| d.kind() == current_kind)
                .unwrap_or(0);
            let next_idx = (current_idx + 1) % descriptors.len();
            descriptors[next_idx].kind()
        };
        self.select_pane_kind(active_pane, next_kind, cx);
    }

    pub fn handle_pane_key_down(
        &mut self,
        pane_id: PaneId,
        event: &gpui::KeyDownEvent,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> bool {
        let prev_text = self
            .pane_state_ref(pane_id)
            .and_then(|p| p.pane.serialize_text(cx));
        let host = self.pane_host.clone();
        let handled = if let Some(pane_state) = self.pane_state_mut(pane_id) {
            pane_state
                .pane
                .handle_key_down(pane_id, event, window, cx, &*host)
        } else {
            false
        };
        if handled {
            let next_text = self
                .pane_state_ref(pane_id)
                .and_then(|p| p.pane.serialize_text(cx));
            if let Some(next_text) = next_text {
                if prev_text.as_ref() != Some(&next_text) {
                    self.update_raw_document_text(next_text, pane_id, cx);
                }
            }
            cx.notify();
        }
        handled
    }

    pub fn handle_pane_mouse_down(
        &mut self,
        pane_id: PaneId,
        event: &gpui::MouseDownEvent,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        self.focus_pane(pane_id, window, cx);
        if let Some(pane_state) = self.pane_state_mut(pane_id) {
            pane_state
                .pane
                .handle_mouse_down(pane_id, event, window, cx);
            cx.notify();
        }
    }

    pub fn handle_pane_mouse_move(
        &mut self,
        pane_id: PaneId,
        event: &gpui::MouseMoveEvent,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        if let Some(pane_state) = self.pane_state_mut(pane_id) {
            pane_state
                .pane
                .handle_mouse_move(pane_id, event, window, cx);
            cx.notify();
        }
    }

    pub fn handle_pane_mouse_up(
        &mut self,
        pane_id: PaneId,
        event: &gpui::MouseUpEvent,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        if let Some(pane_state) = self.pane_state_mut(pane_id) {
            pane_state.pane.handle_mouse_up(pane_id, event, window, cx);
            cx.notify();
        }
    }
}
