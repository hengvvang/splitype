//! Editor canvas mouse handling — hover, wheel, and scrollbar drag.
//!
//! These are thin wrappers that toggle UI state (scrollbar visibility,
//! menu dismissal, table-axis preview clearing) and drive the scrollbar
//! drag session. Page-key and table-cell navigation lives in `navigation`.

use std::time::Duration;

use gpui::*;

use crate::editor_scheduler::engine::controller::*;

impl Editor {
    pub(crate) fn bump_scrollbar_visibility(&mut self, pane_id: PaneId, cx: &mut Context<Self>) {
        // One Editor entity serves one area; the scrollbar belongs to the
        // pane's own document view. The fade task captures the pane id so it
        // clears the right fade task later.
        let duration = Duration::from_millis(900);
        let Some(state) = self.pane_state_mut(pane_id) else {
            return;
        };
        state.scroll.scrollbar_visible_until = Instant::now() + duration;

        let weak_editor = cx.entity().downgrade();
        let Some(state) = self.pane_state_mut(pane_id) else {
            return;
        };
        state.scroll.scrollbar_fade_task = Some(cx.spawn(
            async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
                cx.background_executor()
                    .timer(duration + Duration::from_millis(50))
                    .await;
                let _ = weak_editor.update(cx, |this, cx| {
                    if let Some(state) = this.pane_state_mut(pane_id) {
                        state.scroll.scrollbar_fade_task = None;
                        cx.notify();
                    }
                });
            },
        ));

        cx.notify();
    }

    pub(crate) fn on_editor_hover(
        &mut self,
        pane_id: PaneId,
        hovered: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(state) = self.pane_state_mut(pane_id) {
            state.scroll.scrollbar_hovered = *hovered;
            if *hovered {
                self.bump_scrollbar_visibility(pane_id, cx);
            } else {
                // Pointer left the editor: dismiss the footnote tooltip so it does
                // not linger when there is no further mouse-move to clear it.
                self.footnote_tooltip = None;
                cx.notify();
            }
        }
    }

    pub(crate) fn on_editor_mouse_down(
        &mut self,
        _event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // The in-window menu bar lives on the Shell; a mouse-down in the
        // editor body reaches the Shell's body listener, which closes it.
        self.clear_table_axis_preview(cx);
        self.clear_table_axis_selection(cx);
    }

    pub(crate) fn on_editor_scroll_wheel(
        &mut self,
        pane_id: PaneId,
        _event: &ScrollWheelEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // User scroll input has absolute priority: cancel any pending autoscroll.
        if let Some(state) = self.pane_state_mut(pane_id) {
            state.scroll.pending_autoscroll = None;
        }
        self.bump_scrollbar_visibility(pane_id, cx);
    }

    pub(crate) fn start_scrollbar_drag(
        &mut self,
        pane_id: PaneId,
        pointer_offset_y: f32,
        track_height: f32,
        thumb_height: f32,
        max_scroll_y: f32,
        cx: &mut Context<Self>,
    ) {
        if let Some(state) = self.pane_state_mut(pane_id) {
            state.scroll.pending_autoscroll = None;
            state.scroll.scrollbar_drag = Some(crate::editor_scheduler::engine::controller::ScrollbarDragSession {
                pointer_offset_y: pointer_offset_y.clamp(0.0, thumb_height.max(0.0)),
                track_height,
                thumb_height,
                max_scroll_y,
            });
            self.bump_scrollbar_visibility(pane_id, cx);
            cx.notify();
        }
    }

    pub(crate) fn update_scrollbar_drag(
        &mut self,
        pane_id: PaneId,
        pointer_y_in_track: f32,
        cx: &mut Context<Self>,
    ) {
        let Some(drag) = self
            .pane_state_ref(pane_id)
            .and_then(|state| state.scroll.scrollbar_drag)
        else {
            return;
        };

        let travel = (drag.track_height - drag.thumb_height).max(0.0);
        let thumb_top = (pointer_y_in_track - drag.pointer_offset_y).clamp(0.0, travel);
        let scroll_y = Self::scroll_offset_for_thumb_top(
            thumb_top,
            drag.track_height,
            drag.thumb_height,
            drag.max_scroll_y,
        );

        let mut offset = self
            .pane_state_ref(pane_id)
            .map(|state| state.scroll.handle.offset())
            .unwrap_or_default();
        offset.y = -px(scroll_y);
        if let Some(state) = self.pane_state_mut(pane_id) {
            state.scroll.handle.set_offset(offset);
            self.bump_scrollbar_visibility(pane_id, cx);
            cx.notify();
        }
    }

    pub(crate) fn end_scrollbar_drag(&mut self, pane_id: PaneId, cx: &mut Context<Self>) {
        if let Some(state) = self.pane_state_mut(pane_id) {
            if state.scroll.scrollbar_drag.take().is_some() {
                self.bump_scrollbar_visibility(pane_id, cx);
                cx.notify();
            }
        }
    }
}
