//! Standalone settings window view and window opening lifecycle.

use std::collections::HashSet;

use gpui::*;

use crate::infra::config::settings::*;
use crate::infra::i18n::manager::I18nManager;
use crate::infra::theme::{Theme, ThemeManager};
use crate::settings::components::*;
use crate::settings::state::SettingsTab;
use crate::ui::custom_titlebar::{
    custom_titlebar_height, render_custom_titlebar, splitype_window_options,
};
use crate::ui::tab::nav_tab;

/// Independent standalone settings window view.
pub(crate) struct SettingsWindow {
    pub(crate) nav: SettingsTab,
    pub(crate) focus_handle: FocusHandle,
    pub(crate) expanded_sections: HashSet<String>,
    pub(crate) open_dropdown: Option<String>,
    pub(crate) search_query_ui_font: String,
    pub(crate) search_query_prose_font: String,
    pub(crate) search_query_code_font: String,
    pub(crate) editing_font_size: Option<String>,
    pub(crate) editing_line_height: Option<String>,
    pub(crate) editing_tab_size: Option<String>,
    pub(crate) font_size_focus_handle: Option<FocusHandle>,
    pub(crate) line_height_focus_handle: Option<FocusHandle>,
    pub(crate) tab_size_focus_handle: Option<FocusHandle>,
}

