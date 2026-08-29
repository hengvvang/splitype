//! The reverse seam between pane modes and the coordination layer.
//!
//! Pane modes render and process input inside their own crates; whenever
//! they need something only the coordinating `Editor` entity can do
//! (focus routing, autoscroll, dirty marking, cross-mode sync, undo) they
//! call through [`PaneHost`] instead of naming the entity. The app
//! composition root injects an `Arc<dyn PaneHost>` proxy that re-enters
//! the `Editor` through its weak handle, so the mode crates depend only
//! on this contract — mirroring the existing `EditorHost` shell seam.

use std::sync::Arc;

use gpui::{App, Bounds, Point, Pixels, ScrollHandle, Window};

use crate::{AutoscrollStrategy, PaneId};

/// Render context handed to a pane mode's render entry point: the pane's
/// id, its scroll handle (owned by the view shell) and the host proxy.
pub struct PaneRenderContext<'a> {
    pub pane_id: PaneId,
    pub scroll: &'a ScrollHandle,
    /// The host proxy; mode renderers clone the `Arc` into interaction
    /// callbacks, so the shared handle outlives the frame.
    pub host: &'a Arc<dyn PaneHost>,
}

/// Coordination-layer capabilities a pane mode may request while
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

    /// Flush a Source pane's buffer edits back into the session text
    /// (the model-C text swap + cache invalidation).
    fn sync_source_edit(&self, pane_id: PaneId, cx: &mut App);

    /// Record the Source pane's rendered bounds (IME candidate popup
    /// positioning), written during the element's prepaint.
    fn set_source_last_bounds(&self, pane_id: PaneId, bounds: Bounds<Pixels>, cx: &mut App);

    /// Undo the most recent edit (block-tree based).
    fn undo(&self, window: &mut Window, cx: &mut App);

    /// Redo the most recently undone edit.
    fn redo(&self, window: &mut Window, cx: &mut App);

    /// Preview pane: mouse-down at `block_index` / `position`.
    fn preview_mouse_down(
        &self,
        pane_id: PaneId,
        block_index: usize,
        position: Point<Pixels>,
        cx: &mut App,
    );

    /// Preview pane: mouse-move during a drag selection.
    fn preview_mouse_move(
        &self,
        pane_id: PaneId,
        block_index: usize,
        position: Point<Pixels>,
        cx: &mut App,
    );

    /// Preview pane: mouse-up ends the drag selection.
    fn preview_mouse_up(&self, pane_id: PaneId, cx: &mut App);
}
