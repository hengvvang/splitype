//! Standalone settings window view and window opening lifecycle.

use std::collections::BTreeMap;

use gpui::*;

use crate::editor::keybindings::install_keybindings;
use crate::infra::config::keybindings::normalize_shortcut_config;
use crate::infra::config::settings::{
    AppSettings, DEFAULT_THEME_ID, EditorSettings, ImagePasteBehavior, StartupOpenSetting,
    StatusBarSettings, apply_configured_language, read_app_settings, save_settings_from_window,
};
use crate::infra::i18n::manager::I18nManager;
use crate::infra::theme::{ThemeCatalogEntry, ThemeManager};
use crate::settings::state::SettingsTab;
use crate::ui::custom_titlebar::{
    custom_titlebar_height, render_custom_titlebar, splitype_window_options,
};
use crate::ui::tab::nav_tab;

/// Independent settings window view.
pub(crate) struct SettingsWindow {
    pub(crate) nav: SettingsTab,
    pub(crate) startup_open: StartupOpenSetting,
    pub(crate) selected_theme_id: String,
    pub(crate) image_paste_behavior: ImagePasteBehavior,
    pub(crate) keybindings: BTreeMap<String, Vec<String>>,
    pub(crate) saved_startup_open: StartupOpenSetting,
    pub(crate) saved_theme_id: String,
    pub(crate) saved_image_paste_behavior: ImagePasteBehavior,
    pub(crate) saved_keybindings: BTreeMap<String, Vec<String>>,
    pub(crate) theme_options: Vec<ThemeCatalogEntry>,
    pub(crate) focus_handle: FocusHandle,
    pub(crate) startup_dropdown_open: bool,
    pub(crate) theme_dropdown_open: bool,
    pub(crate) lang_dropdown_open: bool,
    pub(crate) image_dropdown_open: bool,
    pub(crate) font_size: u32,
    pub(crate) line_height: f32,
    pub(crate) editing_stepper: Option<String>,
    pub(crate) status_bar_enabled: bool,
    pub(crate) status_bar_show_word_count: bool,
    pub(crate) status_bar_show_cursor_position: bool,
    pub(crate) saved_status_bar_enabled: bool,
    pub(crate) saved_status_bar_show_word_count: bool,
    pub(crate) saved_status_bar_show_cursor_position: bool,
    pub(crate) expanded_sections: std::collections::HashSet<String>,
}

impl SettingsWindow {
    fn new(
        settings: AppSettings,
        theme_options: Vec<ThemeCatalogEntry>,
        cx: &mut Context<Self>,
    ) -> Self {
        let selected_theme_id = if theme_options
            .iter()
            .any(|entry| entry.id == settings.default_theme_id)
        {
            settings.default_theme_id
        } else {
            DEFAULT_THEME_ID.into()
        };
        let startup_open = settings.startup_open;
        let image_paste_behavior = settings.image_paste_behavior;
        let keybindings = settings.keybindings;

        let mut expanded_sections = std::collections::HashSet::new();
        expanded_sections.insert("theme".to_string());
        expanded_sections.insert("status_bar".to_string());
        expanded_sections.insert("typography".to_string());
        expanded_sections.insert("markdown".to_string());
        expanded_sections.insert("startup".to_string());
        expanded_sections.insert("doc_actions".to_string());
        expanded_sections.insert("view_controls".to_string());
        expanded_sections.insert("keymap".to_string());

        Self {
            nav: SettingsTab::Interface,
            startup_open,
            selected_theme_id: selected_theme_id.clone(),
            image_paste_behavior,
            keybindings: keybindings.clone(),
            saved_startup_open: startup_open,
            saved_theme_id: selected_theme_id,
            saved_image_paste_behavior: image_paste_behavior,
            saved_keybindings: keybindings,
            theme_options,
            focus_handle: cx.focus_handle(),
            startup_dropdown_open: false,
            theme_dropdown_open: false,
            lang_dropdown_open: false,
            image_dropdown_open: false,
            font_size: 14,
            line_height: 1.6,
            editing_stepper: None,
            status_bar_enabled: settings.status_bar.enabled,
            status_bar_show_word_count: settings.status_bar.show_word_count,
            status_bar_show_cursor_position: settings.status_bar.show_cursor_position,
            saved_status_bar_enabled: settings.status_bar.enabled,
            saved_status_bar_show_word_count: settings.status_bar.show_word_count,
            saved_status_bar_show_cursor_position: settings.status_bar.show_cursor_position,
            expanded_sections,
        }
    }

    pub(crate) fn selected_theme_name(&self) -> String {
        self.theme_options
            .iter()
            .find(|entry| entry.id == self.selected_theme_id)
            .map(|entry| entry.name.clone())
            .unwrap_or_else(|| "splitype".into())
    }

