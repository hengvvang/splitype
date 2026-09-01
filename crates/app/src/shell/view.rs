//! Render implementation for the root Shell entity.

use gpui::*;

use super::Shell;
use config::language::I18nManager;
use theme::ThemeManager;

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.last_viewport = Some(window.viewport_size());
        self.push_active_document_context(cx);
        self.install_close_guard(window, cx);

        let theme = cx.global::<ThemeManager>().current_arc();
        let strings = cx.global::<I18nManager>().strings_arc();
        let (titlebar, menu_panel, titlebar_height) = self.render_window_chrome(&theme, window, cx);

        let mut base = div()
            .track_focus(&self.focus_handle)
            .key_context("Shell")
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .relative()
            .bg(theme.colors.editor_background)
            .font(theme::TypographyStore::ui_font(cx))
            .on_action(cx.listener(Self::on_close_window))
            .on_action(cx.listener(Self::on_toggle_explorer_action))
            .on_action(cx.listener(Self::on_toggle_maximize_area_action))
            .on_action(cx.listener(Self::on_close_explorer_folder_action))
            .on_action(cx.listener(Self::on_quit_application))
            .on_action(cx.listener(Self::on_install_cli_tool))
            .on_action(cx.listener(Self::on_uninstall_cli_tool))
            .on_action(cx.listener(Self::on_toggle_kind_dropdown))
            .on_action(cx.listener(Self::on_split_panel))
            .on_action(cx.listener(Self::on_toggle_panel_maximized))
            .on_action(cx.listener(Self::on_close_panel))
            .on_action(cx.listener(Self::on_update_open_tab_paths))
            .on_action(cx.listener(Self::on_open_in_editor))
            .on_action(cx.listener(Self::on_open_in_split))
            .on_any_mouse_down(cx.listener(Self::on_body_mouse_down));

        if let Some(titlebar) = titlebar {
            base = base.child(titlebar);
        }

        let body = div()
            .w_full()
            .flex_1()
            .min_h(px(0.0))
            .pt(px(titlebar_height))
            .flex()
            .min_w(px(0.0))
            .child(self.render_tiled_layout(&theme, &strings, window, cx));
        base = base.child(body);

        if let Some(menu_panel) = menu_panel {
            base = base.child(menu_panel);
        }

        for view in self.panel_views.values_mut() {
            if let Some(overlay) = view.render_overlay(window, cx) {
                base = base.child(overlay);
            }
        }

        if let Some(dialog) = self.render_window_dialogs(&theme, cx) {
            base = base.child(dialog);
        }

        base.into_any_element()
    }
}
