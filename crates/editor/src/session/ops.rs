//! Editor pane operations of an Editor panel.

use gpui::Context;
use splitter::NodeId;
use splitter::sessions::AreaDockTarget;
use splitter::tree::SplitAxis;

use crate::editor::Editor;
use crate::session::{EditorSession, PaneKind, PaneState};
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
        let pane_id = pane_id.into();
        self.session.root.close_leaf(pane_id.0);
        self.forget_pane_state(pane_id.0);
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
        // Splitting is disabled while a pane is maximized, mirroring the
        // window shell's panel-level behavior.
        if self.session.root.tree.find_maximized_leaf().is_some() {
            return;
        }
        self.session.root.split_leaf(pane_id.into().0, axis, ratio);
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

    // ------------------------------------------------------------------
    // Pane-state reconciliation after layout gestures
    // ------------------------------------------------------------------

    /// The active tab's pane-state map. Panes exist only while a tab is
    /// open, so reconciliation below no-ops without tabs.
    fn panes_mut(&mut self) -> Option<&mut std::collections::HashMap<PaneId, PaneState>> {
        self.session.active_tab_mut().map(|tab| &mut tab.panes)
    }

    /// Drop a pane's state after its leaf was removed from the tree.
    pub(crate) fn forget_pane_state(&mut self, pane_id: usize) {
        if let Some(panes) = self.panes_mut() {
            panes.remove(&PaneId(pane_id));
        }
    }

    /// Swap two leaves' pane states after a kind swap (scroll and focus
    /// belong to the leaf position, not the kind).
    pub(crate) fn swap_pane_states(&mut self, a: usize, b: usize) {
        let Some(panes) = self.panes_mut() else {
            return;
        };
        let state_a = panes.remove(&PaneId(a));
        let state_b = panes.remove(&PaneId(b));
        if let Some(state) = state_a {
            panes.insert(PaneId(b), state);
        }
        if let Some(state) = state_b {
            panes.insert(PaneId(a), state);
        }
    }

    /// Re-seat pane states after a move-and-dock, mirroring the window
    /// shell's panel-view handling.
    pub(crate) fn move_and_dock_pane_states(
        &mut self,
        source_id: usize,
        target_id: usize,
        new_leaf_id: usize,
        dock_target: AreaDockTarget,
    ) {
        let Some(panes) = self.panes_mut() else {
            return;
        };
        let source = panes.remove(&PaneId(source_id));
        let target = panes.remove(&PaneId(target_id));
        let source_first = matches!(dock_target, AreaDockTarget::Left | AreaDockTarget::Top);
        if source_first {
            if let Some(state) = source {
                panes.insert(PaneId(target_id), state);
            }
            if let Some(state) = target {
                panes.insert(PaneId(new_leaf_id), state);
            }
        } else {
            if let Some(state) = target {
                panes.insert(PaneId(target_id), state);
            }
            if let Some(state) = source {
                panes.insert(PaneId(new_leaf_id), state);
            }
        }
    }

    /// Switches `pane_id` to `kind`, resetting its viewport and syncing the
    /// freshly created pane with the active document.
    pub fn select_pane_kind(&mut self, pane_id: PaneId, kind: PaneKind, cx: &mut Context<Self>) {
        self.change_pane_kind(pane_id, kind);
        if !self.has_tabs() {
            cx.notify();
            return;
        }
        {
            let state = self.pane_state(pane_id);
            state.scroll.last_viewport_size = None;
        }
        if let Some(tab) = self.session.active_tab_mut() {
            tab.pending.window_title_refresh = true;
            tab.pending.close_dialog_restore_focus = None;
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
        if !self.has_tabs() {
            return false;
        }
        let prev_text = self
            .pane_state_ref(pane_id)
            .and_then(|p| p.pane.document_text(cx));
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
                .and_then(|p| p.pane.document_text(cx));
            if let Some(next_text) = next_text {
                if prev_text.as_ref() != Some(&next_text) {
                    self.commit_document_text(next_text, cx);
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

    /// Scrolls `pane_id`'s viewport vertically by `delta` pixels.
    pub(crate) fn scroll_viewport_by(
        &mut self,
        pane_id: PaneId,
        delta: gpui::Pixels,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let target = self
            .pane_state_ref(pane_id)
            .map(|state| state.scroll.handle.offset().y + delta)
            .unwrap_or_default();
        self.set_vertical_scroll_offset(pane_id, target, _window, cx);
    }

    /// Clamps and applies a vertical scroll offset to `pane_id`'s viewport.
    pub(crate) fn set_vertical_scroll_offset(
        &mut self,
        pane_id: PaneId,
        target_y: gpui::Pixels,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let max_offset_y = self
            .pane_state_ref(pane_id)
            .map(|state| state.scroll.handle.max_offset().y.max(gpui::px(0.0)))
            .unwrap_or_default();
        let mut offset = self
            .pane_state_ref(pane_id)
            .map(|state| state.scroll.handle.offset())
            .unwrap_or_default();
        offset.y = target_y.min(gpui::px(0.0)).max(-max_offset_y);
        let pane = self.pane_state(pane_id);
        pane.scroll.handle.set_offset(offset);
        cx.notify();
    }
}
