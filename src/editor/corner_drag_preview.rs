//! Corner-drag indicator rendering (host-side logic).
//!
//! The splitter engine reports only raw gesture facts ([`CornerDragSession`]);
//! this module owns the *rendering*: what the indicator looks like. Hosts
//! decide when to call [`render_corner_drag_preview`] (e.g. only for
//! no-modifier drags — Shift drags never show an indicator).

use gpui::*;

use splitype_splitter::interaction::{OverlayStyle, overlay_container};
use splitype_splitter::root::SplitterRoot;
use splitype_splitter::sessions::CornerDragSession;
use splitype_splitter::tree::{LeafRect, Axis, Direction};

/// Render the corner-drag indicator, or `None` when there is nothing to
/// show yet (no gesture direction).
///
/// Hovering another leaf draws a join highlight on that leaf; hovering the
/// dragged leaf (or outside) draws the split line inside it. The overlay
/// positions with `relative()` against the container, so the indicator
/// tracks the tree geometry exactly.
pub fn render_corner_drag_preview<T: Copy + PartialEq>(
    root: &SplitterRoot<T>,
    drag: &CornerDragSession,
    container_size: Size<Pixels>,
    style: &OverlayStyle,
) -> Option<AnyElement> {
    let mut rects = Vec::new();
    root.tree.collect_leaf_rects(0.0, 0.0, 1.0, 1.0, &mut rects);
    let target_rect = rect_by_id(&rects, drag.target_id)?;
    match drag.hover_leaf {
        Some(hover) if hover != drag.target_id => {
            let target = rect_by_id(&rects, hover)?;
            let dir = drag.gesture_dir?;
            Some(join_preview_overlay(target, dir, style))
        }
        _ => {
            let (axis, ratio) = root.corner_split_facts(drag, container_size)?;
            Some(split_preview_overlay(target_rect, axis, ratio, style))
        }
    }
}

fn rect_by_id<'a>(rects: &'a [LeafRect], id: usize) -> Option<&'a LeafRect> {
    rects.iter().find(|rect| rect.id == id)
}

/// The split preview line for a horizontal split (left|right): a vertical
/// divider at `ratio` of the target rect's width. Deliberately thin (1px)
/// like IDE split guides. `rect` is normalized (0..1).
fn split_line_horizontal(rect: &LeafRect, ratio: f32, accent: Hsla) -> Div {
    div()
        .absolute()
        .left(relative(rect.x + rect.width * ratio))
        .top(relative(rect.y))
        .w(px(1.0))
        .h(relative(rect.height))
        .bg(accent)
}

/// The split preview line for a vertical split (top|bottom): a horizontal
/// divider at `ratio` of the target rect's height. Deliberately thin (1px)
/// like IDE split guides. `rect` is normalized (0..1).
fn split_line_vertical(rect: &LeafRect, ratio: f32, accent: Hsla) -> Div {
    div()
        .absolute()
        .top(relative(rect.y + rect.height * ratio))
        .left(relative(rect.x))
        .h(px(1.0))
        .w(relative(rect.width))
        .bg(accent)
}

/// A full-window overlay drawing the split line plus a faint highlight over
/// the leaf being split. `rect` is normalized (0..1).
fn split_preview_overlay(
    rect: &LeafRect,
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
                .left(relative(rect.x))
                .top(relative(rect.y))
                .w(relative(rect.width))
                .h(relative(rect.height))
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
/// `target` is normalized (0..1).
fn join_preview_overlay(
    target: &LeafRect,
    direction: Direction,
    style: &OverlayStyle,
) -> AnyElement {
    let arrow = join_arrow(direction);
    overlay_container()
        .child(
            div()
                .absolute()
                .left(relative(target.x))
                .top(relative(target.y))
                .w(relative(target.width))
                .h(relative(target.height))
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
                        .child(format!("{arrow} Join Panel")),
                ),
        )
        .into_any_element()
}
