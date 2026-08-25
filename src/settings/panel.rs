//! In-window tiled settings panel host adapter for Shell.

use gpui::*;

use crate::app::shell::Shell;
use crate::app::window::panels::PanelId;
use crate::infra::config::settings::apply_configured_language;
use crate::infra::i18n::I18nStrings;
use crate::infra::i18n::manager::I18nManager;
use crate::infra::theme::{Theme, ThemeManager};
use crate::settings::components::*;
use crate::settings::state::SettingsTab;
use crate::ui::tab::nav_tab;

impl Shell {
    pub(crate) fn render_settings_body(
        &mut self,
        panel_id: PanelId,
        theme: &Theme,
        _strings: &I18nStrings,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let active_tab = self.panels.settings.tab;

        // Left Navigation Tabs
        let mut left_nav_items = Vec::new();
        for (tab_idx, tab) in SettingsTab::all().iter().enumerate() {
            let is_active = active_tab == *tab;
            let shell = cx.entity().downgrade();
            let tab_item = *tab;

            left_nav_items.push(
                nav_tab(
                    ElementId::Name(format!("pref-tab-{panel_id}-{tab_idx}").into()),
                    c,
                    d,
                )
                .id(ElementId::Name(
                    format!("pref-tab-{panel_id}-{tab_idx}").into(),
                ))
                .cursor_pointer()
                .flex()
                .items_center()
                .bg(if is_active {
                    c.panel_row_selected
                } else {
                    c.dialog_surface
                })
                .child(
                    div()
                        .text_size(px(13.0))
                        .font_weight(if is_active {
                            gpui::FontWeight::BOLD
                        } else {
                            gpui::FontWeight::NORMAL
                        })
                        .text_color(if is_active {
                            c.text_default
                        } else {
                            c.dialog_muted
                        })
                        .child(tab.name()),
                )
                .on_click(move |_event, _window, cx| {
                    let _ = shell.update(cx, |shell, cx| {
                        shell.panels.settings.tab = tab_item;
                        cx.notify();
                    });
                })
                .into_any_element(),
            );
        }

        let left_nav = div()
            .w(px(160.0))
            .h_full()
            .flex_shrink_0()
            .p(px(8.0))
            .border_r_1()
            .border_color(c.dialog_border)
            .flex()
            .flex_col()
            .gap(px(2.0))
            .children(left_nav_items);

        // Right Content Area composed from domain components
        let sections = match active_tab {
            SettingsTab::Interface => self.render_panel_interface_tab(panel_id, theme, cx),
            SettingsTab::Editing => self.render_panel_editing_tab(panel_id, theme, cx),
            SettingsTab::Keymap => self.render_panel_shortcuts_tab(panel_id, theme, cx),
        };

        let right_content = div()
            .id(("pref-right-content", panel_id.0))
            .relative()
            .flex_1()
            .h_full()
            .p(px(14.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .children(sections);

        div()
            .w_full()
            .h_full()
            .flex()
            .flex_row()
            .bg(c.editor_background)
            .child(left_nav)
            .child(right_content)
            .into_any_element()
    }

    pub(crate) fn toggle_settings_section_handler(
        &self,
        cx: &mut Context<Self>,
        key: &'static str,
    ) -> crate::settings::ui_helpers::SettingsClickHandler {
        let handle = cx.entity().downgrade();
        Box::new(move |_event, _window, cx| {
            let _ = handle.update(cx, |this, cx| {
                if this.panels.settings.expanded_sections.contains(key) {
                    this.panels.settings.expanded_sections.remove(key);
                } else {
                    this.panels.settings.expanded_sections.insert(key.to_string());
                }
                cx.notify();
            });
        })
    }

    fn render_panel_interface_tab(
        &mut self,
        panel_id: PanelId,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let mut sections = Vec::new();

        // 1. Theme & Language Section
        let sec1_key = "theme";
        let is_sec1_expanded = self.panels.settings.expanded_sections.contains(sec1_key);
        let toggle_theme_dd = cx.entity().downgrade();
        let toggle_lang_dd = cx.entity().downgrade();
        let select_theme_shell = cx.entity().downgrade();
        let select_lang_shell = cx.entity().downgrade();

        let available_themes = cx
            .global::<ThemeManager>()
            .available_themes()
            .iter()
            .map(|t| (t.id.clone(), t.name.clone()))
            .collect();

        let current_lang_name = match cx.try_global::<I18nManager>().map(|m| m.current_language_id()) {
            Some("zh-CN") => "简体中文 (zh-CN)".to_string(),
            _ => "English (en-US)".to_string(),
        };

        let theme_lang_props = ThemeLangProps {
            current_theme_name: theme.name.clone(),
            is_theme_dropdown_open: self.panels.settings.open_dropdown.as_deref() == Some("theme"),
            on_toggle_theme_dropdown: Box::new(move |_event, _window, cx| {
                let _ = toggle_theme_dd.update(cx, |shell, cx| {
                    if shell.panels.settings.open_dropdown.as_deref() == Some("theme") {
                        shell.panels.settings.open_dropdown = None;
                    } else {
                        shell.panels.settings.open_dropdown = Some("theme".to_string());
                    }
                    cx.notify();
                });
            }),
            available_themes,
            on_select_theme: Box::new(move |theme_id| {
                let shell = select_theme_shell.clone();
                Box::new(move |_event, _window, cx| {
                    let tid = theme_id.clone();
                    cx.update_global::<ThemeManager, _>(|tm, _cx| {
                        tm.set_theme_by_id(&tid);
                    });
                    let _ = shell.update(cx, |shell, cx| {
                        shell.panels.settings.open_dropdown = None;
                        cx.notify();
                    });
                    cx.refresh_windows();
                })
            }),
            current_lang_name,
            is_lang_dropdown_open: self.panels.settings.open_dropdown.as_deref() == Some("lang"),
            on_toggle_lang_dropdown: Box::new(move |_event, _window, cx| {
                let _ = toggle_lang_dd.update(cx, |shell, cx| {
                    if shell.panels.settings.open_dropdown.as_deref() == Some("lang") {
                        shell.panels.settings.open_dropdown = None;
                    } else {
                        shell.panels.settings.open_dropdown = Some("lang".to_string());
                    }
                    cx.notify();
                });
            }),
            lang_options: vec![
                ("en-US", "English (en-US)"),
                ("zh-CN", "简体中文 (zh-CN)"),
            ],
            on_select_lang: Box::new(move |lang_code| {
                let shell = select_lang_shell.clone();
                Box::new(move |_event, _window, cx| {
                    let _ = apply_configured_language(cx, lang_code);
                    let _ = shell.update(cx, |shell, cx| {
                        shell.panels.settings.open_dropdown = None;
                        cx.notify();
                    });
                    cx.refresh_windows();
                })
            }),
        };

        sections.push(render_theme_and_language_section(
            c,
            d,
            ("pref-sec-theme", panel_id.0),
            is_sec1_expanded,
            self.toggle_settings_section_handler(cx, sec1_key),
            theme_lang_props,
        ));

        // 2. Status Bar Section
        let sec2_key = "status_bar";
        let is_sec2_expanded = self.panels.settings.expanded_sections.contains(sec2_key);
        let toggle_sb = cx.entity().downgrade();
        let toggle_wc = cx.entity().downgrade();
        let toggle_cp = cx.entity().downgrade();

        let status_bar_props = StatusBarProps {
            show_status_bar: self.panels.settings.pref_show_status_bar,
            on_toggle_status_bar: Box::new(move |_event, _window, cx| {
                let _ = toggle_sb.update(cx, |shell, cx| {
                    shell.panels.settings.pref_show_status_bar = !shell.panels.settings.pref_show_status_bar;
                    cx.notify();
                });
            }),
            show_word_count: self.panels.settings.pref_show_word_count,
            on_toggle_word_count: Box::new(move |_event, _window, cx| {
                let _ = toggle_wc.update(cx, |shell, cx| {
                    shell.panels.settings.pref_show_word_count = !shell.panels.settings.pref_show_word_count;
                    cx.notify();
                });
            }),
            show_cursor_pos: self.panels.settings.pref_show_cursor_pos,
            on_toggle_cursor_pos: Box::new(move |_event, _window, cx| {
                let _ = toggle_cp.update(cx, |shell, cx| {
                    shell.panels.settings.pref_show_cursor_pos = !shell.panels.settings.pref_show_cursor_pos;
                    cx.notify();
                });
            }),
        };

        sections.push(render_status_bar_section(
            c,
            d,
            ("pref-sec-sb", panel_id.0),
            is_sec2_expanded,
            self.toggle_settings_section_handler(cx, sec2_key),
            status_bar_props,
        ));

        sections
    }

    fn render_panel_editing_tab(
        &mut self,
        panel_id: PanelId,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let mut sections = Vec::new();

        // 1. Typography Section
        let sec1_key = "typography";
        let is_sec1_expanded = self.panels.settings.expanded_sections.contains(sec1_key);
        let font_dec = cx.entity().downgrade();
        let font_inc = cx.entity().downgrade();
        let font_ctr = cx.entity().downgrade();
        let lh_dec = cx.entity().downgrade();
        let lh_inc = cx.entity().downgrade();
        let lh_ctr = cx.entity().downgrade();

        let typography_props = TypographyProps {
            font_size: self.panels.settings.pref_font_size,
            is_editing_font: self.panels.settings.editing_stepper.as_deref() == Some("font"),
            on_font_dec: Box::new(move |_event, _window, cx| {
                let _ = font_dec.update(cx, |shell, cx| {
                    shell.panels.settings.editing_stepper = None;
                    if shell.panels.settings.pref_font_size > 8 {
                        shell.panels.settings.pref_font_size -= 1;
                        cx.notify();
                    }
                });
            }),
            on_font_inc: Box::new(move |_event, _window, cx| {
                let _ = font_inc.update(cx, |shell, cx| {
                    shell.panels.settings.editing_stepper = None;
                    if shell.panels.settings.pref_font_size < 48 {
                        shell.panels.settings.pref_font_size += 1;
                        cx.notify();
                    }
                });
            }),
            on_font_cycle: Box::new(move |_event, _window, cx| {
                let _ = font_ctr.update(cx, |shell, cx| {
                    shell.panels.settings.editing_stepper = Some("font".to_string());
                    shell.panels.settings.pref_font_size = match shell.panels.settings.pref_font_size {
                        12 => 14,
                        14 => 16,
                        16 => 18,
                        18 => 20,
                        20 => 24,
                        24 => 12,
                        _ => 14,
                    };
                    cx.notify();
                });
            }),
            line_height: self.panels.settings.pref_line_height,
            is_editing_lh: self.panels.settings.editing_stepper.as_deref() == Some("line_height"),
            on_lh_dec: Box::new(move |_event, _window, cx| {
                let _ = lh_dec.update(cx, |shell, cx| {
                    shell.panels.settings.editing_stepper = None;
                    if shell.panels.settings.pref_line_height > 1.05 {
                        shell.panels.settings.pref_line_height -= 0.1;
                        cx.notify();
                    }
                });
            }),
            on_lh_inc: Box::new(move |_event, _window, cx| {
                let _ = lh_inc.update(cx, |shell, cx| {
                    shell.panels.settings.editing_stepper = None;
                    if shell.panels.settings.pref_line_height < 2.95 {
                        shell.panels.settings.pref_line_height += 0.1;
                        cx.notify();
                    }
                });
            }),
            on_lh_cycle: Box::new(move |_event, _window, cx| {
                let _ = lh_ctr.update(cx, |shell, cx| {
                    shell.panels.settings.editing_stepper = Some("line_height".to_string());
                    shell.panels.settings.pref_line_height =
                        if (shell.panels.settings.pref_line_height - 1.6).abs() < 0.05 {
                            1.8
                        } else if (shell.panels.settings.pref_line_height - 1.8).abs() < 0.05 {
                            2.0
                        } else if (shell.panels.settings.pref_line_height - 2.0).abs() < 0.05 {
                            1.4
                        } else {
                            1.6
                        };
                    cx.notify();
                });
            }),
        };

        sections.push(render_typography_section(
            c,
            d,
            ("pref-sec-typo", panel_id.0),
            is_sec1_expanded,
            self.toggle_settings_section_handler(cx, sec1_key),
            typography_props,
        ));

        // 2. Markdown Section
        let sec2_key = "markdown";
        let is_sec2_expanded = self.panels.settings.expanded_sections.contains(sec2_key);
        let toggle_tbl = cx.entity().downgrade();
        let toggle_paste_dd = cx.entity().downgrade();
        let select_paste_shell = cx.entity().downgrade();

        let markdown_props = MarkdownProps {
            show_table_headers: self.panels.settings.pref_show_table_headers,
            on_toggle_table_headers: Box::new(move |_event, _window, cx| {
                let _ = toggle_tbl.update(cx, |shell, cx| {
                    shell.panels.settings.pref_show_table_headers = !shell.panels.settings.pref_show_table_headers;
                    cx.notify();
                });
            }),
            image_paste_action: self.panels.settings.pref_image_paste_action,
            is_image_paste_open: self.panels.settings.open_dropdown.as_deref() == Some("image_paste"),
            on_toggle_image_paste: Box::new(move |_event, _window, cx| {
                let _ = toggle_paste_dd.update(cx, |shell, cx| {
                    if shell.panels.settings.open_dropdown.as_deref() == Some("image_paste") {
                        shell.panels.settings.open_dropdown = None;
                    } else {
                        shell.panels.settings.open_dropdown = Some("image_paste".to_string());
                    }
                    cx.notify();
                });
            }),
            on_select_image_paste: Box::new(move |idx| {
                let shell = select_paste_shell.clone();
                Box::new(move |_event, _window, cx| {
                    let _ = shell.update(cx, |shell, cx| {
                        shell.panels.settings.pref_image_paste_action = idx;
                        shell.panels.settings.open_dropdown = None;
                        cx.notify();
                    });
                })
            }),
        };

        sections.push(render_markdown_section(
            c,
            d,
            ("pref-sec-md", panel_id.0),
            is_sec2_expanded,
            self.toggle_settings_section_handler(cx, sec2_key),
            markdown_props,
        ));

        // 3. Startup Section
        let sec3_key = "startup";
        let is_sec3_expanded = self.panels.settings.expanded_sections.contains(sec3_key);
        let toggle_startup_dd = cx.entity().downgrade();
        let select_startup_shell = cx.entity().downgrade();

        let startup_props = StartupProps {
            startup_option: self.panels.settings.pref_startup_option,
            is_startup_open: self.panels.settings.open_dropdown.as_deref() == Some("startup"),
            on_toggle_startup: Box::new(move |_event, _window, cx| {
                let _ = toggle_startup_dd.update(cx, |shell, cx| {
                    if shell.panels.settings.open_dropdown.as_deref() == Some("startup") {
                        shell.panels.settings.open_dropdown = None;
                    } else {
                        shell.panels.settings.open_dropdown = Some("startup".to_string());
                    }
                    cx.notify();
                });
            }),
            on_select_startup: Box::new(move |idx| {
                let shell = select_startup_shell.clone();
                Box::new(move |_event, _window, cx| {
                    let _ = shell.update(cx, |shell, cx| {
                        shell.panels.settings.pref_startup_option = idx;
                        shell.panels.settings.open_dropdown = None;
                        cx.notify();
                    });
                })
            }),
        };

        sections.push(render_startup_section(
            c,
            d,
            ("pref-sec-startup", panel_id.0),
            is_sec3_expanded,
            self.toggle_settings_section_handler(cx, sec3_key),
            startup_props,
        ));

        sections
    }

    fn render_panel_shortcuts_tab(
        &mut self,
        panel_id: PanelId,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let mut sections = Vec::new();

        let sec1_key = "doc_actions";
        let is_sec1_expanded = self.panels.settings.expanded_sections.contains(sec1_key);
        sections.push(render_shortcuts_section(
            c,
            d,
            ("pref-sec-doc-actions", panel_id.0),
            "Document Actions",
            is_sec1_expanded,
            self.toggle_settings_section_handler(cx, sec1_key),
            crate::settings::components::shortcuts_data::doc_action_shortcuts(),
        ));

        let sec2_key = "view_controls";
        let is_sec2_expanded = self.panels.settings.expanded_sections.contains(sec2_key);
        sections.push(render_shortcuts_section(
            c,
            d,
            ("pref-sec-view-controls", panel_id.0),
            "Interface & View Controls",
            is_sec2_expanded,
            self.toggle_settings_section_handler(cx, sec2_key),
            crate::settings::components::shortcuts_data::interface_view_shortcuts(),
        ));

        sections
    }
}
