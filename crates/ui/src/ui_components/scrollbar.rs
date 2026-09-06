//! Reusable interactive scrollbar component for vertical and horizontal scrolling.
//!
//! Provides proportional thumb sizing, click-to-jump, and smooth drag-to-scroll
//! capabilities built on GPUI's [`ScrollHandle`].

use gpui::*;
use theme::{ThemeColors, ThemeDimensions};

/// Orientation of a scrollbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollbarAxis {
    Vertical,
    Horizontal,
}

/// Calculated geometry for a scrollbar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollbarGeometry {
    /// Position of the thumb along the track from the start.
    pub thumb_offset: f32,
    /// Length of the thumb (height for vertical, width for horizontal).
    pub thumb_length: f32,
    /// Total available track length.
    pub track_length: f32,
    /// Whether the content overflows and is scrollable.
    pub is_scrollable: bool,
}

/// Computes the scrollbar geometry from viewport and scroll metrics.
///
/// - `viewport_len`: visible length along the scroll axis.
/// - `max_offset`: maximum scrollable offset (positive value, `content_len - viewport_len`).
/// - `current_offset`: current scroll offset (usually negative or 0 in GPUI).
/// - `track_len`: physical pixel length of the scrollbar track.
/// - `min_thumb_len`: minimum thumb length to keep it grabbable on large documents.
pub fn compute_scrollbar_geometry(
    viewport_len: f32,
    max_offset: f32,
    current_offset: f32,
    track_len: f32,
    min_thumb_len: f32,
) -> Option<ScrollbarGeometry> {
    if track_len <= 0.0 || viewport_len <= 0.0 || max_offset <= 0.0 {
        return None;
    }

    let total_content_len = viewport_len + max_offset;
    let thumb_ratio = (viewport_len / total_content_len).clamp(0.01, 1.0);
    let raw_thumb_len = track_len * thumb_ratio;
    let thumb_length = raw_thumb_len.max(min_thumb_len).min(track_len);

    let usable_track = (track_len - thumb_length).max(0.0);
    let scrolled_amount = (-current_offset).clamp(0.0, max_offset);
    let scroll_ratio = if max_offset > 0.0 {
        (scrolled_amount / max_offset).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let thumb_offset = scroll_ratio * usable_track;

    Some(ScrollbarGeometry {
        thumb_offset,
        thumb_length,
        track_length: track_len,
        is_scrollable: true,
    })
}

/// Converts a click or drag coordinate along the track into the target scroll offset.
pub fn scroll_offset_from_track_pos(
    pos_along_track: f32,
    thumb_length: f32,
    track_len: f32,
    max_offset: f32,
) -> f32 {
    let usable_track = (track_len - thumb_length).max(0.0);
    if usable_track <= 0.0 || max_offset <= 0.0 {
        return 0.0;
    }

    // Center thumb on pos
    let target_thumb_offset = (pos_along_track - thumb_length / 2.0).clamp(0.0, usable_track);
    let scroll_ratio = (target_thumb_offset / usable_track).clamp(0.0, 1.0);
    -(scroll_ratio * max_offset)
}

/// Renders a vertical scrollbar element for the given `ScrollHandle`.
pub fn render_vertical_scrollbar(
    id_suffix: impl Into<ElementId>,
    scroll_handle: &ScrollHandle,
    c: &ThemeColors,
    d: &ThemeDimensions,
) -> AnyElement {
    let bounds = scroll_handle.bounds();
    let viewport_h = f32::from(bounds.size.height);
    let max_offset_y = f32::from(scroll_handle.max_offset().y);
    let current_offset_y = f32::from(scroll_handle.offset().y);

    let track_w = d.scrollbar_width.max(6.0);
    let min_thumb_h = 24.0f32;

    let Some(geom) = compute_scrollbar_geometry(
        viewport_h,
        max_offset_y,
        current_offset_y,
        viewport_h,
        min_thumb_h,
    ) else {
        return div().w(px(track_w)).h_full().into_any_element();
    };

    let handle_down = scroll_handle.clone();
    let handle_move = scroll_handle.clone();
    let id_val = id_suffix.into();

    let thumb = div()
        .id((id_val.clone(), "scrollbar-v-thumb"))
        .absolute()
        .top(px(geom.thumb_offset))
        .left(px(1.0))
        .right(px(1.0))
        .h(px(geom.thumb_length))
        .rounded_full()
        .bg(c.scrollbar_thumb)
        .hover(|style| style.bg(c.scrollbar_thumb))
        .cursor_pointer();

    div()
        .id((id_val, "scrollbar-v-track"))
        .w(px(track_w + 2.0))
        .h_full()
        .flex_shrink_0()
        .relative()
        .overflow_hidden()
        .cursor_pointer()
        .hover(|style| style.bg(c.dialog_surface))
        .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
            let track_pos = f32::from(event.position.y - bounds.origin.y);
            let target_offset = scroll_offset_from_track_pos(
                track_pos,
                geom.thumb_length,
                geom.track_length,
                max_offset_y,
            );
            let current = handle_down.offset();
            handle_down.set_offset(point(current.x, px(target_offset)));
            cx.stop_propagation();
        })
        .on_mouse_move(move |event, _window, cx| {
            if event.pressed_button == Some(MouseButton::Left) {
                let track_pos = f32::from(event.position.y - bounds.origin.y);
                let target_offset = scroll_offset_from_track_pos(
                    track_pos,
                    geom.thumb_length,
                    geom.track_length,
                    max_offset_y,
                );
                let current = handle_move.offset();
                handle_move.set_offset(point(current.x, px(target_offset)));
                cx.stop_propagation();
            }
        })
        .child(thumb)
        .into_any_element()
}

