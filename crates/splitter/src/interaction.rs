//! Content-independent split interaction rendering.
//!
//! The draggable splitter bars between panels, the corner-drag handles and
//! the splitter-bar context menu are pure window-management UI: they depend
//! on the layout tree geometry and a small set of visual parameters, never
//! on what the panels contain. Rendering and gesture state machines live
//! here so any host (window shell, editor panel layout) can reuse them
//! without reimplementing the interaction visuals.
//!
//! Visual parameters are injected via [`OverlayStyle`] and [`MenuStyle`]
//! so this crate stays free of any concrete theme; menu actions are
//! injected as callbacks.

use gpui::*;

use crate::root::SplitterRoot;
use crate::sessions::{CornerDragModifier, CornerDragSession};
use crate::tree::{NodeId, SplitAxis};

/// Visual parameters for split interaction overlays.
#[derive(Clone, Copy, Debug)]
pub struct OverlayStyle {
    /// Accent color for split-preview lines and join highlights.
    pub accent: Hsla,
    /// Corner radius of panel tiles, used to round the highlight overlays.
    pub tile_radius: f32,
    /// Splitter bar base color.
    pub border: Hsla,
    /// Splitter bar hover color.
    pub selection: Hsla,
    /// Splitter bar drag-in-progress color (line + hit-zone glow).
    pub active: Hsla,
    /// Surface card background color for central indicator badges.
    pub surface: Hsla,
    /// Text color for central indicator badges.
    pub text: Hsla,
}

