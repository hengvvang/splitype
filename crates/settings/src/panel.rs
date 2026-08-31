//! In-window tiled settings panel — free-function renderers against the

use gpui::*;

use crate::components::*;
use config::language::{I18nStrings, apply_configured_language};
use config::settings::*;
use core_contracts::PanelId;
use theme::{Theme, ThemeManager, apply_configured_theme};

use crate::state::{SettingsTab, SettingsUiState};
use ui::tab::nav_tab;

pub fn render_settings_body(
    panel_id: PanelId,
    state: Entity<SettingsUiState>,
    theme: &Theme,
    _strings: &I18nStrings,
    cx: &mut App,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let active_tab = state.read(cx).tab;

    // Left Navigation Tabs
    let mut left_nav_items = Vec::new();
    for (tab_idx, tab) in SettingsTab::all().iter().enumerate() {
        let is_active = active_tab == *tab;
        let tab_item = *tab;
        let tab_state = state.clone();

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
                tab_state.update(cx, |s, _cx| {
                    s.tab = tab_item;
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
        SettingsTab::Interface => render_panel_interface_tab(panel_id, state.clone(), theme, cx),
        SettingsTab::Editor => render_panel_editor_tab(panel_id, state.clone(), theme, cx),
        SettingsTab::Markdown => render_panel_markdown_tab(panel_id, state.clone(), theme, cx),
        SettingsTab::Explorer => render_panel_explorer_tab(panel_id, state.clone(), theme, cx),
        SettingsTab::Startup => render_panel_startup_tab(panel_id, state.clone(), theme, cx),
        SettingsTab::Keymap => render_panel_keymap_tab(panel_id, state.clone(), theme, cx),
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

pub fn toggle_settings_section_handler(
    state: &Entity<SettingsUiState>,
    key: &'static str,
) -> crate::ui_helpers::SettingsClickHandler {
    let state = state.clone();
    Box::new(move |_event, _window, cx| {
        state.update(cx, |s, _cx| {
            if s.expanded_sections.contains(key) {
                s.expanded_sections.remove(key);
            } else {
                s.expanded_sections.insert(key.to_string());
            }
        });
    })
}

fn render_panel_interface_tab(
    panel_id: PanelId,
    state: Entity<SettingsUiState>,
    theme: &Theme,
    cx: &mut App,
) -> Vec<AnyElement> {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let mut sections = Vec::new();
    let app_settings = SettingsStore::get(cx).clone();

    // 1. Theme & Language Section
    let sec1_key = "theme";
    let is_sec1_expanded = state.read(cx).expanded_sections.contains(sec1_key);

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

    let toggle_theme_state = state.clone();
    let select_theme_state = state.clone();
    let toggle_lang_state = state.clone();
    let select_lang_state = state.clone();

    let theme_lang_props = ThemeLangProps {
        current_theme_name: theme.name.clone(),
        is_theme_dropdown_open: state.read(cx).open_dropdown.as_deref() == Some("theme"),
        on_toggle_theme_dropdown: Box::new(move |_event, _window, cx| {
            toggle_theme_state.update(cx, |s, _cx| {
                if s.open_dropdown.as_deref() == Some("theme") {
                    s.open_dropdown = None;
                } else {
                    s.open_dropdown = Some("theme".to_string());
                }
            });
        }),
        available_themes,
        on_select_theme: Box::new(move |theme_id| {
            let select_theme_state = select_theme_state.clone();
            Box::new(move |_event, _window, cx| {
                let _ = apply_configured_theme(cx, &theme_id);
                select_theme_state.update(cx, |s, _cx| {
                    s.open_dropdown = None;
                });
            })
        }),
        current_lang_name,
        is_lang_dropdown_open: state.read(cx).open_dropdown.as_deref() == Some("lang"),
        on_toggle_lang_dropdown: Box::new(move |_event, _window, cx| {
            toggle_lang_state.update(cx, |s, _cx| {
                if s.open_dropdown.as_deref() == Some("lang") {
                    s.open_dropdown = None;
                } else {
                    s.open_dropdown = Some("lang".to_string());
                }
            });
        }),
        lang_options: vec![("en-US", "English (en-US)"), ("zh-CN", "简体中文 (zh-CN)")],
        on_select_lang: Box::new(move |lang_code| {
            let select_lang_state = select_lang_state.clone();
            Box::new(move |_event, _window, cx| {
                let _ = apply_configured_language(cx, lang_code);
                select_lang_state.update(cx, |s, _cx| {
                    s.open_dropdown = None;
                });
            })
        }),
    };

    sections.push(render_theme_and_language_section(
        c,
        d,
        ("pref-sec-theme", panel_id.0),
        is_sec1_expanded,
        toggle_settings_section_handler(&state, sec1_key),
        theme_lang_props,
    ));

    // 2. Status Bar Section
    let sec2_key = "status_bar";
    let is_sec2_expanded = state.read(cx).expanded_sections.contains(sec2_key);

    let status_bar_props = StatusBarProps {
        show_status_bar: app_settings.status_bar.enabled,
        on_toggle_status_bar: Box::new(move |_event, _window, cx| {
            let _ = SettingsStore::update(cx, |s| {
                s.status_bar.enabled = !s.status_bar.enabled;
            });
        }),
        show_word_count: app_settings.status_bar.show_word_count,
        on_toggle_word_count: Box::new(move |_event, _window, cx| {
            let _ = SettingsStore::update(cx, |s| {
                s.status_bar.show_word_count = !s.status_bar.show_word_count;
            });
        }),
        show_cursor_pos: app_settings.status_bar.show_cursor_position,
        on_toggle_cursor_pos: Box::new(move |_event, _window, cx| {
            let _ = SettingsStore::update(cx, |s| {
                s.status_bar.show_cursor_position = !s.status_bar.show_cursor_position;
            });
        }),
        show_character_count: app_settings.status_bar.show_character_count,
        on_toggle_character_count: Box::new(move |_event, _window, cx| {
            let _ = SettingsStore::update(cx, |s| {
                s.status_bar.show_character_count = !s.status_bar.show_character_count;
            });
        }),
        show_reading_time: app_settings.status_bar.show_reading_time,
        on_toggle_reading_time: Box::new(move |_event, _window, cx| {
            let _ = SettingsStore::update(cx, |s| {
                s.status_bar.show_reading_time = !s.status_bar.show_reading_time;
            });
        }),
    };

    sections.push(render_status_bar_section(
        c,
        d,
        ("pref-sec-status-bar", panel_id.0),
        is_sec2_expanded,
        toggle_settings_section_handler(&state, sec2_key),
        status_bar_props,
    ));

    sections
}

fn render_panel_editor_tab(
    panel_id: PanelId,
    state: Entity<SettingsUiState>,
    theme: &Theme,
    cx: &mut App,
) -> Vec<AnyElement> {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let mut sections = Vec::new();
    let app_settings = SettingsStore::get(cx).clone();

    // 1. Typography & Fonts Section
    let sec1_key = "typography";
    let is_sec1_expanded = state.read(cx).expanded_sections.contains(sec1_key);

    let available_fonts: Vec<SharedString> = cx
        .text_system()
        .all_font_names()
        .into_iter()
        .map(SharedString::from)
        .collect();

    let is_ui_font_open = state.read(cx).open_dropdown.as_deref() == Some("ui_font");
    let is_prose_font_open = state.read(cx).open_dropdown.as_deref() == Some("prose_font");
    let is_code_font_open = state.read(cx).open_dropdown.as_deref() == Some("code_font");

    let ui_font_name = app_settings
        .typography
        .ui_font_family
        .clone()
        .unwrap_or_else(|| "Lexend (default)".to_string());
    let prose_font_name = app_settings
        .typography
        .prose_font_family
        .clone()
        .unwrap_or_else(|| "Lexend (default)".to_string());
    let code_font_name = app_settings
        .typography
        .code_font_family
        .clone()
        .unwrap_or_else(|| {
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

    let fs_focus = state.update(cx, |s, cx| {
        s.cached_focus_handle(cx, |s| &mut s.font_size_focus_handle)
    });
    let lh_focus = state.update(cx, |s, cx| {
        s.cached_focus_handle(cx, |s| &mut s.line_height_focus_handle)
    });

    let fs_focus_clone = fs_focus.clone();
    let lh_focus_clone = lh_focus.clone();

    let typography_props = TypographyProps {
        ui_font_name,
        is_ui_font_open,
        search_query_ui_font: state.read(cx).search_query_ui_font.clone(),
        on_toggle_ui_font: Box::new({
            let state = state.clone();
            move |_event, _window, cx| {
                state.update(cx, |s, _cx| {
                    if s.open_dropdown.as_deref() == Some("ui_font") {
                        s.open_dropdown = None;
                    } else {
                        s.open_dropdown = Some("ui_font".to_string());
                        s.search_query_ui_font.clear();
                    }
                });
            }
        }),
        on_search_ui_font: Box::new({
            let state = state.clone();
            move |query, _window, cx| {
                state.update(cx, |s, _cx| {
                    s.search_query_ui_font = query;
                });
            }
        }),
        on_select_ui_font: Box::new({
            let state = state.clone();
            move |font_name| {
                let state = state.clone();
                Box::new(move |_event, _window, cx| {
                    let _ = SettingsStore::update(cx, |s| {
                        s.typography.ui_font_family = if font_name == "default" {
                            None
                        } else {
                            Some(font_name.clone())
                        };
                    });
                    state.update(cx, |s, _cx| {
                        s.open_dropdown = None;
                    });
                })
            }
        }),
        on_reset_ui_font: if has_custom_ui {
            Some(Box::new(move |_event, _window, cx| {
                let _ = SettingsStore::update(cx, |s| {
                    s.typography.ui_font_family = None;
                });
            }))
        } else {
            None
        },

        prose_font_name,
        is_prose_font_open,
        search_query_prose_font: state.read(cx).search_query_prose_font.clone(),
        on_toggle_prose_font: Box::new({
            let state = state.clone();
            move |_event, _window, cx| {
                state.update(cx, |s, _cx| {
                    if s.open_dropdown.as_deref() == Some("prose_font") {
                        s.open_dropdown = None;
                    } else {
                        s.open_dropdown = Some("prose_font".to_string());
                        s.search_query_prose_font.clear();
                    }
                });
            }
        }),
        on_search_prose_font: Box::new({
            let state = state.clone();
            move |query, _window, cx| {
                state.update(cx, |s, _cx| {
                    s.search_query_prose_font = query;
                });
            }
        }),
        on_select_prose_font: Box::new({
            let state = state.clone();
            move |font_name| {
                let state = state.clone();
                Box::new(move |_event, _window, cx| {
                    let _ = SettingsStore::update(cx, |s| {
                        s.typography.prose_font_family = if font_name == "default" {
                            None
                        } else {
                            Some(font_name.clone())
                        };
                    });
                    state.update(cx, |s, _cx| {
                        s.open_dropdown = None;
                    });
                })
            }
        }),
        on_reset_prose_font: if has_custom_prose {
            Some(Box::new(move |_event, _window, cx| {
                let _ = SettingsStore::update(cx, |s| {
                    s.typography.prose_font_family = None;
                });
            }))
        } else {
            None
        },

        code_font_name,
        is_code_font_open,
        search_query_code_font: state.read(cx).search_query_code_font.clone(),
        on_toggle_code_font: Box::new({
            let state = state.clone();
            move |_event, _window, cx| {
                state.update(cx, |s, _cx| {
                    if s.open_dropdown.as_deref() == Some("code_font") {
                        s.open_dropdown = None;
                    } else {
                        s.open_dropdown = Some("code_font".to_string());
                        s.search_query_code_font.clear();
                    }
                });
            }
        }),
        on_search_code_font: Box::new({
            let state = state.clone();
            move |query, _window, cx| {
                state.update(cx, |s, _cx| {
                    s.search_query_code_font = query;
                });
            }
        }),
        on_select_code_font: Box::new({
            let state = state.clone();
            move |font_name| {
                let state = state.clone();
                Box::new(move |_event, _window, cx| {
                    let _ = SettingsStore::update(cx, |s| {
                        s.typography.code_font_family = if font_name == "default" {
                            None
                        } else {
                            Some(font_name.clone())
                        };
                    });
                    state.update(cx, |s, _cx| {
                        s.open_dropdown = None;
                    });
                })
            }
        }),
        on_reset_code_font: if has_custom_code {
            Some(Box::new(move |_event, _window, cx| {
                let _ = SettingsStore::update(cx, |s| {
                    s.typography.code_font_family = None;
                });
            }))
        } else {
            None
        },

        available_fonts,

        font_size: app_settings.typography.font_size,
        is_editing_font_size: state.read(cx).editing_font_size.is_some(),
        edit_buffer_font_size: state.read(cx).editing_font_size.clone(),
        font_size_focus_handle: fs_focus,
        on_font_dec: Box::new(move |event, _window, cx| {
            let step = if event.modifiers().shift { 4 } else { 1 };
            let _ = SettingsStore::update(cx, |s| {
                s.typography.font_size = if s.typography.font_size > 8 + step {
                    s.typography.font_size - step
                } else {
                    8
                };
            });
        }),
        on_font_inc: Box::new(move |event, _window, cx| {
            let step = if event.modifiers().shift { 4 } else { 1 };
            let _ = SettingsStore::update(cx, |s| {
                s.typography.font_size = (s.typography.font_size + step).min(72);
            });
        }),
        on_start_edit_font_size: Box::new({
            let state = state.clone();
            move |_event, window, cx| {
                window.focus(&fs_focus_clone, cx);
                let current_val = SettingsStore::get(cx).typography.font_size;
                state.update(cx, |s, _cx| {
                    s.editing_font_size = Some(format!("{}", current_val));
                    s.editing_line_height = None;
                });
            }
        }),
        on_key_down_font_size: Box::new({
            let state = state.clone();
            move |event, _window, cx| {
                let key = event.keystroke.key.as_str();
                state.update(cx, |s, cx| {
                    let mut commit_val = None;
                    let mut cancel = false;

                    if let Some(buf) = &mut s.editing_font_size {
                        match key {
                            "enter" => {
                                if let Ok(v) = buf.parse::<u32>() {
                                    commit_val = Some(v.clamp(8, 72));
                                } else {
                                    cancel = true;
                                }
                            }
                            "escape" => cancel = true,
                            "backspace" => {
                                buf.pop();
                            }
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
                            k if k.len() == 1
                                && k.chars().all(|c| c.is_ascii_digit())
                                && buf.len() < 2 =>
                            {
                                buf.push_str(k);
                            }
                            _ => {}
                        }
                    }

                    if let Some(v) = commit_val {
                        if key == "enter" {
                            s.editing_font_size = None;
                        }
                        let _ = SettingsStore::update(cx, |s| s.typography.font_size = v);
                    } else if cancel {
                        s.editing_font_size = None;
                    }
                });
            }
        }),
        on_reset_font_size: if has_custom_fs {
            Some(Box::new(move |_event, _window, cx| {
                let _ = SettingsStore::update(cx, |s| s.typography.font_size = 16);
            }))
        } else {
            None
        },

        line_height: app_settings.typography.line_height,
        is_editing_line_height: state.read(cx).editing_line_height.is_some(),
        edit_buffer_line_height: state.read(cx).editing_line_height.clone(),
        line_height_focus_handle: lh_focus,
        on_lh_dec: Box::new(move |event, _window, cx| {
            let step = if event.modifiers().shift { 0.2 } else { 0.05 };
            let _ = SettingsStore::update(cx, |s| {
                s.typography.line_height =
                    ((s.typography.line_height - step) * 100.0).round() / 100.0;
                if s.typography.line_height < 1.0 {
                    s.typography.line_height = 1.0;
                }
            });
        }),
        on_lh_inc: Box::new(move |event, _window, cx| {
            let step = if event.modifiers().shift { 0.2 } else { 0.05 };
            let _ = SettingsStore::update(cx, |s| {
                s.typography.line_height =
                    ((s.typography.line_height + step) * 100.0).round() / 100.0;
                if s.typography.line_height > 3.0 {
                    s.typography.line_height = 3.0;
                }
            });
        }),
        on_start_edit_line_height: Box::new({
            let state = state.clone();
            move |_event, window, cx| {
                window.focus(&lh_focus_clone, cx);
                let current_val = SettingsStore::get(cx).typography.line_height;
                state.update(cx, |s, _cx| {
                    s.editing_line_height = Some(format!("{:.2}", current_val));
                    s.editing_font_size = None;
                });
            }
        }),
        on_key_down_line_height: Box::new({
            let state = state.clone();
            move |event, _window, cx| {
                let key = event.keystroke.key.as_str();
                state.update(cx, |s, cx| {
                    let mut commit_val = None;
                    let mut cancel = false;

                    if let Some(buf) = &mut s.editing_line_height {
                        match key {
                            "enter" => {
                                if let Ok(v) = buf.parse::<f32>() {
                                    commit_val = Some(v.clamp(1.0, 3.0));
                                } else {
                                    cancel = true;
                                }
                            }
                            "escape" => cancel = true,
                            "backspace" => {
                                buf.pop();
                            }
                            k if k.len() == 1
                                && (k.chars().all(|c| c.is_ascii_digit())
                                    || (k == "." && !buf.contains('.')))
                                && buf.len() < 4 =>
                            {
                                buf.push_str(k);
                            }
                            _ => {}
                        }
                    }

                    if let Some(v) = commit_val {
                        if key == "enter" {
                            s.editing_line_height = None;
                        }
                        let _ = SettingsStore::update(cx, |s| s.typography.line_height = v);
                    } else if cancel {
                        s.editing_line_height = None;
                    }
                });
            }
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
        ("pref-sec-typography", panel_id.0),
        is_sec1_expanded,
        toggle_settings_section_handler(&state, sec1_key),
        typography_props,
    ));

    // 2. Editor Behavior Section
    let sec2_key = "editor_behavior";
    let is_sec2_expanded = state.read(cx).expanded_sections.contains(sec2_key);

    let tab_size_focus = state.update(cx, |s, cx| {
        s.cached_focus_handle(cx, |s| &mut s.tab_size_focus_handle)
    });
    let tab_size_focus_clone = tab_size_focus.clone();

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
        is_editing_tab_size: state.read(cx).editing_tab_size.is_some(),
        edit_buffer_tab_size: state.read(cx).editing_tab_size.clone(),
        tab_size_focus_handle: tab_size_focus,
        on_tab_size_dec: Box::new(move |_event, _window, cx| {
            let _ = SettingsStore::update(cx, |s| {
                s.editor.tab_size = if s.editor.tab_size > 2 {
                    s.editor.tab_size - 2
                } else {
                    2
                };
            });
        }),
        on_tab_size_inc: Box::new(move |_event, _window, cx| {
            let _ = SettingsStore::update(cx, |s| {
                s.editor.tab_size = (s.editor.tab_size + 2).min(8);
            });
        }),
        on_start_edit_tab_size: Box::new({
            let state = state.clone();
            move |_event, window, cx| {
                window.focus(&tab_size_focus_clone, cx);
                let current_val = SettingsStore::get(cx).editor.tab_size;
                state.update(cx, |s, _cx| {
                    s.editing_tab_size = Some(format!("{}", current_val));
                });
            }
        }),
        on_key_down_tab_size: Box::new({
            let state = state.clone();
            move |event, _window, cx| {
                let key = event.keystroke.key.as_str();
                state.update(cx, |s, cx| {
                    let mut commit_val = None;
                    let mut cancel = false;

                    if let Some(buf) = &mut s.editing_tab_size {
                        match key {
                            "enter" => {
                                if let Ok(v) = buf.parse::<u32>() {
                                    commit_val = Some(v.clamp(2, 8));
                                } else {
                                    cancel = true;
                                }
                            }
                            "escape" => cancel = true,
                            "backspace" => {
                                buf.pop();
                            }
                            k if k.len() == 1
                                && k.chars().all(|c| c.is_ascii_digit())
                                && buf.is_empty() =>
                            {
                                buf.push_str(k);
                            }
                            _ => {}
                        }
                    }

                    if let Some(v) = commit_val {
                        if key == "enter" {
                            s.editing_tab_size = None;
                        }
                        let _ = SettingsStore::update(cx, |s| s.editor.tab_size = v);
                    } else if cancel {
                        s.editing_tab_size = None;
                    }
                });
            }
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
            let _ = SettingsStore::update(cx, |s| {
                s.editor.highlight_active_line = !s.editor.highlight_active_line
            });
        }),
    };

    sections.push(render_editor_behavior_section(
        c,
        d,
        ("pref-sec-editor-behavior", panel_id.0),
        is_sec2_expanded,
        toggle_settings_section_handler(&state, sec2_key),
        editor_behavior_props,
    ));

    sections
}

fn render_panel_markdown_tab(
    panel_id: PanelId,
    state: Entity<SettingsUiState>,
    theme: &Theme,
    cx: &mut App,
) -> Vec<AnyElement> {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let mut sections = Vec::new();
    let app_settings = SettingsStore::get(cx).clone();

    let sec_key = "markdown";
    let is_sec_expanded = state.read(cx).expanded_sections.contains(sec_key);

    let markdown_props = MarkdownProps {
        show_table_headers: app_settings.markdown.show_table_headers,
        on_toggle_table_headers: Box::new(move |_event, _window, cx| {
            let _ = SettingsStore::update(cx, |s| {
                s.markdown.show_table_headers = !s.markdown.show_table_headers
            });
        }),
        image_paste_behavior: app_settings.markdown.image_paste_behavior,
        is_image_paste_open: state.read(cx).open_dropdown.as_deref() == Some("image_paste"),
        on_toggle_image_paste: Box::new({
            let state = state.clone();
            move |_event, _window, cx| {
                state.update(cx, |s, _cx| {
                    if s.open_dropdown.as_deref() == Some("image_paste") {
                        s.open_dropdown = None;
                    } else {
                        s.open_dropdown = Some("image_paste".to_string());
                    }
                });
            }
        }),
        on_select_image_paste: Box::new({
            let state = state.clone();
            move |behavior| {
                let state = state.clone();
                Box::new(move |_event, _window, cx| {
                    let _ =
                        SettingsStore::update(cx, |s| s.markdown.image_paste_behavior = behavior);
                    state.update(cx, |s, _cx| {
                        s.open_dropdown = None;
                    });
                })
            }
        }),
        render_math: app_settings.markdown.render_math,
        on_toggle_render_math: Box::new(move |_event, _window, cx| {
            let _ = SettingsStore::update(cx, |s| s.markdown.render_math = !s.markdown.render_math);
        }),
        render_diagrams: app_settings.markdown.render_diagrams,
        on_toggle_render_diagrams: Box::new(move |_event, _window, cx| {
            let _ = SettingsStore::update(cx, |s| {
                s.markdown.render_diagrams = !s.markdown.render_diagrams
            });
        }),
    };

    sections.push(render_markdown_section(
        c,
        d,
        ("pref-sec-md", panel_id.0),
        is_sec_expanded,
        toggle_settings_section_handler(&state, sec_key),
        markdown_props,
    ));

    sections
}

fn render_panel_explorer_tab(
    panel_id: PanelId,
    state: Entity<SettingsUiState>,
    theme: &Theme,
    cx: &mut App,
) -> Vec<AnyElement> {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let mut sections = Vec::new();
    let app_settings = SettingsStore::get(cx).clone();

    let sec_key = "explorer";
    let is_sec_expanded = state.read(cx).expanded_sections.contains(sec_key);

    let explorer_props = ExplorerProps {
        hide_hidden: app_settings.explorer.hide_hidden,
        on_toggle_hide_hidden: Box::new(move |_event, _window, cx| {
            let _ = SettingsStore::update(cx, |s| s.explorer.hide_hidden = !s.explorer.hide_hidden);
        }),
        sort_mode: app_settings.explorer.sort_mode,
        is_sort_mode_open: state.read(cx).open_dropdown.as_deref() == Some("exp_sort_mode"),
        on_toggle_sort_mode: Box::new({
            let state = state.clone();
            move |_event, _window, cx| {
                state.update(cx, |s, _cx| {
                    if s.open_dropdown.as_deref() == Some("exp_sort_mode") {
                        s.open_dropdown = None;
                    } else {
                        s.open_dropdown = Some("exp_sort_mode".to_string());
                    }
                });
            }
        }),
        on_select_sort_mode: Box::new({
            let state = state.clone();
            move |mode| {
                let state = state.clone();
                Box::new(move |_event, _window, cx| {
                    let _ = SettingsStore::update(cx, |s| s.explorer.sort_mode = mode);
                    state.update(cx, |s, _cx| {
                        s.open_dropdown = None;
                    });
                })
            }
        }),
        sort_order: app_settings.explorer.sort_order,
        is_sort_order_open: state.read(cx).open_dropdown.as_deref() == Some("exp_sort_order"),
        on_toggle_sort_order: Box::new({
            let state = state.clone();
            move |_event, _window, cx| {
                state.update(cx, |s, _cx| {
                    if s.open_dropdown.as_deref() == Some("exp_sort_order") {
                        s.open_dropdown = None;
                    } else {
                        s.open_dropdown = Some("exp_sort_order".to_string());
                    }
                });
            }
        }),
        on_select_sort_order: Box::new({
            let state = state.clone();
            move |order| {
                let state = state.clone();
                Box::new(move |_event, _window, cx| {
                    let _ = SettingsStore::update(cx, |s| s.explorer.sort_order = order);
                    state.update(cx, |s, _cx| {
                        s.open_dropdown = None;
                    });
                })
            }
        }),
        auto_reveal: app_settings.explorer.auto_reveal,
        on_toggle_auto_reveal: Box::new(move |_event, _window, cx| {
            let _ = SettingsStore::update(cx, |s| s.explorer.auto_reveal = !s.explorer.auto_reveal);
        }),
    };

    sections.push(render_explorer_section(
        c,
        d,
        ("pref-sec-explorer", panel_id.0),
        is_sec_expanded,
        toggle_settings_section_handler(&state, sec_key),
        explorer_props,
    ));

    sections
}

fn render_panel_startup_tab(
    panel_id: PanelId,
    state: Entity<SettingsUiState>,
    theme: &Theme,
    cx: &mut App,
) -> Vec<AnyElement> {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let mut sections = Vec::new();
    let app_settings = SettingsStore::get(cx).clone();

    let sec_key = "startup";
    let is_sec_expanded = state.read(cx).expanded_sections.contains(sec_key);

    let startup_props = StartupProps {
        startup_open: app_settings.startup.open,
        is_startup_open: state.read(cx).open_dropdown.as_deref() == Some("startup"),
        on_toggle_startup: Box::new({
            let state = state.clone();
            move |_event, _window, cx| {
                state.update(cx, |s, _cx| {
                    if s.open_dropdown.as_deref() == Some("startup") {
                        s.open_dropdown = None;
                    } else {
                        s.open_dropdown = Some("startup".to_string());
                    }
                });
            }
        }),
        on_select_startup: Box::new({
            let state = state.clone();
            move |open_setting| {
                let state = state.clone();
                Box::new(move |_event, _window, cx| {
                    let _ = SettingsStore::update(cx, |s| s.startup.open = open_setting);
                    state.update(cx, |s, _cx| {
                        s.open_dropdown = None;
                    });
                })
            }
        }),
        restore_window_state: app_settings.startup.restore_window_state,
        on_toggle_restore_window_state: Box::new(move |_event, _window, cx| {
            let _ = SettingsStore::update(cx, |s| {
                s.startup.restore_window_state = !s.startup.restore_window_state
            });
        }),
    };

    sections.push(render_startup_section(
        c,
        d,
        ("pref-sec-startup", panel_id.0),
        is_sec_expanded,
        toggle_settings_section_handler(&state, sec_key),
        startup_props,
    ));

    sections
}

fn render_panel_keymap_tab(
    panel_id: PanelId,
    state: Entity<SettingsUiState>,
    theme: &Theme,
    cx: &mut App,
) -> Vec<AnyElement> {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let mut sections = Vec::new();

    let sec1_key = "doc_actions";
    let is_sec1_expanded = state.read(cx).expanded_sections.contains(sec1_key);
    sections.push(render_shortcuts_section(
        c,
        d,
        ("pref-sec-doc-actions", panel_id.0),
        "Document Actions",
        is_sec1_expanded,
        toggle_settings_section_handler(&state, sec1_key),
        crate::components::shortcuts_data::doc_action_shortcuts(),
    ));

    let sec2_key = "view_controls";
    let is_sec2_expanded = state.read(cx).expanded_sections.contains(sec2_key);
    sections.push(render_shortcuts_section(
        c,
        d,
        ("pref-sec-view-controls", panel_id.0),
        "Interface & View Controls",
        is_sec2_expanded,
        toggle_settings_section_handler(&state, sec2_key),
        crate::components::shortcuts_data::interface_view_shortcuts(),
    ));

    let sec3_key = "editor_shortcuts";
    let is_sec3_expanded = state.read(cx).expanded_sections.contains(sec3_key);
    sections.push(render_shortcuts_section(
        c,
        d,
        ("pref-sec-editor-shortcuts", panel_id.0),
        "Text Editing & Formatting",
        is_sec3_expanded,
        toggle_settings_section_handler(&state, sec3_key),
        crate::components::shortcuts_data::editor_editing_shortcuts(),
    ));

    sections
}
