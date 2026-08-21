//! Interface settings tab: Visual Theme, Display Language, and Status Bar Options.

use gpui::*;

use crate::infra::config::settings::apply_configured_language;
use crate::infra::i18n::manager::I18nManager;
use crate::infra::theme::Theme;
use crate::settings::tabs::common::{make_row, make_section};
use crate::settings::window::SettingsWindow;
use crate::ui::select::{select_option, select_panel, select_trigger};
use crate::ui::switch::Switch;

impl SettingsWindow {
    pub(crate) fn render_interface_tab(
        &mut self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let mut inner_border_color = c.dialog_border;
        inner_border_color.a *= 0.4;
        let toggle_section_ed = cx.entity().downgrade();

        let mut sections: Vec<AnyElement> = Vec::new();

        // Section 1: Visual Theme & Language
        let sec1_key = "theme";
        let mut sec1_items = Vec::new();

        let selected_theme_label = self.selected_theme_name();
        let theme_btn_ed = cx.entity().downgrade();
        let theme_icon_path = if selected_theme_label == "Light" {
            "icons/settings/sun.svg"
        } else {
            "icons/settings/moon.svg"
        };

        let mut theme_btn_wrap = div().relative().child(
            select_trigger("pref-btn-win-theme", c, d)
                .text_size(px(12.0))
                .text_color(c.text_default)
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            svg()
                                .path(theme_icon_path)
                                .size(px(15.0))
                                .text_color(c.text_default),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .truncate()
                                .child(selected_theme_label),
                        ),
                )
                .child(
                    div().flex_shrink_0().pl(px(4.0)).child(
                        svg()
                            .path("icons/settings/select-chevron.svg")
                            .size(px(16.0))
                            .text_color(c.dialog_muted),
                    ),
                )
                .on_click(move |_event, _window, cx| {
                    let _ = theme_btn_ed.update(cx, |this, cx| {
                        this.theme_dropdown_open = !this.theme_dropdown_open;
                        this.lang_dropdown_open = false;
                        this.startup_dropdown_open = false;
                        this.image_dropdown_open = false;
                        cx.notify();
                    });
                }),
        );

        if self.theme_dropdown_open {
            let mut menu_items = Vec::new();
            for entry in &self.theme_options {
                let t_id = entry.id.clone();
                let display_label = entry.name.clone();
                let is_selected = t_id == self.selected_theme_id;
                let item_ed = cx.entity().downgrade();
                let item_icon = if display_label == "Light" {
                    "icons/settings/sun.svg"
                } else {
                    "icons/settings/moon.svg"
                };

                menu_items.push(
                    select_option(
                        ElementId::Name(format!("win-theme-item-{}", t_id).into()),
                        c,
                        d,
                    )
                    .bg(if is_selected {
                        c.panel_row_selected
                    } else {
                        c.dialog_surface
                    })
                    .text_size(px(12.0))
                    .text_color(c.text_default)
                    .child(
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
                            .child(display_label),
                    )
                    .child(if is_selected {
                        svg()
                            .path("icons/settings/checkmark.svg")
                            .size(px(15.0))
                            .text_color(c.dialog_primary_button_bg)
                            .into_any_element()
                    } else {
                        div().w(px(13.0)).into_any_element()
                    })
                    .on_click(move |event, window, cx| {
                        let _ = item_ed.update(cx, |this, cx| {
                            this.selected_theme_id = t_id.clone();
                            this.theme_dropdown_open = false;
                            this.save(event, window, cx);
                        });
                    })
                    .into_any_element(),
                );
            }

            theme_btn_wrap =
                theme_btn_wrap.child(gpui::deferred(select_panel(c, d).children(menu_items)));
        }

        sec1_items.push(make_row(
            inner_border_color,
            c,
            d,
            "Interface Theme",
            "Customize overall application color scheme and appearance",
            theme_btn_wrap.into_any_element(),
        ));

        let cur_lang = cx.global::<I18nManager>().current_language_id();
        let lang_display = if cur_lang == "zh-CN" {
            "简体中文 (zh-CN)"
        } else {
            "English (en-US)"
        };
        let lang_btn_ed = cx.entity().downgrade();
        let mut lang_btn_wrap = div().relative().child(
            select_trigger("pref-btn-win-lang", c, d)
                .text_size(px(12.0))
                .text_color(c.text_default)
                .child(div().flex_1().min_w(px(0.0)).truncate().child(lang_display))
                .child(
                    div().flex_shrink_0().pl(px(4.0)).child(
                        svg()
                            .path("icons/settings/select-chevron.svg")
                            .size(px(16.0))
                            .text_color(c.dialog_muted),
                    ),
                )
                .on_click(move |_event, _window, cx| {
                    let _ = lang_btn_ed.update(cx, |this, cx| {
                        this.lang_dropdown_open = !this.lang_dropdown_open;
                        this.theme_dropdown_open = false;
                        this.startup_dropdown_open = false;
                        this.image_dropdown_open = false;
                        cx.notify();
                    });
                }),
        );

        if self.lang_dropdown_open {
            let lang_opts = [("en-US", "English (en-US)"), ("zh-CN", "简体中文 (zh-CN)")];
            let mut menu_items = Vec::new();
            for (code, label) in lang_opts {
                let is_selected = label == lang_display;
                let item_ed = cx.entity().downgrade();

                menu_items.push(
                    select_option(ElementId::Name(format!("win-lang-item-{}", code).into()), c, d)
                        .bg(if is_selected {
                            c.panel_row_selected
                        } else {
                            c.dialog_surface
                        })
                        .text_size(px(12.0))
                        .text_color(c.text_default)
                        .child(label)
                        .child(if is_selected {
                            svg()
                                .path("icons/settings/checkmark.svg")
                                .size(px(15.0))
                                .text_color(c.dialog_primary_button_bg)
                                .into_any_element()
                        } else {
                            div().w(px(13.0)).into_any_element()
                        })
                        .on_click(move |event, window, cx| {
                            let _ = item_ed.update(cx, |this, cx| {
                                this.lang_dropdown_open = false;
                                let _ = apply_configured_language(cx, code);
                                this.save(event, window, cx);
                            });
                        })
                        .into_any_element(),
                );
            }

            lang_btn_wrap =
                lang_btn_wrap.child(gpui::deferred(select_panel(c, d).children(menu_items)));
        }

        sec1_items.push(make_row(
            inner_border_color,
            c,
            d,
            "Display Language",
            "Select preferred language for editor UI and dialogs",
            lang_btn_wrap.into_any_element(),
        ));

        sections.push(make_section(
            c,
            d,
            "win-sec-theme",
            sec1_key,
            "Visual Theme & Language",
            self.expanded_sections.contains(sec1_key),
            toggle_section_ed.clone(),
            sec1_items,
        ));

        // Section 2: Status Bar Options
        let sec2_key = "status_bar";
        let mut sec2_items = Vec::new();

        let sb_main_ed = cx.entity().downgrade();
        let ctrl_sb_main = Switch::new("win-switch-sb-main")
            .checked(self.status_bar_enabled)
            .on_click(move |event, window, cx| {
                let _ = sb_main_ed.update(cx, |this, cx| {
                    this.status_bar_enabled = !this.status_bar_enabled;
                    this.save(event, window, cx);
                });
            })
            .into_any_element();

        sec2_items.push(make_row(
            inner_border_color,
            c,
            d,
            "Status Bar Visibility",
            "Show or hide the persistent bottom status bar across window",
            ctrl_sb_main,
        ));

        let sb_words_ed = cx.entity().downgrade();
        let ctrl_sb_words = Switch::new("win-switch-sb-words")
            .checked(self.status_bar_show_word_count)
            .on_click(move |event, window, cx| {
                let _ = sb_words_ed.update(cx, |this, cx| {
                    this.status_bar_show_word_count = !this.status_bar_show_word_count;
                    this.save(event, window, cx);
                });
            })
            .into_any_element();

        sec2_items.push(make_row(
            inner_border_color,
            c,
            d,
            "Word Count Badge",
            "Display real-time document word count in status bar",
            ctrl_sb_words,
        ));

        let sb_pos_ed = cx.entity().downgrade();
        let ctrl_sb_pos = Switch::new("win-switch-sb-pos")
            .checked(self.status_bar_show_cursor_position)
            .on_click(move |event, window, cx| {
                let _ = sb_pos_ed.update(cx, |this, cx| {
                    this.status_bar_show_cursor_position = !this.status_bar_show_cursor_position;
                    this.save(event, window, cx);
                });
            })
            .into_any_element();

        sec2_items.push(make_row(
            inner_border_color,
            c,
            d,
            "Cursor Position Badge",
            "Display line and column coordinates in status bar",
            ctrl_sb_pos,
        ));

        sections.push(make_section(
            c,
            d,
            "win-sec-sb",
            sec2_key,
            "Status Bar Options",
            self.expanded_sections.contains(sec2_key),
            toggle_section_ed,
            sec2_items,
        ));

        sections
    }
}