    fn has_unsaved_changes(&self) -> bool {
        self.startup_open != self.saved_startup_open
            || self.selected_theme_id != self.saved_theme_id
            || self.image_paste_behavior != self.saved_image_paste_behavior
            || normalize_shortcut_config(&self.keybindings)
                != normalize_shortcut_config(&self.saved_keybindings)
            || self.status_bar_enabled != self.saved_status_bar_enabled
            || self.status_bar_show_word_count != self.saved_status_bar_show_word_count
            || self.status_bar_show_cursor_position != self.saved_status_bar_show_cursor_position
    }

    fn on_titlebar_close(
        &mut self,
        event: &ClickEvent,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        if event.standard_click() {
            window.remove_window();
        }
    }

    pub(crate) fn save(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_unsaved_changes() {
            return;
        }

        let settings = match save_settings_from_window(
            self.startup_open,
            &self.selected_theme_id,
            self.image_paste_behavior,
            self.keybindings.clone(),
            &StatusBarSettings {
                enabled: self.status_bar_enabled,
                show_word_count: self.status_bar_show_word_count,
                show_cursor_position: self.status_bar_show_cursor_position,
            },
        ) {
            Ok(settings) => settings,
            Err(err) => {
                let strings = cx.global::<I18nManager>().strings().clone();
                let ok = strings.info_dialog_ok;
                let buttons = [ok.as_str()];
                drop(window.prompt(
                    PromptLevel::Critical,
                    &strings.settings_save_failed_title,
                    Some(&err.to_string()),
                    &buttons,
                    cx,
                ));
                return;
            }
        };

        self.apply_saved_settings(settings, window, cx);
    }