/// Renders a horizontal scrollbar element for the given `ScrollHandle`.
pub fn render_horizontal_scrollbar(
    id_suffix: impl Into<ElementId>,
    scroll_handle: &ScrollHandle,
    c: &ThemeColors,
    d: &ThemeDimensions,
) -> AnyElement {
    let bounds = scroll_handle.bounds();
    let viewport_w = f32::from(bounds.size.width);
    let max_offset_x = f32::from(scroll_handle.max_offset().x);
    let current_offset_x = f32::from(scroll_handle.offset().x);

    let track_h = d.scrollbar_width.max(6.0);
    let min_thumb_w = 24.0f32;

    let Some(geom) = compute_scrollbar_geometry(
        viewport_w,
        max_offset_x,
        current_offset_x,
        viewport_w,
        min_thumb_w,
    ) else {
        return div().h(px(0.0)).w_full().into_any_element();
    };

    let handle_down = scroll_handle.clone();
    let handle_move = scroll_handle.clone();
    let id_val = id_suffix.into();

    let thumb = div()
        .id((id_val.clone(), "scrollbar-h-thumb"))
        .absolute()
        .left(px(geom.thumb_offset))
        .top(px(1.0))
        .bottom(px(1.0))
        .w(px(geom.thumb_length))
        .rounded_full()
        .bg(c.scrollbar_thumb)
        .hover(|style| style.bg(c.scrollbar_thumb))
        .cursor_pointer();

    div()
        .id((id_val, "scrollbar-h-track"))
        .h(px(track_h + 2.0))
        .w_full()
        .flex_shrink_0()
        .relative()
        .overflow_hidden()
        .cursor_pointer()
        .hover(|style| style.bg(c.dialog_surface))
        .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
            let track_pos = f32::from(event.position.x - bounds.origin.x);
            let target_offset = scroll_offset_from_track_pos(
                track_pos,
                geom.thumb_length,
                geom.track_length,
                max_offset_x,
            );
            let current = handle_down.offset();
            handle_down.set_offset(point(px(target_offset), current.y));
            cx.stop_propagation();
        })
        .on_mouse_move(move |event, _window, cx| {
            if event.pressed_button == Some(MouseButton::Left) {
                let track_pos = f32::from(event.position.x - bounds.origin.x);
                let target_offset = scroll_offset_from_track_pos(
                    track_pos,
                    geom.thumb_length,
                    geom.track_length,
                    max_offset_x,
                );
                let current = handle_move.offset();
                handle_move.set_offset(point(px(target_offset), current.y));
                cx.stop_propagation();
            }
        })
        .child(thumb)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::prelude::v1::test;

    #[test]
    fn test_compute_scrollbar_geometry_non_scrollable() {
        assert_eq!(
            compute_scrollbar_geometry(500.0, 0.0, 0.0, 500.0, 24.0),
            None
        );
        assert_eq!(
            compute_scrollbar_geometry(500.0, -10.0, 0.0, 500.0, 24.0),
            None
        );
    }

    #[test]
    fn test_compute_scrollbar_geometry_normal() {
        // viewport 500, max_offset 1500 => total 2000 => ratio 0.25 => thumb 125 on track 500
        let geom = compute_scrollbar_geometry(500.0, 1500.0, -300.0, 500.0, 24.0).unwrap();
        assert_eq!(geom.thumb_length, 125.0);
        assert_eq!(geom.track_length, 500.0);
        // scrolled 300 / 1500 = 0.2, usable track = 500 - 125 = 375, 0.2 * 375 = 75
        assert_eq!(geom.thumb_offset, 75.0);
        assert!(geom.is_scrollable);
    }

    #[test]
    fn test_compute_scrollbar_geometry_min_thumb_clamping() {
        // Very large content: viewport 100, max_offset 99900 => total 100000 => raw ratio 0.001
        let geom = compute_scrollbar_geometry(100.0, 99900.0, 0.0, 100.0, 20.0).unwrap();
        assert_eq!(geom.thumb_length, 20.0);
        assert_eq!(geom.thumb_offset, 0.0);
    }

    #[test]
    fn test_scroll_offset_from_track_pos() {
        // track 500, thumb 100 => usable 400. Click at 250 (center of track) => offset should be -50% of max_offset
        let offset = scroll_offset_from_track_pos(250.0, 100.0, 500.0, 1000.0);
        assert_eq!(offset, -500.0);
    }
}
