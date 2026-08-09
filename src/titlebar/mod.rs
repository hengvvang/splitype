//! In-window titlebar menu bar — rendering and the import-menu split
//! detection.
//!
//! The menu-bar state machine lives in `crate::editor::menu_bar` (the Editor
//! entity owns the state); the pure geometry lives in
//! `crate::ui::menu_bar` so both the editor window and this
//! renderer can share it.

use crate::ui::button::menu_bar_button;
use crate::ui::menu_bar::{
    TITLEBAR_MENU_BUTTON_GAP, menu_bar_button_width, menu_panel_left, menu_panel_width_for_labels,
    owned_menu_item_labels, scrollable_import_menu_scroll_height, submenu_bridge_geometry,
    submenu_panel_top,
};
use crate::ui::menu_item::{menu_item, menu_item_row};
use crate::ui::popover::overlay;

use gpui::prelude::FluentBuilder;
use gpui::*;

use crate::app::menus::dispatch_menu_action_for_editor;
use crate::editor::actions::{
    AddLanguageConfig, AddThemeConfig, NoRecentFiles, SelectLanguage, SelectTheme,
};
use crate::editor::controller::Editor;
use crate::infra::i18n::I18nManager;
use crate::infra::theme::{Theme, ThemeManager};

// ── Import menu (theme / language) split detection ────────────────────────

pub(crate) fn import_menu_split_index(items: &[OwnedMenuItem]) -> Option<usize> {
    let [
        prefix @ ..,
        OwnedMenuItem::Separator,
        OwnedMenuItem::Action { action, .. },
    ] = items
    else {
        return None;
    };

    if action.as_ref().as_any().is::<AddThemeConfig>()
        || action.as_ref().as_any().is::<AddLanguageConfig>()
    {
        Some(prefix.len())
    } else {
        None
    }
}

impl Editor {
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
        let editor = cx.entity().downgrade();

        let is_expanded = self.menu_bar.expanded || self.menu_bar.open.is_some();

        let mut row = div()
            .id("titlebar-menu-inline")
            .h_full()
            .flex()
            .items_center()
            .gap(px(TITLEBAR_MENU_BUTTON_GAP))
            .px(px(6.0));

