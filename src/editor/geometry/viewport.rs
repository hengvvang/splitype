//! Viewport geometry — scrollbar math, rendered-row windowing, and centered
//! column sizing.
//!
//! Pure functions over layout inputs; nothing here mutates editor state.

use crate::editor::controller::*;

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

    /// Linearly interpolates the editor content width ratio based on viewport
    /// width. The column stays full-width until `centered_shrink_start`, then
    /// shrinks to `centered_min_ratio` at `centered_shrink_end`.
    pub(crate) fn centered_column_ratio(
        viewport_width: f32,
        dimensions: &crate::infra::theme::ThemeDimensions,
    ) -> f32 {
        if viewport_width <= dimensions.centered_shrink_start {
            return 1.0;
        }

        let t = ((viewport_width - dimensions.centered_shrink_start)
            / (dimensions.centered_shrink_end - dimensions.centered_shrink_start))
            .clamp(0.0, 1.0);
        1.0 - t * (1.0 - dimensions.centered_min_ratio)
    }

    pub(crate) fn centered_column_width(
        viewport_width: f32,
        dimensions: &crate::infra::theme::ThemeDimensions,
    ) -> f32 {
        let available_content_width = (viewport_width - dimensions.editor_padding * 2.0).max(1.0);
        let centered_ratio = Self::centered_column_ratio(viewport_width, dimensions);
        (available_content_width * centered_ratio)
            .max(320.0)
            .min(available_content_width)
    }

    pub(crate) fn request_active_block_scroll_into_view(
        &mut self,
        pane_id: PaneId,
        cx: &mut Context<Self>,
    ) {
        let state = self.pane_state(pane_id);
        state.focus.pending_scroll_recheck_after_layout = true;
        if !state.focus.pending_scroll_active_block_into_view {
            state.focus.pending_scroll_active_block_into_view = true;
            cx.notify();
        }
    }

    pub(crate) fn viewport_size_changed(previous: Size<Pixels>, current: Size<Pixels>) -> bool {
        const EPSILON: f32 = 0.5;

        (f32::from(previous.width) - f32::from(current.width)).abs() > EPSILON
            || (f32::from(previous.height) - f32::from(current.height)).abs() > EPSILON
    }
}