impl SettingsWindow {
    fn new(cx: &mut Context<Self>) -> Self {
        let mut expanded_sections = HashSet::new();
        expanded_sections.insert("theme".to_string());
        expanded_sections.insert("status_bar".to_string());
        expanded_sections.insert("typography".to_string());
        expanded_sections.insert("editor_behavior".to_string());
        expanded_sections.insert("markdown".to_string());
        expanded_sections.insert("explorer".to_string());
        expanded_sections.insert("startup".to_string());
        expanded_sections.insert("doc_actions".to_string());
        expanded_sections.insert("view_controls".to_string());
        expanded_sections.insert("editor_shortcuts".to_string());

        Self {
            nav: SettingsTab::Interface,
            focus_handle: cx.focus_handle(),
            expanded_sections,
            open_dropdown: None,
            search_query_ui_font: String::new(),
            search_query_prose_font: String::new(),
            search_query_code_font: String::new(),
            editing_font_size: None,
            editing_line_height: None,
            editing_tab_size: None,
            font_size_focus_handle: None,
            line_height_focus_handle: None,
            tab_size_focus_handle: None,
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

    fn render_interface_tab(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let mut sections = Vec::new();
        let app_settings = SettingsStore::get(cx).clone();

        // 1. Theme & Language Section
        let sec1_key = "theme";
        let is_sec1_expanded = self.expanded_sections.contains(sec1_key);
        let toggle_theme_dd = cx.entity().downgrade();
        let toggle_lang_dd = cx.entity().downgrade();
        let select_theme_win = cx.entity().downgrade();
        let select_lang_win = cx.entity().downgrade();

        let available_themes = cx
            .global::<ThemeManager>()
            .available_themes()
            .iter()
            .map(|t| (t.id.clone(), t.name.clone()))
            .collect();

        let current_lang_name = match app_settings.interface.language_id.as_str() {
            "zh-CN" => "简体中文 (zh-CN)".to_string(),
            _ => "English (en-US)".to_string(),
        };

        let theme_lang_props = ThemeLangProps {
            current_theme_name: theme.name.clone(),
            is_theme_dropdown_open: self.open_dropdown.as_deref() == Some("theme"),
            on_toggle_theme_dropdown: Box::new(move |_event, _window, cx| {
                let _ = toggle_theme_dd.update(cx, |this, cx| {
                    if this.open_dropdown.as_deref() == Some("theme") {
                        this.open_dropdown = None;
                    } else {
                        this.open_dropdown = Some("theme".to_string());
                    }
                    cx.notify();
                });
            }),
            available_themes,
            on_select_theme: Box::new(move |theme_id| {
                let win = select_theme_win.clone();
                Box::new(move |_event, _window, cx| {
                    let _ = apply_configured_theme(cx, &theme_id);
                    let _ = win.update(cx, |this, cx| {
                        this.open_dropdown = None;
                        cx.notify();
                    });
                })
            }),
            current_lang_name,
            is_lang_dropdown_open: self.open_dropdown.as_deref() == Some("lang"),
            on_toggle_lang_dropdown: Box::new(move |_event, _window, cx| {
                let _ = toggle_lang_dd.update(cx, |this, cx| {
                    if this.open_dropdown.as_deref() == Some("lang") {
                        this.open_dropdown = None;
                    } else {
                        this.open_dropdown = Some("lang".to_string());
                    }
                    cx.notify();
                });
            }),
            lang_options: vec![("en-US", "English (en-US)"), ("zh-CN", "简体中文 (zh-CN)")],
            on_select_lang: Box::new(move |lang_code| {
                let win = select_lang_win.clone();
                Box::new(move |_event, _window, cx| {
                    let _ = apply_configured_language(cx, lang_code);
                    let _ = win.update(cx, |this, cx| {
                        this.open_dropdown = None;
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

        let status_bar_props = StatusBarProps {
            show_status_bar: app_settings.status_bar.enabled,
            on_toggle_status_bar: Box::new(move |_event, _window, cx| {
                let _ = SettingsStore::update(cx, |s| s.status_bar.enabled = !s.status_bar.enabled);
            }),
            show_word_count: app_settings.status_bar.show_word_count,
            on_toggle_word_count: Box::new(move |_event, _window, cx| {
                let _ = SettingsStore::update(cx, |s| s.status_bar.show_word_count = !s.status_bar.show_word_count);
            }),
            show_cursor_pos: app_settings.status_bar.show_cursor_position,
            on_toggle_cursor_pos: Box::new(move |_event, _window, cx| {
                let _ = SettingsStore::update(cx, |s| s.status_bar.show_cursor_position = !s.status_bar.show_cursor_position);
            }),
            show_character_count: app_settings.status_bar.show_character_count,
            on_toggle_character_count: Box::new(move |_event, _window, cx| {
                let _ = SettingsStore::update(cx, |s| s.status_bar.show_character_count = !s.status_bar.show_character_count);
            }),
            show_reading_time: app_settings.status_bar.show_reading_time,
            on_toggle_reading_time: Box::new(move |_event, _window, cx| {
                let _ = SettingsStore::update(cx, |s| s.status_bar.show_reading_time = !s.status_bar.show_reading_time);
            }),
        };

        sections.push(render_status_bar_section(
            c,
            d,
            "win-pref-sec-status-bar",
            is_sec2_expanded,
            self.toggle_section_handler(cx, sec2_key),
            status_bar_props,
        ));

        sections
    }

    fn render_editor_tab(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let mut sections = Vec::new();
        let app_settings = SettingsStore::get(cx).clone();

        // 1. Typography Section
        let sec1_key = "typography";
        let is_sec1_expanded = self.expanded_sections.contains(sec1_key);

        let toggle_ui_font_win = cx.entity().downgrade();
        let toggle_prose_font_win = cx.entity().downgrade();
        let toggle_code_font_win = cx.entity().downgrade();
        let search_ui_win = cx.entity().downgrade();
        let search_prose_win = cx.entity().downgrade();
        let search_code_win = cx.entity().downgrade();
        let select_ui_font_win = cx.entity().downgrade();
        let select_prose_font_win = cx.entity().downgrade();
        let select_code_font_win = cx.entity().downgrade();

        let available_fonts: Vec<SharedString> = cx
            .text_system()
            .all_font_names()
            .into_iter()
            .map(SharedString::from)
            .collect();

        let is_ui_font_open = self.open_dropdown.as_deref() == Some("ui_font");
        let is_prose_font_open = self.open_dropdown.as_deref() == Some("prose_font");
        let is_code_font_open = self.open_dropdown.as_deref() == Some("code_font");

        let ui_font_name = app_settings.typography.ui_font_family.clone().unwrap_or_else(|| "Lexend (default)".to_string());
        let prose_font_name = app_settings.typography.prose_font_family.clone().unwrap_or_else(|| "Lexend (default)".to_string());
        let code_font_name = app_settings.typography.code_font_family.clone().unwrap_or_else(|| {
            if cfg!(target_os = "windows") {
                "Consolas (default)".to_string()
            } else if cfg!(target_os = "macos") {
                "Menlo (default)".to_string()
            } else {
                "monospace (default)".to_string()
            }
        });

        let has_custom_ui = app_settings.typography.ui_font_family.is_some();
        let has_custom_prose = app_settings.typography.prose_font_family.is_some();
        let has_custom_code = app_settings.typography.code_font_family.is_some();
        let has_custom_fs = app_settings.typography.font_size != 16;
        let has_custom_lh = (app_settings.typography.line_height - 1.6).abs() > 0.001;

        let fs_focus = self
            .font_size_focus_handle
            .get_or_insert_with(|| cx.focus_handle())
            .clone();
        let lh_focus = self
            .line_height_focus_handle
            .get_or_insert_with(|| cx.focus_handle())
            .clone();

        let fs_focus_clone = fs_focus.clone();
        let lh_focus_clone = lh_focus.clone();
        let start_fs_win = cx.entity().downgrade();
        let start_lh_win = cx.entity().downgrade();
        let key_fs_win = cx.entity().downgrade();
        let key_lh_win = cx.entity().downgrade();

        let typography_props = TypographyProps {
            ui_font_name,
            is_ui_font_open,
            search_query_ui_font: self.search_query_ui_font.clone(),
            on_toggle_ui_font: Box::new(move |_event, _window, cx| {
                let _ = toggle_ui_font_win.update(cx, |this, cx| {
                    if this.open_dropdown.as_deref() == Some("ui_font") {
                        this.open_dropdown = None;
                    } else {
                        this.open_dropdown = Some("ui_font".to_string());
                        this.search_query_ui_font.clear();
                    }
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
                let win = select_ui_font_win.clone();
                Box::new(move |_event, _window, cx| {
                    let _ = SettingsStore::update(cx, |s| {
                        s.typography.ui_font_family = if font_name == "default" { None } else { Some(font_name.clone()) };
                    });
                    let _ = win.update(cx, |this, cx| {
                        this.open_dropdown = None;
                        cx.notify();
                    });
                })
            }),
            on_reset_ui_font: if has_custom_ui {
                Some(Box::new(move |_event, _window, cx| {
                    let _ = SettingsStore::update(cx, |s| s.typography.ui_font_family = None);
                }))
            } else {
                None
            },

            prose_font_name,
            is_prose_font_open,
            search_query_prose_font: self.search_query_prose_font.clone(),
            on_toggle_prose_font: Box::new(move |_event, _window, cx| {
                let _ = toggle_prose_font_win.update(cx, |this, cx| {
                    if this.open_dropdown.as_deref() == Some("prose_font") {
                        this.open_dropdown = None;
                    } else {
                        this.open_dropdown = Some("prose_font".to_string());
                        this.search_query_prose_font.clear();
                    }
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
                let win = select_prose_font_win.clone();
                Box::new(move |_event, _window, cx| {
                    let _ = SettingsStore::update(cx, |s| {
                        s.typography.prose_font_family = if font_name == "default" { None } else { Some(font_name.clone()) };
                    });
                    let _ = win.update(cx, |this, cx| {
                        this.open_dropdown = None;
                        cx.notify();
                    });
                })
            }),
            on_reset_prose_font: if has_custom_prose {
                Some(Box::new(move |_event, _window, cx| {
                    let _ = SettingsStore::update(cx, |s| s.typography.prose_font_family = None);
                }))
            } else {
                None
            },

            code_font_name,
            is_code_font_open,
            search_query_code_font: self.search_query_code_font.clone(),
            on_toggle_code_font: Box::new(move |_event, _window, cx| {
                let _ = toggle_code_font_win.update(cx, |this, cx| {
                    if this.open_dropdown.as_deref() == Some("code_font") {
                        this.open_dropdown = None;
                    } else {
                        this.open_dropdown = Some("code_font".to_string());
                        this.search_query_code_font.clear();
                    }
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
                let win = select_code_font_win.clone();
                Box::new(move |_event, _window, cx| {
                    let _ = SettingsStore::update(cx, |s| {
                        s.typography.code_font_family = if font_name == "default" { None } else { Some(font_name.clone()) };
                    });
                    let _ = win.update(cx, |this, cx| {
                        this.open_dropdown = None;
                        cx.notify();
                    });
                })
            }),
            on_reset_code_font: if has_custom_code {
                Some(Box::new(move |_event, _window, cx| {
                    let _ = SettingsStore::update(cx, |s| s.typography.code_font_family = None);
                }))
            } else {
                None
            },

            available_fonts,

            font_size: app_settings.typography.font_size,
            is_editing_font_size: self.editing_font_size.is_some(),
            edit_buffer_font_size: self.editing_font_size.clone(),
            font_size_focus_handle: fs_focus,
            on_font_dec: Box::new(move |event, _window, cx| {
                let step = if event.modifiers().shift { 4 } else { 1 };
                let _ = SettingsStore::update(cx, |s| {
                    s.typography.font_size = if s.typography.font_size > 8 + step { s.typography.font_size - step } else { 8 };
                });
            }),
            on_font_inc: Box::new(move |event, _window, cx| {
                let step = if event.modifiers().shift { 4 } else { 1 };
                let _ = SettingsStore::update(cx, |s| {
                    s.typography.font_size = (s.typography.font_size + step).min(72);
                });
            }),
            on_start_edit_font_size: Box::new(move |_event, window, cx| {
                window.focus(&fs_focus_clone, cx);
                let current_val = SettingsStore::get(cx).typography.font_size;
                let _ = start_fs_win.update(cx, |this, cx| {
                    this.editing_font_size = Some(format!("{}", current_val));
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
                            "escape" => cancel = true,
                            "backspace" => { buf.pop(); }
                            "up" => {
                                let curr = buf.parse::<u32>().unwrap_or(16);
                                let new_v = (curr + 1).min(72);
                                *buf = format!("{}", new_v);
                                commit_val = Some(new_v);
                            }
                            "down" => {
                                let curr = buf.parse::<u32>().unwrap_or(16);
                                let new_v = if curr > 9 { curr - 1 } else { 8 };
                                *buf = format!("{}", new_v);
                                commit_val = Some(new_v);
                            }
                            k if k.len() == 1 && k.chars().all(|c| c.is_ascii_digit()) => {
                                if buf.len() < 2 { buf.push_str(k); }
                            }
                            _ => {}
                        }
                    }

                    if let Some(v) = commit_val {
                        if key == "enter" {
                            this.editing_font_size = None;
                        }
                        let _ = SettingsStore::update(cx, |s| s.typography.font_size = v);
                    } else if cancel {
                        this.editing_font_size = None;
                    }
                    cx.notify();
                });
            }),
            on_reset_font_size: if has_custom_fs {
                Some(Box::new(move |_event, _window, cx| {
                    let _ = SettingsStore::update(cx, |s| s.typography.font_size = 16);
                }))
            } else {
                None
            },

            line_height: app_settings.typography.line_height,
            is_editing_line_height: self.editing_line_height.is_some(),
            edit_buffer_line_height: self.editing_line_height.clone(),
            line_height_focus_handle: lh_focus,
            on_lh_dec: Box::new(move |event, _window, cx| {
                let step = if event.modifiers().shift { 0.2 } else { 0.05 };
                let _ = SettingsStore::update(cx, |s| {
                    s.typography.line_height = ((s.typography.line_height - step) * 100.0).round() / 100.0;
                    if s.typography.line_height < 1.0 {
                        s.typography.line_height = 1.0;
                    }
                });
            }),
            on_lh_inc: Box::new(move |event, _window, cx| {
                let step = if event.modifiers().shift { 0.2 } else { 0.05 };
                let _ = SettingsStore::update(cx, |s| {
                    s.typography.line_height = ((s.typography.line_height + step) * 100.0).round() / 100.0;
                    if s.typography.line_height > 3.0 {
                        s.typography.line_height = 3.0;
                    }
                });
            }),
            on_start_edit_line_height: Box::new(move |_event, window, cx| {
                window.focus(&lh_focus_clone, cx);
                let current_val = SettingsStore::get(cx).typography.line_height;
                let _ = start_lh_win.update(cx, |this, cx| {
                    this.editing_line_height = Some(format!("{:.2}", current_val));
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
                            "escape" => cancel = true,
                            "backspace" => { buf.pop(); }
                            k if k.len() == 1 && (k.chars().all(|c| c.is_ascii_digit()) || (k == "." && !buf.contains('.'))) => {
                                if buf.len() < 4 { buf.push_str(k); }
                            }
                            _ => {}
                        }
                    }

                    if let Some(v) = commit_val {
                        if key == "enter" {
                            this.editing_line_height = None;
                        }
                        let _ = SettingsStore::update(cx, |s| s.typography.line_height = v);
                    } else if cancel {
                        this.editing_line_height = None;
                    }
                    cx.notify();
                });
            }),
            on_reset_line_height: if has_custom_lh {
                Some(Box::new(move |_event, _window, cx| {
                    let _ = SettingsStore::update(cx, |s| s.typography.line_height = 1.6);
                }))
            } else {
                None
            },
        };

        sections.push(render_typography_section(
            c,
            d,
            "win-pref-sec-typography",
            is_sec1_expanded,
            self.toggle_section_handler(cx, sec1_key),
            typography_props,
        ));

        // 2. Editor Behavior Section
        let sec2_key = "editor_behavior";
        let is_sec2_expanded = self.expanded_sections.contains(sec2_key);

        let tab_size_focus = self
            .tab_size_focus_handle
            .get_or_insert_with(|| cx.focus_handle())
            .clone();
        let tab_size_focus_clone = tab_size_focus.clone();
        let start_ts_win = cx.entity().downgrade();
        let key_ts_win = cx.entity().downgrade();

        let has_custom_ts = app_settings.editor.tab_size != 4;

        let editor_behavior_props = EditorBehaviorProps {
            line_numbers: app_settings.editor.line_numbers,
            on_toggle_line_numbers: Box::new(move |_event, _window, cx| {
                let _ = SettingsStore::update(cx, |s| s.editor.line_numbers = !s.editor.line_numbers);
            }),
            word_wrap: app_settings.editor.word_wrap,
            on_toggle_word_wrap: Box::new(move |_event, _window, cx| {
                let _ = SettingsStore::update(cx, |s| s.editor.word_wrap = !s.editor.word_wrap);
            }),
            tab_size: app_settings.editor.tab_size,
            is_editing_tab_size: self.editing_tab_size.is_some(),
            edit_buffer_tab_size: self.editing_tab_size.clone(),
            tab_size_focus_handle: tab_size_focus,
            on_tab_size_dec: Box::new(move |_event, _window, cx| {
                let _ = SettingsStore::update(cx, |s| {
                    s.editor.tab_size = if s.editor.tab_size > 2 { s.editor.tab_size - 2 } else { 2 };
                });
            }),
            on_tab_size_inc: Box::new(move |_event, _window, cx| {
                let _ = SettingsStore::update(cx, |s| {
                    s.editor.tab_size = (s.editor.tab_size + 2).min(8);
                });
            }),
            on_start_edit_tab_size: Box::new(move |_event, window, cx| {
                window.focus(&tab_size_focus_clone, cx);
                let current_val = SettingsStore::get(cx).editor.tab_size;
                let _ = start_ts_win.update(cx, |this, cx| {
                    this.editing_tab_size = Some(format!("{}", current_val));
                    cx.notify();
                });
            }),
            on_key_down_tab_size: Box::new(move |event, _window, cx| {
                let key = event.keystroke.key.as_str();
                let _ = key_ts_win.update(cx, |this, cx| {
                    let mut commit_val = None;
                    let mut cancel = false;

                    if let Some(buf) = &mut this.editing_tab_size {
                        match key {
                            "enter" => {
                                if let Ok(v) = buf.parse::<u32>() {
                                    commit_val = Some(v.clamp(2, 8));
                                } else {
                                    cancel = true;
                                }
                            }
                            "escape" => cancel = true,
                            "backspace" => { buf.pop(); }
                            k if k.len() == 1 && k.chars().all(|c| c.is_ascii_digit()) => {
                                if buf.len() < 1 { buf.push_str(k); }
                            }
                            _ => {}
                        }
                    }

                    if let Some(v) = commit_val {
                        if key == "enter" {
                            this.editing_tab_size = None;
                        }
                        let _ = SettingsStore::update(cx, |s| s.editor.tab_size = v);
                    } else if cancel {
                        this.editing_tab_size = None;
                    }
                    cx.notify();
                });
            }),
            on_reset_tab_size: if has_custom_ts {
                Some(Box::new(move |_event, _window, cx| {
                    let _ = SettingsStore::update(cx, |s| s.editor.tab_size = 4);
                }))
            } else {
                None
            },
            insert_spaces: app_settings.editor.insert_spaces,
            on_toggle_insert_spaces: Box::new(move |_event, _window, cx| {
                let _ = SettingsStore::update(cx, |s| s.editor.insert_spaces = !s.editor.insert_spaces);
            }),
            highlight_active_line: app_settings.editor.highlight_active_line,
            on_toggle_highlight_active_line: Box::new(move |_event, _window, cx| {
                let _ = SettingsStore::update(cx, |s| s.editor.highlight_active_line = !s.editor.highlight_active_line);
            }),
        };

        sections.push(render_editor_behavior_section(
            c,
            d,
            "win-pref-sec-editor-behavior",
            is_sec2_expanded,
            self.toggle_section_handler(cx, sec2_key),
            editor_behavior_props,
        ));

        sections
    }

    fn render_markdown_tab(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let mut sections = Vec::new();
        let app_settings = SettingsStore::get(cx).clone();

        let sec_key = "markdown";
        let is_sec_expanded = self.expanded_sections.contains(sec_key);
        let toggle_paste_dd = cx.entity().downgrade();
        let select_paste_win = cx.entity().downgrade();

        let markdown_props = MarkdownProps {
            show_table_headers: app_settings.markdown.show_table_headers,
            on_toggle_table_headers: Box::new(move |_event, _window, cx| {
                let _ = SettingsStore::update(cx, |s| s.markdown.show_table_headers = !s.markdown.show_table_headers);
            }),
            image_paste_behavior: app_settings.markdown.image_paste_behavior,
            is_image_paste_open: self.open_dropdown.as_deref() == Some("image_paste"),
            on_toggle_image_paste: Box::new(move |_event, _window, cx| {
                let _ = toggle_paste_dd.update(cx, |this, cx| {
                    if this.open_dropdown.as_deref() == Some("image_paste") {
                        this.open_dropdown = None;
                    } else {
                        this.open_dropdown = Some("image_paste".to_string());
                    }
                    cx.notify();
                });
            }),
            on_select_image_paste: Box::new(move |behavior| {
                let win = select_paste_win.clone();
                Box::new(move |_event, _window, cx| {
                    let _ = SettingsStore::update(cx, |s| s.markdown.image_paste_behavior = behavior);
                    let _ = win.update(cx, |this, cx| {
                        this.open_dropdown = None;
                        cx.notify();
                    });
                })
            }),
            render_math: app_settings.markdown.render_math,
            on_toggle_render_math: Box::new(move |_event, _window, cx| {
                let _ = SettingsStore::update(cx, |s| s.markdown.render_math = !s.markdown.render_math);
            }),
            render_diagrams: app_settings.markdown.render_diagrams,
            on_toggle_render_diagrams: Box::new(move |_event, _window, cx| {
                let _ = SettingsStore::update(cx, |s| s.markdown.render_diagrams = !s.markdown.render_diagrams);
            }),
        };

        sections.push(render_markdown_section(
            c,
            d,
            "win-pref-sec-md",
            is_sec_expanded,
            self.toggle_section_handler(cx, sec_key),
            markdown_props,
        ));

        sections
    }

    fn render_explorer_tab(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let mut sections = Vec::new();
        let app_settings = SettingsStore::get(cx).clone();

        let sec_key = "explorer";
        let is_sec_expanded = self.expanded_sections.contains(sec_key);
        let toggle_sort_mode_dd = cx.entity().downgrade();
        let toggle_sort_order_dd = cx.entity().downgrade();
        let select_sort_mode_win = cx.entity().downgrade();
        let select_sort_order_win = cx.entity().downgrade();

        let explorer_props = ExplorerProps {
            hide_hidden: app_settings.explorer.hide_hidden,
            on_toggle_hide_hidden: Box::new(move |_event, _window, cx| {
                let _ = SettingsStore::update(cx, |s| s.explorer.hide_hidden = !s.explorer.hide_hidden);
            }),
            sort_mode: app_settings.explorer.sort_mode,
            is_sort_mode_open: self.open_dropdown.as_deref() == Some("exp_sort_mode"),
            on_toggle_sort_mode: Box::new(move |_event, _window, cx| {
                let _ = toggle_sort_mode_dd.update(cx, |this, cx| {
                    if this.open_dropdown.as_deref() == Some("exp_sort_mode") {
                        this.open_dropdown = None;
                    } else {
                        this.open_dropdown = Some("exp_sort_mode".to_string());
                    }
                    cx.notify();
                });
            }),
            on_select_sort_mode: Box::new(move |mode| {
                let win = select_sort_mode_win.clone();
                Box::new(move |_event, _window, cx| {
                    let _ = SettingsStore::update(cx, |s| s.explorer.sort_mode = mode);
                    let _ = win.update(cx, |this, cx| {
                        this.open_dropdown = None;
                        cx.notify();
                    });
                })
            }),
            sort_order: app_settings.explorer.sort_order,
            is_sort_order_open: self.open_dropdown.as_deref() == Some("exp_sort_order"),
            on_toggle_sort_order: Box::new(move |_event, _window, cx| {
                let _ = toggle_sort_order_dd.update(cx, |this, cx| {
                    if this.open_dropdown.as_deref() == Some("exp_sort_order") {
                        this.open_dropdown = None;
                    } else {
                        this.open_dropdown = Some("exp_sort_order".to_string());
                    }
                    cx.notify();
                });
            }),
            on_select_sort_order: Box::new(move |order| {
                let win = select_sort_order_win.clone();
                Box::new(move |_event, _window, cx| {
                    let _ = SettingsStore::update(cx, |s| s.explorer.sort_order = order);
                    let _ = win.update(cx, |this, cx| {
                        this.open_dropdown = None;
                        cx.notify();
                    });
                })
            }),
            auto_reveal: app_settings.explorer.auto_reveal,
            on_toggle_auto_reveal: Box::new(move |_event, _window, cx| {
                let _ = SettingsStore::update(cx, |s| s.explorer.auto_reveal = !s.explorer.auto_reveal);
            }),
        };

        sections.push(render_explorer_section(
            c,
            d,
            "win-pref-sec-explorer",
            is_sec_expanded,
            self.toggle_section_handler(cx, sec_key),
            explorer_props,
        ));

        sections
    }

    fn render_startup_tab(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let mut sections = Vec::new();
        let app_settings = SettingsStore::get(cx).clone();

        let sec_key = "startup";
        let is_sec_expanded = self.expanded_sections.contains(sec_key);
        let toggle_startup_dd = cx.entity().downgrade();
        let select_startup_win = cx.entity().downgrade();

        let startup_props = StartupProps {
            startup_open: app_settings.startup.open,
            is_startup_open: self.open_dropdown.as_deref() == Some("startup"),
            on_toggle_startup: Box::new(move |_event, _window, cx| {
                let _ = toggle_startup_dd.update(cx, |this, cx| {
                    if this.open_dropdown.as_deref() == Some("startup") {
                        this.open_dropdown = None;
                    } else {
                        this.open_dropdown = Some("startup".to_string());
                    }
                    cx.notify();
                });
            }),
            on_select_startup: Box::new(move |open_setting| {
                let win = select_startup_win.clone();
                Box::new(move |_event, _window, cx| {
                    let _ = SettingsStore::update(cx, |s| s.startup.open = open_setting);
                    let _ = win.update(cx, |this, cx| {
                        this.open_dropdown = None;
                        cx.notify();
                    });
                })
            }),
            restore_window_state: app_settings.startup.restore_window_state,
            on_toggle_restore_window_state: Box::new(move |_event, _window, cx| {
                let _ = SettingsStore::update(cx, |s| s.startup.restore_window_state = !s.startup.restore_window_state);
            }),
        };

        sections.push(render_startup_section(
            c,
            d,
            "win-pref-sec-startup",
            is_sec_expanded,
            self.toggle_section_handler(cx, sec_key),
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

        let sec3_key = "editor_shortcuts";
        let is_sec3_expanded = self.expanded_sections.contains(sec3_key);
        sections.push(render_shortcuts_section(
            c,
            d,
            "win-pref-sec-editor-shortcuts",
            "Text Editing & Formatting",
            is_sec3_expanded,
            self.toggle_section_handler(cx, sec3_key),
            crate::settings::components::shortcuts_data::editor_editing_shortcuts(),
        ));

        sections
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

        let mut nav_elements = Vec::new();
        for tab in SettingsTab::all() {
            let t = *tab;
            let is_selected = self.nav == t;
            let win = cx.entity().downgrade();
            let tab_name = t.name();

            nav_elements.push(
                div()
                    .id(ElementId::Name(format!("win-nav-{}", tab_name).into()))
                    .w_full()
                    .child(nav_item(
                        tab_name,
                        tab_name,
                        is_selected,
                    ))
                    .on_click(move |_event, _window, cx| {
                        let _ = win.update(cx, |this, cx| {
                            this.nav = t;
                            cx.notify();
                        });
                    })
                    .into_any_element(),
            );
        }

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
            .children(nav_elements);

        let sections = match self.nav {
            SettingsTab::Interface => self.render_interface_tab(&theme, cx),
            SettingsTab::Editor => self.render_editor_tab(&theme, cx),
            SettingsTab::Markdown => self.render_markdown_tab(&theme, cx),
            SettingsTab::Explorer => self.render_explorer_tab(&theme, cx),
            SettingsTab::Startup => self.render_startup_tab(&theme, cx),
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

pub(crate) fn open_settings_window(cx: &mut App) -> Option<WindowHandle<SettingsWindow>> {
    let bounds = Bounds::centered(None, size(px(760.0), px(520.0)), cx);
    let title = cx
        .global::<I18nManager>()
        .strings()
        .settings_window_title
        .clone();
    let window_title = SharedString::from(title);

    let handle = match cx.open_window(
        splitype_window_options(window_title, bounds),
        move |_window, cx| cx.new(|cx| SettingsWindow::new(cx)),
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

#[cfg(test)]
mod tests {
    use super::AppSettings;
    use crate::infra::config::settings::StartupOpenSetting;

    #[test]
    fn test_app_settings_default() {
        let settings = AppSettings::default();
        assert_eq!(settings.startup.open, StartupOpenSetting::NewFile);
        assert_eq!(settings.interface.language_id, "en-US");
        assert_eq!(settings.interface.theme_id, "splitype");
        assert_eq!(settings.editor.tab_size, 4);
        assert_eq!(settings.editor.line_numbers, true);
        assert_eq!(settings.markdown.show_table_headers, true);
    }
}
