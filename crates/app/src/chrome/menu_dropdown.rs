//! In-window floating menu panel and cascading submenus rendering.

use gpui::prelude::FluentBuilder;
use gpui::*;

use crate::actions::{NoRecentFiles, SelectLanguage, SelectTheme};
use crate::menus::dispatch_menu_action_for_editor;
use crate::shell::Shell;
use config::language::I18nManager;
use editor::Editor;
use theme::{Theme, ThemeManager};
use ui::menu_bar::{
    menu_item_visual_height, menu_items_visual_height_with_gaps, menu_panel_left,
    menu_panel_width_for_labels, owned_menu_item_labels, submenu_panel_top,
};
use ui::menu_item::{menu_item, menu_item_row};

impl Shell {
    pub(crate) fn render_in_window_menu_item(
        &self,
        item: OwnedMenuItem,
        item_index: usize,
        prefix: &'static str,
        theme: &Theme,
        shell: WeakEntity<Shell>,
        editor: WeakEntity<Editor>,
        cx: &Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;

        match item {
            OwnedMenuItem::Separator => div()
                .id((prefix, item_index))
                .flex_shrink_0()
                .mx(px(d.menu_separator_margin_x))
                .my(px(d.menu_separator_margin_y))
                .h(px(d.menu_separator_height))
                .bg(c.dialog_border)
                .into_any_element(),
            OwnedMenuItem::Action { name, action, .. } => {
                let is_disabled = action.as_ref().as_any().is::<NoRecentFiles>();
                let click_shell = shell.clone();
                let hover_shell = shell.clone();

                let mut is_selected = false;
                let mut left_elem: Option<AnyElement> = None;

                if let Some(act) = action.as_ref().as_any().downcast_ref::<SelectTheme>() {
                    let current_theme_id = cx.global::<ThemeManager>().current_theme_id();
                    is_selected = act.theme_id == current_theme_id;
                    let item_icon = if name == "Light" {
                        "icons/titlebar/app_menu/sun.svg"
                    } else {
                        "icons/titlebar/app_menu/moon.svg"
                    };
                    left_elem = Some(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                svg()
                                    .path(item_icon)
                                    .size(px(15.0))
                                    .text_color(c.text_default),
                            )
                            .child(name.clone())
                            .into_any_element(),
                    );
                } else if let Some(act) = action.as_ref().as_any().downcast_ref::<SelectLanguage>()
                {
                    let current_language_id = cx.global::<I18nManager>().current_language_id();
                    is_selected = act.language_id == current_language_id;
                }

                let is_theme_or_lang = action.as_ref().as_any().is::<SelectTheme>()
                    || action.as_ref().as_any().is::<SelectLanguage>();

                let base = menu_item_row(c, d)
                    .id((prefix, item_index))
                    .w_full()
                    .flex_shrink_0()
                    .when(is_theme_or_lang, |this| this.justify_between())
                    .text_size(px(d.menu_text_size))
                    .font_weight(t.dialog_body_weight.to_font_weight())
                    .text_color(if is_disabled {
                        c.dialog_muted
                    } else {
                        c.dialog_secondary_button_text
                    })
                    .child(
                        left_elem.unwrap_or_else(|| {
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .truncate()
                                .child(name.clone())
                                .into_any_element()
                        }),
                    )
                    .when(is_theme_or_lang, |this| {
                        this.child(if is_selected {
                            svg()
                                .path("icons/titlebar/app_menu/checkmark.svg")
                                .size(px(15.0))
                                .text_color(c.dialog_primary_button_bg)
                                .into_any_element()
                        } else {
                            div().w(px(13.0)).into_any_element()
                        })
                    })
                    .on_hover(move |hovered, _window, cx| {
                        if *hovered && prefix == "app-menu" {
                            let _ =
                                hover_shell.update(cx, |shell, cx| shell.close_menu_submenu(cx));
                        }
                    });

                if is_disabled {
                    base.into_any_element()
                } else {
                    base.hover(|this| this.bg(c.panel_row_hover))
                        .active(|this| this.opacity(0.92))
                        .cursor_pointer()
                        .on_click(move |_event, window, cx| {
                            let _ = click_shell.update(cx, |shell, cx| shell.close_menu_bar(cx));
                            dispatch_menu_action_for_editor(
                                action.as_ref(),
                                &shell,
                                &editor,
                                window,
                                cx,
                            );
                        })
                        .into_any_element()
                }
            }
            OwnedMenuItem::Submenu(submenu) => {
                let is_open = self.menu_bar.submenu_open == Some(item_index);
                let hover_shell = shell.clone();
                menu_item((prefix, item_index), c, d)
                    .w_full()
                    .flex_shrink_0()
                    .justify_between()
                    .bg(if is_open {
                        c.panel_row_selected
                    } else {
                        c.dialog_surface
                    })
                    .text_size(px(d.menu_text_size))
                    .font_weight(t.dialog_body_weight.to_font_weight())
                    .text_color(c.dialog_secondary_button_text)
                    .child(submenu.name.to_string())
                    .child(
                        svg()
                            .path("icons/titlebar/app_menu/chevron-right.svg")
                            .size(px(15.0))
                            .text_color(c.dialog_secondary_button_text),
                    )
                    .on_hover(move |hovered, _window, cx| {
                        if *hovered {
                            let _ = hover_shell
                                .update(cx, |shell, cx| shell.open_menu_submenu(item_index, cx));
                        }
                    })
                    .into_any_element()
            }
            OwnedMenuItem::SystemMenu(os_menu) => menu_item_row(c, d)
                .id((prefix, item_index))
                .w_full()
                .flex_shrink_0()
                .text_size(px(d.menu_text_size))
                .text_color(c.dialog_muted)
                .child(os_menu.name.to_string())
                .into_any_element(),
        }
    }

    /// Renders the currently open in-window fallback menu as a floating panel.
    pub(crate) fn render_in_window_menu_panel(
        &self,
        theme: &Theme,
        cx: &mut Context<Self>,
        menus: Option<&[gpui::OwnedMenu]>,
        menu_labels: &[SharedString],
        top_offset: f32,
        viewport_size: Size<Pixels>,
        editor: WeakEntity<Editor>,
    ) -> Option<AnyElement> {
        let viewport_width = f32::from(viewport_size.width.max(px(1.0)));
        let viewport_height = f32::from(viewport_size.height.max(px(1.0)));
        let open_index = self.menu_bar.open?;
        let menus = menus?;
        let menu = menus.get(open_index)?.clone();
        let menu_items = menu.items.clone();
        let c = &theme.colors;
        let d = &theme.dimensions;
        let shell = cx.entity().downgrade();
        let menu_item_labels = owned_menu_item_labels(&menu_items);
        let menu_panel_width = menu_panel_width_for_labels(&menu_item_labels, d);
        let main_left = menu_panel_left(open_index, menu_labels, d);

        let (submenu_panel, submenu_bridge) = if let Some(submenu_index) = self.menu_bar.submenu_open {
            if let Some(OwnedMenuItem::Submenu(submenu)) = menu_items.get(submenu_index) {
                let submenu_labels = owned_menu_item_labels(&submenu.items);
                let raw_left = main_left + menu_panel_width + d.menu_panel_gap;
                let submenu_width = menu_panel_width_for_labels(&submenu_labels, d);
                let is_flipped = raw_left + submenu_width > viewport_width - 8.0
                    && main_left >= submenu_width + d.menu_panel_gap;
                let left = if is_flipped {
                    main_left - submenu_width - d.menu_panel_gap
                } else {
                    raw_left
                };
                let ideal_top = submenu_panel_top(&menu_items, submenu_index, d);
                let total_submenu_height =
                    menu_items_visual_height_with_gaps(&submenu.items, d)
                        + d.menu_panel_padding * 2.0;
                let top = if top_offset + ideal_top + total_submenu_height > viewport_height - 16.0 {
                    (viewport_height - top_offset - total_submenu_height - 16.0).max(d.menu_panel_top)
                } else {
                    ideal_top
                };
                let max_panel_height = (viewport_height - (top_offset + top) - 16.0)
                    .max(d.menu_item_height * 3.0);
                let is_submenu_scrollable = total_submenu_height > max_panel_height;

                let rendered_sub_items = submenu.items.clone().into_iter().enumerate().map(
                    |(sub_index, item)| {
                        self.render_in_window_menu_item(
                            item,
                            submenu_index * 1000 + sub_index,
                            "app-submenu",
                            theme,
                            shell.clone(),
                            editor.clone(),
                            cx,
                        )
                    },
                );

                let sub_panel = div()
                    .id(("app-submenu-panel", open_index * 1000 + submenu_index))
                    .absolute()
                    .occlude()
                    .top(px(top_offset + top))
                    .left(px(left))
                    .w(px(submenu_width))
                    .max_h(px(max_panel_height))
                    .when(is_submenu_scrollable, |this| this.overflow_y_scroll())
                    .p(px(d.menu_panel_padding))
                    .flex()
                    .flex_col()
                    .gap(px(d.menu_panel_gap))
                    .bg(c.dialog_surface)
                    .border(px(d.dialog_border_width))
                    .border_color(c.dialog_border)
                    .rounded(px(d.menu_panel_radius))
                    .shadow_xl()
                    .on_hover(cx.listener(Self::on_menu_submenu_panel_hover))
                    .children(rendered_sub_items)
                    .into_any_element();

                let bridge_left = if is_flipped {
                    left
                } else {
                    main_left + menu_panel_width
                };
                let vertical_tolerance = d.menu_panel_padding + d.menu_panel_gap;
                let bridge_top = (ideal_top - vertical_tolerance).max(d.menu_panel_top);
                let bridge_height =
                    menu_item_visual_height(&menu_items[submenu_index], d) + vertical_tolerance * 2.0;

                let bridge = div()
                    .id(("app-submenu-bridge", open_index * 1000 + submenu_index))
                    .absolute()
                    .occlude()
                    .top(px(top_offset + bridge_top))
                    .left(px(bridge_left))
                    .w(px(d.menu_panel_gap + submenu_width))
                    .h(px(bridge_height))
                    .bg(hsla(0.0, 0.0, 0.0, 0.0))
                    .on_hover(cx.listener(Self::on_menu_submenu_bridge_hover))
                    .into_any_element();

                (Some(sub_panel), Some(bridge))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        let items_height = menu_items_visual_height_with_gaps(&menu_items, d);
        let padding_height = d.menu_panel_padding * 2.0;
        let total_ideal_height = items_height + padding_height;
        let max_panel_height = (viewport_height - (top_offset + d.menu_panel_top) - 16.0)
            .max(d.menu_item_height * 3.0);
        let is_scrollable = total_ideal_height > max_panel_height;

        let rendered_items = menu_items
            .into_iter()
            .enumerate()
            .map(|(item_index, item)| {
                self.render_in_window_menu_item(
                    item,
                    item_index,
                    "app-menu",
                    theme,
                    shell.clone(),
                    editor.clone(),
                    cx,
                )
            });

        let main_panel = div()
            .id(("app-menu-panel", open_index))
            .absolute()
            .occlude()
            .top(px(top_offset + d.menu_panel_top))
            .left(px(main_left))
            .w(px(menu_panel_width))
            .max_h(px(max_panel_height))
            .when(is_scrollable, |this| this.overflow_y_scroll())
            .p(px(d.menu_panel_padding))
            .flex()
            .flex_col()
            .gap(px(d.menu_panel_gap))
            .bg(c.dialog_surface)
            .border(px(d.dialog_border_width))
            .border_color(c.dialog_border)
            .rounded(px(d.menu_panel_radius))
            .shadow_xl()
            .on_hover(cx.listener(Self::on_menu_panel_hover))
            .children(rendered_items);

        let mut container = div()
            .id(("app-menu-overlay-root", open_index))
            .absolute()
            .top_0()
            .left_0()
            .w_full()
            .h_full()
            .child(main_panel);

        if let Some(bridge) = submenu_bridge {
            container = container.child(bridge);
        }
        if let Some(sub) = submenu_panel {
            container = container.child(sub);
        }

        Some(container.into_any_element())
    }
}
