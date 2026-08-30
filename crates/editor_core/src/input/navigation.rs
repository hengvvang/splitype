//! Viewport scrolling and page navigation for the editor frame.

use gpui::*;

use editor_wysiwyg::actions::{JumpToBottom, JumpToTop, PageDown, PageUp};
use crate::engine::controller::{Editor, PaneId};

impl Editor {
    pub(crate) fn on_page_up(&mut self, _: &PageUp, window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_active_tab() {
            return;
        }
        let page = self.active_pane_scroll().handle.bounds().size.height;
        self.scroll_viewport_by(self.active_pane_id(), page, window, cx);
    }

    pub(crate) fn on_page_down(
        &mut self,
        _: &PageDown,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_active_tab() {
            return;
        }
        let page = self.active_pane_scroll().handle.bounds().size.height;
        self.scroll_viewport_by(self.active_pane_id(), -page, window, cx);
    }

    pub(crate) fn on_jump_to_top(
        &mut self,
        _: &JumpToTop,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_active_tab() {
            return;
        }
        self.set_vertical_scroll_offset(self.active_pane_id(), px(0.0), window, cx);
    }

    pub(crate) fn on_jump_to_bottom(
        &mut self,
        _: &JumpToBottom,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_active_tab() {
            return;
        }
        let max_offset_y = self
            .active_pane_scroll()
            .handle
            .max_offset()
            .y
            .max(px(0.0));
        self.set_vertical_scroll_offset(self.active_pane_id(), -max_offset_y, window, cx);
    }

    /// Scrolls the viewport vertically by `delta`. A positive `delta` moves
    /// toward the start of the document; a negative one moves toward the end.
    pub(crate) fn scroll_viewport_by(
        &mut self,
        pane_id: PaneId,
        delta: Pixels,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self
            .pane_state_ref(pane_id)
            .map(|state| state.scroll.handle.offset().y + delta)
            .unwrap_or_default();
        self.set_vertical_scroll_offset(pane_id, target, window, cx);
    }

    /// Applies an absolute vertical scroll offset, clamped to the scrollable
    /// range. Offsets run from 0 at the top to `-max_offset` at the bottom.
    pub(crate) fn set_vertical_scroll_offset(
        &mut self,
        pane_id: PaneId,
        target_y: Pixels,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let max_offset_y = self
            .pane_state_ref(pane_id)
            .map(|state| state.scroll.handle.max_offset().y.max(px(0.0)))
            .unwrap_or_default();
        let mut offset = self
            .pane_state_ref(pane_id)
            .map(|state| state.scroll.handle.offset())
            .unwrap_or_default();
        offset.y = target_y.min(px(0.0)).max(-max_offset_y);
        let pane = self.pane_state(pane_id);
        pane.scroll.handle.set_offset(offset);
        cx.notify();
    }
}
