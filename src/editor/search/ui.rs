//! Search and replace floating overlay panel UI (VS Code / Zed inspired layout with separated results card).

use gpui::*;

use crate::editor::engine::controller::Editor;
use crate::editor::panes::wysiwyg::render::layout::editor_text_font;
use crate::editor::search::input_element::SearchInputElement;
use crate::editor::search::state::{SearchActiveField, SearchScope};
use crate::infra::theme::Theme;
use crate::ui::popover::overlay;

impl Editor {
    /// Renders the floating Search and Replace overlay panel in the top-right corner.
    pub(crate) fn render_search_panel_overlay(
        &mut self,
        theme: &Theme,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if !self.search.visible {
            return None;
        }

        let c = &theme.colors;
        let d = &theme.dimensions;
        let editor = cx.entity().downgrade();

        let show_replace = self.search.show_replace;
        let match_count_label = self.search.match_status_label();
        let scope = self.search.scope;
        let results_expanded = self.search.results_expanded;
        let match_case = self.search.match_case;
        let whole_word = self.search.whole_word;
        let use_regex = self.search.use_regex;
        let preserve_case = self.search.preserve_case;

        // ── Expand/Collapse Chevron (Left Column) ───────────────────────
        let chevron_editor = editor.clone();
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
            .rounded(px(3.0))
            .cursor_pointer()
            .hover(|this| this.bg(c.dialog_secondary_button_hover))
            .child(
                svg()
                    .path(chevron_icon)
                    .size(px(11.0))
                    .text_color(c.dialog_muted),
            )
            .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                let _ = chevron_editor.update(cx, |ed, cx| {
                    ed.search.show_replace = !ed.search.show_replace;
                    cx.notify();
                });
            });

        // ── Search Input Inline Filter Buttons ──────────────────────────
        let case_editor = editor.clone();
        let case_toggle = div()
            .id("search-filter-case")
            .px(px(4.0))
            .py(px(1.0))
            .rounded(px(3.0))
            .bg(if match_case {
                c.panel_row_selected
            } else {
                hsla(0.0, 0.0, 0.0, 0.0)
            })
            .text_color(if match_case {
                c.app_menu_active
            } else {
                c.dialog_muted
            })
            .text_size(px(11.0))
            .cursor_pointer()
            .hover(|this| this.bg(c.panel_row_hover))
            .child("Aa")
            .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                let _ = case_editor.update(cx, |ed, cx| {
                    ed.search.match_case = !ed.search.match_case;
                    ed.execute_search(cx);
                });
            });

        let word_editor = editor.clone();
        let word_toggle = div()
            .id("search-filter-word")
            .px(px(4.0))
            .py(px(1.0))
            .rounded(px(3.0))
            .bg(if whole_word {
                c.panel_row_selected
            } else {
                hsla(0.0, 0.0, 0.0, 0.0)
            })
            .text_color(if whole_word {
                c.app_menu_active
            } else {
                c.dialog_muted
            })
            .text_size(px(11.0))
            .cursor_pointer()
            .hover(|this| this.bg(c.panel_row_hover))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .child("ab")
                    .child(
                        div()
                            .w(px(12.0))
                            .h(px(1.0))
                            .bg(if whole_word {
                                c.app_menu_active
                            } else {
                                c.dialog_muted
                            }),
                    ),
            )
            .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                let _ = word_editor.update(cx, |ed, cx| {
                    ed.search.whole_word = !ed.search.whole_word;
                    ed.execute_search(cx);
                });
            });

        let regex_editor = editor.clone();
        let regex_toggle = div()
            .id("search-filter-regex")
            .px(px(4.0))
            .py(px(1.0))
            .rounded(px(3.0))
            .bg(if use_regex {
                c.panel_row_selected
            } else {
                hsla(0.0, 0.0, 0.0, 0.0)
            })
            .text_color(if use_regex {
                c.app_menu_active
            } else {
                c.dialog_muted
            })
            .text_size(px(11.0))
            .cursor_pointer()
            .hover(|this| this.bg(c.panel_row_hover))
            .child(".*")
            .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                let _ = regex_editor.update(cx, |ed, cx| {
                    ed.search.use_regex = !ed.search.use_regex;
                    ed.execute_search(cx);
                });
            });

        // ── Search Input Box Container ──────────────────────────────────
        let search_focus = self.search.search_focus_handle.clone();
        let search_box_editor = editor.clone();

        let search_input_box = div()
            .id("editor-search-input-box")
            .key_context("SearchQueryInput")
            .track_focus(&search_focus)
            .flex_1()
            .h(px(32.0))
            .px(px(8.0))
            .flex()
            .items_center()
            .gap(px(4.0))
            .bg(c.dialog_surface)
            .border_1()
            .border_color(if self.search.active_field == SearchActiveField::Query {
                c.app_menu_active
            } else {
                c.dialog_border
            })
            .rounded(px(4.0))
            .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                let _ = search_box_editor.update(cx, |ed, cx| {
                    ed.search.active_field = SearchActiveField::Query;
                    window.focus(&ed.search.search_focus_handle, cx);
                    cx.notify();
                });
            })
            .on_key_down(cx.listener(Self::handle_search_key_down))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .child(SearchInputElement {
                        editor: cx.entity(),
                        field: SearchActiveField::Query,
                        placeholder: "Search".into(),
                    }),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.0))
                    .child(case_toggle)
                    .child(word_toggle)
                    .child(regex_toggle),
            );

        // ── Search Right Actions (Counter, Prev, Next, Close) ────────────
        let count_editor = editor.clone();
        let count_badge = div()
            .text_size(px(11.0))
            .text_color(c.dialog_muted)
            .cursor_pointer()
            .hover(|this| this.text_color(c.text_default))
            .child(match_count_label)
            .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                let _ = count_editor.update(cx, |ed, cx| {
                    ed.search.results_expanded = !ed.search.results_expanded;
                    cx.notify();
                });
            });

        let prev_editor = editor.clone();
        let prev_btn = div()
            .size(px(20.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(3.0))
            .hover(|this| this.bg(c.dialog_secondary_button_hover))
            .cursor_pointer()
            .child(
                svg()
                    .path("icons/editor/topbar/prev.svg")
                    .size(px(11.0))
                    .text_color(c.dialog_muted),
            )
            .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                let _ = prev_editor.update(cx, |ed, cx| {
                    ed.search.prev_match();
                    ed.jump_to_active_search_match(window, cx);
                    cx.notify();
                });
            });

        let next_editor = editor.clone();
        let next_btn = div()
            .size(px(20.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(3.0))
            .hover(|this| this.bg(c.dialog_secondary_button_hover))
            .cursor_pointer()
            .child(
                svg()
                    .path("icons/editor/topbar/next.svg")
                    .size(px(11.0))
                    .text_color(c.dialog_muted),
            )
            .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                let _ = next_editor.update(cx, |ed, cx| {
                    ed.search.next_match();
                    ed.jump_to_active_search_match(window, cx);
                    cx.notify();
                });
            });

        let explorer_editor = editor.clone();
        let explorer_search_btn = div()
            .id("search-scope-explorer-toggle")
            .size(px(20.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(3.0))
            .bg(if scope == SearchScope::Worktree {
                c.panel_row_selected
            } else {
                hsla(0.0, 0.0, 0.0, 0.0)
            })
            .hover(|this| this.bg(c.dialog_secondary_button_hover))
            .cursor_pointer()
            .child(
                svg()
                    .path("icons/editor/topbar/search-explorer.svg")
                    .size(px(12.0))
                    .text_color(if scope == SearchScope::Worktree {
                        c.app_menu_active
                    } else {
                        c.dialog_muted
                    }),
            )
            .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                let _ = explorer_editor.update(cx, |ed, cx| {
                    ed.search.scope = if ed.search.scope == SearchScope::Worktree {
                        SearchScope::CurrentTab
                    } else {
                        SearchScope::Worktree
                    };
                    ed.execute_search(cx);
                });
            });

        let close_editor = editor.clone();
        let close_btn = div()
            .size(px(20.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(3.0))
            .hover(|this| this.bg(c.dialog_secondary_button_hover))
            .cursor_pointer()
            .child(
                svg()
                    .path("icons/editor/topbar/close.svg")
                    .size(px(10.0))
                    .text_color(c.dialog_muted),
            )
            .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                let _ = close_editor.update(cx, |ed, cx| {
                    ed.search.visible = false;
                    ed.clear_search_highlights_from_document(cx);
                    cx.notify();
                });
            });

        let search_top_row = div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .child(chevron_btn)
            .child(search_input_box)
            .child(count_badge)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.0))
                    .child(prev_btn)
                    .child(next_btn)
                    .child(explorer_search_btn)
                    .child(close_btn),
            );

        // ── Replace Row (When Expanded) ──────────────────────────────────
        let replace_row = if show_replace {
            let replace_focus = self.search.replace_focus_handle.clone();
            let replace_box_editor = editor.clone();

            let preserve_editor = editor.clone();
            let preserve_toggle = div()
                .id("replace-filter-preserve-case")
                .px(px(4.0))
                .py(px(1.0))
                .rounded(px(3.0))
                .bg(if preserve_case {
                    c.panel_row_selected
                } else {
                    hsla(0.0, 0.0, 0.0, 0.0)
                })
                .text_color(if preserve_case {
                    c.app_menu_active
                } else {
                    c.dialog_muted
                })
                .text_size(px(11.0))
                .cursor_pointer()
                .hover(|this| this.bg(c.panel_row_hover))
                .child("AB")
                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                    let _ = preserve_editor.update(cx, |ed, cx| {
                        ed.search.preserve_case = !ed.search.preserve_case;
                        cx.notify();
                    });
                });

            let replace_input_box = div()
                .id("editor-replace-input-box")
                .key_context("SearchReplaceInput")
                .track_focus(&replace_focus)
                .flex_1()
                .h(px(32.0))
                .px(px(8.0))
                .flex()
                .items_center()
                .gap(px(4.0))
                .bg(c.dialog_surface)
                .border_1()
                .border_color(if self.search.active_field == SearchActiveField::Replace {
                    c.app_menu_active
                } else {
                    c.dialog_border
                })
                .rounded(px(4.0))
                .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                    let _ = replace_box_editor.update(cx, |ed, cx| {
                        ed.search.active_field = SearchActiveField::Replace;
                        window.focus(&ed.search.replace_focus_handle, cx);
                        cx.notify();
                    });
                })
                .on_key_down(cx.listener(Self::handle_search_key_down))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .child(SearchInputElement {
                            editor: cx.entity(),
                            field: SearchActiveField::Replace,
                            placeholder: "Replace".into(),
                        }),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(2.0))
                        .child(preserve_toggle),
                );

            let replace_single_editor = editor.clone();
            let replace_single_btn = div()
                .id("search-replace-single-btn")
                .size(px(20.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(3.0))
                .hover(|this| this.bg(c.dialog_secondary_button_hover))
                .cursor_pointer()
                .child(
                    svg()
                        .path("icons/editor/topbar/replace.svg")
                        .size(px(12.0))
                        .text_color(c.dialog_muted),
                )
                .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                    let _ = replace_single_editor.update(cx, |ed, cx| {
                        ed.replace_current_search_match(window, cx);
                    });
                });

            let replace_all_editor = editor.clone();
            let replace_all_btn = div()
                .id("search-replace-all-btn")
                .size(px(20.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(3.0))
                .hover(|this| this.bg(c.dialog_secondary_button_hover))
                .cursor_pointer()
                .child(
                    svg()
                        .path("icons/splitter/swap.svg")
                        .size(px(12.0))
                        .text_color(c.dialog_muted),
                )
                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                    let _ = replace_all_editor.update(cx, |ed, cx| {
                        ed.replace_all_search_matches(cx);
                    });
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
                            .gap(px(2.0))
                            .child(replace_single_btn)
                            .child(replace_all_btn)
                            // Align with search explorer + close width
                            .child(div().w(px(42.0)).flex_shrink_0()),
                    ),
            )
        } else {
            None
        };

        // ── Top Search Controls Card ─────────────────────────────────────
        let mut top_card = div()
            .id("editor-search-panel-floating-card")
            .w_full()
            .bg(c.dialog_surface)
            .border_1()
            .border_color(c.dialog_border)
            .rounded(px(d.menu_panel_radius))
            .shadow_lg()
            .p(px(8.0))
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

        // ── Separated Match Results Floating Card (with gap) ─────────────
        let results_card = if results_expanded && !self.search.matches.is_empty() {
            let active_idx = self.search.active_match_index;
            let mut match_elements = Vec::new();

            for (idx, m) in self.search.matches.iter().enumerate() {
                let is_active = Some(idx) == active_idx;
                let is_expanded = self.search.is_match_expanded(idx);
                let item_editor = editor.clone();
                let toggle_editor = editor.clone();

                // ── Compact Single-line Header Row ───────────────────────
                let row_header = div()
                    .h(px(24.0))
                    .px(px(6.0))
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
                                    .child(
                                        div()
                                            .flex_shrink_0()
                                            .child(m.preview_prefix.clone()),
                                    )
                                    .child(
                                        div()
                                            .bg(c.app_menu_active.opacity(0.35))
                                            .text_color(c.app_menu_active)
                                            .rounded(px(2.0))
                                            .px(px(2.0))
                                            .flex_shrink_0()
                                            .child(m.preview_match.clone()),
                                    )
                                    .child(
                                        div()
                                            .flex_shrink_0()
                                            .child(m.preview_suffix.clone()),
                                    ),
                            )
                            .on_mouse_down(MouseButton::Left, {
                                let item_editor = item_editor.clone();
                                move |_event, window, cx| {
                                    let _ = item_editor.update(cx, |ed, cx| {
                                        ed.search.active_match_index = Some(idx);
                                        ed.jump_to_active_search_match(window, cx);
                                        cx.notify();
                                    });
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
                            .rounded(px(3.0))
                            .cursor_pointer()
                            .hover(|this| this.bg(c.dialog_secondary_button_hover))
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
                                let _ = toggle_editor.update(cx, |ed, cx| {
                                    ed.search.toggle_match_expanded(idx);
                                    cx.notify();
                                });
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
                            .child(
                                div()
                                    .child(format!("Path: {}", file_display)),
                            )
                            .child(
                                div()
                                    .p(px(6.0))
                                    .rounded(px(3.0))
                                    .bg(c.dialog_secondary_button_bg)
                                    .font(editor_text_font())
                                    .text_size(px(11.0))
                                    .text_color(c.text_default)
                                    .flex()
                                    .items_center()
                                    .child(m.preview_prefix.clone())
                                    .child(
                                        div()
                                            .bg(c.app_menu_active.opacity(0.4))
                                            .text_color(c.app_menu_active)
                                            .rounded(px(2.0))
                                            .px(px(2.0))
                                            .child(m.preview_match.clone()),
                                    )
                                    .child(m.preview_suffix.clone()),
                            ),
                    )
                } else {
                    None
                };

                let mut item_card = div()
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(if is_active {
                        c.app_menu_active
                    } else {
                        hsla(0.0, 0.0, 0.0, 0.0)
                    })
                    .bg(if is_active {
                        c.panel_row_selected
                    } else {
                        hsla(0.0, 0.0, 0.0, 0.0)
                    })
                    .hover(|this| this.bg(c.panel_row_hover))
                    .child(row_header);

                if let Some(details) = details_drawer {
                    item_card = item_card.child(details);
                }

                match_elements.push(item_card.into_any_element());
            }

            let collapse_drawer_editor = editor.clone();
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
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(c.dialog_muted)
                                    .child(format!("MATCHES ({})", self.search.matches.len())),
                            )
                            .child(
                                div()
                                    .size(px(16.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(2.0))
                                    .cursor_pointer()
                                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                                    .child(
                                        svg()
                                            .path("icons/editor/topbar/close.svg")
                                            .size(px(8.0))
                                            .text_color(c.dialog_muted),
                                    )
                                    .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                                        let _ = collapse_drawer_editor.update(cx, |ed, cx| {
                                            ed.search.results_expanded = false;
                                            cx.notify();
                                        });
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .id("search-results-drawer-scroll-container")
                            .w_full()
                            .max_h(px(240.0))
                            .overflow_y_scroll()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .children(match_elements),
                    ),
            )
        } else {
            None
        };

        // ── Root Overlay Container (Positions top card and separated results card) ─
        let panel_top = d.topbar_height + 4.0;
        let mut container = div()
            .id("editor-search-overlay-container")
            .absolute()
            .top(px(panel_top))
            .right(px(12.0))
            .w(px(420.0))
            .flex()
            .flex_col()
            .gap(px(8.0)) // Clear 8px gap separating search input card and results card
            .child(top_card);

        if let Some(results) = results_card {
            container = container.child(results);
        }

        Some(
            deferred(
                overlay()
                    .id("editor-search-overlay")
                    .occlude()
                    .child(container),
            )
            .into_any_element(),
        )
    }
}