        let app_button_editor = editor.clone();
        // Sized to hug the 14px glyph (compact hit target) rather than the
        // 46px-wide window-control buttons; the glyph itself stays the
        // unmodified Segoe GlobalNavButton SVG.
        let app_button = div()
            .id("titlebar-app-icon-button")
            .w(px(34.0))
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .hover(|this| this.bg(c.dialog_secondary_button_hover))
            .active(|this| this.opacity(0.88))
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
            .on_click(move |_, _window, cx| {
                let _ = app_button_editor.update(cx, |ed, cx| {
                    ed.toggle_menu_bar_expanded(cx);
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
                let button_editor = editor.clone();
                let click_editor = editor.clone();
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
                                let _ = button_editor
                                    .update(cx, |editor, cx| editor.open_menu_bar(index, cx));
                            }
                        })
                        .on_click(move |_, _window, cx| {
                            let _ = click_editor
                                .update(cx, |editor, cx| editor.open_menu_bar(index, cx));
                        }),
                );
            }
        }

        Some(row.into_any_element())
    }

    pub(crate) fn render_in_window_menu_item(
        &self,
        item: OwnedMenuItem,
        item_index: usize,
        theme: &Theme,
        editor: WeakEntity<Self>,
        cx: &Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;

        match item {
            OwnedMenuItem::Separator => div()
                .id(("app-menu-separator", item_index))
                .flex_shrink_0()
                .mx(px(d.menu_separator_margin_x))
                .my(px(d.menu_separator_margin_y))
                .h(px(d.menu_separator_height))
                .bg(c.dialog_border)
                .into_any_element(),
            OwnedMenuItem::Action { name, action, .. } => {
                let is_disabled = action.as_ref().as_any().is::<NoRecentFiles>();
                let click_editor = editor.clone();
                let hover_editor = editor.clone();

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
                    .id(("app-menu-item", item_index))
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
                        left_elem.unwrap_or_else(|| div().child(name.clone()).into_any_element()),
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
                        if *hovered {
                            let _ =
                                hover_editor.update(cx, |editor, cx| editor.close_menu_submenu(cx));
                        }
                    });

                if is_disabled {
                    base.into_any_element()
                } else {
                    base.hover(|this| this.bg(c.panel_row_hover))
                        .active(|this| this.opacity(0.92))
                        .cursor_pointer()
                        .on_click(move |_, window, cx| {
                            let _ = click_editor.update(cx, |editor, cx| editor.close_menu_bar(cx));
                            dispatch_menu_action_for_editor(
                                action.as_ref(),
                                &click_editor,
                                window,
                                cx,
                            );
                        })
                        .into_any_element()
                }
            }
            OwnedMenuItem::Submenu(submenu) => {
                let is_open = self.menu_bar.submenu_open == Some(item_index);
                let hover_editor = editor.clone();
                menu_item(("app-menu-submenu", item_index), c, d)
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
                            let _ = hover_editor
                                .update(cx, |editor, cx| editor.open_menu_submenu(item_index, cx));
                        }
                    })
                    .into_any_element()
            }
            OwnedMenuItem::SystemMenu(os_menu) => menu_item_row(c, d)
                .id(("app-menu-system", item_index))
                .w_full()
                .flex_shrink_0()
                .text_size(px(d.menu_text_size))
                .text_color(c.dialog_muted)
                .child(os_menu.name.to_string())
                .into_any_element(),
        }
    }

    /// Renders the currently open in-window fallback menu as a floating
    /// panel. `menus` and `menu_labels` are fetched and computed once at
    /// the caller and shared with [`Self::render_in_window_menu_bar`].
    pub(crate) fn render_in_window_menu_panel(
        &self,
        theme: &Theme,
        cx: &mut Context<Self>,
        menus: Option<&[gpui::OwnedMenu]>,
        menu_labels: &[SharedString],
        top_offset: f32,
        viewport_height: f32,
    ) -> Option<AnyElement> {
        let open_index = self.menu_bar.open?;
        let menus = menus?;
        let menu = menus.get(open_index)?.clone();
        let menu_items = menu.items.clone();
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let editor = cx.entity().downgrade();
        let menu_item_labels = owned_menu_item_labels(&menu_items);
        let menu_panel_width = menu_panel_width_for_labels(&menu_item_labels, d);
        let submenu_bridge = self
            .menu_bar
            .submenu_open
            .and_then(|submenu_index| match menu_items.get(submenu_index)? {
                OwnedMenuItem::Submenu(submenu) => {
                    let submenu_labels = owned_menu_item_labels(&submenu.items);
                    let geometry = submenu_bridge_geometry(
                        open_index,
                        menu_labels,
                        &menu_items,
                        submenu_index,
                        &submenu_labels,
                        d,
                    )?;
                    Some(
                        div()
                            .id(("app-submenu-bridge", open_index * 1000 + submenu_index))
                            .absolute()
                            .occlude()
                            .top(px(top_offset + geometry.top))
                            .left(px(geometry.left))
                            .w(px(geometry.width))
                            .h(px(geometry.height))
                            .bg(hsla(0.0, 0.0, 0.0, 0.0))
                            .on_hover(cx.listener(Self::on_menu_submenu_bridge_hover))
                            .into_any_element(),
                    )
                }
                _ => None,
            });
        let submenu_panel =
            self.menu_bar.submenu_open.and_then(|submenu_index| {
                match menu_items.get(submenu_index)? {
                    OwnedMenuItem::Submenu(submenu) => {
                        let submenu_labels = owned_menu_item_labels(&submenu.items);
                        let left = menu_panel_left(open_index, menu_labels, d)
                            + menu_panel_width
                            + d.menu_panel_gap;
                        let top = submenu_panel_top(&menu_items, submenu_index, d);
                        let submenu_width = menu_panel_width_for_labels(&submenu_labels, d);
                        let submenu_items = submenu.items.clone().into_iter().enumerate().map(
                            |(item_index, item)| match item {
                                OwnedMenuItem::Separator => div()
                                    .id((
                                        "app-submenu-separator",
                                        submenu_index * 1000 + item_index,
                                    ))
                                    .mx(px(d.menu_separator_margin_x))
                                    .my(px(d.menu_separator_margin_y))
                                    .h(px(d.menu_separator_height))
                                    .bg(c.dialog_border)
                                    .into_any_element(),
                                OwnedMenuItem::Action { name, action, .. } => {
                                    let is_disabled =
                                        action.as_ref().as_any().is::<NoRecentFiles>();
                                    let editor = editor.clone();

                                    let mut is_selected = false;
                                    let mut left_elem: Option<AnyElement> = None;

                                    if let Some(act) =
                                        action.as_ref().as_any().downcast_ref::<SelectTheme>()
                                    {
                                        let current_theme_id =
                                            cx.global::<ThemeManager>().current_theme_id();
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
                                    } else if let Some(act) =
                                        action.as_ref().as_any().downcast_ref::<SelectLanguage>()
                                    {
                                        let current_language_id =
                                            cx.global::<I18nManager>().current_language_id();
                                        is_selected = act.language_id == current_language_id;
                                    }

                                    let is_theme_or_lang =
                                        action.as_ref().as_any().is::<SelectTheme>()
                                            || action.as_ref().as_any().is::<SelectLanguage>();

                                    let base = div()
                                        .id(("app-submenu-item", submenu_index * 1000 + item_index))
                                        .w_full()
                                        .h(px(d.menu_item_height))
                                        .px(px(d.menu_item_padding_x))
                                        .flex()
                                        .items_center()
                                        .when(is_theme_or_lang, |this| this.justify_between())
                                        .rounded(px(d.menu_item_radius))
                                        .bg(if is_selected {
                                            c.panel_row_selected
                                        } else {
                                            c.dialog_surface
                                        })
                                        .text_size(px(d.menu_text_size))
                                        .font_weight(t.dialog_body_weight.to_font_weight())
                                        .text_color(if is_disabled {
                                            c.dialog_muted
                                        } else {
                                            c.dialog_secondary_button_text
                                        })
                                        .child(left_elem.unwrap_or_else(|| {
                                            div().child(name.clone()).into_any_element()
                                        }))
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
                                        });

                                    if is_disabled {
                                        base.into_any_element()
                                    } else {
                                        base.hover(|this| this.bg(c.panel_row_hover))
                                            .active(|this| this.opacity(0.92))
                                            .cursor_pointer()
                                            .on_click(move |_, window, cx| {
                                                let _ = editor.update(cx, |editor, cx| {
                                                    editor.close_menu_bar(cx)
                                                });
                                                dispatch_menu_action_for_editor(
                                                    action.as_ref(),
                                                    &editor,
                                                    window,
                                                    cx,
                                                );
                                            })
                                            .into_any_element()
                                    }
                                }
                                OwnedMenuItem::Submenu(submenu) => div()
                                    .id(("app-submenu-nested", submenu_index * 1000 + item_index))
                                    .w_full()
                                    .h(px(d.menu_item_height))
                                    .px(px(d.menu_item_padding_x))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .rounded(px(d.menu_item_radius))
                                    .bg(c.dialog_surface)
                                    .text_size(px(d.menu_text_size))
                                    .text_color(c.dialog_muted)
                                    .child(submenu.name.to_string())
                                    .child(
                                        svg()
                                            .path("icons/titlebar/app_menu/chevron-right.svg")
                                            .size(px(16.0))
                                            .text_color(c.dialog_muted),
                                    )
                                    .into_any_element(),
                                OwnedMenuItem::SystemMenu(os_menu) => div()
                                    .id(("app-submenu-system", submenu_index * 1000 + item_index))
                                    .w_full()
                                    .h(px(d.menu_item_height))
                                    .px(px(d.menu_item_padding_x))
                                    .flex()
                                    .items_center()
                                    .rounded(px(d.menu_item_radius))
                                    .bg(c.dialog_surface)
                                    .text_size(px(d.menu_text_size))
                                    .text_color(c.dialog_muted)
                                    .child(os_menu.name.to_string())
                                    .into_any_element(),
                            },
                        );

                        Some(
                            div()
                                .id(("app-submenu-panel", open_index * 1000 + submenu_index))
                                .absolute()
                                .occlude()
                                .top(px(top_offset + top))
                                .left(px(left))
                                .w(px(submenu_width))
                                .p(px(d.menu_panel_padding))
                                .flex()
                                .flex_col()
                                .gap(px(d.menu_panel_gap))
                                .bg(c.dialog_surface)
                                .border(px(d.dialog_border_width))
                                .border_color(c.dialog_border)
                                .rounded(px(d.menu_panel_radius))
                                .shadow_lg()
                                .on_hover(cx.listener(Self::on_menu_submenu_panel_hover))
                                .children(submenu_items)
                                .into_any_element(),
                        )
                    }
                    _ => None,
                }
            });

        let main_panel = div()
            .id(("app-menu-panel", open_index))
            .absolute()
            .occlude()
            .top(px(top_offset + d.menu_panel_top))
            .left(px(menu_panel_left(open_index, menu_labels, d)))
            .w(px(menu_panel_width))
            .p(px(d.menu_panel_padding))
            .flex()
            .flex_col()
            .gap(px(d.menu_panel_gap))
            .bg(c.dialog_surface)
            .border(px(d.dialog_border_width))
            .border_color(c.dialog_border)
            .rounded(px(d.menu_panel_radius))
            .shadow_lg()
            .on_hover(cx.listener(Self::on_menu_panel_hover));
        let main_panel = if let Some(split_index) = import_menu_split_index(&menu_items) {
            let scroll_items = &menu_items[..split_index];
            let footer_items = &menu_items[split_index..];
            let scroll_height = scrollable_import_menu_scroll_height(
                scroll_items,
                footer_items,
                viewport_height,
                top_offset,
                d,
            );
            let scroll_area = (!scroll_items.is_empty()).then(|| {
                div()
                    .id(("app-menu-scroll-area", open_index))
                    .w_full()
                    .h(px(scroll_height))
                    .flex_shrink_0()
                    .min_h(px(0.0))
                    .overflow_y_scroll()
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap(px(d.menu_panel_gap))
                            .children(scroll_items.iter().cloned().enumerate().map(
                                |(item_index, item)| {
                                    self.render_in_window_menu_item(
                                        item,
                                        item_index,
                                        theme,
                                        editor.clone(),
                                        cx,
                                    )
                                },
                            )),
                    )
                    .into_any_element()
            });
            let footer_elements =
                footer_items
                    .iter()
                    .cloned()
                    .enumerate()
                    .map(|(footer_index, item)| {
                        self.render_in_window_menu_item(
                            item,
                            split_index + footer_index,
                            theme,
                            editor.clone(),
                            cx,
                        )
                    });

            main_panel
                .children(scroll_area)
                .children(footer_elements)
                .into_any_element()
        } else {
            let items = menu_items
                .iter()
                .cloned()
                .enumerate()
                .map(|(item_index, item)| {
                    self.render_in_window_menu_item(item, item_index, theme, editor.clone(), cx)
                });

            main_panel.children(items).into_any_element()
        };

        let layer = overlay()
            .id(("app-menu-panel-layer", open_index))
            .child(main_panel);
        let layer = if let Some(submenu_bridge) = submenu_bridge {
            layer.child(submenu_bridge)
        } else {
            layer
        };
        let layer = if let Some(submenu_panel) = submenu_panel {
            layer.child(submenu_panel)
        } else {
            layer
        };

        Some(layer.into_any_element())
    }
}

