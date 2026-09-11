//! Content-independent split interaction chrome: the draggable divider
//! bars between tiles, the corner-drag handles and the splitter-bar
//! context menu.
//!
//! Visual parameters are injected via [`OverlayStyle`] and [`MenuStyle`] so
//! this module stays free of any concrete theme; menu actions are injected
//! as callbacks.

use gpui::*;
use theme::Theme;

use splitter::gesture::{CornerDragModifier, CornerDragSession, past_shortcut_threshold};

/// Visual parameters for split interaction overlays.
#[derive(Clone, Copy, Debug)]
pub struct OverlayStyle {
    /// Accent color for split-preview lines and join highlights.
    pub accent: Hsla,
    /// Corner radius of panel tiles, used to round the highlight overlays.
    pub tile_radius: f32,
    /// Corner radius of the cursor-following action panels.
    pub panel_radius: f32,
    /// Splitter bar base color.
    pub border: Hsla,
    /// Hover background for divider bars and corner drag handles, unified
    /// with the explorer row-hover color.
    pub hover: Hsla,
    /// Splitter bar drag-in-progress color (line + hit-zone glow).
    pub active: Hsla,
    /// Surface card background color for central indicator badges.
    pub surface: Hsla,
    /// Text color for central indicator badges.
    pub text: Hsla,
}

impl OverlayStyle {
    /// Maps a theme to the split interaction overlay parameters.
    pub fn from_theme(theme: &Theme) -> Self {
        let c = &theme.colors;
        let d = &theme.dimensions;
        Self {
            accent: c.split_indicator,
            tile_radius: d.panel_tile_radius,
            panel_radius: theme::dimensions::CONTROL_CORNER_RADIUS,
            border: c.dialog_border,
            hover: c.panel_row_hover,
            active: c.focus_accent,
            surface: c.dialog_surface,
            text: c.dialog_title,
        }
    }
}

/// Overlay container covering the whole host area, absolutely positioned.
///
/// Rendered as the topmost layer so drag previews and menus can draw over
/// every tile.
pub fn overlay_container() -> Div {
    div().absolute().top_0().left_0().right_0().bottom_0()
}

/// Default span for corner-drag handles along each edge (in pixels).
///
/// Kept in sync between [`corner_drag_handles`] and the safe interactive
/// zones of [`splitter_bar_h`] / [`splitter_bar_v`].
pub const SPLITTER_CORNER_SPAN: f32 = 48.0;

/// Horizontal splitter bar (resizes columns separated by a vertical divider).
///
/// An overlay floating at `ratio` across the container width:
/// - The 1px guide line runs continuously from `top_0()` to `bottom_0()` across the full height.
/// - The 12px interactive hit zone and resize cursor are inset by `SPLITTER_CORNER_SPAN` (48px)
///   from the top and bottom edges, guaranteeing that corner drag handles belonging to the
///   panel and panel-card four corners have complete, unobstructed priority.
pub fn splitter_bar_h<OnDrag, OnMenu>(
    id: impl Into<ElementId>,
    ratio: f32,
    active: bool,
    style: &OverlayStyle,
    on_drag: OnDrag,
    on_menu: OnMenu,
) -> Stateful<Div>
where
    OnDrag: Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    OnMenu: Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
{
    let hit_width = 12.0;
    let offset = -hit_width / 2.0;
    let line_width = if active { 2.5 } else { 1.0 };
    let line_color = if active { style.active } else { style.border };

    let mut handle = div()
        .absolute()
        .left_0()
        .right_0()
        .cursor_col_resize()
        .on_mouse_down(MouseButton::Left, on_drag)
        .on_mouse_down(MouseButton::Right, on_menu);

    if active {
        handle = handle.top_0().bottom_0().bg(style.hover);
    } else {
        handle = handle
            .top(px(SPLITTER_CORNER_SPAN))
            .bottom(px(SPLITTER_CORNER_SPAN))
            .hover(move |this| this.bg(style.hover));
    }

    div()
        .id(id)
        .absolute()
        .left(relative(ratio))
        .ml(px(offset))
        .top_0()
        .bottom_0()
        .w(px(hit_width))
        .child(
            div()
                .absolute()
                .left(px((hit_width - line_width) / 2.0))
                .top_0()
                .bottom_0()
                .w(px(line_width))
                .bg(line_color),
        )
        .child(handle)
}

/// Vertical splitter bar (resizes rows separated by a horizontal divider).
///
/// An overlay floating at `ratio` across the container height:
/// - The 1px guide line runs continuously from `left_0()` to `right_0()` across the full width.
/// - The 12px interactive hit zone and resize cursor are inset by `SPLITTER_CORNER_SPAN` (48px)
///   from the left and right edges, guaranteeing that corner drag handles belonging to the
///   panel and panel-card four corners have complete, unobstructed priority.
pub fn splitter_bar_v<OnDrag, OnMenu>(
    id: impl Into<ElementId>,
    ratio: f32,
    active: bool,
    style: &OverlayStyle,
    on_drag: OnDrag,
    on_menu: OnMenu,
) -> Stateful<Div>
where
    OnDrag: Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    OnMenu: Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
{
    let hit_height = 12.0;
    let offset = -hit_height / 2.0;
    let line_height = if active { 2.5 } else { 1.0 };
    let line_color = if active { style.active } else { style.border };

    let mut handle = div()
        .absolute()
        .top_0()
        .bottom_0()
        .cursor_row_resize()
        .on_mouse_down(MouseButton::Left, on_drag)
        .on_mouse_down(MouseButton::Right, on_menu);

    if active {
        handle = handle.left_0().right_0().bg(style.hover);
    } else {
        handle = handle
            .left(px(SPLITTER_CORNER_SPAN))
            .right(px(SPLITTER_CORNER_SPAN))
            .hover(move |this| this.bg(style.hover));
    }

    div()
        .id(id)
        .absolute()
        .top(relative(ratio))
        .mt(px(offset))
        .left_0()
        .right_0()
        .h(px(hit_height))
        .child(
            div()
                .absolute()
                .top(px((hit_height - line_height) / 2.0))
                .left_0()
                .right_0()
                .h(px(line_height))
                .bg(line_color),
        )
        .child(handle)
}

