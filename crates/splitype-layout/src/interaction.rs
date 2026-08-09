//! Content-independent split interaction rendering.
//!
//! The draggable splitter bars between areas and the corner-drag preview
//! overlays are pure window-management UI: they depend on the layout tree
//! geometry and a small set of visual parameters, never on what the areas
//! contain. Rendering lives here so any host (window shell, editor panel
//! layout) can reuse it without reimplementing the gesture visuals.
//!
//! Visual parameters are injected via [`OverlayStyle`] so this crate stays
//! free of any concrete theme.

use gpui::*;

use crate::sessions::{CornerDragPreview, CornerDragSession};
use crate::tree::{AreaRect, Axis, Direction};

/// Visual parameters for split interaction overlays.
#[derive(Clone, Copy, Debug)]
pub struct OverlayStyle {
    /// Accent color for split-preview lines and join highlights.
    pub accent: Hsla,
    /// Corner radius of area tiles, used to round the highlight overlays.
    pub tile_radius: f32,
    /// Splitter bar base color.
    pub border: Hsla,
    /// Splitter bar hover color.
    pub selection: Hsla,
}

impl Default for OverlayStyle {
    fn default() -> Self {
        Self {
            accent: hsla(0.36, 0.73, 0.57, 0.8),
            tile_radius: 8.0,
            border: hsla(0.0, 0.0, 0.0, 0.2),
            selection: hsla(0.58, 0.6, 0.6, 0.8),
        }
    }
}

/// Window-level overlay container: covers the whole window, absolutely.
///
/// Rendered as the topmost layer so drag previews and menus can draw over
/// every area, including the titlebar strip.
pub fn overlay_container() -> Div {
    div().absolute().top_0().left_0().right_0().bottom_0()
}

/// Horizontal splitter bar (resizes rows).
pub fn splitter_bar_h(id: impl Into<ElementId>, style: &OverlayStyle) -> Stateful<Div> {
    div()
        .id(id)
        .w(px(2.0))
        .h_full()
        .flex_shrink_0()
        .cursor_col_resize()
        .bg(style.border)
        .hover(|this| this.bg(style.selection))
}

/// Vertical splitter bar (resizes columns).
pub fn splitter_bar_v(id: impl Into<ElementId>, style: &OverlayStyle) -> Stateful<Div> {
    div()
        .id(id)
        .h(px(2.0))
        .w_full()
        .flex_shrink_0()
        .cursor_row_resize()
        .bg(style.border)
        .hover(|this| this.bg(style.selection))
}

/// Find the rect with the given id in a rect list.
fn rect_by_id<'a>(rects: &'a [AreaRect], id: usize) -> Option<&'a AreaRect> {
    rects.iter().find(|rect| rect.id == id)
}

/// The split preview line for a horizontal split (left|right): a vertical
/// divider at `ratio` of the target rect's width.
fn split_line_horizontal(rect: &AreaRect, ratio: f32, accent: Hsla) -> Div {
    div()
        .absolute()
        .left(px(rect.x + rect.width * ratio))
        .top(px(rect.y))
        .w(px(3.0))
        .h(px(rect.height))
        .bg(accent)
}

/// The split preview line for a vertical split (top|bottom): a horizontal
/// divider at `ratio` of the target rect's height.
fn split_line_vertical(rect: &AreaRect, ratio: f32, accent: Hsla) -> Div {
    div()
        .absolute()
        .top(px(rect.y + rect.height * ratio))
        .left(px(rect.x))
        .h(px(3.0))
        .w(px(rect.width))
        .bg(accent)
}

/// A full-window overlay drawing the split line plus a faint highlight over
/// the leaf being split.
fn split_preview_overlay(
    rect: &AreaRect,
    direction: Axis,
    ratio: f32,
    style: &OverlayStyle,
) -> AnyElement {
    let line = match direction {
        Axis::Horizontal => split_line_horizontal(rect, ratio, style.accent),
        Axis::Vertical => split_line_vertical(rect, ratio, style.accent),
    };
    overlay_container()
        .child(
            div()
                .absolute()
                .left(px(rect.x))
                .top(px(rect.y))
                .w(px(rect.width))
                .h(px(rect.height))
                .rounded(px(style.tile_radius))
                .bg(style.accent.opacity(0.1)),
        )
        .child(line)
        .into_any_element()
}