#[cfg(test)]
mod tests {
    // NOTE: import explicitly, not via `use super::*` — the parent module's
    // element builders (`render_in_window_menu_item`, …) push `#[test]`
    // expansion past the recursion limit on Windows.
    use super::import_menu_split_index;
    use crate::editor::actions::{AddLanguageConfig, AddThemeConfig, NoRecentFiles};
    use gpui::OwnedMenuItem;

    fn disabled_menu_action(name: &str) -> OwnedMenuItem {
        OwnedMenuItem::Action {
            name: name.into(),
            action: Box::new(NoRecentFiles),
            os_action: None,
        }
    }

    fn add_theme_menu_action() -> OwnedMenuItem {
        OwnedMenuItem::Action {
            name: "Add Theme Config".into(),
            action: Box::new(AddThemeConfig),
            os_action: None,
        }
    }

    fn add_language_menu_action() -> OwnedMenuItem {
        OwnedMenuItem::Action {
            name: "Add Language Config".into(),
            action: Box::new(AddLanguageConfig),
            os_action: None,
        }
    }

    #[test]
    fn import_menu_split_detects_theme_and_language_import_tails() {
        let theme_items = vec![
            disabled_menu_action("splitype"),
            OwnedMenuItem::Separator,
            add_theme_menu_action(),
        ];
        let language_items = vec![
            disabled_menu_action("English"),
            OwnedMenuItem::Separator,
            add_language_menu_action(),
        ];
        let regular_items = vec![
            disabled_menu_action("Open"),
            OwnedMenuItem::Separator,
            disabled_menu_action("Save"),
        ];
        let malformed_import_items =
            vec![disabled_menu_action("splitype"), add_theme_menu_action()];

        assert_eq!(import_menu_split_index(&theme_items), Some(1));
        assert_eq!(import_menu_split_index(&language_items), Some(1));
        assert_eq!(import_menu_split_index(&regular_items), None);
        assert_eq!(import_menu_split_index(&malformed_import_items), None);
    }
}
