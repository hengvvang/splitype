//! Window-level chrome for Shell-rooted windows — the custom system
//! titlebar and the in-window fallback menu bar.
//!
//! Owns [`MenuBarState`] (the open/hover/close state machine) and renders
//! the titlebar plus its floating menu panel. Both are window-level
//! concerns: the Shell entity owns and renders them, while the Editor
//! renders only its own content below this chrome.
//!
//! The menu-tree data and action dispatch live in `crate::app::menus`;
//! this module only renders and drives that data. The pure geometry lives
//! in `ui::menu_bar` so both the Shell chrome and the Editor can
//! share it.

use std::time::Duration;

use gpui::prelude::FluentBuilder;
use gpui::*;

use crate::app::actions::{
    AddLanguageConfig, AddThemeConfig, NoRecentFiles, SelectLanguage, SelectTheme,
};
use crate::app::menus::dispatch_menu_action_for_editor;
use crate::app::shell::Shell;
use crate::editor::engine::controller::Editor;
use i18n::I18nManager;
use theme::{Theme, ThemeManager};
use ui::button::menu_bar_button;
use ui::custom_titlebar::{custom_titlebar_height, render_custom_titlebar};
use ui::menu_bar::{
    TITLEBAR_MENU_BUTTON_GAP, menu_bar_button_width, menu_items_visual_height_with_gaps,
    menu_panel_left, menu_panel_width_for_labels, owned_menu_item_labels,
    scrollable_import_menu_scroll_height, submenu_bridge_geometry, submenu_panel_top,
    supports_in_window_menu,
};
use ui::menu_item::{menu_item, menu_item_row};
use ui::popover::overlay;

/// Open/hover state for the in-window titlebar menu bar.
#[derive(Default)]
pub(crate) struct MenuBarState {
    /// Open top-level menu in the in-window fallback menu bar.
    pub(crate) open: Option<usize>,
    pub(crate) expanded: bool,
    /// Open child submenu inside the in-window fallback menu panel.
    pub(crate) submenu_open: Option<usize>,
    pub(crate) panel_hovered: bool,
    pub(crate) submenu_panel_hovered: bool,
    /// Hover state for the invisible bridge spanning the gap between the menu
    /// panel and an open submenu. Tracked separately from
    /// `submenu_panel_hovered` so the handoff between the two regions cannot
    /// clobber a single shared flag and tear the menu down.
    pub(crate) submenu_bridge_hovered: bool,
    pub(crate) close_task: Option<Task<()>>,
}

impl Shell {
    // ── Menu-bar state machine ────────────────────────────────────────────

    pub(crate) fn toggle_menu_bar_expanded(&mut self, cx: &mut Context<Self>) {
        self.menu_bar.expanded = !self.menu_bar.expanded;
        if !self.menu_bar.expanded {
            self.menu_bar.open = None;
            self.menu_bar.submenu_open = None;
        }
        cx.notify();
    }

    pub(crate) fn on_menu_panel_hover(
        &mut self,
        hovered: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_menu_panel_hovered(*hovered, cx);
    }

    pub(crate) fn on_menu_submenu_panel_hover(
        &mut self,
        hovered: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_menu_submenu_panel_hovered(*hovered, cx);
    }

    pub(crate) fn on_menu_submenu_bridge_hover(
        &mut self,
        hovered: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_menu_submenu_bridge_hovered(*hovered, cx);
    }

    pub(crate) fn open_menu_bar(&mut self, index: usize, cx: &mut Context<Self>) {
        self.menu_bar.close_task = None;
        if self.menu_bar.open != Some(index) {
            self.menu_bar.open = Some(index);
            self.menu_bar.submenu_open = None;
            self.menu_bar.submenu_panel_hovered = false;
            self.menu_bar.submenu_bridge_hovered = false;
            cx.notify();
        }
    }

    pub(crate) fn open_menu_submenu(&mut self, index: usize, cx: &mut Context<Self>) {
        self.menu_bar.close_task = None;
        if self.menu_bar.submenu_open != Some(index) {
            self.menu_bar.submenu_open = Some(index);
            cx.notify();
        }
    }

    pub(crate) fn close_menu_submenu(&mut self, cx: &mut Context<Self>) {
        let had_open_submenu = self.menu_bar.submenu_open.take().is_some();
        let had_submenu_hover =
            self.menu_bar.submenu_panel_hovered || self.menu_bar.submenu_bridge_hovered;
        self.menu_bar.submenu_panel_hovered = false;
        self.menu_bar.submenu_bridge_hovered = false;
        if had_open_submenu || had_submenu_hover {
            cx.notify();
        }
    }

