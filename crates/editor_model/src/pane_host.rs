//! The reverse seam between pane modes and the coordination layer.
//!
//! Pane modes render and process input inside their own crates; whenever
//! they need something only the coordinating `Editor` entity can do
//! (focus routing, autoscroll, dirty marking, source sync, undo) they
//! call through [`PaneHost`] instead of naming the entity. The app
//! composition root injects an `Arc<dyn PaneHost>` proxy that re-enters
//! the `Editor` through its weak handle, so the mode crates depend only
//! on this contract — mirroring the existing `EditorHost` shell seam.

use std::sync::Arc;

use gpui::{App, ScrollHandle, Window};

use crate::{AutoscrollStrategy, PaneId};

/// Render context handed to a pane mode's render entry point: the pane's
/// id, its focus state, its scroll handle (owned by the view shell) and the host proxy.
pub struct PaneRenderContext<'a> {
    pub pane_id: PaneId,
    pub is_focused: bool,
    pub scroll: &'a ScrollHandle,
    /// The host proxy; mode renderers clone the `Arc` into interaction
    /// callbacks, so the shared handle outlives the frame.
    pub host: &'a Arc<dyn PaneHost>,
}

/// Universal coordination-layer capabilities a pane plugin may request while
/// rendering or handling input. Implemented by a proxy that re-enters
/// the `Editor` entity.
pub trait PaneHost: Send + Sync + 'static {
    /// Route window keyboard focus to `pane_id`.
    fn focus_pane(&self, pane_id: PaneId, window: &mut Window, cx: &mut App);

    /// Apply pending focus routing for `pane_id` (render-phase bookkeeping).
    fn apply_pending_focus(&self, pane_id: PaneId, window: &mut Window, cx: &mut App);

    /// Apply pending autoscroll for `pane_id` (render-phase bookkeeping).
    fn apply_pending_autoscroll(&self, pane_id: PaneId, window: &mut Window, cx: &mut App);

    /// Request an autoscroll for `pane_id`.
    fn request_autoscroll(
        &self,
        pane_id: PaneId,
        strategy: AutoscrollStrategy,
        cx: &mut App,
    );

    /// Mark the active tab dirty (edit happened) and bump its revision.
    fn mark_dirty(&self, cx: &mut App);

    /// Notify the editor entity so it re-renders.
    fn notify(&self, cx: &mut App);

    /// Flush a Source pane's buffer edits back into the session text.
    fn sync_source_edit(&self, pane_id: PaneId, cx: &mut App);

    /// Undo the most recent edit.
    fn undo(&self, window: &mut Window, cx: &mut App);

    /// Redo the most recently undone edit.
    fn redo(&self, window: &mut Window, cx: &mut App);



    /// Navigate to outline heading index in the active pane.
    fn navigate_to_outline(&self, pane_id: PaneId, index: usize, cx: &mut App);

    /// Report outline popover hover state changes.
    fn set_outline_hovered(&self, pane_id: PaneId, hovered: bool, window: &mut Window, cx: &mut App);

    /// Key-down event routing for any pane mode.
    fn handle_pane_key_down(&self, pane_id: PaneId, event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut App) -> bool;

    /// Mouse-down event routing for any pane mode.
    fn handle_pane_mouse_down(&self, pane_id: PaneId, event: &gpui::MouseDownEvent, window: &mut Window, cx: &mut App);

    /// Mouse-move event routing for any pane mode.
    fn handle_pane_mouse_move(&self, pane_id: PaneId, event: &gpui::MouseMoveEvent, window: &mut Window, cx: &mut App);

    /// Mouse-up event routing for any pane mode.
    fn handle_pane_mouse_up(&self, pane_id: PaneId, event: &gpui::MouseUpEvent, window: &mut Window, cx: &mut App);
}

/// Outline host adapter forwarding to [`PaneHost`].
pub struct PaneOutlineHost {
    pub pane_id: PaneId,
    pub host: Arc<dyn PaneHost>,
}

impl editor_outline::OutlineHost for PaneOutlineHost {
    fn navigate_to(&self, index: usize, cx: &mut App) {
        self.host.navigate_to_outline(self.pane_id, index, cx);
    }

    fn set_hovered(&self, hovered: bool, window: &mut Window, cx: &mut App) {
        self.host.set_outline_hovered(self.pane_id, hovered, window, cx);
    }
}
