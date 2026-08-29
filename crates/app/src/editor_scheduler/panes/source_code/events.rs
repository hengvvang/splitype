//! Source Code pane event routing — the state transitions live in
//! `editor_source_code::input`; this file downcasts the pane state and
//! forwards it together with the coordination-layer host.

use gpui::*;

use crate::editor_scheduler::engine::controller::{Editor, PaneId};

impl Editor {
    /// Dispatches key-down events for a Source Code pane.
    pub(crate) fn handle_source_key_down(
        &mut self,
        pane_id: PaneId,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let host = self.pane_host.clone();
        let Some(state) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) else {
            return false;
        };
        editor_source_code::handle_key_down(state, pane_id, event, window, cx, &*host)
    }

    /// Handles mouse down events on the Source Code pane.
    pub(crate) fn handle_source_mouse_down(
        &mut self,
        pane_id: PaneId,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
            editor_source_code::handle_mouse_down(source, event, window, cx);
            cx.notify();
        }
    }

    /// Handles mouse move events during dragging on the Source Code pane.
    pub(crate) fn handle_source_mouse_move(
        &mut self,
        pane_id: PaneId,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
            editor_source_code::handle_mouse_move(source, event, window, cx);
            cx.notify();
        }
    }

    /// Handles mouse up events on the Source Code pane.
    pub(crate) fn handle_source_mouse_up(
        &mut self,
        pane_id: PaneId,
        _event: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
            editor_source_code::handle_mouse_up(source);
            cx.notify();
        }
    }

    /// Sync changes made in the Source pane buffer back to the shared document AST.
    pub(crate) fn sync_source_edit_to_document(&mut self, pane_id: PaneId, cx: &mut Context<Self>) {
        let Some(text) = self
            .pane_state_ref(pane_id)
            .and_then(|s| s.as_source_code())
            .map(|s| s.text.clone())
        else {
            return;
        };

        self.rebuild_document_from_markdown(&text, cx);
        self.mark_dirty(cx);

        // Model C: the source text just became the authoritative session
        // text (the block tree, if any, was invalidated), so the sync hash
        // is computed from it directly — no re-parse, no serialization.
        let synced_hash = Self::hash_str(&text);
        let revision = self.active_tab().map(|t| t.document_revision).unwrap_or(0);
        let tab_index = self.session.active_tab_index();

        if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
            source.synced_doc_hash = synced_hash;
            source.synced_revision = Some(revision);
            source.synced_tab_index = Some(tab_index);
        }
        cx.notify();
    }
}
