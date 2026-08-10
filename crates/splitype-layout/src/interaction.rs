//! Content-independent split interaction rendering.
//!
//! The draggable splitter bars between areas, the corner-drag handles and
//! their preview overlays, and the splitter-bar context menu are pure
//! window-management UI: they depend on the layout tree geometry and a
//! small set of visual parameters, never on what the areas contain.
//! Rendering and gesture state machines live here so any host (window
//! shell, editor panel layout) can reuse them without reimplementing the
//! interaction visuals.
//!
//! Visual parameters are injected via [`OverlayStyle`] and [`MenuStyle`]
//! so this crate stays free of any concrete theme; menu actions are
//! injected as callbacks.

use gpui::*;

use crate::sessions::{CornerDragModifier, CornerDragPreview, CornerDragSession};
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
    /// Splitter bar drag-in-progress color (line + hit-zone glow).
    pub active: Hsla,
}

impl Default for OverlayStyle {
    fn default() -> Self {
        Self {
            // Professional blue used by IDE split previews (≈ #60a5fa).
            accent: hsla(0.592, 0.94, 0.68, 0.9),
            tile_radius: 8.0,
            border: hsla(0.0, 0.0, 0.0, 0.2),
            selection: hsla(0.58, 0.6, 0.6, 0.8),
            // Sky blue (≈ #72cffe) for the active drag highlight.
            active: hsla(0.556, 0.99, 0.72, 0.9),
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
///
/// An overlay, not a flex child: the split areas tile seamlessly and the
/// bar floats on the boundary at `ratio` (a fraction of the container
/// width). The 4px hit zone sits on the second side; a 1px guide line
/// marks the boundary. `active` is the drag-in-progress state (driven by
/// the host from the drag session): the hit zone glows stronger and the
/// guide line takes the selection color.
pub fn splitter_bar_h(
    id: impl Into<ElementId>,
    ratio: f32,
    active: bool,
    style: &OverlayStyle,
) -> Stateful<Div> {
    if active {
        // Drag-in-progress: highlight only the 1px boundary line.
        div()
            .id(id)
            .absolute()
            .left(relative(ratio))
            .top_0()
            .bottom_0()
            .w(px(4.0))
            .cursor_col_resize()
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top_0()
                    .bottom_0()
                    .w(px(1.0))
                    .bg(style.active),
            )
    } else {
        div()
            .id(id)
            .absolute()
            .left(relative(ratio))
            .top_0()
            .bottom_0()
            .w(px(4.0))
            .cursor_col_resize()
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top_0()
                    .bottom_0()
                    .w(px(1.0))
                    .bg(style.border),
            )
            .hover(move |this| this.bg(style.selection.opacity(0.15)))
    }
}

/// Vertical splitter bar (resizes columns).
///
/// Same overlay model as [`splitter_bar_h`]: floats on the boundary at
/// `ratio` of the container height, 4px hit zone plus a 1px guide line.
pub fn splitter_bar_v(
    id: impl Into<ElementId>,
    ratio: f32,
    active: bool,
    style: &OverlayStyle,
) -> Stateful<Div> {
    if active {
        // Drag-in-progress: highlight only the 1px boundary line.
        div()
            .id(id)
            .absolute()
            .top(relative(ratio))
            .left_0()
            .right_0()
            .h(px(4.0))
            .cursor_row_resize()
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .h(px(1.0))
                    .bg(style.active),
            )
    } else {
        div()
            .id(id)
            .absolute()
            .top(relative(ratio))
            .left_0()
            .right_0()
            .h(px(4.0))
            .cursor_row_resize()
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .h(px(1.0))
                    .bg(style.border),
            )
            .hover(move |this| this.bg(style.selection.opacity(0.15)))
    }
}

/// Find the rect with the given id in a rect list.
fn rect_by_id<'a>(rects: &'a [AreaRect], id: usize) -> Option<&'a AreaRect> {
    rects.iter().find(|rect| rect.id == id)
}

/// Start a splitter-bar drag on a split node.
///
/// `current_ratio` is the split ratio at drag start; the real span is
/// refreshed on the first move event.
pub fn start_splitter_drag(
    layout: &mut crate::state::WindowLayout,
    split_id: usize,
    direction: Axis,
    start_pointer_pos: f32,
    current_ratio: f32,
) {
    layout.active_window_area_splitter_drag = Some(crate::sessions::SplitterDragSession {
        split_id,
        direction,
        start_pointer_pos,
        start_ratio: current_ratio,
        total_span: 1000.0,
    });
}

