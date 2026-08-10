//! Settings panel rendered inside the editor's tiled layout.

use crate::ui::section::{section_card, section_header, settings_row};
use crate::ui::stepper::{stepper_container, stepper_divider, stepper_step_button, stepper_value};
use crate::ui::tab::nav_tab;

use crate::ui::select::{select_option, select_panel, select_trigger};

use gpui::*;

use crate::editor::controller::*;
use crate::editor::settings::SettingsTab;
use crate::infra::i18n::I18nStrings;
use crate::infra::theme::{Theme, ThemeManager};
use crate::ui::switch::Switch;

impl Editor {
    pub(crate) fn render_settings_midcontainer(
        &mut self,
        area_id: usize,
        theme: &Theme,
        _strings: &I18nStrings,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let active_tab = self.panels.settings.tab;

        let mut inner_border_color = c.dialog_border;
        inner_border_color.a *= 0.4;

        // --- Left Sidebar (3 Main Tabs: Interface, Editing, Keymap) ---
        let mut left_nav_items = Vec::new();
        for (tab_idx, tab) in SettingsTab::all().iter().enumerate() {
            let is_active = active_tab == *tab;
            let editor = cx.entity().downgrade();
            let tab_item = *tab;

            left_nav_items.push(
                nav_tab(
                    ElementId::Name(format!("pref-tab-{area_id}-{tab_idx}").into()),
                    c,
                    d,
                )
                .id(ElementId::Name(
                    format!("pref-tab-{area_id}-{tab_idx}").into(),
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
                    let _ = editor.update(cx, |ed, cx| {
                        ed.panels.settings.tab = tab_item;
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

        // --- Right Content Area ---
        let mut sections: Vec<AnyElement> = Vec::new();

        // Helper closures / local constructors to produce shallow type-erased elements
        let make_row = |title: &'static str,
                        desc: &'static str,
                        control: AnyElement,
                        theme: &Theme,
                        border_col: Hsla|
         -> AnyElement {
            let tc = &theme.colors;
            let td = &theme.dimensions;
            settings_row(border_col, tc, td)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(
                            div()
                                .text_size(px(12.5))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(tc.text_default)
                                .child(title),
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(tc.dialog_muted)
                                .child(desc),
                        ),
                )
                .child(control)
                .into_any_element()
        };

        let render_zed_stepper =
            |id_dec: &'static str,
             id_inc: &'static str,
             val_num: String,
             unit_str: &'static str,
             is_editing: bool,
             on_dec: Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>,
             on_inc: Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>,
             on_click_center: Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>,
             theme: &Theme|
             -> AnyElement {
                let tc = &theme.colors;
                let td = &theme.dimensions;

                let mut center_box = stepper_value()
                    .id(ElementId::Name(
                        format!("{}-center-{}", id_dec, area_id).into(),
                    ))
                    .bg(if is_editing {
                        tc.dialog_surface
                    } else {
                        tc.dialog_secondary_button_bg
                    })
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(tc.text_default)
                            .child(val_num),
                    );

                if is_editing {
                    center_box = center_box
                        .border_1()
                        .border_color(tc.dialog_primary_button_bg)
                        .child(div().w(px(1.5)).h(px(12.0)).bg(tc.dialog_primary_button_bg));
                }

                if !unit_str.is_empty() {
                    center_box = center_box.child(
                        div()
                            .text_size(px(11.0))
                            .text_color(tc.dialog_muted)
                            .child(unit_str),
                    );
                }

                let center_box = center_box.on_click(on_click_center);

                stepper_container(tc, td)
                    .child(
                        stepper_step_button(id_dec, tc)
                            .id((id_dec, area_id))
                            .child(
                                svg()
                                    .path("icons/settings/minus.svg")
                                    .size(px(12.0))
                                    .text_color(tc.dialog_secondary_button_text),
                            )
                            .on_click(on_dec),
                    )
                    .child(stepper_divider(tc))
                    .child(center_box)
                    .child(stepper_divider(tc))
                    .child(
                        stepper_step_button(id_inc, tc)
                            .id((id_inc, area_id))
                            .child(
                                svg()
                                    .path("icons/settings/plus.svg")
                                    .size(px(12.0))
                                    .text_color(tc.dialog_secondary_button_text),
                            )
                            .on_click(on_inc),
                    )
                    .into_any_element()
            };

        let make_section = |sec_id: &'static str,
                            title: &'static str,
                            is_expanded: bool,
                            toggle_fn: Box<dyn Fn(&gpui::ClickEvent, &mut Window, &mut App)>,
                            items: Vec<AnyElement>,
                            theme: &Theme|
         -> AnyElement {
            let tc = &theme.colors;
            let td = &theme.dimensions;

            let header = section_header()
                .id((sec_id, area_id))
                .child(
                    svg()
                        .path(if is_expanded {
                            "icons/settings/chevron-down.svg"
                        } else {
                            "icons/settings/chevron-right.svg"
                        })
                        .size(px(16.0))
                        .text_color(tc.text_default),
                )
                .child(
                    div()
                        .text_size(px(13.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(tc.text_default)
                        .child(title),
                )
                .on_click(move |ev, window, cx| toggle_fn(ev, window, cx));

            let mut card = section_card(tc, td).child(header);

            if is_expanded && !items.is_empty() {
                let body = div()
                    .w_full()
                    .px(px(10.0))
                    .pb(px(10.0))
                    .pt(px(2.0))
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .children(items);

                card = card.child(body);
            }

            card.into_any_element()
        };

        match active_tab {
            SettingsTab::Interface => {
                // Section 1: Visual Theme & Language
                let sec1_key = "theme";
                let is_sec1_expanded = self.panels.settings.expanded_sections.contains(sec1_key);
                let mut sec1_items = Vec::new();

                let theme_ed = cx.entity().downgrade();
                let available_themes = cx.global::<ThemeManager>().available_themes();
                let current_theme_name = theme.name.clone();

                let lang_ed = cx.entity().downgrade();
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
                                let theme_ed = theme_ed.clone();
                                move |_ev, _win, cx| {
                                    let _ = theme_ed.update(cx, |ed, cx| {
                                        if ed.panels.settings.open_dropdown.as_deref()
                                            == Some("theme")
                                        {
                                            ed.panels.settings.open_dropdown = None;
                                        } else {
                                            ed.panels.settings.open_dropdown =
                                                Some("theme".to_string());
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
                            let item_ed = theme_ed.clone();
                            let item_icon = if display_label == "Light" {
                                "icons/settings/sun.svg"
                            } else {
                                "icons/settings/moon.svg"
                            };

                            menu_items.push(
                                select_option(
                                    ElementId::Name(format!("theme-item-{}", t_id).into()),
                                    c,
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
                                .on_click(move |_ev, _win, cx| {
                                    let _ = item_ed.update(cx, |ed, cx| {
                                        cx.update_global::<ThemeManager, _>(|manager, _cx| {
                                            let _ = manager.set_theme_by_id(&t_id);
                                        });
                                        ed.panels.settings.open_dropdown = None;
                                        cx.notify();
                                    });
                                })
                                .into_any_element(),
                            );
                        }

                        theme_btn_wrap = theme_btn_wrap
                            .child(gpui::deferred(select_panel(c).children(menu_items)));
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
                                let lang_ed = lang_ed.clone();
                                move |_ev, _win, cx| {
                                    let _ = lang_ed.update(cx, |ed, cx| {
                                        if ed.panels.settings.open_dropdown.as_deref()
                                            == Some("lang")
                                        {
                                            ed.panels.settings.open_dropdown = None;
                                        } else {
                                            ed.panels.settings.open_dropdown =
                                                Some("lang".to_string());
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
                            let item_ed = lang_ed.clone();

                            menu_items.push(
                                select_option(
                                    ElementId::Name(format!("lang-item-{}", code).into()),
                                    c,
                                )
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
                                .on_click(move |_ev, _win, cx| {
                                    let _ = item_ed.update(cx, |ed, cx| {
                                        ed.panels.settings.open_dropdown = None;
                                        cx.notify();
                                    });
                                })
                                .into_any_element(),
                            );
                        }

                        lang_btn_wrap = lang_btn_wrap
                            .child(gpui::deferred(select_panel(c).children(menu_items)));
                    }

                    sec1_items.push(make_row(
                        "Display Language",
                        "Select preferred language for editor UI and dialogs",
                        lang_btn_wrap.into_any_element(),
                        theme,
                        inner_border_color,
                    ));
                }

                let sec1_ed = cx.entity().downgrade();
                sections.push(make_section(
                    "pref-sec-theme",
                    "Visual Theme & Language",
                    is_sec1_expanded,
                    Box::new(move |_ev, _win, cx| {
                        let _ = sec1_ed.update(cx, |ed, cx| {
                            ed.panels.settings.toggle_section(sec1_key);
                            cx.notify();
                        });
                    }),
                    sec1_items,
                    theme,
                ));

                // Section 2: Status Bar Options
                let sec2_key = "status_bar";
                let is_sec2_expanded = self.panels.settings.expanded_sections.contains(sec2_key);
                let mut sec2_items = Vec::new();

                if is_sec2_expanded {
                    let sub1_ed = cx.entity().downgrade();
                    let ctrl_sb_main = Switch::new("switch-sb-main")
                        .checked(self.panels.settings.pref_show_status_bar)
                        .on_click(move |_ev, _win, cx| {
                            let _ = sub1_ed.update(cx, |ed, cx| {
                                ed.panels.settings.pref_show_status_bar =
                                    !ed.panels.settings.pref_show_status_bar;
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

                    let sub2_ed = cx.entity().downgrade();
                    let ctrl_sb_words = Switch::new("switch-sb-words")
                        .checked(self.panels.settings.pref_show_word_count)
                        .on_click(move |_ev, _win, cx| {
                            let _ = sub2_ed.update(cx, |ed, cx| {
                                ed.panels.settings.pref_show_word_count =
                                    !ed.panels.settings.pref_show_word_count;
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

                    let sub3_ed = cx.entity().downgrade();
                    let ctrl_sb_pos = Switch::new("switch-sb-pos")
                        .checked(self.panels.settings.pref_show_cursor_pos)
                        .on_click(move |_ev, _win, cx| {
                            let _ = sub3_ed.update(cx, |ed, cx| {
                                ed.panels.settings.pref_show_cursor_pos =
                                    !ed.panels.settings.pref_show_cursor_pos;
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

                let sec2_ed = cx.entity().downgrade();
                sections.push(make_section(
                    "pref-sec-sb",
                    "Status Bar Options",
                    is_sec2_expanded,
                    Box::new(move |_ev, _win, cx| {
                        let _ = sec2_ed.update(cx, |ed, cx| {
                            ed.panels.settings.toggle_section(sec2_key);
                            cx.notify();
                        });
                    }),
                    sec2_items,
                    theme,
                ));
            }
            SettingsTab::Editing => {
                // Section 1: Typography & Formatting
                let sec1_key = "typography";
                let is_sec1_expanded = self.panels.settings.expanded_sections.contains(sec1_key);
                let mut sec1_items = Vec::new();

                if is_sec1_expanded {
                    let font_dec = cx.entity().downgrade();
                    let font_inc = cx.entity().downgrade();
                    let font_ctr = cx.entity().downgrade();
                    let curr_size = self.panels.settings.pref_font_size;
                    let is_editing_font =
                        self.panels.settings.editing_stepper.as_deref() == Some("font");

                    let ctrl_font = render_zed_stepper(
                        "font-dec",
                        "font-inc",
                        format!("{}", curr_size),
                        "px",
                        is_editing_font,
                        Box::new(move |_ev, _win, cx| {
                            let _ = font_dec.update(cx, |ed, cx| {
                                ed.panels.settings.editing_stepper = None;
                                if ed.panels.settings.pref_font_size > 8 {
                                    ed.panels.settings.pref_font_size -= 1;
                                    cx.notify();
                                }
                            });
                        }),
                        Box::new(move |_ev, _win, cx| {
                            let _ = font_inc.update(cx, |ed, cx| {
                                ed.panels.settings.editing_stepper = None;
                                if ed.panels.settings.pref_font_size < 48 {
                                    ed.panels.settings.pref_font_size += 1;
                                    cx.notify();
                                }
                            });
                        }),
                        Box::new(move |_ev, _win, cx| {
                            let _ = font_ctr.update(cx, |ed, cx| {
                                ed.panels.settings.editing_stepper = Some("font".to_string());
                                ed.panels.settings.pref_font_size =
                                    match ed.panels.settings.pref_font_size {
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
                        theme,
                    );

                    sec1_items.push(make_row(
                        "Editor Font Size",
                        "Baseline font size in pixels for text editor content",
                        ctrl_font,
                        theme,
                        inner_border_color,
                    ));

                    let lh_dec = cx.entity().downgrade();
                    let lh_inc = cx.entity().downgrade();
                    let lh_ctr = cx.entity().downgrade();
                    let curr_lh = self.panels.settings.pref_line_height;
                    let is_editing_lh =
                        self.panels.settings.editing_stepper.as_deref() == Some("line_height");

                    let ctrl_lh = render_zed_stepper(
                        "lh-dec",
                        "lh-inc",
                        format!("{:.1}", curr_lh),
                        "",
                        is_editing_lh,
                        Box::new(move |_ev, _win, cx| {
                            let _ = lh_dec.update(cx, |ed, cx| {
                                ed.panels.settings.editing_stepper = None;
                                if ed.panels.settings.pref_line_height > 1.05 {
                                    ed.panels.settings.pref_line_height =
                                        (ed.panels.settings.pref_line_height - 0.1).max(1.0);
                                    cx.notify();
                                }
                            });
                        }),
                        Box::new(move |_ev, _win, cx| {
                            let _ = lh_inc.update(cx, |ed, cx| {
                                ed.panels.settings.editing_stepper = None;
                                if ed.panels.settings.pref_line_height < 3.0 {
                                    ed.panels.settings.pref_line_height =
                                        (ed.panels.settings.pref_line_height + 0.1).min(3.0);
                                    cx.notify();
                                }
                            });
                        }),
                        Box::new(move |_ev, _win, cx| {
                            let _ = lh_ctr.update(cx, |ed, cx| {
                                ed.panels.settings.editing_stepper =
                                    Some("line_height".to_string());
                                ed.panels.settings.pref_line_height = if (ed
                                    .panels
                                    .settings
                                    .pref_line_height
                                    - 1.2)
                                    .abs()
                                    < 0.05
                                {
                                    1.4
                                } else if (ed.panels.settings.pref_line_height - 1.4).abs() < 0.05 {
                                    1.6
                                } else if (ed.panels.settings.pref_line_height - 1.6).abs() < 0.05 {
                                    1.8
                                } else if (ed.panels.settings.pref_line_height - 1.8).abs() < 0.05 {
                                    2.0
                                } else {
                                    1.2
                                };
                                cx.notify();
                            });
                        }),
                        theme,
                    );

                    sec1_items.push(make_row(
                        "Line Height Multiplier",
                        "Adjust vertical line spacing ratio for reading comfort",
                        ctrl_lh,
                        theme,
                        inner_border_color,
                    ));
                }

                let sec1_ed = cx.entity().downgrade();
                sections.push(make_section(
                    "pref-sec-typo",
                    "Typography & Formatting",
                    is_sec1_expanded,
                    Box::new(move |_ev, _win, cx| {
                        let _ = sec1_ed.update(cx, |ed, cx| {
                            ed.panels.settings.toggle_section(sec1_key);
                            cx.notify();
                        });
                    }),
                    sec1_items,
                    theme,
                ));

                // Section 2: Markdown & Assets
                let sec2_key = "markdown";
                let is_sec2_expanded = self.panels.settings.expanded_sections.contains(sec2_key);
                let mut sec2_items = Vec::new();

                let img_ed = cx.entity().downgrade();
                let img_options = [
                    (0, "Save to Local Assets"),
                    (1, "Copy to Document Folder"),
                    (2, "Insert Direct Link"),
                ];
                let curr_img_idx = self.panels.settings.pref_image_paste_action % img_options.len();
                let curr_img_label = img_options[curr_img_idx].1;
                let is_img_open = self.panels.settings.open_dropdown.as_deref() == Some("image");

                if is_sec2_expanded {
                    let tbl_ed = cx.entity().downgrade();
                    let ctrl_tbl = Switch::new("switch-table-headers")
                        .checked(self.panels.settings.pref_show_table_headers)
                        .on_click(move |_ev, _win, cx| {
                            let _ = tbl_ed.update(cx, |ed, cx| {
                                ed.panels.settings.pref_show_table_headers =
                                    !ed.panels.settings.pref_show_table_headers;
                                crate::infra::config::settings::EditorSettings::set_show_table_headers(
                                    cx,
                                    ed.panels.settings.pref_show_table_headers,
                                );
                                cx.notify();
                            });
                        })
                        .into_any_element();

                    sec2_items.push(make_row(
                        "Table Column Headers",
                        "Automatically render header row when formatting markdown tables",
                        ctrl_tbl,
                        theme,
                        inner_border_color,
                    ));

                    let mut img_btn_wrap = div().relative().child(
                        select_trigger("pref-btn-img", c, d)
                            .text_size(px(12.0))
                            .text_color(c.text_default)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .truncate()
                                    .child(curr_img_label),
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
                                let img_ed = img_ed.clone();
                                move |_ev, _win, cx| {
                                    let _ = img_ed.update(cx, |ed, cx| {
                                        if ed.panels.settings.open_dropdown.as_deref()
                                            == Some("image")
                                        {
                                            ed.panels.settings.open_dropdown = None;
                                        } else {
                                            ed.panels.settings.open_dropdown =
                                                Some("image".to_string());
                                        }
                                        cx.notify();
                                    });
                                }
                            }),
                    );

                    if is_img_open {
                        let mut menu_items = Vec::new();
                        for (idx, label) in img_options {
                            let is_selected = idx == curr_img_idx;
                            let item_ed = img_ed.clone();

                            menu_items.push(
                                select_option(
                                    ElementId::Name(format!("img-item-{}", idx).into()),
                                    c,
                                )
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
                                .on_click(move |_ev, _win, cx| {
                                    let _ = item_ed.update(cx, |ed, cx| {
                                        ed.panels.settings.pref_image_paste_action = idx;
                                        ed.panels.settings.open_dropdown = None;
                                        cx.notify();
                                    });
                                })
                                .into_any_element(),
                            );
                        }

                        img_btn_wrap = img_btn_wrap
                            .child(gpui::deferred(select_panel(c).children(menu_items)));
                    }

                    sec2_items.push(make_row(
                        "Image Paste Action",
                        "Default storage location when pasting images into document",
                        img_btn_wrap.into_any_element(),
                        theme,
                        inner_border_color,
                    ));
                }

                let sec2_ed = cx.entity().downgrade();
                sections.push(make_section(
                    "pref-sec-md",
                    "Markdown & Assets",
                    is_sec2_expanded,
                    Box::new(move |_ev, _win, cx| {
                        let _ = sec2_ed.update(cx, |ed, cx| {
                            ed.panels.settings.toggle_section(sec2_key);
                            cx.notify();
                        });
                    }),
                    sec2_items,
                    theme,
                ));

                // Section 3: Startup Behavior
                let sec3_key = "startup";
                let is_sec3_expanded = self.panels.settings.expanded_sections.contains(sec3_key);
                let mut sec3_items = Vec::new();

                let startup_ed = cx.entity().downgrade();
                let startup_options = [(0, "New Blank Document"), (1, "Open Last Opened File")];
                let curr_startup_idx =
                    self.panels.settings.pref_startup_option % startup_options.len();
                let curr_startup_label = startup_options[curr_startup_idx].1;
                let is_startup_open =
                    self.panels.settings.open_dropdown.as_deref() == Some("startup");

                if is_sec3_expanded {
                    let mut startup_btn_wrap = div().relative().child(
                        select_trigger("pref-btn-startup", c, d)
                            .text_size(px(12.0))
                            .text_color(c.text_default)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .truncate()
                                    .child(curr_startup_label),
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
                                let startup_ed = startup_ed.clone();
                                move |_ev, _win, cx| {
                                    let _ = startup_ed.update(cx, |ed, cx| {
                                        if ed.panels.settings.open_dropdown.as_deref()
                                            == Some("startup")
                                        {
                                            ed.panels.settings.open_dropdown = None;
                                        } else {
                                            ed.panels.settings.open_dropdown =
                                                Some("startup".to_string());
                                        }
                                        cx.notify();
                                    });
                                }
                            }),
                    );

                    if is_startup_open {
                        let mut menu_items = Vec::new();
                        for (idx, label) in startup_options {
                            let is_selected = idx == curr_startup_idx;
                            let item_ed = startup_ed.clone();

                            menu_items.push(
                                select_option(
                                    ElementId::Name(format!("startup-item-{}", idx).into()),
                                    c,
                                )
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
                                .on_click(move |_ev, _win, cx| {
                                    let _ = item_ed.update(cx, |ed, cx| {
                                        ed.panels.settings.pref_startup_option = idx;
                                        ed.panels.settings.open_dropdown = None;
                                        cx.notify();
                                    });
                                })
                                .into_any_element(),
                            );
                        }

                        startup_btn_wrap = startup_btn_wrap
                            .child(gpui::deferred(select_panel(c).children(menu_items)));
                    }

                    sec3_items.push(make_row(
                        "On Startup",
                        "Choose default document state when launching splitype editor",
                        startup_btn_wrap.into_any_element(),
                        theme,
                        inner_border_color,
                    ));
                }

                let sec3_ed = cx.entity().downgrade();
                sections.push(make_section(
                    "pref-sec-startup",
                    "Startup Behavior",
                    is_sec3_expanded,
                    Box::new(move |_ev, _win, cx| {
                        let _ = sec3_ed.update(cx, |ed, cx| {
                            ed.panels.settings.toggle_section(sec3_key);
                            cx.notify();
                        });
                    }),
                    sec3_items,
                    theme,
                ));
            }
            SettingsTab::Keymap => {
                // Section 1: Document Actions
                let sec1_key = "doc_actions";
                let is_sec1_expanded = self.panels.settings.expanded_sections.contains(sec1_key);
                let mut sec1_items = Vec::new();

                if is_sec1_expanded {
                    let doc_shortcuts = [
                        (
                            "Save Document",
                            "Save active file changes to disk",
                            "Ctrl + S",
                        ),
                        (
                            "Save Document As",
                            "Save active document with a new name",
                            "Ctrl + Shift + S",
                        ),
                        (
                            "New Window",
                            "Open a new editor window instance",
                            "Ctrl + N",
                        ),
                        (
                            "Close Window",
                            "Close the currently focused editor window",
                            "Ctrl + W",
                        ),
                    ];

                    for (name, desc, sc) in doc_shortcuts.iter() {
                        let ctrl_sc = div()
                            .px(px(8.0))
                            .py(px(2.0))
                            .rounded(px(d.menu_item_radius))
                            .bg(c.dialog_secondary_button_hover)
                            .text_size(px(11.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(c.text_default)
                            .child(*sc)
                            .into_any_element();

                        sec1_items.push(make_row(*name, *desc, ctrl_sc, theme, inner_border_color));
                    }
                }

                let sec1_ed = cx.entity().downgrade();
                sections.push(make_section(
                    "pref-sec-doc-actions",
                    "Document Actions",
                    is_sec1_expanded,
                    Box::new(move |_ev, _win, cx| {
                        let _ = sec1_ed.update(cx, |ed, cx| {
                            ed.panels.settings.toggle_section(sec1_key);
                            cx.notify();
                        });
                    }),
                    sec1_items,
                    theme,
                ));

                // Section 2: Interface & View Controls
                let sec2_key = "view_controls";
                let is_sec2_expanded = self.panels.settings.expanded_sections.contains(sec2_key);
                let mut sec2_items = Vec::new();

                if is_sec2_expanded {
                    let view_shortcuts = [
                        (
                            "Toggle View Mode",
                            "Switch between Edit, Preview, and Dual view layouts",
                            "Ctrl + M",
                        ),
                        (
                            "Toggle ExplorerState Tree",
                            "Show or collapse the left file navigation sidebar",
                            "Ctrl + E",
                        ),
                        (
                            "Quit Application",
                            "Safely exit application and save session",
                            "Ctrl + Q",
                        ),
                    ];

                    for (name, desc, sc) in view_shortcuts.iter() {
                        let ctrl_sc = div()
                            .px(px(8.0))
                            .py(px(2.0))
                            .rounded(px(d.menu_item_radius))
                            .bg(c.dialog_secondary_button_hover)
                            .text_size(px(11.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(c.text_default)
                            .child(*sc)
                            .into_any_element();

                        sec2_items.push(make_row(*name, *desc, ctrl_sc, theme, inner_border_color));
                    }
                }

                let sec2_ed = cx.entity().downgrade();
                sections.push(make_section(
                    "pref-sec-view-controls",
                    "Interface & View Controls",
                    is_sec2_expanded,
                    Box::new(move |_ev, _win, cx| {
                        let _ = sec2_ed.update(cx, |ed, cx| {
                            ed.panels.settings.toggle_section(sec2_key);
                            cx.notify();
                        });
                    }),
                    sec2_items,
                    theme,
                ));
            }
        }

        let right_content = div()
            .id(("pref-right-content", area_id))
            .relative()
            .flex_1()
            .h_full()
            .p(px(14.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .children(sections);

        // --- Main Root Layout ---
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
}
