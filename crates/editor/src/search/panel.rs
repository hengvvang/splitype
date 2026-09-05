//! Search and replace floating overlay panel UI (VS Code / Zed inspired layout with separated results card).
//!
//! Pure presentation over [`editor_contracts::SearchPanelState`]; coordination
//! actions re-enter the editor through [`editor_contracts::SearchHost`].

use std::sync::Arc;

use gpui::prelude::FluentBuilder;
use gpui::*;

use crate::search::input::SearchInputElement;
use editor_contracts::{
    SearchActiveField, SearchHost, SearchIme, SearchPanelState, SearchScope, SearchStateView,
};
use theme::{Theme, TypographyScope, TypographyStore};

/// Renders the floating Search and Replace overlay panel in the top-right
/// corner, or `None` when the panel is hidden.
pub fn render_search_panel_overlay(
    state: &SearchPanelState,
    view: &Arc<dyn SearchStateView>,
    ime: &Arc<dyn SearchIme>,
    host: &Arc<dyn SearchHost>,
    theme: &Theme,
    _window: &mut Window,
    _cx: &mut App,
) -> Option<AnyElement> {
    if !state.visible {
        return None;
    }

    let c = &theme.colors;
    let d = &theme.dimensions;

    let show_replace = state.show_replace;
    let scope = state.scope;
    let results_expanded = state.results_expanded;
    let match_case = state.match_case;
    let whole_word = state.whole_word;
    let use_regex = state.use_regex;
    let preserve_case = state.preserve_case;

    // ── Expand/Collapse Chevron (Left Column) ───────────────────────
    let chevron_editor = host.clone();
    let chevron_icon = if show_replace {
        "icons/explorer/worktree/chevron-down.svg"
    } else {
        "icons/explorer/worktree/chevron-right.svg"
    };
    let chevron_btn = div()
        .id("search-replace-expand-toggle")
        .size(px(20.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(d.icon_button_radius))
        .cursor_pointer()
        .hover(|this| this.bg(c.panel_row_hover))
        .child(
            svg()
                .path(chevron_icon)
                .size(px(11.0))
                .text_color(c.dialog_muted),
        )
        .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
            chevron_editor.toggle_show_replace(cx);
        });

    // ── Search Input Inline Filter Buttons ──────────────────────────
    let case_editor = host.clone();
    let case_toggle = div()
        .id("search-filter-case")
        .px(px(4.0))
        .py(px(1.0))
        .rounded(px(d.icon_button_radius))
        .when(match_case, |this| this.bg(c.panel_row_hover))
        .text_color(if match_case {
            c.focus_accent
        } else {
            c.dialog_muted
        })
        .text_size(px(11.0))
        .cursor_pointer()
        .hover(|this| this.bg(c.panel_row_hover))
        .child("Aa")
        .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
            case_editor.toggle_match_case(cx);
        });

    let word_editor = host.clone();
    let word_toggle = div()
        .id("search-filter-word")
        .px(px(4.0))
        .py(px(1.0))
        .rounded(px(d.icon_button_radius))
        .when(whole_word, |this| this.bg(c.panel_row_hover))
        .text_color(if whole_word {
            c.focus_accent
        } else {
            c.dialog_muted
        })
        .text_size(px(11.0))
        .cursor_pointer()
        .hover(|this| this.bg(c.panel_row_hover))
        .child(div().flex().flex_col().items_center().child("ab").child(
            div().w(px(12.0)).h(px(1.0)).bg(if whole_word {
                c.focus_accent
            } else {
                c.dialog_muted
            }),
        ))
        .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
            word_editor.toggle_whole_word(cx);
        });

    let regex_editor = host.clone();
    let regex_toggle = div()
        .id("search-filter-regex")
        .px(px(4.0))
        .py(px(1.0))
        .rounded(px(d.icon_button_radius))
        .when(use_regex, |this| this.bg(c.panel_row_hover))
        .text_color(if use_regex {
            c.focus_accent
        } else {
            c.dialog_muted
        })
        .text_size(px(11.0))
        .cursor_pointer()
        .hover(|this| this.bg(c.panel_row_hover))
        .child(".*")
        .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
            regex_editor.toggle_use_regex(cx);
        });

    // ── Search Input Box Container ──────────────────────────────────
    let search_focus = state.search_focus_handle.clone();
    let search_box_editor = host.clone();
    let is_query_active = state.active_field == SearchActiveField::Query;

    let search_bottom_indicator = div()
        .absolute()
        .bottom_0()
        .left_0()
        .right_0()
        .h(px(2.0))
        .rounded_b(px(d.select_trigger_radius))
        .bg(if is_query_active {
            c.focus_accent
        } else {
            c.dialog_border
        });

    let search_input_box = div()
        .id("editor-search-input-box")
        .key_context("SearchQueryInput")
        .track_focus(&search_focus)
        .relative()
        .overflow_hidden()
        .flex_1()
        .h(px(32.0))
        .px(px(8.0))
        .flex()
        .items_center()
        .gap(px(4.0))
        .bg(c.dialog_surface)
        .border_1()
        .border_color(c.dialog_border)
        .rounded(px(d.select_trigger_radius))
        .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
            search_box_editor.focus_query(window, cx);
        })
        .on_key_down({
            let host_key_down = host.clone();
            move |event, window, cx| {
                host_key_down.handle_key_down(event, window, cx);
            }
        })
        .child(div().flex_1().min_w(px(0.0)).child(SearchInputElement {
            view: view.clone(),
            ime: ime.clone(),
            host: host.clone(),
            field: SearchActiveField::Query,
            placeholder: "Search".into(),
        }))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(2.0))
                .child(case_toggle)
                .child(word_toggle)
                .child(regex_toggle),
        )
        .child(search_bottom_indicator);

    // ── Search Right Actions (Prev, Next, Scope, Close) ────────────
    let prev_editor = host.clone();
    let prev_btn = div()
        .size(px(20.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(d.icon_button_radius))
        .hover(|this| this.bg(c.panel_row_hover))
        .cursor_pointer()
        .child(
            svg()
                .path("icons/editor/topbar/prev.svg")
                .size(px(11.0))
                .text_color(c.dialog_muted),
        )
        .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
            prev_editor.prev_match(window, cx);
        });

    let next_editor = host.clone();
    let next_btn = div()
        .size(px(20.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(d.icon_button_radius))
        .hover(|this| this.bg(c.panel_row_hover))
        .cursor_pointer()
        .child(
            svg()
                .path("icons/editor/topbar/next.svg")
                .size(px(11.0))
                .text_color(c.dialog_muted),
        )
        .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
            next_editor.next_match(window, cx);
        });

    let explorer_editor = host.clone();
    let explorer_search_btn = div()
        .id("search-scope-explorer-toggle")
        .size(px(20.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(d.icon_button_radius))
        .when(scope == SearchScope::Worktree, |this| {
            this.bg(c.panel_row_hover)
        })
        .hover(|this| this.bg(c.panel_row_hover))
        .cursor_pointer()
        .child(
            svg()
                .path("icons/editor/topbar/search-explorer.svg")
                .size(px(12.0))
                .text_color(if scope == SearchScope::Worktree {
                    c.focus_accent
                } else {
                    c.dialog_muted
                }),
        )
        .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
            explorer_editor.toggle_scope(cx);
        });

    // ── 1. Independent Search Strip Card ─────────────────────────────
    let search_top_row = div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .child(chevron_btn)
        .child(search_input_box)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(1.0))
                .child(prev_btn)
                .child(next_btn)
                .child(explorer_search_btn),
        );

    // ── Replace Row (When Expanded) ──────────────────────────────────
    let replace_row = if show_replace {
        let replace_focus = state.replace_focus_handle.clone();
        let replace_box_editor = host.clone();
        let is_replace_active = state.active_field == SearchActiveField::Replace;

        let replace_bottom_indicator = div()
            .absolute()
            .bottom_0()
            .left_0()
            .right_0()
            .h(px(2.0))
            .rounded_b(px(d.select_trigger_radius))
            .bg(if is_replace_active {
                c.focus_accent
            } else {
                c.dialog_border
            });

        let preserve_editor = host.clone();
        let preserve_toggle = div()
            .id("replace-filter-preserve-case")
            .px(px(4.0))
            .py(px(1.0))
            .rounded(px(d.icon_button_radius))
            .when(preserve_case, |this| this.bg(c.panel_row_hover))
            .text_color(if preserve_case {
                c.focus_accent
            } else {
                c.dialog_muted
            })
            .text_size(px(11.0))
            .cursor_pointer()
            .hover(|this| this.bg(c.panel_row_hover))
            .child("AB")
            .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                preserve_editor.toggle_preserve_case(cx);
            });

        let replace_input_box = div()
            .id("editor-replace-input-box")
            .key_context("SearchReplaceInput")
            .track_focus(&replace_focus)
            .relative()
            .overflow_hidden()
            .flex_1()
            .h(px(32.0))
            .px(px(8.0))
            .flex()
            .items_center()
            .gap(px(4.0))
            .bg(c.dialog_surface)
            .border_1()
            .border_color(c.dialog_border)
            .rounded(px(d.select_trigger_radius))
            .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                replace_box_editor.focus_replace(window, cx);
            })
            .on_key_down({
                let host_key_down = host.clone();
                move |event, window, cx| {
                    host_key_down.handle_key_down(event, window, cx);
                }
            })
            .child(div().flex_1().min_w(px(0.0)).child(SearchInputElement {
                view: view.clone(),
                ime: ime.clone(),
                host: host.clone(),
                field: SearchActiveField::Replace,
                placeholder: "Replace".into(),
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.0))
                    .child(preserve_toggle),
            )
            .child(replace_bottom_indicator);

        let replace_single_editor = host.clone();
        let replace_single_btn = div()
            .id("search-replace-single-btn")
            .size(px(20.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(d.icon_button_radius))
            .hover(|this| this.bg(c.panel_row_hover))
            .cursor_pointer()
            .child(
                svg()
                    .path("icons/editor/topbar/replace.svg")
                    .size(px(12.0))
                    .text_color(c.dialog_muted),
            )
            .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                replace_single_editor.replace_current(window, cx);
            });

        let replace_all_editor = host.clone();
        let replace_all_btn = div()
            .id("search-replace-all-btn")
            .size(px(20.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(d.icon_button_radius))
            .hover(|this| this.bg(c.panel_row_hover))
            .cursor_pointer()
            .child(
                svg()
                    .path("icons/splitter/swap.svg")
                    .size(px(12.0))
                    .text_color(c.dialog_muted),
            )
            .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                replace_all_editor.replace_all(cx);
            });

        Some(
            div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .child(
                    // Align with top row chevron width (20px)
                    div().w(px(20.0)).flex_shrink_0(),
                )
                .child(replace_input_box)
                .child(
                    div()
                            .flex()
                            .items_center()
                            .gap(px(1.0))
                            .child(replace_single_btn)
                            .child(replace_all_btn)
                            // Spacer to align with the 3 right buttons on search card (21px)
                            .child(div().w(px(21.0)).flex_shrink_0()),
                ),
        )
    } else {
        None
    };

    // ── Top Search Controls Card (Integrated Search & Replace) ─────────
    let mut top_card = div()
        .id("editor-search-panel-floating-card")
        .w_full()
        .bg(c.dialog_surface)
        .border_1()
        .border_color(c.dialog_border)
        .rounded(px(d.menu_panel_radius))
        .shadow_lg()
        .p(px(6.0))
        .flex()
        .flex_col()
        .gap(px(6.0))
        .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
            cx.stop_propagation();
        })
        .child(search_top_row);

    if let Some(replace) = replace_row {
        top_card = top_card.child(replace);
    }

    // ── 3. Separated Match Results Floating Card (with gap) ──────────
    let results_card = if results_expanded {
        let active_idx = state.active_match_index;
        let mut match_elements = Vec::new();

        for (idx, m) in state.matches.iter().enumerate() {
            let is_active = Some(idx) == active_idx;
            let is_expanded = state.is_match_expanded(idx);
            let item_editor = host.clone();
            let toggle_editor = host.clone();

            let selected_indicator = if is_active {
                Some(
                    div()
                        .absolute()
                        .left(px(2.0))
                        .top(px(4.0))
                        .bottom(px(4.0))
                        .w(px(3.0))
                        .rounded_full()
                        .bg(c.focus_accent),
                )
            } else {
                None
            };

            // ── Compact Single-line Header Row ───────────────────────
            let row_header = div()
                .h(px(24.0))
                .pl(px(9.0))
                .pr(px(6.0))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(6.0))
                .child(
                    // Left click-to-jump area
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex()
                        .items_center()
                        .gap(px(5.0))
                        .cursor_pointer()
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(c.dialog_muted)
                                .flex_shrink_0()
                                .child(format!("{}:{}", m.line_number, m.column_number)),
                        )
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(c.dialog_muted)
                                .flex_shrink_0()
                                .child("|"),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .text_size(px(11.0))
                                .text_color(c.text_default)
                                .flex()
                                .items_center()
                                .overflow_hidden()
                                .child(div().flex_shrink_0().child(m.preview_prefix.clone()))
                                .child(
                                    div()
                                        .text_color(c.focus_accent)
                                        .flex_shrink_0()
                                        .child(m.preview_match.clone()),
                                )
                                .child(div().flex_shrink_0().child(m.preview_suffix.clone())),
                        )
                        .on_mouse_down(MouseButton::Left, {
                            let item_editor = item_editor.clone();
                            move |_event, window, cx| {
                                item_editor.activate_match(idx, window, cx);
                            }
                        }),
                )
                .child(
                    // Right item expand/collapse details toggle
                    div()
                        .id(("search-match-item-expand-toggle", idx))
                        .size(px(18.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(d.icon_button_radius))
                        .cursor_pointer()
                        .hover(|this| this.bg(c.panel_row_hover))
                        .child(
                            svg()
                                .path(if is_expanded {
                                    "icons/explorer/worktree/chevron-down.svg"
                                } else {
                                    "icons/explorer/worktree/chevron-right.svg"
                                })
                                .size(px(9.0))
                                .text_color(c.dialog_muted),
                        )
                        .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                            toggle_editor.toggle_match_expanded(idx, cx);
                        }),
                );

            // ── Expanded Details Panel (Shown when chevron is toggled) ─
            let details_drawer = if is_expanded {
                let file_display = m
                    .file_path
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| m.file_name.clone());

                Some(
                    div()
                        .px(px(8.0))
                        .py(px(6.0))
                        .border_t_1()
                        .border_color(c.dialog_border)
                        .bg(c.dialog_surface)
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .text_size(px(10.0))
                        .text_color(c.dialog_muted)
                        .child(div().child(format!("Path: {}", file_display)))
                        .child(
                            div()
                                .p(px(6.0))
                                .rounded(px(d.code_bg_radius))
                                .bg(c.dialog_secondary_button_bg)
                                .font(TypographyStore::default_font(TypographyScope::Code))
                                .text_size(px(11.0))
                                .text_color(c.text_default)
                                .flex()
                                .items_center()
                                .child(m.preview_prefix.clone())
                                .child(
                                    div()
                                        .text_color(c.focus_accent)
                                        .child(m.preview_match.clone()),
                                )
                                .child(m.preview_suffix.clone()),
                        ),
                )
            } else {
                None
            };

            let mut item_card = div()
                .id(("search-match-item-card", idx))
                .relative()
                .rounded(px(d.menu_item_radius))
                .when(is_active, |this| this.bg(c.panel_row_hover))
                .hover(|this| this.bg(c.panel_row_hover))
                .children(selected_indicator)
                .child(row_header);

            if let Some(details) = details_drawer {
                item_card = item_card.child(details);
            }

            match_elements.push(item_card.into_any_element());
        }

        let collapse_drawer_editor = host.clone();

        let (header_left, results_body) = if state.matches.is_empty() {
            let msg = if state.query().is_empty() {
                "Type to search in document"
            } else {
                "No matches found"
            };
            let title = if state.query().is_empty() {
                "MATCHES (0)".to_string()
            } else {
                "NO RESULTS".to_string()
            };

            let header_left = div()
                .text_size(px(10.0))
                .text_color(c.dialog_muted)
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child(title);

            let empty_row = div()
                .w_full()
                .py(px(12.0))
                .px(px(8.0))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(11.0))
                .text_color(c.dialog_muted)
                .child(msg);

            (header_left, empty_row.into_any_element())
        } else {
            let active_num = active_idx.map(|i| i + 1).unwrap_or(1);
            let total_num = state.matches.len();

            let header_left = div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(c.dialog_muted)
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child(format!("MATCHES ({})", total_num)),
                )
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(c.focus_accent)
                        .child(format!("{}/{}", active_num, total_num)),
                );

            let list_view = div()
                .id("search-results-drawer-scroll-container")
                .w_full()
                .max_h(px(240.0))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .children(match_elements);

            (header_left, list_view.into_any_element())
        };

        Some(
            div()
                .id("editor-search-results-floating-card")
                .w_full()
                .bg(c.dialog_surface)
                .border_1()
                .border_color(c.dialog_border)
                .rounded(px(d.menu_panel_radius))
                .shadow_lg()
                .p(px(6.0))
                .flex()
                .flex_col()
                .gap(px(4.0))
                .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                    cx.stop_propagation();
                })
                .child(
                    div()
                        .px(px(4.0))
                        .py(px(2.0))
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(header_left)
                        .child(
                            div()
                                .size(px(16.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(d.tab_close_button_radius))
                                .cursor_pointer()
                                .hover(|this| this.bg(c.panel_row_hover))
                                .child(
                                    svg()
                                        .path("icons/editor/topbar/close.svg")
                                        .size(px(8.0))
                                        .text_color(c.dialog_muted),
                                )
                                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                                    collapse_drawer_editor.collapse_results(cx);
                                }),
                        ),
                )
                .child(results_body),
        )
    } else {
        None
    };

    // ── Root Floating Container (Positions top card and separated results card) ─
    let panel_top = d.topbar_height + 4.0;
    let mut container = div()
        .id("editor-search-overlay-container")
        .occlude()
        .absolute()
        .top(px(panel_top))
        .right(px(12.0))
        .w(px(420.0))
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(top_card);

    if let Some(results) = results_card {
        container = container.child(results);
    }

    Some(deferred(container.into_any_element()).into_any_element())
}
