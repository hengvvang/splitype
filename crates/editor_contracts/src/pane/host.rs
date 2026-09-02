use crate::edit::EditTransaction;
use crate::pane::PaneId;
use gpui::{App, ScrollHandle, Window};
use std::sync::Arc;

pub struct PaneRenderContext<'a> {
    pub pane_id: PaneId,
    pub is_focused: bool,
    pub scroll: &'a ScrollHandle,
    pub host: &'a Arc<dyn PaneHost>,
    pub is_outline_hovered: bool,
}

/// Host seam a pane uses to reach back into the coordinating editor.
///
/// Only operations that panes actually invoke are exposed; everything the
/// editor does on its own schedule (focus, refresh) stays editor-internal.
pub trait PaneHost: Send + Sync + 'static {
    /// Commits a pane-produced edit into the shared document buffer.
    /// The buffer records it as one undo transaction (merged into the
    /// previous one when [`EditTransaction::merge`] is set), bumps its
    /// revision, and notifies every observing editor, which re-syncs all
    /// of its panes with the new snapshot.
    fn commit_edit(&self, edit: EditTransaction, cx: &mut App);
    fn navigate_to_outline(&self, pane_id: PaneId, index: usize, cx: &mut App);
    fn set_outline_hovered(
        &self,
        pane_id: PaneId,
        hovered: bool,
        window: &mut Window,
        cx: &mut App,
    );
    fn handle_pane_key_down(
        &self,
        pane_id: PaneId,
        event: &gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut App,
    ) -> bool;
    fn handle_pane_mouse_down(
        &self,
        pane_id: PaneId,
        event: &gpui::MouseDownEvent,
        window: &mut Window,
        cx: &mut App,
    );
    fn handle_pane_mouse_move(
        &self,
        pane_id: PaneId,
        event: &gpui::MouseMoveEvent,
        window: &mut Window,
        cx: &mut App,
    );
    fn handle_pane_mouse_up(
        &self,
        pane_id: PaneId,
        event: &gpui::MouseUpEvent,
        window: &mut Window,
        cx: &mut App,
    );
}

pub struct PaneOutlineHost {
    pub pane_id: PaneId,
    pub host: Arc<dyn PaneHost>,
}

impl crate::outline::OutlineHost for PaneOutlineHost {
    fn navigate_to(&self, index: usize, cx: &mut App) {
        self.host.navigate_to_outline(self.pane_id, index, cx);
    }

    fn set_hovered(&self, hovered: bool, window: &mut Window, cx: &mut App) {
        self.host
            .set_outline_hovered(self.pane_id, hovered, window, cx);
    }
}
