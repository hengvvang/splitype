//! Presentation for the floating Outline widget overlay.

use gpui::prelude::FluentBuilder;
use gpui::*;
use std::sync::Arc;
use theme::Theme;

use editor_contracts::{OutlineNode, PaneId};

/// Actions supported by the outline widget host.
pub trait OutlineWidgetHost: Send + Sync + 'static {
    fn navigate_to(&self, index: usize, cx: &mut App);
    fn close_widget(&self, cx: &mut App);

    /// Backwards-compatible alias.
    fn close_panel(&self, cx: &mut App) {
        self.close_widget(cx);
    }
}

/// Backwards-compatible type alias.
pub use self::OutlineWidgetHost as OutlinePanelHost;

/// Renders the floating Outline widget overlay for a specific pane.
pub fn render_outline_widget(
    pane_id: PaneId,
    headings: &[OutlineNode],
    active_index: Option<usize>,
    host: &Arc<dyn OutlineWidgetHost>,
    theme: &Theme,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;

    // ── 1. Header row: Outline title + count + close button ─────────
    let host_close = host.clone();
    let close_btn = div()
        .id("outline-panel-close-btn")
        .size(px(16.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(d.tab_close_button_radius))
        .cursor_pointer()
        .hover(|this| this.bg(c.panel_row_hover))
        .child(
            svg()
                .path("plugin://splitype.editor/topbar/close.svg")
                .size(px(8.5))
                .text_color(c.dialog_muted),
        )
        .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
            host_close.close_panel(cx);
        });

    let header_row = div()
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    svg()
                        .path("plugin://splitype.editor/outline/outline.svg")
                        .size(px(13.5))
                        .text_color(c.focus_accent),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c.dialog_title)
                        .child("Outline"),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(c.dialog_muted)
                        .child(format!("({})", headings.len())),
                ),
        )
        .child(close_btn);

    // ── 2. Content list ──────────────────────────────────────────────
    let content_list = if headings.is_empty() {
        div()
            .py(px(20.0))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(c.dialog_muted)
                    .child("No headings found"),
            )
            .into_any_element()
    } else {
        let active_idx = active_index.unwrap_or(0);
        let mut items = Vec::with_capacity(headings.len());
        for (idx, node) in headings.iter().enumerate() {
            let is_active = idx == active_idx;
            let indent = match node.level {
                1 => 6.0,
                2 => 14.0,
                3 => 22.0,
                4 => 30.0,
                5 => 38.0,
                _ => 44.0,
            };
            let label = node.label.clone();
            let host_nav = host.clone();

            items.push(
                div()
                    .id(ElementId::Name(
                        format!("outline-popover-item-{}-{idx}", pane_id.as_usize()).into(),
                    ))
                    .w_full()
                    .pl(px(indent))
                    .pr(px(8.0))
                    .py(px(4.0))
                    .rounded(px(d.outline_node_radius))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .when(is_active, |this| this.bg(c.panel_row_hover))
                    .hover(|style| style.bg(c.panel_row_hover))
                    .children(node.kind.badge_label().map(|tag| {
                        div()
                            .flex_shrink_0()
                            .px(px(3.5))
                            .py(px(0.5))
                            .rounded(px(2.0))
                            .bg(c.panel_row_hover)
                            .border(px(1.0))
                            .border_color(c.dialog_border)
                            .text_size(px(9.5))
                            .text_color(c.focus_accent)
                            .child(tag)
                    }))
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
                                c.dialog_title
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
                        host_nav.navigate_to(idx, cx);
                    }),
            );
        }

        div()
            .flex()
            .flex_col()
            .gap(px(1.0))
            .children(items)
            .into_any_element()
    };

    // ── 3. Main Popover Container (top: 28px, right: 8px) ───────────
    let panel_top = 28.0;

    deferred(
        div()
            .id(ElementId::Name(
                format!("editor-outline-popover-{}", pane_id.as_usize()).into(),
            ))
            .occlude()
            .absolute()
            .top(px(panel_top))
            .right(px(8.0))
            .w(px(300.0))
            .max_w(relative(0.96))
            .max_h(px(420.0))
            .rounded(px(d.button_radius))
            .bg(c.dialog_surface)
            .border(px(1.0))
            .border_color(c.dialog_border)
            .shadow_lg()
            .p(px(10.0))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                cx.stop_propagation();
            })
            .child(header_row)
            .child(div().h(px(1.0)).bg(c.dialog_border))
            .child(
                div()
                    .id("outline-panel-content-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .child(content_list),
            )
            .into_any_element(),
    )
    .into_any_element()
}

#[inline]
pub fn render_outline_panel_overlay(
    pane_id: PaneId,
    headings: &[OutlineNode],
    active_index: Option<usize>,
    host: &Arc<dyn OutlineWidgetHost>,
    theme: &Theme,
) -> AnyElement {
    render_outline_widget(pane_id, headings, active_index, host, theme)
}