    fn apply_saved_settings(
        &mut self,
        settings: AppSettings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let theme_changed = cx.update_global::<ThemeManager, _>(|theme_manager, _cx| {
            theme_manager.set_theme_by_id(&settings.default_theme_id)
        });
        if !theme_changed {
            let _ = cx.update_global::<ThemeManager, _>(|theme_manager, _cx| {
                theme_manager.set_theme_by_id(DEFAULT_THEME_ID)
            });
        }
        cx.clear_key_bindings();
        install_keybindings(cx, &settings.keybindings);
        crate::app::menus::install_menus(cx);
        cx.update_global::<EditorSettings, _>(|ed_settings, _cx| {
            ed_settings.status_bar_settings.enabled = settings.status_bar.enabled;
            ed_settings.status_bar_settings.show_word_count = settings.status_bar.show_word_count;
            ed_settings.status_bar_settings.show_cursor_position =
                settings.status_bar.show_cursor_position;
        });

        let _strings = cx.global::<I18nManager>().strings().clone();
        let _ = apply_configured_language(cx, &settings.default_language_id);

        self.saved_startup_open = settings.startup_open;
        self.saved_theme_id = settings.default_theme_id.clone();
        self.saved_image_paste_behavior = settings.image_paste_behavior;
        self.saved_keybindings = settings.keybindings;
        self.saved_status_bar_enabled = settings.status_bar.enabled;
        self.saved_status_bar_show_word_count = settings.status_bar.show_word_count;
        self.saved_status_bar_show_cursor_position = settings.status_bar.show_cursor_position;

        self.selected_theme_id = settings.default_theme_id;
        self.startup_open = settings.startup_open;
        self.image_paste_behavior = settings.image_paste_behavior;

        window.refresh();
        cx.refresh_windows();
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ThemeManager>().current().clone();
        let strings = cx.global::<I18nManager>().strings().clone();
        let c = &theme.colors;
        let d = &theme.dimensions;
        let window_title = SharedString::from(strings.settings_window_title.clone());
        window.set_window_title(window_title.as_ref());
        let titlebar_height = custom_titlebar_height(window, d);

        // Left Sidebar Navigation
        let nav_item = |id: &'static str, label: &'static str, is_selected: bool| -> AnyElement {
            nav_tab(id, c, d)
                .bg(if is_selected {
                    c.panel_row_selected
                } else {
                    c.dialog_surface
                })
                .active(|this| this.opacity(0.92))
                .text_size(px(d.menu_text_size))
                .font_weight(if is_selected {
                    gpui::FontWeight::BOLD
                } else {
                    gpui::FontWeight::NORMAL
                })
                .text_color(if is_selected {
                    c.text_default
                } else {
                    c.dialog_muted
                })
                .child(label)
                .into_any_element()
        };

        let nav_interface = div()
            .id("win-nav-wrap-1")
            .w_full()
            .child(nav_item(
                "nav-interface",
                "Interface",
                self.nav == SettingsTab::Interface,
            ))
            .on_click({
                let ed = cx.entity().downgrade();
                move |_event, _window, cx| {
                    let _ = ed.update(cx, |this, cx| {
                        this.nav = SettingsTab::Interface;
                        cx.notify();
                    });
                }
            });

        let nav_editing = div()
            .id("win-nav-wrap-2")
            .w_full()
            .child(nav_item(
                "nav-editing",
                "Editing",
                self.nav == SettingsTab::Editing,
            ))
            .on_click({
                let ed = cx.entity().downgrade();
                move |_event, _window, cx| {
                    let _ = ed.update(cx, |this, cx| {
                        this.nav = SettingsTab::Editing;
                        cx.notify();
                    });
                }
            });

        let nav_keymap = div()
            .id("win-nav-wrap-3")
            .w_full()
            .child(nav_item(
                "nav-keymap",
                "Keymap",
                self.nav == SettingsTab::Keymap,
            ))
            .on_click({
                let ed = cx.entity().downgrade();
                move |_event, _window, cx| {
                    let _ = ed.update(cx, |this, cx| {
                        this.nav = SettingsTab::Keymap;
                        cx.notify();
                    });
                }
            });

        let left_nav = div()
            .id("win-pref-left-nav")
            .w(px(160.0))
            .h_full()
            .flex_shrink_0()
            .p(px(8.0))
            .border_r_1()
            .border_color(c.dialog_border)
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(nav_interface)
            .child(nav_editing)
            .child(nav_keymap);

        let sections = match self.nav {
            SettingsTab::Interface => self.render_interface_tab(&theme, cx),
            SettingsTab::Editing => self.render_editing_tab(&theme, cx),
            SettingsTab::Keymap => self.render_shortcuts_tab(&theme, cx),
        };

        let right_content = div()
            .id("win-pref-right-content")
            .relative()
            .w_full()
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .p(px(14.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .children(sections);

        let main_body = div()
            .w_full()
            .h_full()
            .flex()
            .flex_row()
            .child(left_nav)
            .child(right_content);

        let content = div()
            .size_full()
            .pt(px(titlebar_height))
            .flex()
            .flex_col()
            .key_context("Settings")
            .track_focus(&self.focus_handle)
            .bg(c.editor_background)
            .text_color(c.dialog_body)
            .child(main_body);

        let root = div()
            .size_full()
            .relative()
            .bg(c.editor_background)
            .child(content);

        if let Some(titlebar) = render_custom_titlebar(
            "win-pref-titlebar",
            window_title,
            None,
            &theme,
            window,
            cx,
            Self::on_titlebar_close,
        ) {
            root.child(titlebar)
        } else {
            root
        }
    }
}

fn open_settings_window_with_state(
    cx: &mut App,
    settings: AppSettings,
    theme_options: Vec<ThemeCatalogEntry>,
    title: String,
) -> Option<WindowHandle<SettingsWindow>> {
    let bounds = Bounds::centered(None, size(px(720.0), px(480.0)), cx);
    let window_title = SharedString::from(title);
    let handle = match cx.open_window(
        splitype_window_options(window_title, bounds),
        move |_window, cx| cx.new(move |cx| SettingsWindow::new(settings, theme_options, cx)),
    ) {
        Ok(handle) => handle,
        Err(err) => {
            tracing::error!(error = %err, "failed to open settings window");
            return None;
        }
    };

    if let Err(err) = handle.update(cx, |settings_win, window, _cx| {
        window.activate_window();
        settings_win.focus_handle.focus(window);
    }) {
        tracing::warn!(error = %err, "failed to activate settings window");
    }

    Some(handle)
}

pub(crate) fn open_settings_window(cx: &mut App) -> Option<WindowHandle<SettingsWindow>> {
    let settings = match read_app_settings() {
        Ok(settings) => settings,
        Err(err) => {
            tracing::warn!(error = %err, "failed to read app settings, falling back to default");
            AppSettings::default()
        }
    };
    let theme_options = cx.global::<ThemeManager>().available_themes().to_vec();
    let title = cx
        .global::<I18nManager>()
        .strings()
        .settings_window_title
        .clone();
    open_settings_window_with_state(cx, settings, theme_options, title)
}

#[cfg(test)]
mod tests {
    use super::{AppSettings, StartupOpenSetting};

    #[test]
    fn test_app_settings_default() {
        let settings = AppSettings::default();
        assert_eq!(settings.startup_open, StartupOpenSetting::NewFile);
        assert_eq!(settings.default_language_id, "en-US");
        assert_eq!(settings.default_theme_id, "splitype");
    }
}