    pub(crate) fn schedule_menu_bar_close(&mut self, cx: &mut Context<Self>) {
        if self.menu_bar.open.is_none() {
            return;
        }

        let weak_shell = cx.entity().downgrade();
        self.menu_bar.close_task = Some(cx.spawn(
            async move |_this: WeakEntity<Shell>, cx: &mut AsyncApp| {
                cx.background_executor()
                    .timer(Duration::from_millis(120))
                    .await;
                let _ = weak_shell.update(cx, |shell, cx| {
                    shell.menu_bar.close_task = None;
                    if !shell.menu_bar.panel_hovered
                        && !shell.menu_bar.submenu_panel_hovered
                        && !shell.menu_bar.submenu_bridge_hovered
                    {
                        shell.close_menu_bar(cx);
                    }
                });
            },
        ));
    }

    pub(crate) fn set_menu_panel_hovered(&mut self, hovered: bool, cx: &mut Context<Self>) {
        self.menu_bar.panel_hovered = hovered;
        if hovered {
            self.menu_bar.close_task = None;
        } else if !self.menu_bar.submenu_panel_hovered && !self.menu_bar.submenu_bridge_hovered {
            self.schedule_menu_bar_close(cx);
        }
    }

    pub(crate) fn set_menu_submenu_panel_hovered(&mut self, hovered: bool, cx: &mut Context<Self>) {
        self.menu_bar.submenu_panel_hovered = hovered;
        if hovered {
            self.menu_bar.close_task = None;
        } else if !self.menu_bar.panel_hovered && !self.menu_bar.submenu_bridge_hovered {
            self.schedule_menu_bar_close(cx);
        }
    }

    /// Hover handler for the invisible gap bridge. The bridge and the submenu
    /// panel overlap, so the cursor crossing between them fires a `false` for
    /// one region and a `true` for the other in the same gesture. Keeping their
    /// hover state in separate flags lets either one hold the menu open
    /// regardless of the order those events arrive.
    pub(crate) fn set_menu_submenu_bridge_hovered(
        &mut self,
        hovered: bool,
        cx: &mut Context<Self>,
    ) {
        self.menu_bar.submenu_bridge_hovered = hovered;
        if hovered {
            self.menu_bar.close_task = None;
        } else if !self.menu_bar.panel_hovered && !self.menu_bar.submenu_panel_hovered {
            self.schedule_menu_bar_close(cx);
        }
    }

    /// Closes the menu bar and the explorer context menu when the window
    /// body (outside the titlebar and any open menu panel) receives a
    /// mouse-down. The menu panels are siblings of the body container and
    /// are `.occlude()`d, so their clicks never reach this listener.
    pub(crate) fn on_body_mouse_down(
        &mut self,
        _: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_menu_bar(cx);
        self.close_explorer_file_menu(cx);
    }

    pub(crate) fn close_menu_bar(&mut self, cx: &mut Context<Self>) {
        let had_open_menu = self.menu_bar.open.take().is_some();
        let had_open_submenu = self.menu_bar.submenu_open.take().is_some();
        let had_hover_state = self.menu_bar.panel_hovered
            || self.menu_bar.submenu_panel_hovered
            || self.menu_bar.submenu_bridge_hovered;
        let had_pending_close = self.menu_bar.close_task.take().is_some();
        self.menu_bar.panel_hovered = false;
        self.menu_bar.submenu_panel_hovered = false;
        self.menu_bar.submenu_bridge_hovered = false;
        if had_open_menu || had_open_submenu || had_hover_state || had_pending_close {
            cx.notify();
        }
    }

    // ── Chrome rendering ──────────────────────────────────────────────────

