//! In-window inline titlebar menu button row rendering.

use gpui::*;

use crate::shell::Shell;
use theme::Theme;
use ui::button::menu_bar_button;
use ui::menu_bar::{TITLEBAR_MENU_BUTTON_GAP, menu_bar_button_width};

impl Shell {
    /// Renders the in-window menu bar row (the fallback for platforms
    /// without a native application menu): the app icon toggle button plus
    /// one button per top-level menu.
    pub(crate) fn render_inline_titlebar_menu(
        &self,
        theme: &Theme,
        cx: &mut Context<Self>,
        menus: Option<&[gpui::OwnedMenu]>,
        menu_labels: &[SharedString],
    ) -> Option<AnyElement> {
        let menus = menus?;
        if menus.is_empty() {
            return None;
        }

        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let shell = cx.entity().downgrade();

        let is_expanded = self.menu_bar.expanded || self.menu_bar.open.is_some();

        let mut row = div()
            .id("titlebar-menu-inline")
            .h_full()
            .flex()
            .items_center()
            .gap(px(TITLEBAR_MENU_BUTTON_GAP))
            .px(px(6.0));

        let app_button_shell = shell.clone();
        let app_button = div()
            .id("titlebar-app-icon-button")
            .w(px(d.menu_bar_button_height))
            .h(px(d.menu_bar_button_height))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(d.menu_bar_button_radius))
            .hover(|this| this.bg(c.dialog_secondary_button_hover))
            .active(|this| this.opacity(0.92))
            .cursor_pointer()
            .child(
                svg()
                    .path("icons/titlebar/app_menu/app_menu.svg")
                    .size(px(14.0))
                    .text_color(if is_expanded {
                        c.app_menu_active
                    } else {
                        c.dialog_secondary_button_text
                    }),
            )
            .on_click(move |_event, _window, cx| {
                let _ = app_button_shell.update(cx, |shell, cx| {
                    shell.toggle_menu_bar_expanded(cx);
                });
            });

        row = row.child(app_button);

        if is_expanded && !menu_labels.is_empty() {
            let button_widths = menu_labels
                .iter()
                .map(|label| menu_bar_button_width(label, d))
                .collect::<Vec<_>>();

            for (index, label) in menu_labels.iter().enumerate() {
                let label = label.clone();
                let is_open = self.menu_bar.open == Some(index);
                let button_shell = shell.clone();
                let click_shell = shell.clone();
                let button_width = button_widths[index];

                row = row.child(
                    menu_bar_button(("app-menu-button", index), c, d)
                        .w(px(button_width))
                        .bg(if is_open {
                            c.dialog_secondary_button_hover
                        } else {
                            hsla(0.0, 0.0, 0.0, 0.0)
                        })
                        .text_size(px(d.menu_text_size))
                        .font_weight(t.dialog_button_weight.to_font_weight())
                        .text_color(c.dialog_secondary_button_text)
                        .whitespace_nowrap()
                        .child(label)
                        .on_hover(move |hovered, _window, cx| {
                            if *hovered {
                                let _ = button_shell
                                    .update(cx, |shell, cx| shell.open_menu_bar(index, cx));
                            }
                        })
                        .on_click(move |_event, _window, cx| {
                            let _ =
                                click_shell.update(cx, |shell, cx| shell.open_menu_bar(index, cx));
                        }),
                );
            }
        }

        Some(row.into_any_element())
    }
}