/// Open the border context menu on a split bar (right click).
pub fn open_border_menu(
    layout: &mut crate::state::WindowLayout,
    split_id: usize,
    direction: Axis,
    position: Point<Pixels>,
) {
    layout.active_window_area_border_menu = Some(crate::sessions::BorderMenuState {
        split_id,
        direction,
        position,
    });
}

/// Progress the active window-area drag gesture (splitter or corner).
///
/// Returns whether a gesture was active (the host should repaint) and an
/// action to execute immediately — split and join are deferred to
/// [`finish_window_drag`].
pub fn update_window_drag(
    layout: &mut crate::state::WindowLayout,
    pos: Point<Pixels>,
    viewport: Size<Pixels>,
) -> (bool, Option<crate::sessions::WindowAreaDragAction>) {
    if let Some(drag) = layout.active_window_area_splitter_drag {
        let current_pos = match drag.direction {
            Axis::Horizontal => f32::from(pos.x),
            Axis::Vertical => f32::from(pos.y),
        };
        let span = layout
            .window_area_split_pixel_span(drag.split_id, viewport)
            .unwrap_or_else(|| match drag.direction {
                Axis::Horizontal => f32::from(viewport.width),
                Axis::Vertical => f32::from(viewport.height),
            });
        if span > 1.0 {
            let mut session = drag;
            session.total_span = span;
            layout.active_window_area_splitter_drag = Some(session);
        }
        layout.update_window_area_splitter_drag(current_pos);
        (true, None)
    } else if layout.active_window_area_corner_drag.is_some() {
        let action = layout.update_window_area_corner_drag(pos, viewport);
        (true, action)
    } else {
        (false, None)
    }
}

/// End the active window-area drag gesture; returns the final action
/// (split or join) the host should apply.
pub fn finish_window_drag(
    layout: &mut crate::state::WindowLayout,
) -> Option<crate::sessions::WindowAreaDragAction> {
    if layout.active_window_area_splitter_drag.is_some() {
        layout.end_window_area_splitter_drag();
        None
    } else if layout.active_window_area_corner_drag.is_some() {
        layout.finish_window_area_corner_drag()
    } else {
        None
    }
}

/// The modifier key held during a corner drag, decoded from a mouse event.
fn corner_drag_modifier(event: &MouseDownEvent) -> CornerDragModifier {
    if event.modifiers.control {
        CornerDragModifier::Swap
    } else if event.modifiers.shift {
        CornerDragModifier::Duplicate
    } else {
        CornerDragModifier::None
    }
}

/// Build the four corner-drag handles of a tile.
///
/// `id_prefix` names the handles ("area-corner" for window areas,
/// "inner-corner" for editor panels) so ids stay unique per tree level;
/// `target_id` is the leaf id embedded in each handle's id and passed to
/// the drag callback. `on_start_drag` receives the decoded modifier, the
/// pointer position, and the app context on a left mouse-down.
pub fn corner_drag_handles<F>(
    id_prefix: &'static str,
    target_id: usize,
    gap: f32,
    handle_size: f32,
    rounded: bool,
    occlude: bool,
    on_start_drag: F,
) -> Stateful<Div>
where
    F: Fn(CornerDragModifier, Point<Pixels>, &mut App) + 'static + Clone,
{
    let make = |id_str: &'static str, top: bool, left: bool| {
        let on_start_drag = on_start_drag.clone();
        let mut corner_div = div()
            .id((
                SharedString::from(format!("{id_prefix}-{id_str}")),
                target_id,
            ))
            .absolute()
            .size(px(handle_size))
            .cursor_crosshair();
        if rounded {
            corner_div = corner_div.rounded(px(4.0));
        }
        if occlude {
            corner_div = corner_div.occlude();
        }
        if top {
            corner_div = corner_div.top(px(gap));
        } else {
            corner_div = corner_div.bottom(px(gap));
        }
        if left {
            corner_div = corner_div.left(px(gap));
        } else {
            corner_div = corner_div.right(px(gap));
        }
        corner_div.on_mouse_down(MouseButton::Left, move |event, _window, cx| {
            let modifier = corner_drag_modifier(event);
            on_start_drag(modifier, event.position, cx);
        })
    };

    div()
        .id((
            SharedString::from(format!("{id_prefix}-corners")),
            target_id,
        ))
        .absolute()
        .inset(px(-gap))
        .child(make("tl", true, true))
        .child(make("tr", true, false))
        .child(make("bl", false, true))
        .child(make("br", false, false))
}