impl Default for OverlayStyle {
    fn default() -> Self {
        Self {
            // Professional blue used by IDE split previews (≈ #60a5fa).
            accent: hsla(0.592, 0.94, 0.68, 0.9),
            tile_radius: 8.0,
            border: hsla(0.0, 0.0, 1.0, 0.15),
            selection: hsla(0.58, 0.6, 0.6, 0.8),
            // Sky blue (≈ #72cffe) for the active drag highlight.
            active: hsla(0.556, 0.99, 0.72, 0.9),
            surface: hsla(0.0, 0.0, 0.12, 0.95),
            text: hsla(0.0, 0.0, 0.95, 1.0),
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
/// An overlay, not a flex child: the split leaves tile seamlessly and the
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
    let hit_width = 12.0;
    let offset = -hit_width / 2.0;
    if active {
        // Drag-in-progress: highlight the boundary line with 2.5px thickness.
        div()
            .id(id)
            .absolute()
            .left(relative(ratio))
            .ml(px(offset))
            .top_0()
            .bottom_0()
            .w(px(hit_width))
            .cursor_col_resize()
            .child(
                div()
                    .absolute()
                    .left(px((hit_width - 2.5) / 2.0))
                    .top_0()
                    .bottom_0()
                    .w(px(2.5))
                    .bg(style.active),
            )
    } else {
        div()
            .id(id)
            .absolute()
            .left(relative(ratio))
            .ml(px(offset))
            .top_0()
            .bottom_0()
            .w(px(hit_width))
            .cursor_col_resize()
            .child(
                div()
                    .absolute()
                    .left(px((hit_width - 1.0) / 2.0))
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
/// `ratio` of the container height, 12px hit zone plus a centered guide line.
pub fn splitter_bar_v(
    id: impl Into<ElementId>,
    ratio: f32,
    active: bool,
    style: &OverlayStyle,
) -> Stateful<Div> {
    let hit_height = 12.0;
    let offset = -hit_height / 2.0;
    if active {
        // Drag-in-progress: highlight the boundary line with 2.5px thickness.
        div()
            .id(id)
            .absolute()
            .top(relative(ratio))
            .mt(px(offset))
            .left_0()
            .right_0()
            .h(px(hit_height))
            .cursor_row_resize()
            .child(
                div()
                    .absolute()
                    .top(px((hit_height - 2.5) / 2.0))
                    .left_0()
                    .right_0()
                    .h(px(2.5))
                    .bg(style.active),
            )
    } else {
        div()
            .id(id)
            .absolute()
            .top(relative(ratio))
            .mt(px(offset))
            .left_0()
            .right_0()
            .h(px(hit_height))
            .cursor_row_resize()
            .child(
                div()
                    .absolute()
                    .top(px((hit_height - 1.0) / 2.0))
                    .left_0()
                    .right_0()
                    .h(px(1.0))
                    .bg(style.border),
            )
            .hover(move |this| this.bg(style.selection.opacity(0.15)))
    }
}

/// Start a splitter-bar drag on a split node.
///
/// `current_ratio` is the split ratio at drag start; the real span is
/// refreshed on the first move event.
pub fn start_splitter_drag<T: Clone + PartialEq>(
    container: &mut SplitterRoot<T>,
    split_id: NodeId,
    axis: SplitAxis,
    start_pointer_pos: f32,
    current_ratio: f32,
) {
    if container.tree.find_maximized_leaf().is_some() {
        return;
    }
    container.active_splitter_drag = Some(crate::sessions::SplitterDragSession {
        split_id,
        axis,
        start_pointer_pos,
        start_ratio: current_ratio,
        total_span: 1000.0,
    });
}

/// Open the border context menu on a split bar (right click).
pub fn open_border_menu<T: Clone + PartialEq>(
    container: &mut SplitterRoot<T>,
    split_id: NodeId,
    axis: SplitAxis,
    position: Point<Pixels>,
) {
    if container.tree.find_maximized_leaf().is_some() {
        return;
    }
    container.active_border_menu = Some(crate::sessions::BorderMenuState {
        split_id,
        axis,
        position,
    });
}

/// Progress the active drag gesture (splitter or corner) of a root.
///
/// Generic over the layout level: the outer window root passes the
/// pointer and viewport in window coordinates; an editor pane layout
/// passes them in its local space. Returns whether a gesture was active
/// (the host should repaint). The host reads the root's drag sessions to
/// apply its own policy; `finish_splitter_drag` returns the corner-drag
/// facts on release.
pub fn update_splitter_drag<T: Clone + PartialEq>(
    container: &mut SplitterRoot<T>,
    pos: Point<Pixels>,
    viewport: Size<Pixels>,
) -> bool {
    if let Some(drag) = container.active_splitter_drag {
        let current_pos = match drag.axis {
            SplitAxis::Horizontal => f32::from(pos.x),
            SplitAxis::Vertical => f32::from(pos.y),
        };
        let span = container
            .split_pixel_span(drag.split_id, viewport)
            .unwrap_or_else(|| match drag.axis {
                SplitAxis::Horizontal => f32::from(viewport.width),
                SplitAxis::Vertical => f32::from(viewport.height),
            });
        if span > 1.0 {
            let mut session = drag;
            session.total_span = span;
            container.active_splitter_drag = Some(session);
        }
        container.update_splitter_drag(current_pos);
        true
    } else if container.corner_drag_panel().is_some() {
        container.update_corner_drag(pos, viewport);
        true
    } else {
        false
    }
}

/// End the active drag gesture of a root; returns the final corner-
/// drag facts (splitter-bar drags just end).
pub fn finish_splitter_drag<T: Clone + PartialEq>(
    container: &mut SplitterRoot<T>,
) -> Option<CornerDragSession> {
    if container.active_splitter_drag.is_some() {
        container.end_splitter_drag();
        None
    } else if container.corner_drag_panel().is_some() {
        container.finish_corner_drag()
    } else {
        None
    }
}

/// The modifier key held during a corner drag, decoded from a mouse event.
fn corner_drag_modifier(event: &MouseDownEvent) -> CornerDragModifier {
    if event.modifiers.control {
        CornerDragModifier::Ctrl
    } else if event.modifiers.shift {
        CornerDragModifier::Shift
    } else if event.modifiers.alt {
        CornerDragModifier::Alt
    } else {
        CornerDragModifier::None
    }
}

/// Build the four corner-gap drag handles of a tile located in the difference
/// area between the tile rectangle and the inner panel card.
///
/// - `gap`: Margin thickness around the inner panel card.
/// - `corner_span`: Span of the corner drag zone along each edge (e.g. 48.0px).
pub fn corner_drag_handles<F>(
    id_prefix: &'static str,
    target_id: NodeId,
    gap: f32,
    corner_span: f32,
    rounded: bool,
    occlude: bool,
    on_start_drag: F,
) -> Stateful<Div>
where
    F: Fn(CornerDragModifier, Point<Pixels>, &mut App) + 'static + Clone,
{
    corner_drag_handles_sides(
        id_prefix,
        target_id,
        gap,
        corner_span,
        rounded,
        occlude,
        true,
        true,
        on_start_drag,
    )
}

/// Build the corner-gap drag handles of a tile with selectable top/bottom sides.
pub fn corner_drag_handles_sides<F>(
    id_prefix: &'static str,
    target_id: NodeId,
    gap: f32,
    corner_span: f32,
    _rounded: bool,
    _occlude: bool,
    include_top: bool,
    include_bottom: bool,
    on_start_drag: F,
) -> Stateful<Div>
where
    F: Fn(CornerDragModifier, Point<Pixels>, &mut App) + 'static + Clone,
{
    let gap_thickness = gap.max(6.0);
    let make_corner = |corner_str: &'static str, top: bool, left: bool| {
        let on_start_drag = on_start_drag.clone();
        let make_arm = |dir_str: &'static str, is_h: bool| {
            let on_start_drag = on_start_drag.clone();
            let mut arm = div()
                .id((
                    SharedString::from(format!("{id_prefix}-{corner_str}-{dir_str}")),
                    target_id,
                ))
                .absolute()
                .cursor_crosshair()
                .hover(|s| s.bg(hsla(0.0, 0.0, 1.0, 0.15)));

            if top {
                arm = arm.top_0();
            } else {
                arm = arm.bottom_0();
            }
            if left {
                arm = arm.left_0();
            } else {
                arm = arm.right_0();
            }

            if is_h {
                arm = arm.h(px(gap_thickness)).w(px(corner_span));
            } else {
                arm = arm.w(px(gap_thickness)).h(px(corner_span));
            }

            arm.on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                let modifier = corner_drag_modifier(event);
                on_start_drag(modifier, event.position, cx);
            })
        };

        let mut corner_box = div()
            .id((
                SharedString::from(format!("{id_prefix}-{corner_str}")),
                target_id,
            ))
            .absolute()
            .w(px(corner_span))
            .h(px(corner_span));

        if top {
            corner_box = corner_box.top_0();
        } else {
            corner_box = corner_box.bottom_0();
        }
        if left {
            corner_box = corner_box.left_0();
        } else {
            corner_box = corner_box.right_0();
        }

        corner_box
            .child(make_arm("h", true))
            .child(make_arm("v", false))
    };

    let mut container = div()
        .id((
            SharedString::from(format!("{id_prefix}-corners")),
            target_id,
        ))
        .absolute()
        .top_0()
        .bottom_0()
        .left_0()
        .right_0();

    if include_top {
        container = container
            .child(make_corner("tl", true, true))
            .child(make_corner("tr", true, false));
    }
    if include_bottom {
        container = container
            .child(make_corner("bl", false, true))
            .child(make_corner("br", false, false));
    }

    container
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
