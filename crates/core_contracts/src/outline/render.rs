//! Outline HUD — Notion-style floating equal-length ticks rail and hover
//! TOC popover. Pure presentation over [`OutlineNode`] data; navigation
//! re-enters the coordinating crate through [`OutlineHost`].

use std::sync::Arc;

use gpui::*;

use crate::OutlineNode;
use theme::Theme;

/// Navigation seam: clicking a heading in the HUD asks the coordinating
/// crate to move the active pane to that heading.
pub trait OutlineHost: Send + Sync + 'static {
    /// Navigate to the heading at `index`.
    fn navigate_to(&self, index: usize, cx: &mut App);

    /// Report popover hover-state changes (debounced by the host). The
    /// window is passed so the host can schedule its debounce through a
    /// try-borrow-safe handle.
    fn set_hovered(&self, hovered: bool, window: &mut Window, cx: &mut App);
}

/// Renders the floating outline HUD (equal-length micro-ticks rail plus
/// the hover TOC popover card) for the given heading data.
pub fn render_floating_outline_hud(
    pane_id: usize,
    headings: &[OutlineNode],
    active_index: Option<usize>,
    is_hovered: bool,
    theme: &Theme,
    host: &Arc<dyn OutlineHost>,
) -> AnyElement {
    if headings.is_empty() {
        return div().into_any_element();
    }

    let c = &theme.colors;
    let d = &theme.dimensions;
    let active_index = active_index.unwrap_or(0);

    // ── Popover Card (Expanded Notion-style TOC) ──
    let popover_el = if is_hovered {
        let mut items = Vec::with_capacity(headings.len());
        for (idx, node) in headings.iter().enumerate() {
            let is_active = idx == active_index;
            let indent = match node.level {
                1 => 6.0,
                2 => 16.0,
                3 => 26.0,
                4 => 36.0,
                _ => 44.0,
            };
            let label = node.label.clone();
            let host_navigate = host.clone();

            items.push(
                div()
                    .id(ElementId::Name(
                        format!("outline-popover-item-{idx}").into(),
                    ))
                    .w_full()
                    .pl(px(indent))
                    .pr(px(10.0))
                    .py(px(4.0))
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .bg(if is_active {
                        c.source_mode_block_bg
                    } else {
                        hsla(0.0, 0.0, 0.0, 0.0)
                    })
                    .hover(|style| style.bg(c.panel_row_hover))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .overflow_hidden()
                            .truncate()
                            .text_size(px(12.5))
                            .text_color(if is_active {
                                c.focus_accent
                            } else {
                                c.text_default
                            })
                            .font_weight(if is_active {
                                FontWeight::SEMIBOLD
                            } else {
                                FontWeight::NORMAL
                            })
                            .child(label),
                    )
                    .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                        host_navigate.navigate_to(idx, cx);
                    }),
            );
        }

        Some(
            div()
                .id(ElementId::Name("floating-outline-popover".into()))
                .mr(px(8.0))
                .w(px(260.0))
                .max_h(px(420.0))
                .overflow_y_scroll()
                .bg(c.dialog_surface)
                .border_1()
                .border_color(c.dialog_border)
                .rounded(px(d.panel_tile_radius.max(8.0)))
                .shadow_lg()
                .p(px(6.0))
                .flex()
                .flex_col()
                .gap(px(2.0))
                .children(items),
        )
    } else {
        None
    };

    // ── Equal-Length Micro-ticks Rail (Notion-style) ──
    let mut ticks = Vec::with_capacity(headings.len());
    for (idx, _node) in headings.iter().enumerate() {
        let is_active = idx == active_index;
        let (w, h) = if is_active { (18.0, 3.0) } else { (14.0, 2.0) };

        let tick_color = if is_active {
            c.focus_accent
        } else {
            c.dialog_border
        };

        let host_navigate = host.clone();

        ticks.push(
            div()
                .id(ElementId::Name(format!("outline-rail-tick-{idx}").into()))
                .h(px(8.0))
                .w_full()
                .flex()
                .items_center()
                .justify_end()
                .cursor_pointer()
                .child(div().w(px(w)).h(px(h)).rounded_full().bg(tick_color))
                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                    host_navigate.navigate_to(idx, cx);
                }),
        );
    }

    let rail_el = div()
        .id(ElementId::Name("floating-outline-rail".into()))
        .w(px(24.0))
        .py(px(6.0))
        .px(px(3.0))
        .flex()
        .flex_col()
        .items_end()
        .gap(px(2.0))
        .cursor_pointer()
        .children(ticks);

    let host_hover = host.clone();
    div()
        .id(ElementId::Name(
            format!("floating-outline-hud-{pane_id}").into(),
        ))
        .absolute()
        .top(px(40.0))
        .right(px(14.0))
        .flex()
        .flex_row()
        .items_start()
        .on_hover(move |hovered: &bool, window, cx| {
            host_hover.set_hovered(*hovered, window, cx);
        })
        .children(popover_el)
        .child(rail_el)
        .into_any_element()
}
