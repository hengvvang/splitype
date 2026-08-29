//! Viewport geometry — scrollbar math, rendered-row windowing, and centered
//! column sizing.
//!
//! Pure functions over layout inputs; nothing here mutates editor state.

use crate::editor::engine::controller::*;

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

    /// Request an autoscroll targeting the active block / caret with the given strategy.
    pub(crate) fn request_autoscroll(
        &mut self,
        pane_id: PaneId,
        strategy: crate::editor::engine::controller::AutoscrollStrategy,
        cx: &mut Context<Self>,
    ) {
        let state = self.pane_state(pane_id);
        if state.scroll.pending_autoscroll != Some(strategy) {
            state.scroll.pending_autoscroll = Some(strategy);
            cx.notify();
        }
    }

    /// Request an autoscroll on the currently active pane.
    pub(crate) fn request_autoscroll_active_pane(
        &mut self,
        strategy: crate::editor::engine::controller::AutoscrollStrategy,
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
}