/// The modifier key held during a corner drag, decoded from a mouse event.
fn corner_drag_modifier(event: &MouseDownEvent) -> CornerDragModifier {
    if event.modifiers.control {
        CornerDragModifier::Ctrl
    } else if event.modifiers.shift {
        CornerDragModifier::Shift
    } else {
        CornerDragModifier::None
    }
}

/// Build the four corner-gap drag handles of a tile located in the
/// difference area between the tile rectangle and the inner panel card.
///
/// - `gap`: margin thickness around the inner panel card.
/// - `corner_span`: span of the corner drag zone along each edge (e.g. 48px).
pub fn corner_drag_handles<F>(
    id_prefix: &'static str,
    target_id: impl Into<splitter::LeafId>,
    gap: f32,
    corner_span: f32,
    style: &OverlayStyle,
    on_start_drag: F,
) -> Stateful<Div>
where
    F: Fn(CornerDragModifier, Point<Pixels>, &mut App) + 'static + Clone,
{
    let target_id = target_id.into().as_usize();
    let gap_thickness = gap.max(6.0);
    let hover_bg = style.hover;
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
                .hover(move |s| s.bg(hover_bg));

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

    div()
        .id((
            SharedString::from(format!("{id_prefix}-corners")),
            target_id,
        ))
        .absolute()
        .top_0()
        .bottom_0()
        .left_0()
        .right_0()
        .child(make_corner("tl", true, true))
        .child(make_corner("tr", true, false))
        .child(make_corner("bl", false, true))
        .child(make_corner("br", false, false))
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

/// Map a theme to the splitter border-menu style parameters.
pub fn border_menu_style(theme: &Theme) -> MenuStyle {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;
    MenuStyle {
        surface: c.dialog_surface,
        border: c.dialog_border,
        border_width: d.dialog_border_width,
        radius: d.menu_panel_radius,
        width: d.menu_panel_width,
        padding: d.menu_panel_padding,
        gap: d.menu_panel_gap,
        text: c.dialog_secondary_button_text,
        text_size: d.menu_text_size,
        text_weight: t.dialog_body_weight.to_font_weight(),
        item_height: d.menu_item_height,
        item_padding_x: d.menu_item_padding_x,
        item_radius: d.menu_item_radius,
        item_hover: c.panel_row_hover,
        separator_margin_x: d.menu_separator_margin_x,
        separator_margin_y: d.menu_separator_margin_y,
        separator_height: d.menu_separator_height,
    }
}

/// Host-supplied behavior for the standard border-menu actions.
pub struct BorderMenuActions {
    pub split_horizontal: Box<dyn Fn(&mut App)>,
    pub split_vertical: Box<dyn Fn(&mut App)>,
    pub swap: Box<dyn Fn(&mut App)>,
    pub close: Box<dyn Fn(&mut App)>,
}

/// Render the border context menu (right click on a splitter bar) with the
/// standard actions: split horizontally/vertically, swap and close, with
/// host-supplied behavior. `on_dismiss` runs when the user clicks the
/// full-window overlay outside the menu.
pub fn render_standard_border_menu(
    position: Point<Pixels>,
    actions: BorderMenuActions,
    style: &MenuStyle,
    on_dismiss: Box<dyn Fn(&mut App)>,
) -> AnyElement {
    type MenuAction = Box<dyn Fn(&mut App)>;
    let items: Vec<(&'static str, MenuAction)> = vec![
        ("Split Horizontally", actions.split_horizontal),
        ("Split Vertically", actions.split_vertical),
        ("Swap Panels", actions.swap),
        ("Close Panel", actions.close),
    ];

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
        .children(
            items
                .into_iter()
                .enumerate()
                .flat_map(|(idx, (label, on_activate))| {
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
                            .cursor_pointer()
                            .text_size(px(style.text_size))
                            .font_weight(style.text_weight)
                            .text_color(style.text)
                            .child(label)
                            .on_click(move |_event, _window, cx| on_activate(cx))
                            .into_any_element(),
                    );
                    elements
                }),
        );

    overlay_container()
        .id("tiled-border-context-menu-wrapper")
        .on_mouse_down(MouseButton::Left, move |_event, _window, cx| on_dismiss(cx))
        .child(panel)
        .into_any_element()
}

/// Whether a corner-drag gesture may show a preview (and be applied) by
/// hosts: plain and Ctrl drags always; Shift (duplicate into a new window)
/// requires a minimum movement so a plain click doesn't clone.
pub fn preview_allowed(drag: &CornerDragSession) -> bool {
    match drag.modifier {
        CornerDragModifier::None | CornerDragModifier::Ctrl => true,
        CornerDragModifier::Shift => past_shortcut_threshold(drag),
    }
}
