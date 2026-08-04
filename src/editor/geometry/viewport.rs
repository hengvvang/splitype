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

    /// Picks the contiguous run of rows to mount; the culled runs become two
    /// spacers and the focused row stays mounted. `strides[i]` is row `i`'s
    /// footprint (height plus trailing gap); being scroll-invariant, their running
    /// sum places each row against a band from the current scroll offset.
    /// Unmeasured rows use a lower-bound estimate, so the window never lands on a
    /// spacer. Pure, so it is unit-tested headlessly.
    pub(crate) fn rendered_window(
        strides: &[f32],
        scroll_y: f32,
        viewport_height: f32,
        overdraw: f32,
        focus_row: Option<usize>,
    ) -> RenderWindow {
        let n = strides.len();
        if n == 0 {
            return RenderWindow {
                run_start: 0,
                run_end: 0,
                top_h: 0.0,
                bottom_h: 0.0,
            };
        }

        let band_top = scroll_y - overdraw;
        let band_bottom = scroll_y + viewport_height + overdraw;

        let mut run_start = n;
        let mut run_end = 0usize;
        let mut top_of_start = 0.0f32;
        let mut bottom_of_end = 0.0f32;
        let mut cursor = 0.0f32;
        for (index, &stride) in strides.iter().enumerate() {
            let top = cursor;
            let bottom = cursor + stride.max(0.0);
            if bottom >= band_top && top <= band_bottom {
                if index < run_start {
                    run_start = index;
                    top_of_start = top;
                }
                run_end = index + 1;
                bottom_of_end = bottom;
            }
            cursor = bottom;
        }
        let total = cursor;

        // Nothing hit the band (float edge, or estimate short of scroll): mount
        // the last row so the viewport never lands on a spacer.
        if run_start >= run_end {
            run_start = n - 1;
            run_end = n;
            top_of_start = total - strides[n - 1].max(0.0);
            bottom_of_end = total;
        }

        // Keep the focused row mounted; GPUI blurs an unmounted caret. Reaching a
        // far focus row widens the run, but autoscroll makes that rare.
        if let Some(focus_row) = focus_row {
            let focus_row = focus_row.min(n - 1);
            if focus_row < run_start {
                run_start = focus_row;
                top_of_start = strides[..focus_row].iter().map(|s| s.max(0.0)).sum();
            }
            if focus_row + 1 > run_end {
                run_end = focus_row + 1;
                bottom_of_end = strides[..=focus_row].iter().map(|s| s.max(0.0)).sum();
            }
        }

        RenderWindow {
            run_start,
            run_end,
            top_h: top_of_start.max(0.0),
            bottom_h: (total - bottom_of_end).max(0.0),
        }
    }

    /// Linearly interpolates the editor content width ratio based on viewport
    /// width. The column stays full-width until `centered_shrink_start`, then
    /// shrinks to `centered_min_ratio` at `centered_shrink_end`.
    pub(crate) fn centered_column_ratio(
        viewport_width: f32,
        dimensions: &crate::theme::ThemeDimensions,
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
        dimensions: &crate::theme::ThemeDimensions,
    ) -> f32 {
        let available_content_width = (viewport_width - dimensions.editor_padding * 2.0).max(1.0);
        let centered_ratio = Self::centered_column_ratio(viewport_width, dimensions);
        (available_content_width * centered_ratio)
            .max(320.0)
            .min(available_content_width)
    }

    pub(crate) fn request_active_block_scroll_into_view(&mut self, cx: &mut Context<Self>) {
        self.focus.pending_scroll_recheck_after_layout = true;
        if !self.focus.pending_scroll_active_block_into_view {
            self.focus.pending_scroll_active_block_into_view = true;
            cx.notify();
        }
    }

    pub(crate) fn viewport_size_changed(previous: Size<Pixels>, current: Size<Pixels>) -> bool {
        const EPSILON: f32 = 0.5;

        (f32::from(previous.width) - f32::from(current.width)).abs() > EPSILON
            || (f32::from(previous.height) - f32::from(current.height)).abs() > EPSILON
    }
}
