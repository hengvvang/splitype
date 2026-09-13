//! Presentation for the floating Link Relationships widget.

use gpui::prelude::FluentBuilder;
use gpui::*;
use std::path::PathBuf;
use std::sync::Arc;
use theme::Theme;

use super::types::{BacklinkGroup, LinksWidgetState, LinksWidgetTab, OutgoingLinkItem};

/// Actions supported by the links widget host.
pub trait LinksWidgetHost: Send + Sync + 'static {
    fn switch_tab(&self, tab: LinksWidgetTab, cx: &mut App);
    fn update_filter(&self, query: String, cx: &mut App);
    fn close_widget(&self, cx: &mut App);
    fn navigate_to_backlink(&self, path: PathBuf, line: usize, anchor: Option<String>, cx: &mut App);
    fn navigate_to_outgoing(&self, target: String, cx: &mut App);

    /// Backwards-compatible alias for close_widget.
    fn close_panel(&self, cx: &mut App) {
        self.close_widget(cx);
    }
}

/// Backwards-compatible type alias for LinksWidgetHost.
pub use self::LinksWidgetHost as LinksPanelHost;

/// Renders the floating Link Relationships widget.
pub fn render_links_widget(
    state: &LinksWidgetState,
    host: &Arc<dyn LinksWidgetHost>,
    theme: &Theme,
    _window: &mut Window,
    _cx: &mut App,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;

    let report = state.report.as_ref();
    let backlinks_count = report.map(|r| r.backlinks.len()).unwrap_or(0);
    let outgoing_count = report.map(|r| r.outgoing.len()).unwrap_or(0);

    let active_tab = state.active_tab;
    let query_lower = state.search_query.trim().to_lowercase();

    // ── 1. Header: Segmented Tabs + Close Button ──────────────────────
    let host_tab_b = host.clone();
    let tab_backlinks_btn = div()
        .id("links-tab-backlinks")
        .px(px(10.0))
        .py(px(4.0))
        .rounded(px(d.button_radius))
        .cursor_pointer()
        .when(active_tab == LinksWidgetTab::Backlinks, |this| {
            this.bg(c.panel_row_hover)
                .text_color(c.focus_accent)
                .font_weight(FontWeight::SEMIBOLD)
        })
        .when(active_tab != LinksWidgetTab::Backlinks, |this| {
            this.text_color(c.dialog_muted)
                .hover(|s| s.bg(c.panel_row_hover))
        })
        .text_size(px(12.0))
        .flex()
        .items_center()
        .gap(px(4.0))
        .child(format!("Backlinks ({backlinks_count})"))
        .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
            host_tab_b.switch_tab(LinksWidgetTab::Backlinks, cx);
        });

    let host_tab_o = host.clone();
    let tab_outgoing_btn = div()
        .id("links-tab-outgoing")
        .px(px(10.0))
        .py(px(4.0))
        .rounded(px(d.button_radius))
        .cursor_pointer()
        .when(active_tab == LinksWidgetTab::Outgoing, |this| {
            this.bg(c.panel_row_hover)
                .text_color(c.focus_accent)
                .font_weight(FontWeight::SEMIBOLD)
        })
        .when(active_tab != LinksWidgetTab::Outgoing, |this| {
            this.text_color(c.dialog_muted)
                .hover(|s| s.bg(c.panel_row_hover))
        })
        .text_size(px(12.0))
        .flex()
        .items_center()
        .gap(px(4.0))
        .child(format!("Outgoing ({outgoing_count})"))
        .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
            host_tab_o.switch_tab(LinksWidgetTab::Outgoing, cx);
        });

    let host_close = host.clone();
    let close_btn = div()
        .id("links-panel-close")
        .size(px(20.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(d.icon_button_radius))
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
                .gap(px(4.0))
                .child(tab_backlinks_btn)
                .child(tab_outgoing_btn),
        )
        .child(close_btn);

    // ── 2. Filter input row ───────────────────────────────────────────
    let filter_input = div()
        .w_full()
        .h(px(26.0))
        .px(px(8.0))
        .rounded(px(d.button_radius))
        .bg(c.editor_background)
        .border(px(1.0))
        .border_color(c.dialog_border)
        .flex()
        .items_center()
        .child(
            div()
                .text_size(px(11.5))
                .text_color(if state.search_query.is_empty() {
                    c.dialog_muted
                } else {
                    c.dialog_title
                })
                .child(if state.search_query.is_empty() {
                    "Filter links...".to_string()
                } else {
                    state.search_query.clone()
                }),
        );

    // ── 3. Content list based on active tab ───────────────────────────
    let content_list = if state.is_loading {
        div()
            .py(px(24.0))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(12.0))
            .text_color(c.dialog_muted)
            .child("Scanning workspace links...")
            .into_any_element()
    } else {
        match active_tab {
            LinksWidgetTab::Backlinks => {
                let all_groups: Vec<&BacklinkGroup> = report
                    .map(|r| {
                        r.backlinks
                            .iter()
                            .filter(|g| {
                                if query_lower.is_empty() {
                                    true
                                } else {
                                    g.source_title.to_lowercase().contains(&query_lower)
                                        || g.mentions.iter().any(|m| {
                                            m.context_snippet.to_lowercase().contains(&query_lower)
                                        })
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                if all_groups.is_empty() {
                    div()
                        .py(px(24.0))
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap(px(4.0))
                        .child(
                            div()
                                .text_size(px(12.5))
                                .text_color(c.dialog_muted)
                                .child("No backlinks found"),
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(c.dialog_muted)
                                .child("No other documents reference this note"),
                        )
                        .into_any_element()
                } else {
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.0))
                        .children(all_groups.into_iter().map(|group| {
                            let source_title = group.source_title.clone();
                            let source_path = group.source_path.clone();

                            div()
                                .flex()
                                .flex_col()
                                .gap(px(3.0))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(4.0))
                                        .child(
                                            svg()
                                                .path("plugin://splitype.editor/links/links.svg")
                                                .size(px(11.0))
                                                .text_color(c.focus_accent),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.0))
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(c.dialog_title)
                                                .child(source_title),
                                        ),
                                )
                                .children(group.mentions.iter().map(|m| {
                                    let host_nav = host.clone();
                                    let path_clone = source_path.clone();
                                    let line_num = m.line_number;
                                    let anchor_clone = m.target_anchor.clone();
                                    let snippet = m.context_snippet.clone();

                                    div()
                                        .px(px(6.0))
                                        .py(px(4.0))
                                        .rounded(px(d.button_radius))
                                        .cursor_pointer()
                                        .hover(|s| s.bg(c.panel_row_hover))
                                        .flex()
                                        .flex_row()
                                        .items_start()
                                        .gap(px(6.0))
                                        .child(
                                            div()
                                                .px(px(4.0))
                                                .py(px(1.0))
                                                .rounded(px(3.0))
                                                .bg(c.focus_accent.opacity(0.12))
                                                .text_size(px(10.0))
                                                .text_color(c.focus_accent)
                                                .whitespace_nowrap()
                                                .child(format!("L{}", line_num)),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .text_size(px(11.5))
                                                .text_color(c.dialog_body)
                                                .line_height(relative(1.35))
                                                .child(snippet),
                                        )
                                        .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                                            host_nav.navigate_to_backlink(
                                                path_clone.clone(),
                                                line_num,
                                                anchor_clone.clone(),
                                                cx,
                                            );
                                        })
                                }))
                        }))
                        .into_any_element()
                }
            }
            LinksWidgetTab::Outgoing => {
                let all_links: Vec<&OutgoingLinkItem> = report
                    .map(|r| {
                        r.outgoing
                            .iter()
                            .filter(|item| {
                                if query_lower.is_empty() {
                                    true
                                } else {
                                    item.display_text.to_lowercase().contains(&query_lower)
                                        || item.target.to_lowercase().contains(&query_lower)
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                if all_links.is_empty() {
                    div()
                        .py(px(24.0))
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap(px(4.0))
                        .child(
                            div()
                                .text_size(px(12.5))
                                .text_color(c.dialog_muted)
                                .child("No outgoing links"),
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(c.dialog_muted)
                                .child("This note does not reference any other notes or URLs"),
                        )
                        .into_any_element()
                } else {
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .children(all_links.into_iter().map(|item| {
                            let host_out = host.clone();
                            let target_str = item.target.clone();
                            let is_ext = item.is_external;
                            let display = item.display_text.clone();
                            let anchor_text = item.anchor.clone();

                            div()
                                .px(px(6.0))
                                .py(px(4.0))
                                .rounded(px(d.button_radius))
                                .cursor_pointer()
                                .hover(|s| s.bg(c.panel_row_hover))
                                .flex()
                                .flex_row()
                                .items_center()
                                .justify_between()
                                .gap(px(6.0))
                                .child(
                                    div()
                                        .flex()
                                        .flex_row()
                                        .items_center()
                                        .gap(px(6.0))
                                        .child(
                                            svg()
                                                .path("plugin://splitype.editor/links/links.svg")
                                                .size(px(11.0))
                                                .text_color(if is_ext {
                                                    c.dialog_muted
                                                } else {
                                                    c.focus_accent
                                                }),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.0))
                                                .text_color(c.dialog_title)
                                                .child(display),
                                        )
                                        .children(anchor_text.map(|anc| {
                                            div()
                                                .px(px(4.0))
                                                .py(px(1.0))
                                                .rounded(px(3.0))
                                                .bg(c.focus_accent.opacity(0.12))
                                                .text_size(px(10.0))
                                                .text_color(c.focus_accent)
                                                .child(format!("#{anc}"))
                                        })),
                                )
                                .child(
                                    div()
                                        .text_size(px(10.5))
                                        .text_color(c.dialog_muted)
                                        .child(if is_ext { "External" } else { "Internal" }),
                                )
                                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                                    host_out.navigate_to_outgoing(target_str.clone(), cx);
                                })
                        }))
                        .into_any_element()
                }
            }
        }
    };

    // ── 4. Main Popover Container (Positioned below pane breadcrumb) ─
    let panel_top = 28.0;

    deferred(
        div()
            .id("editor-links-popover")
            .occlude()
            .absolute()
            .top(px(panel_top))
            .right(px(8.0))
            .w(px(380.0))
            .max_w(relative(0.96))
            .max_h(px(440.0))
            .rounded(px(d.button_radius))
            .bg(c.dialog_surface)
            .border(px(1.0))
            .border_color(c.dialog_border)
            .shadow_lg()
            .p(px(10.0))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(header_row)
            .child(filter_input)
            .child(div().h(px(1.0)).bg(c.dialog_border))
            .child(
                div()
                    .id("links-panel-content-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .child(content_list),
            )
            .into_any_element(),
    )
    .into_any_element()
}

#[inline]
pub fn render_links_panel_overlay(
    state: &LinksWidgetState,
    host: &Arc<dyn LinksWidgetHost>,
    theme: &Theme,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    render_links_widget(state, host, theme, window, cx)
}
