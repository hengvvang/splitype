use gpui::*;

use config::language::I18nManager;
use platform_contracts::PanelId;
use theme::Theme;
use ui::bottombar::bottombar_container;
use ui::button::{icon_chip_button, toolbar_icon_size};
use ui::menu_item::menu_item;
use ui::popover::menu_panel;

use crate::state::ExplorerState;

/// Free-function entry point: renders the explorer bottom bar from the
/// given panel state entity (the shell owns no explorer state).
pub fn render_explorer_bottombar(
    panel_id: PanelId,
    state: &Entity<ExplorerState>,
    theme: &Theme,
    cx: &mut App,
) -> AnyElement {
    state.update(cx, |state, cx| {
        state.render_explorer_bottombar(panel_id, theme, cx)
    })
}

impl ExplorerState {
    /// Bottom bar of an Explorer area: worktree count on the left, and a
    /// three-dots action menu button on the far right. Clicking the three-dots
    /// button displays a menu panel containing all file-tree actions (text only,
    /// without icons).
    pub(crate) fn render_explorer_bottombar(
        &mut self,
        panel_id: PanelId,
        theme: &Theme,
        cx: &mut App,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let s = cx.global::<I18nManager>().strings().clone();

        let worktree_count = self.worktrees.len();
        let btn_icon_size = toolbar_icon_size(d.bottombar_height);
        let weak = self.self_weak.clone();

        let is_zh = s.explorer_new_file.contains("文件");
        let folder_label = match worktree_count {
            0 => {
                if is_zh {
                    "无打开文件夹".to_string()
                } else {
                    "0 folders".to_string()
                }
            }
            1 => {
                if is_zh {
                    "1 个文件夹".to_string()
                } else {
                    "1 folder".to_string()
                }
            }
            n => {
                if is_zh {
                    format!("{n} 个文件夹")
                } else {
                    format!("{n} folders")
                }
            }
        };

        let is_menu_open = self.bottombar_menu_open;
        let menu_toggle = weak.clone();

        let menu_btn = icon_chip_button(c, d)
            .id(("explorer-bottombar-menu-btn", panel_id.0))
            .bg(if is_menu_open {
                c.panel_row_hover
            } else {
                hsla(0.0, 0.0, 0.0, 0.0)
            })
            .child(
                svg()
                    .path("icons/explorer/bottombar/v_three_points.svg")
                    .size(px(btn_icon_size))
                    .text_color(if is_menu_open {
                        c.text_default
                    } else {
                        c.dialog_muted
                    }),
            )
            .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                let _ = menu_toggle.update(cx, |state, cx| {
                    state.bottombar_menu_open = !state.bottombar_menu_open;
                    cx.refresh_windows();
                });
                cx.stop_propagation();
            });

        let mut bar = bottombar_container(c, d.bottombar_height, d.bottombar_padding_x)
            .id(("explorer-bottombar", panel_id.0))
            .relative()
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        div()
                            .text_size(px(10.5))
                            .text_color(c.dialog_muted)
                            .child(folder_label),
                    ),
            )
            .child(menu_btn);

        if is_menu_open {
            let hide_hidden =
                config::settings::PluginSettings::<crate::settings::ExplorerSettings>::get(cx)
                    .hide_hidden;
            let has_worktrees = !self.worktrees.is_empty();

            let separator = || {
                div()
                    .mx(px(d.menu_separator_margin_x))
                    .my(px(d.menu_separator_margin_y))
                    .h(px(d.menu_separator_height))
                    .bg(c.dialog_border)
                    .into_any_element()
            };

            type BottombarMenuItemHandler =
                Box<dyn Fn(&mut ExplorerState, &mut Window, &mut App) + 'static>;

            let make_item = |id: &'static str,
                             label: String,
                             color: Hsla,
                             handler: BottombarMenuItemHandler|
             -> AnyElement {
                let weak = weak.clone();
                menu_item((id, panel_id.0), c, d)
                    .w_full()
                    .child(
                        div()
                            .text_size(px(t.text_size * 0.8))
                            .text_color(color)
                            .child(label),
                    )
                    .on_mouse_down(MouseButton::Left, {
                        move |_event, window, cx| {
                            let handler = &handler;
                            let _ = weak.update(cx, |state, cx| {
                                state.bottombar_menu_open = false;
                                handler(state, window, cx);
                                cx.refresh_windows();
                            });
                            cx.stop_propagation();
                        }
                    })
                    .into_any_element()
            };

            let mut items = Vec::new();

            // 1. Open / Add folder
            items.push(make_item(
                "explorer-bb-menu-open-folder",
                s.explorer_add_folder.clone(),
                c.text_default,
                Box::new(|state, window, cx| {
                    state.prompt_open_explorer_folder(window, cx);
                }),
            ));

            // 2. New File / New Folder (when worktrees are open)
            if has_worktrees {
                items.push(make_item(
                    "explorer-bb-menu-new-file",
                    s.explorer_new_file.clone(),
                    c.text_default,
                    Box::new(|state, window, cx| {
                        state.on_explorer_new_file(&crate::ops::selection::NewFile, window, cx);
                    }),
                ));
                items.push(make_item(
                    "explorer-bb-menu-new-folder",
                    s.explorer_new_folder.clone(),
                    c.text_default,
                    Box::new(|state, window, cx| {
                        state.on_explorer_new_directory(
                            &crate::ops::selection::NewDirectory,
                            window,
                            cx,
                        );
                    }),
                ));
            }

            items.push(separator());

            // 3. Refresh
            let refresh_label = if is_zh { "刷新" } else { "Refresh" };
            items.push(make_item(
                "explorer-bb-menu-refresh",
                refresh_label.to_string(),
                c.text_default,
                Box::new(|state, _window, cx| {
                    state.rescan_and_sync_explorer(cx);
                }),
            ));

            // 4. Collapse All
            items.push(make_item(
                "explorer-bb-menu-collapse-all",
                s.explorer_collapse_all.clone(),
                c.text_default,
                Box::new(|state, _window, cx| {
                    state.collapse_all_explorer_nodes(cx);
                }),
            ));

            // 5. Toggle Hidden Files
            let hidden_label = if hide_hidden {
                if is_zh {
                    "显示隐藏文件"
                } else {
                    "Show Hidden Files"
                }
            } else if is_zh {
                "隐藏隐藏文件"
            } else {
                "Hide Hidden Files"
            };
            items.push(make_item(
                "explorer-bb-menu-toggle-hidden",
                hidden_label.to_string(),
                c.text_default,
                Box::new(|state, _window, cx| {
                    state.toggle_explorer_hidden(cx);
                }),
            ));

            // 6. Close All Folders (when worktrees are open)
            if has_worktrees {
                items.push(separator());
                let close_all_label = if is_zh {
                    "关闭全部文件夹"
                } else {
                    "Close All Folders"
                };
                items.push(make_item(
                    "explorer-bb-menu-close-all",
                    close_all_label.to_string(),
                    c.dialog_danger_button_bg,
                    Box::new(|state, _window, cx| {
                        state.close_all_explorer_worktrees(cx);
                    }),
                ));
            }

            let menu = menu_panel(c, d)
                .id(("explorer-bottombar-menu-panel", panel_id.0))
                .absolute()
                .occlude()
                .bottom(px(d.bottombar_height + 4.0))
                .right(px(d.bottombar_padding_x))
                .w(px(160.0))
                .children(items);

            bar = bar.child(menu);
        }

        bar.into_any_element()
    }
}