    /// Renders the window chrome: the custom system titlebar (with the
    /// in-window menu bar when the platform has no native menu) and the
    /// floating menu panel for the currently open menu.
    ///
    /// Returns `(titlebar, menu_panel, titlebar_height)`; the titlebar is
    /// absolutely positioned over the window top, so the caller must offset
    /// the window body by `titlebar_height`.
    pub(crate) fn render_window_chrome(
        &mut self,
        theme: &Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (Option<AnyElement>, Option<AnyElement>, f32) {
        let titlebar_height = custom_titlebar_height(window, &theme.dimensions);
        // Fetch menus + collect labels once for both renderers; previously each
        // of render_inline_titlebar_menu / render_in_window_menu_panel called
        // cx.get_menus() and walked menus.iter().map(|m| m.name.to_string())
        // independently — two redundant Vec<OwnedMenu> + two redundant
        // Vec<String>-of-N-allocations per frame.
        let menus = supports_in_window_menu()
            .then(|| cx.get_menus())
            .flatten()
            .filter(|m| !m.is_empty());
        let menu_labels: Vec<SharedString> = menus
            .as_ref()
            .map(|m| m.iter().map(|menu| menu.name.clone()).collect())
            .unwrap_or_default();
        // The titlebar never shows a title text; the window title lives in the
        // OS title bar / task bar via the Editor's `sync_window_title`.
        let window_title: SharedString = SharedString::new("");
        let inline_menu =
            self.render_inline_titlebar_menu(theme, cx, menus.as_deref(), &menu_labels);

        let titlebar = render_custom_titlebar(
            "editor-titlebar",
            window_title,
            inline_menu,
            theme,
            window,
            cx,
            Shell::on_titlebar_close,
        );

        let editor = self.primary_editor().map(|editor| editor.downgrade());
        let menu_panel = editor.and_then(|editor| {
            self.render_in_window_menu_panel(
                theme,
                cx,
                menus.as_deref(),
                &menu_labels,
                titlebar_height,
                window.viewport_size(),
                editor,
            )
        });

        (titlebar, menu_panel, titlebar_height)
    }

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
        // Sized as a square with side length equal to menu_bar_button_height,
        // matching the height and corner radius of the adjacent menu buttons.
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

    pub(crate) fn render_in_window_menu_item(
        &self,
        item: OwnedMenuItem,
        item_index: usize,
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
                .id(("app-menu-separator", item_index))
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
                        if *hovered {
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
                            dispatch_menu_action_for_editor(action.as_ref(), &editor, window, cx);
                        })
                        .into_any_element()
                }
            }
            OwnedMenuItem::Submenu(submenu) => {
                let is_open = self.menu_bar.submenu_open == Some(item_index);
                let hover_shell = shell.clone();
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
                            let _ = hover_shell
                                .update(cx, |shell, cx| shell.open_menu_submenu(item_index, cx));
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
    /// the caller and shared with [`Self::render_inline_titlebar_menu`].
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
        let t = &theme.typography;
        let shell = cx.entity().downgrade();
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
                        let main_panel_left = menu_panel_left(open_index, menu_labels, d);
                        let raw_left = main_panel_left
                            + menu_panel_width
                            + d.menu_panel_gap;
                        let submenu_width = menu_panel_width_for_labels(&submenu_labels, d);
                        let left = if raw_left + submenu_width > viewport_width - 8.0
                            && main_panel_left >= submenu_width + d.menu_panel_gap
                        {
                            main_panel_left - submenu_width - d.menu_panel_gap
                        } else {
                            raw_left
                        };
                        let ideal_top = submenu_panel_top(&menu_items, submenu_index, d);
                        let total_submenu_height =
                            menu_items_visual_height_with_gaps(&submenu.items, d)
                                + d.menu_panel_padding * 2.0;
                        let top = if top_offset + ideal_top + total_submenu_height
                            > viewport_height - 16.0
                        {
                            (viewport_height - top_offset - total_submenu_height - 16.0)
                                .max(d.menu_panel_top)
                        } else {
                            ideal_top
                        };
                        let max_panel_height = (viewport_height - (top_offset + top) - 16.0)
                            .max(d.menu_item_height * 3.0);
                        let submenu_items = submenu.items.clone().into_iter().enumerate().map(
                            |(item_index, item)| match item {
                                OwnedMenuItem::Separator => div()
                                    .id((
                                        "app-submenu-separator",
                                        submenu_index * 1000 + item_index,
                                    ))
                                    .flex_shrink_0()
                                    .mx(px(d.menu_separator_margin_x))
                                    .my(px(d.menu_separator_margin_y))
                                    .h(px(d.menu_separator_height))
                                    .bg(c.dialog_border)
                                    .into_any_element(),
                                OwnedMenuItem::Action { name, action, .. } => {
                                    let is_disabled =
                                        action.as_ref().as_any().is::<NoRecentFiles>();
                                    let click_shell = shell.clone();
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
                                        .flex_shrink_0()
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
                                            div()
                                                .flex_1()
                                                .min_w(px(0.0))
                                                .truncate()
                                                .child(name.clone())
                                                .into_any_element()
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
                                            .on_click(move |_event, window, cx| {
                                                let _ = click_shell.update(cx, |shell, cx| {
                                                    shell.close_menu_bar(cx)
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
                                    .flex_shrink_0()
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
                                    .flex_shrink_0()
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
                                .max_h(px(max_panel_height))
                                .overflow_y_scroll()
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
            let scroll_region = (!scroll_items.is_empty()).then(|| {
                div()
                    .id(("app-menu-scroll-region", open_index))
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
                                        shell.clone(),
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
                            shell.clone(),
                            editor.clone(),
                            cx,
                        )
                    });

            main_panel
                .children(scroll_region)
                .children(footer_elements)
                .into_any_element()
        } else {
            let items = menu_items
                .iter()
                .cloned()
                .enumerate()
                .map(|(item_index, item)| {
                    self.render_in_window_menu_item(
                        item,
                        item_index,
                        theme,
                        shell.clone(),
                        editor.clone(),
                        cx,
                    )
                });

            let max_main_height = (viewport_height - (top_offset + d.menu_panel_top) - 16.0)
                .max(d.menu_item_height * 3.0);
            main_panel
                .max_h(px(max_main_height))
                .overflow_y_scroll()
                .children(items)
                .into_any_element()
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

/// Detects the trailing "Add Theme Config" / "Add Language Config" items
/// pinned to the bottom of the theme / language import menus, returning the
/// split index (start of the pinned footer).
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

