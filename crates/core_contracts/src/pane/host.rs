use std::sync::Arc;
use gpui::{App, ScrollHandle, Window};
use crate::pane::PaneId;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AutoscrollStrategy {
    Fit { margin: gpui::Pixels },
    Center,
    Top { margin: gpui::Pixels },
    Bottom { margin: gpui::Pixels },
}

pub struct PaneRenderContext<'a> {
    pub pane_id: PaneId,
    pub is_focused: bool,
    pub scroll: &'a ScrollHandle,
    pub host: &'a Arc<dyn PaneHost>,
}

pub trait PaneHost: Send + Sync + 'static {
    fn focus_pane(&self, pane_id: PaneId, window: &mut Window, cx: &mut App);
    fn apply_pending_focus(&self, pane_id: PaneId, window: &mut Window, cx: &mut App);
    fn apply_pending_autoscroll(&self, pane_id: PaneId, window: &mut Window, cx: &mut App);
    fn request_autoscroll(&self, pane_id: PaneId, strategy: AutoscrollStrategy, cx: &mut App);
    fn mark_dirty(&self, cx: &mut App);
    fn notify(&self, cx: &mut App);
    fn sync_source_text(&self, pane_id: PaneId, text: String, cx: &mut App);
    fn undo(&self, window: &mut Window, cx: &mut App);
    fn redo(&self, window: &mut Window, cx: &mut App);
    fn navigate_to_outline(&self, pane_id: PaneId, index: usize, cx: &mut App);
    fn set_outline_hovered(&self, pane_id: PaneId, hovered: bool, window: &mut Window, cx: &mut App);
    fn handle_pane_key_down(&self, pane_id: PaneId, event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut App) -> bool;
    fn handle_pane_mouse_down(&self, pane_id: PaneId, event: &gpui::MouseDownEvent, window: &mut Window, cx: &mut App);
    fn handle_pane_mouse_move(&self, pane_id: PaneId, event: &gpui::MouseMoveEvent, window: &mut Window, cx: &mut App);
    fn handle_pane_mouse_up(&self, pane_id: PaneId, event: &gpui::MouseUpEvent, window: &mut Window, cx: &mut App);
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
        self.host.set_outline_hovered(self.pane_id, hovered, window, cx);
    }
}
