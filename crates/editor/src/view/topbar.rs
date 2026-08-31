//! Top bar of an Editor area: area type selector, split/close controls and tab bar.

use gpui::prelude::FluentBuilder;
use gpui::*;
use splitter::SplitAxis;
use theme::Theme;
use ui::button::{icon_chip_button, small_pill_button, toolbar_button_size, toolbar_icon_size};
use ui::topbar::topbar_container;
use window::{PanelKind, panel_topbar_icon};

use crate::editor::Editor;

impl Editor {
    /// Top bar of an Editor area: type selector and split/close controls plus the Editor-specific tab bar.
    pub(crate) fn render_editor_topbar(
        &mut self,
        kind: PanelKind,
        theme: &Theme,
        leaf_count: usize,
        is_maximized: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let panel_id = self.panel_id;
        let editor = cx.entity().downgrade();

        let type_editor = editor.clone();
        let is_active_editor = self.is_active_panel;
        let type_button = small_pill_button(c, d)
            .id(("panel-topbar-type", panel_id.0))
            .text_size(px(12.0))
            .text_color(c.text_default)
            .child("Editor")
            .when(is_active_editor, |this| {
                this.child(
                    svg()
                        .path(panel_topbar_icon(kind, "active"))
                        .size(px(d.topbar_height * 0.5))
                        .text_color(c.app_menu_active),
                )
            })
            .on_click(move |_event, _window, cx| {
                let _ = type_editor.update(cx, |editor, cx| {
                    editor.defer_host_action(cx, move |host, cx| {
                        host.toggle_panel_dropdown(panel_id, cx);
                    });
                    cx.notify();
                });
            });

        let btn_icon_size = toolbar_icon_size(d.topbar_height);

        let split_h_editor = editor.clone();
        let split_h_button = icon_chip_button(c, d)
            .id(("panel-topbar-split-h", panel_id.0))
            .child(
                svg()
                    .path(panel_topbar_icon(kind, "split-h"))
                    .size(px(btn_icon_size))
                    .text_color(c.dialog_muted),
            )
            .on_click(move |_event, _window, cx| {
                let _ = split_h_editor.update(cx, |editor, cx| {
                    editor.defer_host_action(cx, move |host, cx| {
                        host.split_panel(panel_id, SplitAxis::Horizontal, 0.5, true, cx);
                    });
                    cx.notify();
                });
            });

        let split_v_editor = editor.clone();
        let split_v_button = icon_chip_button(c, d)
            .id(("panel-topbar-split-v", panel_id.0))
            .child(
                svg()
                    .path(panel_topbar_icon(kind, "split-v"))
                    .size(px(btn_icon_size))
                    .text_color(c.dialog_muted),
            )
            .on_click(move |_event, _window, cx| {
                let _ = split_v_editor.update(cx, |editor, cx| {
                    editor.defer_host_action(cx, move |host, cx| {
                        host.split_panel(panel_id, SplitAxis::Vertical, 0.5, true, cx);
                    });
                    cx.notify();
                });
            });

        let search_editor = editor.clone();
        let is_search_active = self.search.visible;
        let search_button = icon_chip_button(c, d)
            .id(("panel-topbar-search", panel_id.0))
            .bg(if is_search_active {
                c.panel_row_selected
            } else {
                hsla(0.0, 0.0, 0.0, 0.0)
            })
            .child(
                svg()
                    .path("icons/editor/topbar/search.svg")
                    .size(px(btn_icon_size))
                    .text_color(if is_search_active {
                        c.app_menu_active
                    } else {
                        c.dialog_muted
                    }),
            )
            .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                let _ = search_editor.update(cx, |editor, cx| {
                    editor.toggle_search(window, cx);
                });
            });

        let mut actions = div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .child(search_button)
            .child(split_v_button)
            .child(split_h_button);

        if leaf_count > 1 {
            let max_editor = editor.clone();
            let max_button = icon_chip_button(c, d)
                .id(("panel-topbar-max", panel_id.0))
                .child(
                    svg()
                        .path(if is_maximized {
                            panel_topbar_icon(kind, "restore")
                        } else {
                            panel_topbar_icon(kind, "maximize")
                        })
                        .size(px(btn_icon_size))
                        .text_color(c.dialog_muted),
                )
                .on_click(move |_event, _window, cx| {
                    let _ = max_editor.update(cx, |ed, cx| {
                        ed.defer_host_action(cx, move |host, cx| {
                            host.toggle_panel_maximize(panel_id, cx);
                        });
                        cx.notify();
                    });
                });

            let close_editor = editor.clone();
            let close_button = icon_chip_button(c, d)
                .id(("panel-topbar-close", panel_id.0))
                .child(
                    svg()
                        .path(panel_topbar_icon(kind, "close"))
                        .size(px(btn_icon_size))
                        .text_color(c.dialog_muted),
                )
                .on_click(move |_event, _window, cx| {
                    let _ = close_editor.update(cx, |ed, cx| {
                        ed.defer_host_action(cx, move |host, cx| {
                            host.request_close_panel(panel_id, cx);
                        });
                        cx.notify();
                    });
                });

            actions = actions.child(max_button).child(close_button);
        }

        let mut left_section = div().flex().items_center().gap(px(8.0)).child(type_button);

        {
            let list = self.tab_list_mut();
            let active_tab = list.active_index();
            let tab_infos: Vec<(String, bool, bool)> = list
                .iter()
                .map(|tab| {
                    let name = tab
                        .file
                        .path
                        .as_ref()
                        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                        .unwrap_or_else(|| "Untitled".to_string());
                    (name, tab.is_transient(), tab.file.dirty)
                })
                .collect();

            let mut tab_elements: Vec<AnyElement> = Vec::new();
            for (index, (file_name, is_transient, _is_dirty)) in tab_infos.iter().enumerate() {
                let is_active = index == active_tab;
                let tab_bg = if is_active {
                    c.panel_row_selected
                } else {
                    hsla(0.0, 0.0, 0.0, 0.0)
                };
                let tab_text = if is_active {
                    c.text_default
                } else {
                    c.dialog_muted
                };

                let tab_editor = editor.clone();
                let close_editor = editor.clone();

                let mut title_div = div()
                    .text_color(tab_text)
                    .cursor_pointer()
                    .child(file_name.clone());

                if *is_transient {
                    title_div = title_div.italic();
                }

                tab_elements.push(
                    small_pill_button(c, d)
                        .px(px(6.0))
                        .bg(tab_bg)
                        .text_size(px(11.0))
                        .cursor_pointer()
                        .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                            let is_double = event.click_count > 1;
                            let _ = tab_editor.update(cx, |ed, cx| {
                                ed.defer_host_action(cx, move |host, cx| {
                                    host.activate_panel(panel_id, cx);
                                });
                                if is_double {
                                    if let Some(tab) = ed.session.tab_mut(index) {
                                        tab.persist();
                                    }
                                }
                                ed.activate_tab(index, cx);
                                cx.notify();
                            });
                        })
                        .child(title_div)
                        .child(
                            div()
                                .size(px(12.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(d.tab_close_button_radius))
                                .hover(|this| this.bg(c.dialog_secondary_button_bg.opacity(0.6)))
                                .cursor_pointer()
                                .child(
                                    svg()
                                        .path(panel_topbar_icon(kind, "close"))
                                        .size(px(8.0))
                                        .text_color(c.dialog_muted),
                                )
                                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                                    cx.stop_propagation();
                                    let _ = close_editor.update(cx, |ed, cx| {
                                        ed.request_close_tab(index, cx);
                                        cx.notify();
                                    });
                                }),
                        )
                        .into_any_element(),
                );
            }

            let add_editor = editor.clone();
            tab_elements.push(
                div()
                    .size(px(toolbar_button_size(d.topbar_height)))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(d.icon_button_radius))
                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                    .cursor_pointer()
                    .text_color(c.dialog_muted)
                    .child(
                        svg()
                            .path("icons/settings/plus.svg")
                            .size(px(btn_icon_size))
                            .text_color(c.dialog_muted),
                    )
                    .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                        let _ = add_editor.update(cx, |ed, cx| {
                            ed.defer_host_action(cx, move |host, cx| {
                                host.activate_panel(panel_id, cx);
                            });
                            ed.new_untitled_tab(cx);
                            cx.notify();
                        });
                    })
                    .into_any_element(),
            );

            left_section = left_section.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.0))
                    .children(tab_elements),
            );
        }

        topbar_container(c, d.topbar_height, 8.0)
            .id(("panel-topbar", panel_id.0))
            .child(left_section)
            .child(div().flex().items_center().gap(px(6.0)).child(actions))
            .into_any_element()
    }
}
