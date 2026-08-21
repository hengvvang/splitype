//! Top bar of an Editor area: the area type selector, split/close controls
//! and the Editor-specific tab bar.

use gpui::prelude::FluentBuilder;
use gpui::*;

use crate::app::window_layout::panel_topbar_icon;
use crate::app::window_panels::WindowPanelKind;
use crate::infra::theme::Theme;
use crate::splitter::SplitAxis;
use crate::ui::button::{icon_chip_button, small_pill_button};
use crate::ui::topbar::topbar_container;

impl crate::editor::controller::Editor {
    /// Top bar of an Editor area: type selector and split/close controls
    /// plus the Editor-specific tab bar.
    pub(crate) fn render_editor_topbar(
        &mut self,
        kind: crate::app::window_panels::WindowPanelKind,
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
        // The active editor (the target for explorer file opens) shows a
        // link icon after its name; other kinds and inactive editors stay
        // plain text.
        let is_active_editor = self.is_active_panel;
        let type_button = small_pill_button(c, d)
            .id(("panel-topbar-type", panel_id))
            .text_size(px(12.0))
            .text_color(c.text_default)
            .child(kind.name().to_string())
            .when(is_active_editor, |this| {
                this.child(
                    svg()
                        .path(panel_topbar_icon(kind, "active"))
                        .size(px(d.topbar_height * 0.5))
                        .text_color(c.app_menu_active),
                )
            })
            .on_click(move |_event, _window, cx| {
                let _ = type_editor.update(cx, |ed, cx| {
                    ed.defer_shell_action(cx, move |shell, cx| {
                        shell.toggle_panel_dropdown(panel_id, cx);
                    });
                    cx.notify();
                });
            });

        let split_h_editor = editor.clone();
        let split_h_button = icon_chip_button(c, d)
            .id(("panel-topbar-split-h", panel_id))
            .child(
                svg()
                    .path(panel_topbar_icon(kind, "split-h"))
                    .size(px(d.topbar_height * 0.5 + 2.0))
                    .text_color(c.dialog_muted),
            )
            .on_click(move |_event, _window, cx| {
                let _ = split_h_editor.update(cx, |ed, cx| {
                    // Same-kind split; Editor panels deep-copy their tabs.
                    ed.defer_shell_action(cx, move |shell, cx| {
                        shell.split_panel(panel_id, SplitAxis::Horizontal, 0.5, true, cx);
                    });
                    cx.notify();
                });
            });

        let split_v_editor = editor.clone();
        let split_v_button = icon_chip_button(c, d)
            .id(("panel-topbar-split-v", panel_id))
            .child(
                svg()
                    .path(panel_topbar_icon(kind, "split-v"))
                    .size(px(d.topbar_height * 0.5 + 2.0))
                    .text_color(c.dialog_muted),
            )
            .on_click(move |_event, _window, cx| {
                let _ = split_v_editor.update(cx, |ed, cx| {
                    // Same-kind split; Editor panels deep-copy their tabs.
                    ed.defer_shell_action(cx, move |shell, cx| {
                        shell.split_panel(panel_id, SplitAxis::Vertical, 0.5, true, cx);
                    });
                    cx.notify();
                });
            });

        let mut actions = div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .child(split_v_button)
            .child(split_h_button);

        if leaf_count > 1 {
            let max_editor = editor.clone();
            let max_button = icon_chip_button(c, d)
                .id(("panel-topbar-max", panel_id))
                .child(
                    svg()
                        .path(if is_maximized {
                            panel_topbar_icon(kind, "restore")
                        } else {
                            panel_topbar_icon(kind, "maximize")
                        })
                        .size(px(d.topbar_height * 0.5 - 2.0))
                        .text_color(c.dialog_muted),
                )
                .on_click(move |_event, _window, cx| {
                    let _ = max_editor.update(cx, |ed, cx| {
                        ed.defer_shell_action(cx, move |shell, cx| {
                            shell.toggle_panel_maximize(panel_id, cx);
                        });
                        cx.notify();
                    });
                });

            let close_editor = editor.clone();
            let close_button = icon_chip_button(c, d)
                .id(("panel-topbar-close", panel_id))
                .child(
                    svg()
                        .path(panel_topbar_icon(kind, "close"))
                        .size(px(d.topbar_height * 0.5 - 2.0))
                        .text_color(c.dialog_muted),
                )
                .on_click(move |_event, _window, cx| {
                    let _ = close_editor.update(cx, |ed, cx| {
                        ed.defer_shell_action(cx, move |shell, cx| {
                            shell.close_panel(panel_id, cx);
                        });
                        cx.notify();
                    });
                });

            actions = actions.child(max_button).child(close_button);
        }

        // Build tab bar for Edit panels from THAT editor.s own tab set.
        let mut left_section = div().flex().items_center().gap(px(8.0)).child(type_button);

        if kind == WindowPanelKind::Editor {
            // Ensure the session exists: an Editor area may have had its
            // (empty) session dropped while switched to another kind and
            // switched back, or may be brand new — rendering must never
            // panic on a missing session.
            let list = self.tab_list_mut();
            let active_tab = list.active_tab;
            let tab_names: Vec<String> = list
                .tabs
                .iter()
                .map(|tab| {
                    tab.file
                        .path
                        .as_ref()
                        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                        .unwrap_or_else(|| "Untitled".to_string())
                })
                .collect();

            let mut tab_elements: Vec<AnyElement> = Vec::new();
            for (index, file_name) in tab_names.iter().enumerate() {
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

                tab_elements.push(
                    small_pill_button(c, d)
                        .px(px(6.0))
                        .bg(tab_bg)
                        .text_size(px(11.0))
                        .child(
                            // Switch area: clicking the file name switches to this tab.
                            div()
                                .text_color(tab_text)
                                .child(file_name.clone())
                                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                                    let _ = tab_editor.update(cx, |ed, cx| {
                                        ed.defer_shell_action(cx, move |shell, cx| {
                                            shell.activate_panel(panel_id, cx);
                                        });
                                        ed.activate_tab(index, cx);
                                        cx.notify();
                                    });
                                }),
                        )
                        .child(
                            // Close button: separate click area.
                            div()
                                .size(px(12.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(2.0))
                                .hover(|this| this.bg(c.dialog_secondary_button_bg.opacity(0.6)))
                                .cursor_pointer()
                                .child(
                                    svg()
                                        .path(panel_topbar_icon(kind, "close"))
                                        .size(px(8.0))
                                        .text_color(c.dialog_muted),
                                )
                                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                                    let _ = close_editor.update(cx, |ed, cx| {
                                        ed.defer_shell_action(cx, move |shell, cx| {
                                            shell.activate_panel(panel_id, cx);
                                        });
                                        ed.close_tab(index, cx);
                                        cx.notify();
                                    });
                                }),
                        )
                        .into_any_element(),
                );
            }

            // Add button opens a fresh untitled tab in THIS editor.
            let add_editor = editor.clone();
            tab_elements.push(
                div()
                    .size(px(18.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(d.menu_item_radius))
                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                    .cursor_pointer()
                    .text_color(c.dialog_muted)
                    .child(
                        svg()
                            .path("icons/settings/plus.svg")
                            .size(px(14.0))
                            .text_color(c.dialog_muted),
                    )
                    .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                        let _ = add_editor.update(cx, |ed, cx| {
                            ed.defer_shell_action(cx, move |shell, cx| {
                                shell.activate_panel(panel_id, cx);
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
            .id(("panel-topbar", panel_id))
            .child(left_section)
            .child(div().flex().items_center().gap(px(6.0)).child(actions))
            .into_any_element()
    }
}
