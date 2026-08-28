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
use crate::infra::theme::{Theme, ThemeCatalogEntry, ThemeManager};
use crate::settings::components::*;
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
    pub(crate) saved_font_size: u32,
    pub(crate) saved_line_height: f32,
    pub(crate) ui_font_family: Option<String>,
    pub(crate) prose_font_family: Option<String>,
    pub(crate) code_font_family: Option<String>,
    pub(crate) saved_ui_font_family: Option<String>,
    pub(crate) saved_prose_font_family: Option<String>,
    pub(crate) saved_code_font_family: Option<String>,
    pub(crate) ui_font_dropdown_open: bool,
    pub(crate) prose_font_dropdown_open: bool,
    pub(crate) code_font_dropdown_open: bool,
    pub(crate) search_query_ui_font: String,
    pub(crate) search_query_prose_font: String,
    pub(crate) search_query_code_font: String,
    pub(crate) editing_font_size: Option<String>,
    pub(crate) editing_line_height: Option<String>,
    pub(crate) font_size_focus_handle: Option<FocusHandle>,
    pub(crate) line_height_focus_handle: Option<FocusHandle>,
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

        let ui_font_family = settings.typography.ui_font_family.clone();
        let prose_font_family = settings.typography.prose_font_family.clone();
        let code_font_family = settings.typography.code_font_family.clone();
        let font_size = settings.typography.font_size;
        let line_height = settings.typography.line_height;

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
            ui_font_dropdown_open: false,
            prose_font_dropdown_open: false,
            code_font_dropdown_open: false,
            search_query_ui_font: String::new(),
            search_query_prose_font: String::new(),
            search_query_code_font: String::new(),
            ui_font_family: ui_font_family.clone(),
            prose_font_family: prose_font_family.clone(),
            code_font_family: code_font_family.clone(),
            saved_ui_font_family: ui_font_family,
            saved_prose_font_family: prose_font_family,
            saved_code_font_family: code_font_family,
            font_size,
            line_height,
            saved_font_size: font_size,
            saved_line_height: line_height,
            editing_font_size: None,
            editing_line_height: None,
            font_size_focus_handle: None,
            line_height_focus_handle: None,
            status_bar_enabled: settings.status_bar.enabled,
            status_bar_show_word_count: settings.status_bar.show_word_count,
            status_bar_show_cursor_position: settings.status_bar.show_cursor_position,
            saved_status_bar_enabled: settings.status_bar.enabled,
            saved_status_bar_show_word_count: settings.status_bar.show_word_count,
            saved_status_bar_show_cursor_position: settings.status_bar.show_cursor_position,
            expanded_sections,
        }
    }

    pub(crate) fn toggle_section_handler(
        &self,
        cx: &mut Context<Self>,
        key: &'static str,
    ) -> crate::settings::ui_helpers::SettingsClickHandler {
        let handle = cx.entity().downgrade();
        Box::new(move |_event, _window, cx| {
            let _ = handle.update(cx, |this, cx| {
                if this.expanded_sections.contains(key) {
                    this.expanded_sections.remove(key);
                } else {
                    this.expanded_sections.insert(key.to_string());
                }
                cx.notify();
            });
        })
    }

    pub(crate) fn selected_theme_name(&self) -> String {
        self.theme_options
            .iter()
            .find(|entry| entry.id == self.selected_theme_id)
            .map(|entry| entry.name.clone())
            .unwrap_or_else(|| "splitype".into())
    }

    fn render_interface_tab(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let mut sections = Vec::new();

        // 1. Theme & Language Section
        let sec1_key = "theme";
        let is_sec1_expanded = self.expanded_sections.contains(sec1_key);
        let toggle_theme_dd = cx.entity().downgrade();
        let toggle_lang_dd = cx.entity().downgrade();
        let select_theme_win = cx.entity().downgrade();
        let select_lang_win = cx.entity().downgrade();

        let available_themes: Vec<(String, String)> = self
            .theme_options
            .iter()
            .map(|t| (t.id.clone(), t.name.clone()))
            .collect();

        let current_lang_name = match cx.try_global::<I18nManager>().map(|m| m.current_language_id()) {
            Some("zh-CN") => "简体中文 (zh-CN)".to_string(),
            _ => "English (en-US)".to_string(),
        };

        let theme_lang_props = ThemeLangProps {
            current_theme_name: self.selected_theme_name(),
            is_theme_dropdown_open: self.theme_dropdown_open,
            on_toggle_theme_dropdown: Box::new(move |_event, _window, cx| {
                let _ = toggle_theme_dd.update(cx, |this, cx| {
                    this.theme_dropdown_open = !this.theme_dropdown_open;
                    cx.notify();
                });
            }),
            available_themes,
            on_select_theme: Box::new(move |theme_id| {
                let win = select_theme_win.clone();
                Box::new(move |event, window, cx| {
                    let tid = theme_id.clone();
                    let _ = win.update(cx, |this, cx| {
                        this.selected_theme_id = tid;
                        this.theme_dropdown_open = false;
                        this.save(event, window, cx);
                        cx.notify();
                    });
                })
            }),
            current_lang_name,
            is_lang_dropdown_open: self.lang_dropdown_open,
            on_toggle_lang_dropdown: Box::new(move |_event, _window, cx| {
                let _ = toggle_lang_dd.update(cx, |this, cx| {
                    this.lang_dropdown_open = !this.lang_dropdown_open;
                    cx.notify();
                });
            }),
            lang_options: vec![
                ("en-US", "English (en-US)"),
                ("zh-CN", "简体中文 (zh-CN)"),
            ],
            on_select_lang: Box::new(move |lang_code| {
                let win = select_lang_win.clone();
                Box::new(move |_event, _window, cx| {
                    let _ = apply_configured_language(cx, lang_code);
                    let _ = win.update(cx, |this, cx| {
                        this.lang_dropdown_open = false;
                        cx.notify();
                    });
                })
            }),
        };

        sections.push(render_theme_and_language_section(
            c,
            d,
            "win-pref-sec-theme",
            is_sec1_expanded,
            self.toggle_section_handler(cx, sec1_key),
            theme_lang_props,
        ));

        // 2. Status Bar Section
        let sec2_key = "status_bar";
        let is_sec2_expanded = self.expanded_sections.contains(sec2_key);
        let toggle_sb = cx.entity().downgrade();
        let toggle_wc = cx.entity().downgrade();
        let toggle_cp = cx.entity().downgrade();

        let status_bar_props = StatusBarProps {
            show_status_bar: self.status_bar_enabled,
            on_toggle_status_bar: Box::new(move |event, window, cx| {
                let _ = toggle_sb.update(cx, |this, cx| {
                    this.status_bar_enabled = !this.status_bar_enabled;
                    this.save(event, window, cx);
                    cx.notify();
                });
            }),
            show_word_count: self.status_bar_show_word_count,
            on_toggle_word_count: Box::new(move |event, window, cx| {
                let _ = toggle_wc.update(cx, |this, cx| {
                    this.status_bar_show_word_count = !this.status_bar_show_word_count;
                    this.save(event, window, cx);
                    cx.notify();
                });
            }),
            show_cursor_pos: self.status_bar_show_cursor_position,
            on_toggle_cursor_pos: Box::new(move |event, window, cx| {
                let _ = toggle_cp.update(cx, |this, cx| {
                    this.status_bar_show_cursor_position = !this.status_bar_show_cursor_position;
                    this.save(event, window, cx);
                    cx.notify();
                });
            }),
        };

        sections.push(render_status_bar_section(
            c,
            d,
            "win-pref-sec-sb",
            is_sec2_expanded,
            self.toggle_section_handler(cx, sec2_key),
            status_bar_props,
        ));

        sections
    }

    fn render_editing_tab(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let mut sections = Vec::new();

        // 1. Typography Section
        let sec1_key = "typography";
        let is_sec1_expanded = self.expanded_sections.contains(sec1_key);
        let font_dec = cx.entity().downgrade();
        let font_inc = cx.entity().downgrade();
        let lh_dec = cx.entity().downgrade();
        let lh_inc = cx.entity().downgrade();

        let toggle_ui_font = cx.entity().downgrade();
        let toggle_prose_font = cx.entity().downgrade();
        let toggle_code_font = cx.entity().downgrade();
        let select_ui_font = cx.entity().downgrade();
        let select_prose_font = cx.entity().downgrade();
        let select_code_font = cx.entity().downgrade();

        let ui_font_name = self
            .ui_font_family
            .clone()
            .unwrap_or_else(|| "Default (Encode Sans)".to_string());
        let prose_font_name = self
            .prose_font_family
            .clone()
            .unwrap_or_else(|| "Default (Encode Sans)".to_string());
        let code_font_name = self
            .code_font_family
            .clone()
            .unwrap_or_else(|| "Default (Consolas / Menlo)".to_string());

        let available_fonts = crate::infra::theme::FontFamilyCache::list_font_families(cx);

        let search_ui_win = cx.entity().downgrade();
        let search_prose_win = cx.entity().downgrade();
        let search_code_win = cx.entity().downgrade();

        let reset_ui_win = cx.entity().downgrade();
        let reset_prose_win = cx.entity().downgrade();
        let reset_code_win = cx.entity().downgrade();
        let reset_fs_win = cx.entity().downgrade();
        let reset_lh_win = cx.entity().downgrade();

        let has_custom_ui = self.ui_font_family.is_some();
        let has_custom_prose = self.prose_font_family.is_some();
        let has_custom_code = self.code_font_family.is_some();
        let has_custom_fs = self.font_size != 16;
        let has_custom_lh = (self.line_height - 1.6).abs() > 0.01;

        let fs_focus = self
            .font_size_focus_handle
            .get_or_insert_with(|| cx.focus_handle())
            .clone();
        let lh_focus = self
            .line_height_focus_handle
            .get_or_insert_with(|| cx.focus_handle())
            .clone();

        let start_fs_win = cx.entity().downgrade();
        let key_fs_win = cx.entity().downgrade();
        let fs_focus_clone = fs_focus.clone();

        let start_lh_win = cx.entity().downgrade();
        let key_lh_win = cx.entity().downgrade();
        let lh_focus_clone = lh_focus.clone();

        let typography_props = TypographyProps {
            ui_font_name,
            is_ui_font_open: self.ui_font_dropdown_open,
            search_query_ui_font: self.search_query_ui_font.clone(),
            on_toggle_ui_font: Box::new(move |_event, _window, cx| {
                let _ = toggle_ui_font.update(cx, |this, cx| {
                    this.ui_font_dropdown_open = !this.ui_font_dropdown_open;
                    this.prose_font_dropdown_open = false;
                    this.code_font_dropdown_open = false;
                    this.search_query_ui_font.clear();
                    cx.notify();
                });
            }),
            on_search_ui_font: Box::new(move |query, _window, cx| {
                let _ = search_ui_win.update(cx, |this, cx| {
                    this.search_query_ui_font = query;
                    cx.notify();
                });
            }),
            on_select_ui_font: Box::new(move |font_name| {
                let handle = select_ui_font.clone();
                Box::new(move |_event, _window, cx| {
                    let _ = handle.update(cx, |this, cx| {
                        this.ui_font_family = if font_name == "default" { None } else { Some(font_name.clone()) };
                        this.ui_font_dropdown_open = false;
                        cx.notify();
                    });
                })
            }),
            on_reset_ui_font: if has_custom_ui {
                Some(Box::new(move |_event, _window, cx| {
                    let _ = reset_ui_win.update(cx, |this, cx| {
                        this.ui_font_family = None;
                        cx.notify();
                    });
                }))
            } else {
                None
            },

            prose_font_name,
            is_prose_font_open: self.prose_font_dropdown_open,
            search_query_prose_font: self.search_query_prose_font.clone(),
            on_toggle_prose_font: Box::new(move |_event, _window, cx| {
                let _ = toggle_prose_font.update(cx, |this, cx| {
                    this.prose_font_dropdown_open = !this.prose_font_dropdown_open;
                    this.ui_font_dropdown_open = false;
                    this.code_font_dropdown_open = false;
                    this.search_query_prose_font.clear();
                    cx.notify();
                });
            }),
            on_search_prose_font: Box::new(move |query, _window, cx| {
                let _ = search_prose_win.update(cx, |this, cx| {
                    this.search_query_prose_font = query;
                    cx.notify();
                });
            }),
            on_select_prose_font: Box::new(move |font_name| {
                let handle = select_prose_font.clone();
                Box::new(move |_event, _window, cx| {
                    let _ = handle.update(cx, |this, cx| {
                        this.prose_font_family = if font_name == "default" { None } else { Some(font_name.clone()) };
                        this.prose_font_dropdown_open = false;
                        cx.notify();
                    });
                })
            }),
            on_reset_prose_font: if has_custom_prose {
                Some(Box::new(move |_event, _window, cx| {
                    let _ = reset_prose_win.update(cx, |this, cx| {
                        this.prose_font_family = None;
                        cx.notify();
                    });
                }))
            } else {
                None
            },

            code_font_name,
            is_code_font_open: self.code_font_dropdown_open,
            search_query_code_font: self.search_query_code_font.clone(),
            on_toggle_code_font: Box::new(move |_event, _window, cx| {
                let _ = toggle_code_font.update(cx, |this, cx| {
                    this.code_font_dropdown_open = !this.code_font_dropdown_open;
                    this.ui_font_dropdown_open = false;
                    this.prose_font_dropdown_open = false;
                    this.search_query_code_font.clear();
                    cx.notify();
                });
            }),
            on_search_code_font: Box::new(move |query, _window, cx| {
                let _ = search_code_win.update(cx, |this, cx| {
                    this.search_query_code_font = query;
                    cx.notify();
                });
            }),
            on_select_code_font: Box::new(move |font_name| {
                let handle = select_code_font.clone();
                Box::new(move |_event, _window, cx| {
                    let _ = handle.update(cx, |this, cx| {
                        this.code_font_family = if font_name == "default" { None } else { Some(font_name.clone()) };
                        this.code_font_dropdown_open = false;
                        cx.notify();
                    });
                })
            }),
            on_reset_code_font: if has_custom_code {
                Some(Box::new(move |_event, _window, cx| {
                    let _ = reset_code_win.update(cx, |this, cx| {
                        this.code_font_family = None;
                        cx.notify();
                    });
                }))
            } else {
                None
            },

            available_fonts,

            font_size: self.font_size,
            is_editing_font_size: self.editing_font_size.is_some(),
            edit_buffer_font_size: self.editing_font_size.clone(),
            font_size_focus_handle: fs_focus,
            on_font_dec: Box::new(move |event, _window, cx| {
                let step = if event.modifiers().shift { 4 } else { 1 };
                let _ = font_dec.update(cx, |this, cx| {
                    if this.font_size > (8 + step) {
                        this.font_size -= step;
                    } else {
                        this.font_size = 8;
                    }
                    if this.editing_font_size.is_some() {
                        this.editing_font_size = Some(format!("{}", this.font_size));
                    }
                    cx.notify();
                });
            }),
            on_font_inc: Box::new(move |event, _window, cx| {
                let step = if event.modifiers().shift { 4 } else { 1 };
                let _ = font_inc.update(cx, |this, cx| {
                    if this.font_size + step <= 72 {
                        this.font_size += step;
                    } else {
                        this.font_size = 72;
                    }
                    if this.editing_font_size.is_some() {
                        this.editing_font_size = Some(format!("{}", this.font_size));
                    }
                    cx.notify();
                });
            }),
            on_start_edit_font_size: Box::new(move |_event, window, cx| {
                window.focus(&fs_focus_clone, cx);
                let _ = start_fs_win.update(cx, |this, cx| {
                    this.editing_font_size = Some(format!("{}", this.font_size));
                    this.editing_line_height = None;
                    cx.notify();
                });
            }),
            on_key_down_font_size: Box::new(move |event, _window, cx| {
                let key = event.keystroke.key.as_str();
                let _ = key_fs_win.update(cx, |this, cx| {
                    let mut commit_val = None;
                    let mut cancel = false;

                    if let Some(buf) = &mut this.editing_font_size {
                        match key {
                            "enter" => {
                                if let Ok(v) = buf.parse::<u32>() {
                                    commit_val = Some(v.clamp(8, 72));
                                } else {
                                    cancel = true;
                                }
                            }
                            "escape" => {
                                cancel = true;
                            }
                            "backspace" => {
                                buf.pop();
                            }
                            "up" => {
                                let step = if event.keystroke.modifiers.shift { 4 } else { 1 };
                                let curr = buf.parse::<u32>().unwrap_or(this.font_size);
                                let new_v = (curr + step).min(72);
                                *buf = format!("{}", new_v);
                                commit_val = Some(new_v);
                            }
                            "down" => {
                                let step = if event.keystroke.modifiers.shift { 4 } else { 1 };
                                let curr = buf.parse::<u32>().unwrap_or(this.font_size);
                                let new_v = if curr > 8 + step { curr - step } else { 8 };
                                *buf = format!("{}", new_v);
                                commit_val = Some(new_v);
                            }
                            k if k.len() == 1 && k.chars().all(|c| c.is_ascii_digit()) => {
                                if buf.len() < 2 {
                                    buf.push_str(k);
                                }
                            }
                            _ => {}
                        }
                    }

                    if let Some(v) = commit_val {
                        this.font_size = v;
                        if key == "enter" {
                            this.editing_font_size = None;
                        }
                    } else if cancel {
                        this.editing_font_size = None;
                    }

                    cx.notify();
                });
            }),
            on_reset_font_size: if has_custom_fs {
                Some(Box::new(move |_event, _window, cx| {
                    let _ = reset_fs_win.update(cx, |this, cx| {
                        this.font_size = 16;
                        this.editing_font_size = None;
                        cx.notify();
                    });
                }))
            } else {
                None
            },

            line_height: self.line_height,
            is_editing_line_height: self.editing_line_height.is_some(),
            edit_buffer_line_height: self.editing_line_height.clone(),
            line_height_focus_handle: lh_focus,
            on_lh_dec: Box::new(move |event, _window, cx| {
                let step = if event.modifiers().shift { 0.5 } else if event.modifiers().alt { 0.05 } else { 0.1 };
                let _ = lh_dec.update(cx, |this, cx| {
                    if this.line_height > (1.0 + step) {
                        this.line_height -= step;
                    } else {
                        this.line_height = 1.0;
                    }
                    if this.editing_line_height.is_some() {
                        this.editing_line_height = Some(format!("{:.2}", this.line_height));
                    }
                    cx.notify();
                });
            }),
            on_lh_inc: Box::new(move |event, _window, cx| {
                let step = if event.modifiers().shift { 0.5 } else if event.modifiers().alt { 0.05 } else { 0.1 };
                let _ = lh_inc.update(cx, |this, cx| {
                    if this.line_height + step <= 3.0 {
                        this.line_height += step;
                    } else {
                        this.line_height = 3.0;
                    }
                    if this.editing_line_height.is_some() {
                        this.editing_line_height = Some(format!("{:.2}", this.line_height));
                    }
                    cx.notify();
                });
            }),
            on_start_edit_line_height: Box::new(move |_event, window, cx| {
                window.focus(&lh_focus_clone, cx);
                let _ = start_lh_win.update(cx, |this, cx| {
                    this.editing_line_height = Some(format!("{:.2}", this.line_height));
                    this.editing_font_size = None;
                    cx.notify();
                });
            }),
            on_key_down_line_height: Box::new(move |event, _window, cx| {
                let key = event.keystroke.key.as_str();
                let _ = key_lh_win.update(cx, |this, cx| {
                    let mut commit_val = None;
                    let mut cancel = false;

                    if let Some(buf) = &mut this.editing_line_height {
                        match key {
                            "enter" => {
                                if let Ok(v) = buf.parse::<f32>() {
                                    commit_val = Some(v.clamp(1.0, 3.0));
                                } else {
                                    cancel = true;
                                }
                            }
                            "escape" => {
                                cancel = true;
                            }
                            "backspace" => {
                                buf.pop();
                            }
                            "up" => {
                                let step = if event.keystroke.modifiers.shift { 0.5 } else if event.keystroke.modifiers.alt { 0.05 } else { 0.1 };
                                let curr = buf.parse::<f32>().unwrap_or(this.line_height);
                                let new_v = (curr + step).min(3.0);
                                *buf = format!("{:.2}", new_v);
                                commit_val = Some(new_v);
                            }
                            "down" => {
                                let step = if event.keystroke.modifiers.shift { 0.5 } else if event.keystroke.modifiers.alt { 0.05 } else { 0.1 };
                                let curr = buf.parse::<f32>().unwrap_or(this.line_height);
                                let new_v = if curr > 1.0 + step { curr - step } else { 1.0 };
                                *buf = format!("{:.2}", new_v);
                                commit_val = Some(new_v);
                            }
                            k if (k.len() == 1 && k.chars().all(|c| c.is_ascii_digit())) || k == "." => {
                                if buf.len() < 4 {
                                    if k != "." || !buf.contains('.') {
                                        buf.push_str(k);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    if let Some(v) = commit_val {
                        this.line_height = v;
                        if key == "enter" {
                            this.editing_line_height = None;
                        }
                    } else if cancel {
                        this.editing_line_height = None;
                    }

                    cx.notify();
                });
            }),
            on_reset_line_height: if has_custom_lh {
                Some(Box::new(move |_event, _window, cx| {
                    let _ = reset_lh_win.update(cx, |this, cx| {
                        this.line_height = 1.6;
                        this.editing_line_height = None;
                        cx.notify();
                    });
                }))
            } else {
                None
            },
        };

        sections.push(render_typography_section(
            c,
            d,
            "win-pref-sec-typo",
            is_sec1_expanded,
            self.toggle_section_handler(cx, sec1_key),
            typography_props,
        ));

        // 2. Markdown Section
        let sec2_key = "markdown";
        let is_sec2_expanded = self.expanded_sections.contains(sec2_key);
        let toggle_paste_dd = cx.entity().downgrade();
        let select_paste_win = cx.entity().downgrade();

        let paste_action_idx = match self.image_paste_behavior {
            ImagePasteBehavior::CopyToAssetsFolder => 0,
            ImagePasteBehavior::CopyToDocumentFolder => 1,
            ImagePasteBehavior::CopyToNamedAssetsFolder => 2,
            ImagePasteBehavior::None => 0,
        };

        let markdown_props = MarkdownProps {
            show_table_headers: true,
            on_toggle_table_headers: Box::new(|_event, _window, _cx| {}),
            image_paste_action: paste_action_idx,
            is_image_paste_open: self.image_dropdown_open,
            on_toggle_image_paste: Box::new(move |_event, _window, cx| {
                let _ = toggle_paste_dd.update(cx, |this, cx| {
                    this.image_dropdown_open = !this.image_dropdown_open;
                    cx.notify();
                });
            }),
            on_select_image_paste: Box::new(move |idx| {
                let win = select_paste_win.clone();
                Box::new(move |event, window, cx| {
                    let behavior = match idx {
                        0 => ImagePasteBehavior::CopyToAssetsFolder,
                        1 => ImagePasteBehavior::CopyToDocumentFolder,
                        _ => ImagePasteBehavior::CopyToNamedAssetsFolder,
                    };
                    let _ = win.update(cx, |this, cx| {
                        this.image_paste_behavior = behavior;
                        this.image_dropdown_open = false;
                        this.save(event, window, cx);
                        cx.notify();
                    });
                })
            }),
        };

        sections.push(render_markdown_section(
            c,
            d,
            "win-pref-sec-md",
            is_sec2_expanded,
            self.toggle_section_handler(cx, sec2_key),
            markdown_props,
        ));

        // 3. Startup Section
        let sec3_key = "startup";
        let is_sec3_expanded = self.expanded_sections.contains(sec3_key);
        let toggle_startup_dd = cx.entity().downgrade();
        let select_startup_win = cx.entity().downgrade();

        let startup_opt_idx = match self.startup_open {
            StartupOpenSetting::NewFile => 0,
            StartupOpenSetting::LastOpenedFile => 1,
        };

        let startup_props = StartupProps {
            startup_option: startup_opt_idx,
            is_startup_open: self.startup_dropdown_open,
            on_toggle_startup: Box::new(move |_event, _window, cx| {
                let _ = toggle_startup_dd.update(cx, |this, cx| {
                    this.startup_dropdown_open = !this.startup_dropdown_open;
                    cx.notify();
                });
            }),
            on_select_startup: Box::new(move |idx| {
                let win = select_startup_win.clone();
                Box::new(move |event, window, cx| {
                    let setting = match idx {
                        0 => StartupOpenSetting::NewFile,
                        _ => StartupOpenSetting::LastOpenedFile,
                    };
                    let _ = win.update(cx, |this, cx| {
                        this.startup_open = setting;
                        this.startup_dropdown_open = false;
                        this.save(event, window, cx);
                        cx.notify();
                    });
                })
            }),
        };

        sections.push(render_startup_section(
            c,
            d,
            "win-pref-sec-startup",
            is_sec3_expanded,
            self.toggle_section_handler(cx, sec3_key),
            startup_props,
        ));

        sections
    }

    fn render_shortcuts_tab(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let mut sections = Vec::new();

        let sec1_key = "doc_actions";
        let is_sec1_expanded = self.expanded_sections.contains(sec1_key);
        sections.push(render_shortcuts_section(
            c,
            d,
            "win-pref-sec-doc-actions",
            "Document Actions",
            is_sec1_expanded,
            self.toggle_section_handler(cx, sec1_key),
            crate::settings::components::shortcuts_data::doc_action_shortcuts(),
        ));

        let sec2_key = "view_controls";
        let is_sec2_expanded = self.expanded_sections.contains(sec2_key);
        sections.push(render_shortcuts_section(
            c,
            d,
            "win-pref-sec-view-controls",
            "Interface & View Controls",
            is_sec2_expanded,
            self.toggle_section_handler(cx, sec2_key),
            crate::settings::components::shortcuts_data::interface_view_shortcuts(),
        ));

        sections
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
            || self.ui_font_family != self.saved_ui_font_family
            || self.prose_font_family != self.saved_prose_font_family
            || self.code_font_family != self.saved_code_font_family
            || self.font_size != self.saved_font_size
            || (self.line_height - self.saved_line_height).abs() > 0.001
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
            &crate::infra::config::settings::TypographySettings {
                ui_font_family: self.ui_font_family.clone(),
                prose_font_family: self.prose_font_family.clone(),
                code_font_family: self.code_font_family.clone(),
                font_size: self.font_size,
                line_height: self.line_height,
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
        crate::infra::theme::TypographyStore::update(cx, settings.typography.clone());
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
        self.saved_ui_font_family = settings.typography.ui_font_family.clone();
        self.saved_prose_font_family = settings.typography.prose_font_family.clone();
        self.saved_code_font_family = settings.typography.code_font_family.clone();
        self.saved_font_size = settings.typography.font_size;
        self.saved_line_height = settings.typography.line_height;

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

    if let Err(err) = handle.update(cx, |settings_win, window, cx| {
        window.activate_window();
        settings_win.focus_handle.focus(window, cx);
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
