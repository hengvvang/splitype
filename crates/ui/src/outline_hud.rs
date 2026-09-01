//! Outline HUD — Notion-style floating equal-length ticks rail and hover
//! TOC popover. Pure presentation over [`editor_contracts::OutlineNode`] data;
//! navigation re-enters the coordinating crate through
//! [`editor_contracts::OutlineHost`].

use std::sync::Arc;

use gpui::*;

use editor_contracts::{OutlineHost, OutlineNode};
use theme::Theme;

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
                    .rounded(px(d.outline_node_radius))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .bg(if is_active {
                        c.panel_row_selected
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
                        cx.stop_propagation();
                        host_navigate.navigate_to(idx, cx);
                    }),
            );
        }

        Some(
            div()
                .id(ElementId::Name(
                    format!("floating-outline-popover-{pane_id}").into(),
                ))
                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                    cx.stop_propagation();
                })
                .mr(px(8.0))
                .w(px(260.0))
                .max_h(px(420.0))
                .overflow_y_scroll()
                .bg(c.dialog_surface)
                .border(px(d.dialog_border_width))
                .border_color(c.dialog_border)
                .rounded(px(d.menu_panel_radius))
                .shadow_lg()
                .p(px(d.menu_panel_padding))
                .flex()
                .flex_col()
                .gap(px(d.menu_panel_gap))
                .children(items),
        )
    } else {
        None
    };

    // ── Equal-Length / Level-Scaled Micro-ticks Rail (Notion-style) ──
    let mut ticks = Vec::with_capacity(headings.len());
    for (idx, node) in headings.iter().enumerate() {
        let is_active = idx == active_index;
        let base_w = match node.level {
            1 => 16.0,
            2 => 13.0,
            3 => 10.0,
            _ => 8.0,
        };
        let (w, h) = if is_active {
            (18.0f32.max(base_w + 2.0), 3.0)
        } else {
            (base_w, 2.0)
        };

        let tick_color = if is_active {
            c.focus_accent
        } else {
            c.dialog_border
        };

        let host_navigate = host.clone();

        ticks.push(
            div()
                .id(ElementId::Name(
                    format!("outline-rail-tick-{pane_id}-{idx}").into(),
                ))
                .h(px(8.0))
                .w_full()
                .flex()
                .items_center()
                .justify_end()
                .cursor_pointer()
                .child(div().w(px(w)).h(px(h)).rounded_full().bg(tick_color))
                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                    cx.stop_propagation();
                    host_navigate.navigate_to(idx, cx);
                }),
        );
    }

    let rail_el = div()
        .id(ElementId::Name(
            format!("floating-outline-rail-{pane_id}").into(),
        ))
        .w(px(28.0))
        .py(px(6.0))
        .px(px(4.0))
        .rounded(px(6.0))
        .flex()
        .flex_col()
        .items_end()
        .gap(px(3.0))
        .cursor_pointer()
        .hover(|s| s.bg(c.panel_row_hover))
        .children(ticks);

    let host_hover = host.clone();
    div()
        .id(ElementId::Name(
            format!("floating-outline-hud-{pane_id}").into(),
        ))
        .occlude()
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
