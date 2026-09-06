//! Outline Indicator Strip — slim right-side ticks rail that expands TOC on hover.
//!
//! Replaces floating HUD and heavy dock panels with an unobtrusive 18px indicator
//! column placed to the right of the document text.

use std::sync::Arc;
use gpui::prelude::FluentBuilder;
use gpui::*;
use crate::outline::{OutlineHost, OutlineNode};
use theme::Theme;

/// Renders a slim (18px) outline indicator rail on the right side of the content.
///
/// Rest state: subtle micro-ticks representing document headings.
/// Hover/Pinned state: opens a floating TOC card popover to the left over the content.
pub fn render_outline_indicator_strip(
    pane_id: usize,
    headings: &[OutlineNode],
    active_index: Option<usize>,
    is_hovered: bool,
    theme: &Theme,
    host: &Arc<dyn OutlineHost>,
) -> AnyElement {
    if headings.is_empty() {
        return div().w(px(0.0)).into_any_element();
    }

    let c = &theme.colors;
    let d = &theme.dimensions;
    let active_index = active_index.unwrap_or(0);

    // ── Popover Card (Expanded Notion-style TOC on Hover) ──
    let popover_el = if is_hovered {
        let mut items = Vec::with_capacity(headings.len());
        for (idx, node) in headings.iter().enumerate() {
            let is_active = idx == active_index;
            let indent = match node.level {
                1 => 8.0,
                2 => 16.0,
                3 => 24.0,
                4 => 32.0,
                _ => 38.0,
            };
            let label = node.label.clone();
            let host_navigate = host.clone();
            let host_item_move = host.clone();
            let host_item_hover = host.clone();

            items.push(
                div()
                    .id(ElementId::Name(format!("outline-popover-item-{pane_id}-{idx}").into()))
                    .relative()
                    .w_full()
                    .pl(px(indent))
                    .pr(px(10.0))
                    .py(px(4.0))
                    .rounded(px(d.outline_node_radius))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .when(is_active, |this| this.bg(c.panel_row_hover))
                    .hover(|style| style.bg(c.panel_row_hover))
                    .on_mouse_move(move |_event, window, cx| {
                        host_item_move.set_hovered(true, window, cx);
                    })
                    .on_hover(move |hovered: &bool, window, cx| {
                        if *hovered {
                            host_item_hover.set_hovered(true, window, cx);
                        }
                    })
                    .children(if is_active {
                        Some(
                            div()
                                .absolute()
                                .left(px(4.0))
                                .top(px(5.0))
                                .bottom(px(5.0))
                                .w(px(3.0))
                                .rounded_full()
                                .bg(c.focus_accent),
                        )
                    } else {
                        None
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .overflow_hidden()
                            .truncate()
                            .text_size(px(12.0))
                            .text_color(if is_active {
                                c.focus_accent
                            } else {
                                c.text_default
                            })
                            .font_weight(FontWeight::NORMAL)
                            .child(label),
                    )
                    .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                        cx.stop_propagation();
                        host_navigate.navigate_to(idx, cx);
                    }),
            );
        }

        let host_popover_move = host.clone();
        let host_popover_hover = host.clone();
        Some(
            div()
                .id(ElementId::Name(format!("outline-popover-{pane_id}").into()))
                .occlude()
                .absolute()
                .top(px(2.0))
                .right(px(14.0))
                .w(px(260.0))
                .max_h(px(420.0))
                .overflow_y_scroll()
                .bg(c.dialog_surface)
                .border(px(1.0))
                .border_color(c.dialog_border)
                .rounded(px(d.menu_panel_radius))
                .shadow_xl()
                .p(px(6.0))
                .flex()
                .flex_col()
                .gap(px(2.0))
                .on_mouse_move(move |_event, window, cx| {
                    host_popover_move.set_hovered(true, window, cx);
                })
                .on_hover(move |hovered: &bool, window, cx| {
                    host_popover_hover.set_hovered(*hovered, window, cx);
                })
                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                    cx.stop_propagation();
                })
                .children(items),
        )
    } else {
        None
    };

    // ── Slim Micro-ticks Rail ──
    let mut ticks = Vec::with_capacity(headings.len());
    for (idx, node) in headings.iter().enumerate() {
        let is_active = idx == active_index;
        let base_w = match node.level {
            1 => 12.0,
            2 => 9.0,
            3 => 7.0,
            _ => 5.0,
        };
        let (w, h) = if is_active {
            (14.0f32.max(base_w + 2.0), 3.0)
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
                .id(ElementId::Name(format!("outline-rail-tick-{pane_id}-{idx}").into()))
                .h(px(7.0))
                .w_full()
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .child(div().w(px(w)).h(px(h)).rounded_full().bg(tick_color))
                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                    cx.stop_propagation();
                    host_navigate.navigate_to(idx, cx);
                }),
        );
    }

    let host_hover = host.clone();
    let host_rail_move = host.clone();
    div()
        .id(ElementId::Name(format!("outline-indicator-strip-{pane_id}").into()))
        .w(px(18.0))
        .h_full()
        .flex_shrink_0()
        .relative()
        .flex()
        .flex_col()
        .items_center()
        .pt(px(12.0))
        .cursor_pointer()
        .on_mouse_move(move |_event, window, cx| {
            host_rail_move.set_hovered(true, window, cx);
        })
        .on_hover(move |hovered: &bool, window, cx| {
            host_hover.set_hovered(*hovered, window, cx);
        })
        .children(ticks)
        .children(popover_el)
        .into_any_element()
}
