//! Corner-drag indicator rendering (host-side logic).
//!
//! The splitter engine reports only raw gesture facts ([`CornerDragSession`]);
//! this module owns the *rendering*: what the indicator looks like.

use gpui::*;

use splitype_splitter::interaction::{OverlayStyle, overlay_container};
use splitype_splitter::root::SplitterRoot;
use splitype_splitter::sessions::{
    AreaDockTarget, CornerDragModifier, CornerDragSession, calculate_join_slice_rect,
};
use splitype_splitter::tree::{LeafRect, SplitAxis};

/// Render the corner-drag indicator, or `None` when there is nothing to
/// show yet (no gesture direction).
pub fn render_corner_drag_preview<T: Copy + PartialEq>(
    root: &SplitterRoot<T>,
    drag: &CornerDragSession,
    container_size: Size<Pixels>,
    style: &OverlayStyle,
) -> Option<AnyElement> {
    let mut rects = Vec::new();
    root.tree.collect_leaf_rects(0.0, 0.0, 1.0, 1.0, &mut rects);
    let target_rect = rect_by_id(&rects, drag.target_id)?;

    // 0. Shift drag preview: Duplicate Area into New Window
    if drag.modifier == CornerDragModifier::Shift {
        return Some(new_window_preview_overlay(
            target_rect,
            drag.pointer_pos,
            container_size,
            style,
        ));
    }

    match drag.hover_leaf {
        Some(hover) if hover != drag.target_id => {
            let hover_target = rect_by_id(&rects, hover)?;

            // 1. Swap preview (Ctrl held or hovering Center)
            if drag.modifier == CornerDragModifier::Ctrl
                || drag.dock_target == AreaDockTarget::Center
            {
                return Some(swap_preview_overlay(
                    target_rect,
                    hover_target,
                    drag.pointer_pos,
                    container_size,
                    style,
                ));
            }

            // 2. Dock preview (Top, Bottom, Left, Right)
            if matches!(
                drag.dock_target,
                AreaDockTarget::Top
                    | AreaDockTarget::Bottom
                    | AreaDockTarget::Left
                    | AreaDockTarget::Right
            ) {
                return Some(dock_preview_overlay(
                    target_rect,
                    hover_target,
                    drag.dock_target,
                    drag.dock_ratio,
                    drag.pointer_pos,
                    container_size,
                    style,
                ));
            }

            // 3. Direct Join preview (when dock_target is None)
            Some(join_preview_overlay(
                target_rect,
                hover_target,
                drag.pointer_pos,
                container_size,
                style,
            ))
        }
        _ => {
            let (axis, ratio) = root.corner_split_facts(drag, container_size)?;
            Some(split_preview_overlay(
                target_rect,
                axis,
                ratio,
                drag.pointer_pos,
                container_size,
                style,
            ))
        }
    }
}

fn rect_by_id(rects: &[LeafRect], id: usize) -> Option<&LeafRect> {
    rects.iter().find(|rect| rect.id == id)
}

/// The split preview line for a horizontal split (left|right): a vertical
/// divider at `ratio` of the target rect's width with prominent 2.5px thickness.
/// `rect` is normalized (0..1).
fn split_line_horizontal(rect: &LeafRect, ratio: f32, accent: Hsla) -> Div {
    div()
        .absolute()
        .left(relative(rect.x + rect.width * ratio))
        .top(relative(rect.y))
        .w(px(2.5))
        .h(relative(rect.height))
        .bg(accent)
}

/// The split preview line for a vertical split (top|bottom): a horizontal
/// divider at `ratio` of the target rect's height with prominent 2.5px thickness.
/// `rect` is normalized (0..1).
fn split_line_vertical(rect: &LeafRect, ratio: f32, accent: Hsla) -> Div {
    div()
        .absolute()
        .top(relative(rect.y + rect.height * ratio))
        .left(relative(rect.x))
        .h(px(2.5))
        .w(relative(rect.width))
        .bg(accent)
}

/// A full-window overlay drawing the split line plus a pure background highlight
/// over the leaf being split. `rect` is normalized (0..1).
fn split_preview_overlay(
    rect: &LeafRect,
    direction: SplitAxis,
    ratio: f32,
    pointer_pos: Option<Point<Pixels>>,
    container_size: Size<Pixels>,
    style: &OverlayStyle,
) -> AnyElement {
    let line = match direction {
        SplitAxis::Horizontal => split_line_horizontal(rect, ratio, style.accent),
        SplitAxis::Vertical => split_line_vertical(rect, ratio, style.accent),
    };
    let ratio_percent = format!("{:.1}%", ratio * 100.0);

    overlay_container()
        // Highlighted panel being split with central split-area icon
        .child(
            div()
                .absolute()
                .left(relative(rect.x))
                .top(relative(rect.y))
                .w(relative(rect.width))
                .h(relative(rect.height))
                .rounded(px(style.tile_radius))
                .bg(style.accent.opacity(0.12))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    svg()
                        .path("icons/splitter/split-area.svg")
                        .size(px(48.0))
                        .text_color(style.accent),
                ),
        )
        .child(line)
        .child(cursor_action_panel(
            pointer_pos,
            container_size,
            Some("icons/splitter/split-area.svg"),
            "Split Area",
            Some(ratio_percent),
            Some("Ctrl: 1/12 Snap • Esc: Cancel"),
            style,
        ))
        .into_any_element()
}