/// Arrow symbol for a join direction.
fn join_arrow(direction: Direction) -> &'static str {
    match direction {
        Direction::Up => "▲",
        Direction::Down => "▼",
        Direction::Right => "▶",
        Direction::Left => "◀",
    }
}

/// A full-window overlay highlighting the join target with an arrow badge.
fn join_preview_overlay(
    target: &AreaRect,
    direction: Direction,
    style: &OverlayStyle,
) -> AnyElement {
    let arrow = join_arrow(direction);
    overlay_container()
        .child(
            div()
                .absolute()
                .left(px(target.x))
                .top(px(target.y))
                .w(px(target.width))
                .h(px(target.height))
                .rounded(px(style.tile_radius))
                .bg(style.accent.opacity(0.25))
                .border(px(2.0))
                .border_color(style.accent)
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .px(px(12.0))
                        .py(px(6.0))
                        .rounded_md()
                        .bg(hsla(0.0, 0.0, 0.0, 0.75))
                        .text_color(hsla(0.0, 0.0, 1.0, 0.95))
                        .text_size(px(15.0))
                        .font_weight(FontWeight::BOLD)
                        .child(format!("{arrow} Join Area")),
                ),
        )
        .into_any_element()
}

/// Render the window-level corner-drag preview overlay.
///
/// `rects` are the pixel-space rects of the outer window areas. Returns
/// `None` while the gesture is still in its initial `Dragging` state or
/// when the target area cannot be located.
pub fn render_corner_drag_preview(
    drag: &CornerDragSession,
    rects: &[AreaRect],
    style: &OverlayStyle,
) -> Option<AnyElement> {
    match drag.preview {
        CornerDragPreview::SplitPreview { direction, ratio } => rect_by_id(rects, drag.target_id)
            .map(|rect| split_preview_overlay(rect, direction, ratio, style)),
        CornerDragPreview::JoinPreview {
            target_id,
            direction,
        } => rect_by_id(rects, target_id)
            .map(|target| join_preview_overlay(target, direction, style)),
        CornerDragPreview::Dragging => None,
    }
}

/// Render the inner-panel corner-drag preview overlay.
///
/// `outer_rect` is the pixel-space rect of the containing window area;
/// `inner_rects` are the panel rects relative to that area's top-left.
pub fn render_inner_corner_drag_preview(
    drag: &CornerDragSession,
    outer_rect: &AreaRect,
    inner_rects: &[AreaRect],
    style: &OverlayStyle,
) -> Option<AnyElement> {
    match drag.preview {
        CornerDragPreview::SplitPreview { direction, ratio } => {
            // The line spans the whole outer area at the split ratio.
            let line = match direction {
                Axis::Horizontal => split_line_horizontal(outer_rect, ratio, style.accent),
                Axis::Vertical => split_line_vertical(outer_rect, ratio, style.accent),
            };
            Some(
                overlay_container()
                    .child(
                        div()
                            .absolute()
                            .left(px(outer_rect.x))
                            .top(px(outer_rect.y))
                            .w(px(outer_rect.width))
                            .h(px(outer_rect.height))
                            .rounded(px(style.tile_radius))
                            .bg(style.accent.opacity(0.1)),
                    )
                    .child(line)
                    .into_any_element(),
            )
        }
        CornerDragPreview::JoinPreview {
            target_id,
            direction,
        } => rect_by_id(inner_rects, target_id).map(|inner_rect| {
            join_preview_overlay(
                &AreaRect {
                    id: inner_rect.id,
                    x: outer_rect.x + inner_rect.x,
                    y: outer_rect.y + inner_rect.y,
                    width: inner_rect.width,
                    height: inner_rect.height,
                },
                direction,
                style,
            )
        }),
        CornerDragPreview::Dragging => None,
    }
}
