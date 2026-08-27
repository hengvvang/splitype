use gpui::prelude::FluentBuilder;
use gpui::*;

use crate::editor::engine::controller::Editor;
use crate::editor::search::state::{SearchActiveField, SearchScope};
use crate::infra::theme::Theme;
use crate::ui::button::small_pill_button;
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

        let match_count_label = self.search.match_status_label();
        let show_replace = self.search.show_replace;
        let scope = self.search.scope;
        let results_expanded = self.search.results_expanded;
        let match_case = self.search.match_case;
        let whole_word = self.search.whole_word;
        let use_regex = self.search.use_regex;

        // ── 1. Search Query Input Box ────────────────────────────────────
        let search_focus = self.search.search_focus_handle.clone();
        let query_text = self.search.search_input.text.clone();

        let query_box_editor = editor.clone();
        let query_box = div()
            .id("editor-search-query-input-box")
            .key_context("SearchQueryInput")
            .track_focus(&search_focus)
            .flex_1()
            .h(px(26.0))
            .px(px(6.0))
            .flex()
            .items_center()
            .bg(c.dialog_surface)
            .border_1()
            .border_color(if self.search.active_field == SearchActiveField::Query {
                c.app_menu_active
            } else {
                c.dialog_border
            })
            .rounded(px(4.0))
            .cursor_text()
            .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                let _ = query_box_editor.update(cx, |ed, cx| {
                    ed.search.active_field = SearchActiveField::Query;
                    window.focus(&ed.search.search_focus_handle, cx);
                    cx.notify();
                });
            })
            .on_key_down(cx.listener(Self::handle_search_key_down))
            .child(
                div()
                    .flex()
                    .items_center()
                    .text_size(px(12.0))
                    .text_color(if query_text.is_empty() {
                        c.dialog_muted
                    } else {
                        c.text_default
                    })
                    .child(if query_text.is_empty() {
                        "Find...".to_string()
                    } else {
                        query_text.clone()
                    })
                    .when(
                        !query_text.is_empty()
                            && self.search.active_field == SearchActiveField::Query,
                        |this| {
                            this.child(
                                div()
                                    .w(px(1.5))
                                    .h(px(13.0))
                                    .bg(c.cursor)
                                    .ml(px(1.0)),
                            )
                        },
                    ),
            );

        // Filter toggles (Aa, \b, .*)
        let case_editor = editor.clone();
        let case_toggle = div()
            .px(px(4.0))
            .py(px(2.0))
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
            .text_size(px(10.0))
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
            .px(px(4.0))
            .py(px(2.0))
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
            .text_size(px(10.0))
            .cursor_pointer()
            .hover(|this| this.bg(c.panel_row_hover))
            .child("\\b")
            .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                let _ = word_editor.update(cx, |ed, cx| {
                    ed.search.whole_word = !ed.search.whole_word;
                    ed.execute_search(cx);
                });
            });

        let regex_editor = editor.clone();
        let regex_toggle = div()
            .px(px(4.0))
            .py(px(2.0))
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
            .text_size(px(10.0))
            .cursor_pointer()
            .hover(|this| this.bg(c.panel_row_hover))
            .child(".*")
            .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                let _ = regex_editor.update(cx, |ed, cx| {
                    ed.search.use_regex = !ed.search.use_regex;
                    ed.execute_search(cx);
                });
            });

        let prev_editor = editor.clone();
        let prev_btn = div()
            .size(px(18.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(4.0))
            .hover(|this| this.bg(c.dialog_secondary_button_hover))
            .cursor_pointer()
            .text_color(c.dialog_muted)
            .text_size(px(10.0))
            .child("▲")
            .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                let _ = prev_editor.update(cx, |ed, cx| {
                    ed.search.prev_match();
                    ed.jump_to_active_search_match(window, cx);
                    cx.notify();
                });
            });

        let next_editor = editor.clone();
        let next_btn = div()
            .size(px(18.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(4.0))
            .hover(|this| this.bg(c.dialog_secondary_button_hover))
            .cursor_pointer()
            .text_color(c.dialog_muted)
            .text_size(px(10.0))
            .child("▼")
            .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                let _ = next_editor.update(cx, |ed, cx| {
                    ed.search.next_match();
                    ed.jump_to_active_search_match(window, cx);
                    cx.notify();
                });
            });

        let replace_toggle_editor = editor.clone();
        let replace_toggle_btn = div()
            .size(px(18.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(4.0))
            .bg(if show_replace {
                c.panel_row_selected
            } else {
                hsla(0.0, 0.0, 0.0, 0.0)
            })
            .text_color(if show_replace {
                c.app_menu_active
            } else {
                c.dialog_muted
            })
            .hover(|this| this.bg(c.dialog_secondary_button_hover))
            .cursor_pointer()
            .child(
                svg()
                    .path("icons/editor/topbar/replace.svg")
                    .size(px(11.0))
                    .text_color(if show_replace {
                        c.app_menu_active
                    } else {
                        c.dialog_muted
                    }),
            )
            .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                let _ = replace_toggle_editor.update(cx, |ed, cx| {
                    ed.search.show_replace = !ed.search.show_replace;
                    cx.notify();
                });
            });

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

        let close_editor = editor.clone();
        let close_btn = div()
            .size(px(18.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(4.0))
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
                    cx.notify();
                });
            });

        let search_row = div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .child(
                svg()
                    .path("icons/editor/topbar/search.svg")
                    .size(px(14.0))
                    .text_color(c.dialog_muted),
            )
            .child(query_box)
            .child(case_toggle)
            .child(word_toggle)
            .child(regex_toggle)
            .child(count_badge)
            .child(prev_btn)
            .child(next_btn)
            .child(replace_toggle_btn)
            .child(close_btn);

        // ── 2. Replace Row (Optional) ────────────────────────────────────
        let replace_row = if show_replace {
            let replace_focus = self.search.replace_focus_handle.clone();
            let replace_text = self.search.replace_input.text.clone();

            let replace_box_editor = editor.clone();
            let replace_box = div()
                .id("editor-search-replace-input-box")
                .key_context("SearchReplaceInput")
                .track_focus(&replace_focus)
                .flex_1()
                .h(px(26.0))
                .px(px(6.0))
                .flex()
                .items_center()
                .bg(c.dialog_surface)
                .border_1()
                .border_color(if self.search.active_field == SearchActiveField::Replace {
                    c.app_menu_active
                } else {
                    c.dialog_border
                })
                .rounded(px(4.0))
                .cursor_text()
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
                        .flex()
                        .items_center()
                        .text_size(px(12.0))
                        .text_color(if replace_text.is_empty() {
                            c.dialog_muted
                        } else {
                            c.text_default
                        })
                        .child(if replace_text.is_empty() {
                            "Replace with...".to_string()
                        } else {
                            replace_text.clone()
                        })
                        .when(
                            !replace_text.is_empty()
                                && self.search.active_field == SearchActiveField::Replace,
                            |this| {
                                this.child(
                                    div()
                                        .w(px(1.5))
                                        .h(px(13.0))
                                        .bg(c.cursor)
                                        .ml(px(1.0)),
                                )
                            },
                        ),
                );

            let replace_single_editor = editor.clone();
            let replace_single_btn = small_pill_button(c, d)
                .px(px(6.0))
                .text_size(px(11.0))
                .child("Replace")
                .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                    let _ = replace_single_editor.update(cx, |ed, cx| {
                        ed.replace_current_search_match(window, cx);
                    });
                });

            let replace_all_editor = editor.clone();
            let replace_all_btn = small_pill_button(c, d)
                .px(px(6.0))
                .text_size(px(11.0))
                .child("Replace All")
                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                    let _ = replace_all_editor.update(cx, |ed, cx| {
                        ed.replace_all_search_matches(cx);
                    });
                });

            Some(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        svg()
                            .path("icons/editor/topbar/replace.svg")
                            .size(px(14.0))
                            .text_color(c.dialog_muted),
                    )
                    .child(replace_box)
                    .child(replace_single_btn)
                    .child(replace_all_btn),
            )
        } else {
            None
        };

        // ── 3. Scope Switcher Row ────────────────────────────────────────
        let scope_curr_editor = editor.clone();
        let scope_worktree_editor = editor.clone();

        let scope_row = div()
            .flex()
            .items_center()
            .justify_between()
            .text_size(px(11.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        div()
                            .px(px(6.0))
                            .py(px(2.0))
                            .rounded(px(3.0))
                            .bg(if scope == SearchScope::CurrentTab {
                                c.panel_row_selected
                            } else {
                                hsla(0.0, 0.0, 0.0, 0.0)
                            })
                            .text_color(if scope == SearchScope::CurrentTab {
                                c.text_default
                            } else {
                                c.dialog_muted
                            })
                            .cursor_pointer()
                            .child("Current File")
                            .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                                let _ = scope_curr_editor.update(cx, |ed, cx| {
                                    ed.search.scope = SearchScope::CurrentTab;
                                    ed.execute_search(cx);
                                });
                            }),
                    )
                    .child(
                        div()
                            .px(px(6.0))
                            .py(px(2.0))
                            .rounded(px(3.0))
                            .bg(if scope == SearchScope::Worktree {
                                c.panel_row_selected
                            } else {
                                hsla(0.0, 0.0, 0.0, 0.0)
                            })
                            .text_color(if scope == SearchScope::Worktree {
                                c.text_default
                            } else {
                                c.dialog_muted
                            })
                            .cursor_pointer()
                            .child("Workspace")
                            .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                                let _ = scope_worktree_editor.update(cx, |ed, cx| {
                                    ed.search.scope = SearchScope::Worktree;
                                    ed.execute_search(cx);
                                });
                            }),
                    ),
            );

        // ── 4. Expandable Match Results List ─────────────────────────────
        let results_drawer = if results_expanded && !self.search.matches.is_empty() {
            let active_idx = self.search.active_match_index;
            let mut match_elements = Vec::new();

            for (idx, m) in self.search.matches.iter().enumerate() {
                let is_active = Some(idx) == active_idx;
                let item_editor = editor.clone();

                match_elements.push(
                    div()
                        .px(px(6.0))
                        .py(px(3.0))
                        .rounded(px(3.0))
                        .bg(if is_active {
                            c.panel_row_selected
                        } else {
                            hsla(0.0, 0.0, 0.0, 0.0)
                        })
                        .hover(|this| this.bg(c.panel_row_hover))
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(c.dialog_muted)
                                .child(format!("{}:{}", m.file_name, m.line_number)),
                        )
                        .child(
                            div()
                                .flex_1()
                                .text_size(px(11.0))
                                .text_color(c.text_default)
                                .flex()
                                .items_center()
                                .child(m.preview_prefix.clone())
                                .child(
                                    div()
                                        .bg(c.app_menu_active.opacity(0.3))
                                        .text_color(c.app_menu_active)
                                        .rounded(px(2.0))
                                        .px(px(2.0))
                                        .child(m.preview_match.clone()),
                                )
                                .child(m.preview_suffix.clone()),
                        )
                        .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                            let _ = item_editor.update(cx, |ed, cx| {
                                ed.search.active_match_index = Some(idx);
                                ed.jump_to_active_search_match(window, cx);
                                cx.notify();
                            });
                        })
                        .into_any_element(),
                );
            }

            Some(
                div()
                    .id("search-results-drawer-scroll-container")
                    .w_full()
                    .max_h(px(200.0))
                    .overflow_y_scroll()
                    .border_t_1()
                    .border_color(c.dialog_border)
                    .pt(px(4.0))
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(c.dialog_muted)
                            .child(format!("MATCHES ({})", self.search.matches.len())),
                    )
                    .children(match_elements),
            )
        } else {
            None
        };

        // ── Assemble floating card ───────────────────────────────────────
        let panel_top = d.topbar_height + 4.0;
        let mut card = div()
            .id("editor-search-panel-floating-card")
            .absolute()
            .top(px(panel_top))
            .right(px(12.0))
            .w(px(420.0))
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
            .child(search_row);

        if let Some(replace) = replace_row {
            card = card.child(replace);
        }

        card = card.child(scope_row);

        if let Some(drawer) = results_drawer {
            card = card.child(drawer);
        }

        Some(
            deferred(
                overlay()
                    .id("editor-search-overlay")
                    .occlude()
                    .child(card),
            )
            .into_any_element(),
        )
    }
}