/// Cursor-following operation tooltip panel matching Blender's area drag status indicator.
/// Positioned dynamically near the mouse pointer and clamped inside the window bounds.
fn cursor_action_panel(
    pointer_pos: Option<Point<Pixels>>,
    container_size: Size<Pixels>,
    icon_path: Option<&'static str>,
    title: &'static str,
    detail: Option<String>,
    shortcut_hint: Option<&'static str>,
    style: &OverlayStyle,
) -> AnyElement {
    let (left_px, top_px) = if let Some(pos) = pointer_pos {
        let x = f32::from(pos.x) + 16.0;
        let y = f32::from(pos.y) + 16.0;
        let max_x = (f32::from(container_size.width) - 210.0).max(8.0);
        let max_y = (f32::from(container_size.height) - 65.0).max(8.0);
        (x.clamp(8.0, max_x), y.clamp(8.0, max_y))
    } else {
        (16.0, 16.0)
    };

    div()
        .absolute()
        .left(px(left_px))
        .top(px(top_px))
        .px(px(10.0))
        .py(px(6.0))
        .rounded(px(splitype_infra::theme::dimensions::CONTROL_CORNER_RADIUS))
        .bg(style.surface)
        .border(px(1.0))
        .border_color(style.border)
        .shadow_md()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .children(icon_path.map(|p| {
                    svg()
                        .path(p)
                        .size(px(14.0))
                        .text_color(style.accent)
                }))
                .child(
                    div()
                        .text_size(px(12.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(style.text)
                        .child(title),
                )
                .children(detail.map(|d| {
                    div()
                        .text_size(px(11.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(style.accent)
                        .child(d)
                })),
        )
        .children(shortcut_hint.map(|hint| {
            div()
                .text_size(px(10.0))
                .text_color(style.text.opacity(0.65))
                .child(hint)
        }))
        .into_any_element()
}

/// A full-window overlay highlighting the dragged panel with a clean accent border.
fn new_window_preview_overlay(
    target: &LeafRect,
    pointer_pos: Option<Point<Pixels>>,
    container_size: Size<Pixels>,
    style: &OverlayStyle,
) -> AnyElement {
    overlay_container()
        .child(
            div()
                .absolute()
                .left(relative(target.x))
                .top(relative(target.y))
                .w(relative(target.width))
                .h(relative(target.height))
                .rounded(px(style.tile_radius))
                .bg(style.accent.opacity(0.18))
                .border(px(1.5))
                .border_color(style.accent),
        )
        .child(cursor_action_panel(
            pointer_pos,
            container_size,
            None,
            "Duplicate Area",
            None,
            Some("Release to open in new window"),
            style,
        ))
        .into_any_element()
}

/// Deduce the SVG icon path corresponding to the merge direction.
fn join_direction_icon_path(source: &LeafRect, target: &LeafRect) -> &'static str {
    const EPS: f32 = 0.01;
    let shares_vertical_border = (source.x + source.width - target.x).abs() <= EPS
        || (target.x + target.width - source.x).abs() <= EPS;
    if shares_vertical_border {
        if source.x < target.x {
            "icons/splitter/arrow-right.svg"
        } else {
            "icons/splitter/arrow-left.svg"
        }
    } else if source.y < target.y {
        "icons/splitter/arrow-down.svg"
    } else {
        "icons/splitter/arrow-up.svg"
    }
}

/// A full-window overlay highlighting the exact merged slice matching Blender's `screen_draw_join_highlight`.
/// When joining adjacent areas across a shared border with asymmetric extents (e.g. source is height 0.5
/// and target is height 1.0), the preview highlights only the shared slice (using the overlapping dimension)
/// so unrelated sibling areas are never incorrectly covered.
fn join_preview_overlay(
    source: &LeafRect,
    target: &LeafRect,
    pointer_pos: Option<Point<Pixels>>,
    container_size: Size<Pixels>,
    style: &OverlayStyle,
) -> AnyElement {
    let (merged_x, merged_y, merged_w, merged_h) = calculate_join_slice_rect(source, target);
    let icon_path = join_direction_icon_path(source, target);

    overlay_container()
        // Single merged rectangle covering the exact merged slice
        .child(
            div()
                .absolute()
                .left(relative(merged_x))
                .top(relative(merged_y))
                .w(relative(merged_w))
                .h(relative(merged_h))
                .rounded(px(style.tile_radius))
                .bg(style.accent.opacity(0.18))
                .border(px(1.5))
                .border_color(style.accent)
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .child(
                    svg()
                        .path(icon_path)
                        .size(px(48.0))
                        .text_color(style.accent),
                ),
        )
        .child(cursor_action_panel(
            pointer_pos,
            container_size,
            Some(icon_path),
            "Join Area",
            None,
            Some("Release to merge • Esc: Cancel"),
            style,
        ))
        .into_any_element()
}

/// A full-window overlay highlighting BOTH the source and target panels with themed tint and center swap icon.
fn swap_preview_overlay(
    source: &LeafRect,
    target: &LeafRect,
    pointer_pos: Option<Point<Pixels>>,
    container_size: Size<Pixels>,
    style: &OverlayStyle,
) -> AnyElement {
    overlay_container()
        // Source panel highlight
        .child(
            div()
                .absolute()
                .left(relative(source.x))
                .top(relative(source.y))
                .w(relative(source.width))
                .h(relative(source.height))
                .rounded(px(style.tile_radius))
                .bg(style.accent.opacity(0.18)),
        )
        // Target panel highlight with center swap icon
        .child(
            div()
                .absolute()
                .left(relative(target.x))
                .top(relative(target.y))
                .w(relative(target.width))
                .h(relative(target.height))
                .rounded(px(style.tile_radius))
                .bg(style.accent.opacity(0.18))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    svg()
                        .path("icons/splitter/swap.svg")
                        .size(px(48.0))
                        .text_color(style.accent),
                ),
        )
        .child(cursor_action_panel(
            pointer_pos,
            container_size,
            Some("icons/splitter/swap.svg"),
            "Swap Areas",
            None,
            Some("Release to swap contents"),
            style,
        ))
        .into_any_element()
}

/// A full-window overlay previewing moving and docking a source panel onto a target panel's edge.
fn dock_preview_overlay(
    source: &LeafRect,
    target: &LeafRect,
    dock_target: AreaDockTarget,
    ratio: f32,
    pointer_pos: Option<Point<Pixels>>,
    container_size: Size<Pixels>,
    style: &OverlayStyle,
) -> AnyElement {
    let (sub_x, sub_y, sub_w, sub_h, line_div, target_label, dock_icon) = match dock_target {
        AreaDockTarget::Left => (
            target.x,
            target.y,
            target.width * ratio,
            target.height,
            div()
                .absolute()
                .left(relative(target.x + target.width * ratio))
                .top(relative(target.y))
                .w(px(2.5))
                .h(relative(target.height))
                .bg(style.accent),
            "Dock Left",
            Some("icons/splitter/dock-left.svg"),
        ),
        AreaDockTarget::Right => (
            target.x + target.width * (1.0 - ratio),
            target.y,
            target.width * ratio,
            target.height,
            div()
                .absolute()
                .left(relative(target.x + target.width * (1.0 - ratio)))
                .top(relative(target.y))
                .w(px(2.5))
                .h(relative(target.height))
                .bg(style.accent),
            "Dock Right",
            Some("icons/splitter/dock-right.svg"),
        ),
        AreaDockTarget::Top => (
            target.x,
            target.y,
            target.width,
            target.height * ratio,
            div()
                .absolute()
                .top(relative(target.y + target.height * ratio))
                .left(relative(target.x))
                .h(px(2.5))
                .w(relative(target.width))
                .bg(style.accent),
            "Dock Top",
            Some("icons/splitter/dock-up.svg"),
        ),
        AreaDockTarget::Bottom => (
            target.x,
            target.y + target.height * (1.0 - ratio),
            target.width,
            target.height * ratio,
            div()
                .absolute()
                .top(relative(target.y + target.height * (1.0 - ratio)))
                .left(relative(target.x))
                .h(px(2.5))
                .w(relative(target.width))
                .bg(style.accent),
            "Dock Bottom",
            Some("icons/splitter/dock-down.svg"),
        ),
        _ => (
            target.x,
            target.y,
            target.width,
            target.height,
            div().absolute(),
            "Dock Area",
            None,
        ),
    };

    let ratio_percent = format!("{:.1}%", ratio * 100.0);

    overlay_container()
        // Source panel highlight (being moved)
        .child(
            div()
                .absolute()
                .left(relative(source.x))
                .top(relative(source.y))
                .w(relative(source.width))
                .h(relative(source.height))
                .rounded(px(style.tile_radius))
                .bg(style.accent.opacity(0.12)),
        )
        // Target sub-area highlight (destination slot) with central dock icon
        .child(
            div()
                .absolute()
                .left(relative(sub_x))
                .top(relative(sub_y))
                .w(relative(sub_w))
                .h(relative(sub_h))
                .rounded(px(style.tile_radius))
                .bg(style.accent.opacity(0.20))
                .flex()
                .items_center()
                .justify_center()
                .children(dock_icon.map(|icon| {
                    svg()
                        .path(icon)
                        .size(px(48.0))
                        .text_color(style.accent)
                })),
        )
        .child(line_div)
        .child(cursor_action_panel(
            pointer_pos,
            container_size,
            dock_icon,
            target_label,
            Some(ratio_percent),
            Some("Ctrl: 1/12 Snap • Esc: Cancel"),
            style,
        ))
        .into_any_element()
}
