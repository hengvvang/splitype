//! Window chrome titlebar orchestration.

use gpui::*;

use crate::shell::Shell;
use theme::Theme;
use ui::custom_titlebar::{custom_titlebar_height, render_custom_titlebar};
use ui::menu_bar::supports_in_window_menu;

impl Shell {
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
        let menus = supports_in_window_menu()
            .then(|| cx.get_menus())
            .flatten()
            .filter(|m| !m.is_empty());
        let menu_labels: Vec<SharedString> = menus
            .as_ref()
            .map(|m| m.iter().map(|menu| menu.name.clone()).collect())
            .unwrap_or_default();
        let inline_menu =
            self.render_inline_titlebar_menu(theme, cx, menus.as_deref(), &menu_labels);

        // The editor titlebar stays minimal: no document name, just the
        // drag area, the menu, and the window controls.
        let titlebar = render_custom_titlebar(
            "editor-titlebar",
            SharedString::default(),
            inline_menu,
            theme,
            window,
            cx,
            Shell::on_titlebar_close,
        );

        let target_panel_id = self.active_document_panel_id();
        let menu_panel = self.render_in_window_menu_panel(
            theme,
            cx,
            menus.as_deref(),
            &menu_labels,
            titlebar_height,
            window.viewport_size(),
            target_panel_id,
        );

        (titlebar, menu_panel, titlebar_height)
    }
}
