//! Preview pane input routing — the state transitions themselves live in
//! `editor_preview::input`; this file downcasts the pane state and
//! notifies after a change.

use gpui::*;

use crate::engine::controller::{Editor, PaneId};

impl Editor {
    pub(crate) fn on_preview_mouse_down(
        &mut self,
        pane_id: PaneId,
        block_index: usize,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        if let Some(preview) = self.pane_state_mut(pane_id).and_then(|p| p.as_preview_mut()) {
            editor_preview::handle_mouse_down(preview, block_index, position);
            cx.notify();
        }
    }

    pub(crate) fn on_preview_mouse_move(
        &mut self,
        pane_id: PaneId,
        block_index: usize,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        if let Some(preview) = self.pane_state_mut(pane_id).and_then(|p| p.as_preview_mut()) {
            editor_preview::handle_mouse_move(preview, block_index, position);
            cx.notify();
        }
    }

    pub(crate) fn on_preview_mouse_up(&mut self, pane_id: PaneId, cx: &mut Context<Self>) {
        if let Some(preview) = self.pane_state_mut(pane_id).and_then(|p| p.as_preview_mut()) {
            editor_preview::handle_mouse_up(preview);
            cx.notify();
        }
    }
}
