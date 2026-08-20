//! Interface settings tab for the in-editor slide-over panel.

use gpui::*;

use crate::app::shell::Shell;
use crate::infra::theme::{Theme, ThemeManager};
use crate::settings::panels::common::{make_row, make_section};
use crate::ui::select::{select_option, select_panel, select_trigger};
use crate::ui::switch::Switch;

impl Shell {
    pub(crate) fn render_panel_interface_tab(
        &mut self,
        panel_id: usize,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let mut inner_border_color = c.dialog_border;
        inner_border_color.a *= 0.4;

        let mut sections: Vec<AnyElement> = Vec::new();

        // Section 1: Visual Theme & Language
        let sec1_key = "theme";
        let is_sec1_expanded = self.panels.settings.expanded_sections.contains(sec1_key);
        let mut sec1_items = Vec::new();

        let theme_shell = cx.entity().downgrade();
        let available_themes = cx.global::<ThemeManager>().available_themes();
        let current_theme_name = theme.name.clone();

        let lang_shell = cx.entity().downgrade();
        let lang_options = [("en-US", "English (en-US)"), ("zh-CN", "简体中文 (zh-CN)")];
        let current_lang = "English (en-US)";

        let is_theme_open = self.panels.settings.open_dropdown.as_deref() == Some("theme");
        let is_lang_open = self.panels.settings.open_dropdown.as_deref() == Some("lang");

        if is_sec1_expanded {
            let theme_icon_path = if current_theme_name == "Light" {
                "icons/settings/sun.svg"
            } else {
                "icons/settings/moon.svg"
            };

            let mut theme_btn_wrap = div().relative().child(
                select_trigger("pref-btn-theme", c, d)
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
                                    .child(current_theme_name.clone()),
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
                    .on_click({
                        let theme_shell = theme_shell.clone();
                        move |_event, _window, cx| {
                            let _ = theme_shell.update(cx, |shell, cx| {
                                if shell.panels.settings.open_dropdown.as_deref() == Some("theme") {
                                    shell.panels.settings.open_dropdown = None;
                                } else {
                                    shell.panels.settings.open_dropdown = Some("theme".to_string());
                                }
                                cx.notify();
                            });
                        }
                    }),
            );

            if is_theme_open {
                let mut menu_items = Vec::new();
                for t_entry in available_themes {
                    let t_id = t_entry.id.clone();
                    let display_label = t_entry.name.clone();
                    let is_selected = display_label == current_theme_name;
                    let item_shell = theme_shell.clone();
                    let item_icon = if display_label == "Light" {
                        "icons/settings/sun.svg"
                    } else {
                        "icons/settings/moon.svg"
                    };

                    menu_items.push(
                        select_option(ElementId::Name(format!("theme-item-{}", t_id).into()), c)
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
                            .on_click(move |_event, _window, cx| {
                                let _ = item_shell.update(cx, |shell, cx| {
                                    cx.update_global::<ThemeManager, _>(|manager, _cx| {
                                        let _ = manager.set_theme_by_id(&t_id);
                                    });
                                    shell.panels.settings.open_dropdown = None;
                                    cx.notify();
                                });
                            })
                            .into_any_element(),
                    );
                }

                theme_btn_wrap =
                    theme_btn_wrap.child(gpui::deferred(select_panel(c).children(menu_items)));
            }

            sec1_items.push(make_row(
                "Interface Theme",
                "Customize overall application color scheme and appearance",
                theme_btn_wrap.into_any_element(),
                theme,
                inner_border_color,
            ));

            let mut lang_btn_wrap = div().relative().child(
                select_trigger("pref-btn-lang", c, d)
                    .text_size(px(12.0))
                    .text_color(c.text_default)
                    .child(div().flex_1().min_w(px(0.0)).truncate().child(current_lang))
                    .child(
                        div().flex_shrink_0().pl(px(4.0)).child(
                            svg()
                                .path("icons/settings/select-chevron.svg")
                                .size(px(16.0))
                                .text_color(c.dialog_muted),
                        ),
                    )
                    .on_click({
                        let lang_shell = lang_shell.clone();
                        move |_event, _window, cx| {
                            let _ = lang_shell.update(cx, |shell, cx| {
                                if shell.panels.settings.open_dropdown.as_deref() == Some("lang") {
                                    shell.panels.settings.open_dropdown = None;
                                } else {
                                    shell.panels.settings.open_dropdown = Some("lang".to_string());
                                }
                                cx.notify();
                            });
                        }
                    }),
            );

            if is_lang_open {
                let mut menu_items = Vec::new();
                for (code, label) in lang_options {
                    let is_selected = label == current_lang;
                    let item_shell = lang_shell.clone();

                    menu_items.push(
                        select_option(ElementId::Name(format!("lang-item-{}", code).into()), c)
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
                            .on_click(move |_event, _window, cx| {
                                let _ = item_shell.update(cx, |shell, cx| {
                                    shell.panels.settings.open_dropdown = None;
                                    cx.notify();
                                });
                            })
                            .into_any_element(),
                    );
                }

                lang_btn_wrap =
                    lang_btn_wrap.child(gpui::deferred(select_panel(c).children(menu_items)));
            }

            sec1_items.push(make_row(
                "Display Language",
                "Select preferred language for editor UI and dialogs",
                lang_btn_wrap.into_any_element(),
                theme,
                inner_border_color,
            ));
        }

        let sec1_shell = cx.entity().downgrade();
        sections.push(make_section(
            "pref-sec-theme",
            "Visual Theme & Language",
            is_sec1_expanded,
            Box::new(move |_event, _window, cx| {
                let _ = sec1_shell.update(cx, |shell, cx| {
                    shell.panels.settings.toggle_section(sec1_key);
                    cx.notify();
                });
            }),
            sec1_items,
            theme,
            panel_id,
        ));

        // Section 2: Status Bar Options
        let sec2_key = "status_bar";
        let is_sec2_expanded = self.panels.settings.expanded_sections.contains(sec2_key);
        let mut sec2_items = Vec::new();

        if is_sec2_expanded {
            let sub1_shell = cx.entity().downgrade();
            let ctrl_sb_main = Switch::new("switch-sb-main")
                .checked(self.panels.settings.pref_show_status_bar)
                .on_click(move |_event, _window, cx| {
                    let _ = sub1_shell.update(cx, |shell, cx| {
                        shell.panels.settings.pref_show_status_bar =
                            !shell.panels.settings.pref_show_status_bar;
                        cx.notify();
                    });
                })
                .into_any_element();

            sec2_items.push(make_row(
                "Status Bar Visibility",
                "Show or hide the persistent bottom status bar across window",
                ctrl_sb_main,
                theme,
                inner_border_color,
            ));

            let sub2_shell = cx.entity().downgrade();
            let ctrl_sb_words = Switch::new("switch-sb-words")
                .checked(self.panels.settings.pref_show_word_count)
                .on_click(move |_event, _window, cx| {
                    let _ = sub2_shell.update(cx, |shell, cx| {
                        shell.panels.settings.pref_show_word_count =
                            !shell.panels.settings.pref_show_word_count;
                        cx.notify();
                    });
                })
                .into_any_element();

            sec2_items.push(make_row(
                "Word Count Badge",
                "Display real-time document word count in status bar",
                ctrl_sb_words,
                theme,
                inner_border_color,
            ));

            let sub3_shell = cx.entity().downgrade();
            let ctrl_sb_pos = Switch::new("switch-sb-pos")
                .checked(self.panels.settings.pref_show_cursor_pos)
                .on_click(move |_event, _window, cx| {
                    let _ = sub3_shell.update(cx, |shell, cx| {
                        shell.panels.settings.pref_show_cursor_pos =
                            !shell.panels.settings.pref_show_cursor_pos;
                        cx.notify();
                    });
                })
                .into_any_element();

            sec2_items.push(make_row(
                "Cursor Position Badge",
                "Display line and column coordinates in status bar",
                ctrl_sb_pos,
                theme,
                inner_border_color,
            ));
        }

        let sec2_shell = cx.entity().downgrade();
        sections.push(make_section(
            "pref-sec-sb",
            "Status Bar Options",
            is_sec2_expanded,
            Box::new(move |_event, _window, cx| {
                let _ = sec2_shell.update(cx, |shell, cx| {
                    shell.panels.settings.toggle_section(sec2_key);
                    cx.notify();
                });
            }),
            sec2_items,
            theme,
            panel_id,
        ));

        sections
    }
}
