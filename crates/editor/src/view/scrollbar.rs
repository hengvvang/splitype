//! Scrollbar geometry calculation and canvas mouse drag handling.

use std::time::{Duration, Instant};

use gpui::*;

use crate::editor::Editor;
use crate::session::ScrollbarDragSession;
use core_contracts::{AutoscrollStrategy, PaneId};

#[derive(Clone, Copy, Debug)]
pub struct ScrollbarGeometry {
    pub track_height: f32,
    pub thumb_height: f32,
    pub thumb_top: f32,
    pub max_scroll_y: f32,
}

#[allow(dead_code)]
impl Editor {
    pub(crate) fn scrollbar_geometry(
        viewport_height: f32,
        max_scroll_y: f32,
        current_scroll_y: f32,
    ) -> ScrollbarGeometry {
        let track_height = viewport_height.max(20.0);
        let content_height = viewport_height + max_scroll_y;
        let thumb_height = if max_scroll_y > 0.5 {
            (track_height * (viewport_height / content_height)).clamp(28.0, track_height)
        } else {
            track_height
        };
        let progress = if max_scroll_y > 0.0 {
            current_scroll_y.clamp(0.0, max_scroll_y) / max_scroll_y
        } else {
            0.0
        };
        let thumb_top = (track_height - thumb_height).max(0.0) * progress;
        ScrollbarGeometry {
            track_height,
            thumb_height,
            thumb_top,
            max_scroll_y,
        }
    }

    pub(crate) fn scroll_offset_for_thumb_top(
        thumb_top: f32,
        track_height: f32,
        thumb_height: f32,
        max_scroll_y: f32,
    ) -> f32 {
        if max_scroll_y <= 0.0 {
            return 0.0;
        }

        let travel = (track_height - thumb_height).max(0.0);
        if travel <= 0.0 {
            return 0.0;
        }

        let progress = (thumb_top / travel).clamp(0.0, 1.0);
        max_scroll_y * progress
    }

    pub(crate) fn request_autoscroll(
        &mut self,
        pane_id: PaneId,
        strategy: AutoscrollStrategy,
        cx: &mut Context<Self>,
    ) {
        let state = self.pane_state(pane_id);
        if state.scroll.pending_autoscroll != Some(strategy) {
            state.scroll.pending_autoscroll = Some(strategy);
            cx.notify();
        }
    }

    pub(crate) fn request_autoscroll_active_pane(
        &mut self,
        strategy: AutoscrollStrategy,
        cx: &mut Context<Self>,
    ) {
        let pane_id = self.active_pane_id();
        self.request_autoscroll(pane_id, strategy, cx);
    }

    pub(crate) fn viewport_size_changed(previous: Size<Pixels>, current: Size<Pixels>) -> bool {
        const EPSILON: f32 = 0.5;

        (f32::from(previous.width) - f32::from(current.width)).abs() > EPSILON
            || (f32::from(previous.height) - f32::from(current.height)).abs() > EPSILON
    }

    pub(crate) fn bump_scrollbar_visibility(
        &mut self,
        pane_id: PaneId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let duration = Duration::from_millis(900);
        let Some(state) = self.pane_state_mut(pane_id) else {
            return;
        };
        state.scroll.scrollbar_visible_until = Instant::now() + duration;

        let window_handle = window.window_handle();
        let weak_editor = cx.entity().downgrade();
        let Some(state) = self.pane_state_mut(pane_id) else {
            return;
        };
        state.scroll.scrollbar_fade_task = Some(cx.spawn(
            async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
                cx.background_executor()
                    .timer(duration + Duration::from_millis(50))
                    .await;
                let _ = window_handle.update(cx, |_view, _window, cx| {
                    let _ = weak_editor.update(cx, |this, cx| {
                        if let Some(state) = this.pane_state_mut(pane_id) {
                            state.scroll.scrollbar_fade_task = None;
                            cx.notify();
                        }
                    });
                });
            },
        ));

        cx.notify();
    }

    pub(crate) fn on_editor_hover(
        &mut self,
        pane_id: PaneId,
        hovered: &bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(state) = self.pane_state_mut(pane_id) {
            state.scroll.scrollbar_hovered = *hovered;
            if *hovered {
                self.bump_scrollbar_visibility(pane_id, window, cx);
            }
        }
    }

    pub(crate) fn on_editor_scroll_wheel(
        &mut self,
        pane_id: PaneId,
        _event: &ScrollWheelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(state) = self.pane_state_mut(pane_id) {
            state.scroll.pending_autoscroll = None;
        }
        self.bump_scrollbar_visibility(pane_id, window, cx);
    }

    pub(crate) fn start_scrollbar_drag(
        &mut self,
        pane_id: PaneId,
        pointer_offset_y: f32,
        track_height: f32,
        thumb_height: f32,
        max_scroll_y: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(state) = self.pane_state_mut(pane_id) {
            state.scroll.pending_autoscroll = None;
            state.scroll.scrollbar_drag = Some(ScrollbarDragSession {
                pointer_offset_y: pointer_offset_y.clamp(0.0, thumb_height.max(0.0)),
                track_height,
                thumb_height,
                max_scroll_y,
            });
            self.bump_scrollbar_visibility(pane_id, window, cx);
            cx.notify();
        }
    }

    pub(crate) fn update_scrollbar_drag(
        &mut self,
        pane_id: PaneId,
        pointer_y_in_track: f32,
        window: &mut Window,
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
            self.bump_scrollbar_visibility(pane_id, window, cx);
            cx.notify();
        }
    }

    pub(crate) fn end_scrollbar_drag(
        &mut self,
        pane_id: PaneId,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(state) = self.pane_state_mut(pane_id) {
            if state.scroll.scrollbar_drag.take().is_some() {
                cx.notify();
            }
        }
    }
}