/// The split preview line for a horizontal split (left|right): a vertical
/// divider at `ratio` of the target rect's width. Deliberately thin (1px)
/// like IDE split guides. `rect` is normalized (0..1) so the preview is
/// independent of the container's pixel size.
fn split_line_horizontal(rect: &AreaRect, ratio: f32, accent: Hsla) -> Div {
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
fn split_line_vertical(rect: &AreaRect, ratio: f32, accent: Hsla) -> Div {
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
    target: &AreaRect,
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
                        .child(format!("{arrow} Join Area")),
                ),
        )
        .into_any_element()
}

/// Render the corner-drag preview overlay.
///
/// `rects` are the leaf rects of the split tree in **normalized** layout
/// coordinates (0..1 of the initialized container), which the overlay
/// positions with `relative()` — so the preview tracks the tree geometry
/// exactly, regardless of the container's pixel size or any chrome
/// (topbar/bottombar) around it. Returns `None` while the gesture is
/// still in its initial `Dragging` state or when the target area cannot
/// be located.
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

/// Visual parameters for floating menu panels (border context menus).
///
/// Kept separate from [`OverlayStyle`] because menus need far more knobs
/// (panel surface, item rows, separators, text) than the drag overlays.
#[derive(Clone, Copy, Debug)]
pub struct MenuStyle {
    /// Menu panel surface.
    pub surface: Hsla,
    /// Panel border and separator color.
    pub border: Hsla,
    pub border_width: f32,
    pub radius: f32,
    pub width: f32,
    pub padding: f32,
    pub gap: f32,
    /// Item label text.
    pub text: Hsla,
    pub text_size: f32,
    pub text_weight: FontWeight,
    pub item_height: f32,
    pub item_padding_x: f32,
    pub item_radius: f32,
    /// Item hover highlight.
    pub item_hover: Hsla,
    pub separator_margin_x: f32,
    pub separator_margin_y: f32,
    pub separator_height: f32,
}

/// One entry of a border context menu: label plus activation callback.
pub struct BorderMenuItem<F> {
    pub label: &'static str,
    pub on_activate: F,
}

/// Render the border context menu (right click on a splitter bar).
///
/// `items` appear in order, each separated by a divider line; `on_dismiss`
/// runs when the user clicks the full-window overlay outside the menu. Each
/// action is invoked exactly once, so callbacks are moved in without a
/// `Clone` bound.
pub fn render_border_menu<F>(
    position: Point<Pixels>,
    items: Vec<BorderMenuItem<F>>,
    style: &MenuStyle,
    on_dismiss: F,
) -> AnyElement
where
    F: Fn(&mut App) + 'static,
{
    let panel = div()
        .id("tiled-border-context-menu")
        .absolute()
        .occlude()
        .top(px(f32::from(position.y)))
        .left(px(f32::from(position.x)))
        .w(px(style.width))
        .p(px(style.padding))
        .flex()
        .flex_col()
        .gap(px(style.gap))
        .bg(style.surface)
        .border(px(style.border_width))
        .border_color(style.border)
        .rounded(px(style.radius))
        .shadow_lg()
        .children(items.into_iter().enumerate().flat_map(|(idx, item)| {
            let mut elements: Vec<AnyElement> = Vec::with_capacity(2);
            if idx > 0 {
                elements.push(
                    div()
                        .id(("tiled-border-menu-sep", idx))
                        .mx(px(style.separator_margin_x))
                        .my(px(style.separator_margin_y))
                        .h(px(style.separator_height))
                        .bg(style.border)
                        .into_any_element(),
                );
            }
            let on_activate = item.on_activate;
            elements.push(
                div()
                    .id(("tiled-border-menu-item", idx))
                    .h(px(style.item_height))
                    .px(px(style.item_padding_x))
                    .flex()
                    .items_center()
                    .rounded(px(style.item_radius))
                    .bg(style.surface)
                    .hover(|this| this.bg(style.item_hover))
                    .active(|this| this.opacity(0.92))
                    .cursor_pointer()
                    .text_size(px(style.text_size))
                    .font_weight(style.text_weight)
                    .text_color(style.text)
                    .child(item.label)
                    .on_click(move |_event, _window, cx| on_activate(cx))
                    .into_any_element(),
            );
            elements
        }));

    overlay_container()
        .id("tiled-border-context-menu-wrapper")
        .on_mouse_down(MouseButton::Left, move |_event, _window, cx| on_dismiss(cx))
        .child(panel)
        .into_any_element()
}
