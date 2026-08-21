//! Editing settings tab for the in-editor slide-over panel.

use gpui::*;

use crate::app::shell::Shell;
use crate::infra::theme::Theme;
use crate::settings::panels::common::{make_row, make_section, render_zed_stepper};
use crate::ui::select::{select_option, select_panel, select_trigger};
use crate::ui::switch::Switch;

impl Shell {
    pub(crate) fn render_panel_editing_tab(
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

        // Section 1: Typography & Formatting
        let sec1_key = "typography";
        let is_sec1_expanded = self.panels.settings.expanded_sections.contains(sec1_key);
        let mut sec1_items = Vec::new();

        if is_sec1_expanded {
            let font_dec = cx.entity().downgrade();
            let font_inc = cx.entity().downgrade();
            let font_ctr = cx.entity().downgrade();
            let curr_size = self.panels.settings.pref_font_size;
            let is_editing_font = self.panels.settings.editing_stepper.as_deref() == Some("font");

            let ctrl_font = render_zed_stepper(
                "font-dec",
                "font-inc",
                format!("{}", curr_size),
                "px",
                is_editing_font,
                Box::new(move |_event, _window, cx| {
                    let _ = font_dec.update(cx, |shell, cx| {
                        shell.panels.settings.editing_stepper = None;
                        if shell.panels.settings.pref_font_size > 8 {
                            shell.panels.settings.pref_font_size -= 1;
                            cx.notify();
                        }
                    });
                }),
                Box::new(move |_event, _window, cx| {
                    let _ = font_inc.update(cx, |shell, cx| {
                        shell.panels.settings.editing_stepper = None;
                        if shell.panels.settings.pref_font_size < 48 {
                            shell.panels.settings.pref_font_size += 1;
                            cx.notify();
                        }
                    });
                }),
                Box::new(move |_event, _window, cx| {
                    let _ = font_ctr.update(cx, |shell, cx| {
                        shell.panels.settings.editing_stepper = Some("font".to_string());
                        shell.panels.settings.pref_font_size =
                            match shell.panels.settings.pref_font_size {
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
                panel_id,
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
                Box::new(move |_event, _window, cx| {
                    let _ = lh_dec.update(cx, |shell, cx| {
                        shell.panels.settings.editing_stepper = None;
                        if shell.panels.settings.pref_line_height > 1.05 {
                            shell.panels.settings.pref_line_height =
                                (shell.panels.settings.pref_line_height - 0.1).max(1.0);
                            cx.notify();
                        }
                    });
                }),
                Box::new(move |_event, _window, cx| {
                    let _ = lh_inc.update(cx, |shell, cx| {
                        shell.panels.settings.editing_stepper = None;
                        if shell.panels.settings.pref_line_height < 3.0 {
                            shell.panels.settings.pref_line_height =
                                (shell.panels.settings.pref_line_height + 0.1).min(3.0);
                            cx.notify();
                        }
                    });
                }),
                Box::new(move |_event, _window, cx| {
                    let _ = lh_ctr.update(cx, |shell, cx| {
                        shell.panels.settings.editing_stepper = Some("line_height".to_string());
                        shell.panels.settings.pref_line_height =
                            if (shell.panels.settings.pref_line_height - 1.2).abs() < 0.05 {
                                1.4
                            } else if (shell.panels.settings.pref_line_height - 1.4).abs() < 0.05 {
                                1.6
                            } else if (shell.panels.settings.pref_line_height - 1.6).abs() < 0.05 {
                                1.8
                            } else if (shell.panels.settings.pref_line_height - 1.8).abs() < 0.05 {
                                2.0
                            } else {
                                1.2
                            };
                        cx.notify();
                    });
                }),
                theme,
                panel_id,
            );

            sec1_items.push(make_row(
                "Line Height Multiplier",
                "Adjust vertical line spacing ratio for reading comfort",
                ctrl_lh,
                theme,
                inner_border_color,
            ));
        }

        let sec1_shell = cx.entity().downgrade();
        sections.push(make_section(
            "pref-sec-typo",
            "Typography & Formatting",
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

        // Section 2: Markdown & Assets
        let sec2_key = "markdown";
        let is_sec2_expanded = self.panels.settings.expanded_sections.contains(sec2_key);
        let mut sec2_items = Vec::new();

        let img_shell = cx.entity().downgrade();
        let img_options = [
            (0, "Save to Local Assets"),
            (1, "Copy to Document Folder"),
            (2, "Insert Direct Link"),
        ];
        let curr_img_idx = self.panels.settings.pref_image_paste_action % img_options.len();
        let curr_img_label = img_options[curr_img_idx].1;
        let is_img_open = self.panels.settings.open_dropdown.as_deref() == Some("image");

        if is_sec2_expanded {
            let tbl_shell = cx.entity().downgrade();
            let ctrl_tbl = Switch::new("switch-table-headers")
                .checked(self.panels.settings.pref_show_table_headers)
                .on_click(move |_event, _window, cx| {
                    let _ = tbl_shell.update(cx, |shell, cx| {
                        shell.panels.settings.pref_show_table_headers =
                            !shell.panels.settings.pref_show_table_headers;
                        crate::infra::config::settings::EditorSettings::set_show_table_headers(
                            cx,
                            shell.panels.settings.pref_show_table_headers,
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
                        let img_shell = img_shell.clone();
                        move |_event, _window, cx| {
                            let _ = img_shell.update(cx, |shell, cx| {
                                if shell.panels.settings.open_dropdown.as_deref() == Some("image") {
                                    shell.panels.settings.open_dropdown = None;
                                } else {
                                    shell.panels.settings.open_dropdown = Some("image".to_string());
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
                    let item_shell = img_shell.clone();

                    menu_items.push(
                        select_option(ElementId::Name(format!("img-item-{}", idx).into()), c, d)
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
                                    shell.panels.settings.pref_image_paste_action = idx;
                                    shell.panels.settings.open_dropdown = None;
                                    cx.notify();
                                });
                            })
                            .into_any_element(),
                    );
                }

                img_btn_wrap =
                    img_btn_wrap.child(gpui::deferred(select_panel(c, d).children(menu_items)));
            }

            sec2_items.push(make_row(
                "Image Paste Action",
                "Default storage location when pasting images into document",
                img_btn_wrap.into_any_element(),
                theme,
                inner_border_color,
            ));
        }

        let sec2_shell = cx.entity().downgrade();
        sections.push(make_section(
            "pref-sec-md",
            "Markdown & Assets",
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

        // Section 3: Startup Behavior
        let sec3_key = "startup";
        let is_sec3_expanded = self.panels.settings.expanded_sections.contains(sec3_key);
        let mut sec3_items = Vec::new();

        let startup_shell = cx.entity().downgrade();
        let startup_options = [(0, "New Blank Document"), (1, "Open Last Opened File")];
        let curr_startup_idx = self.panels.settings.pref_startup_option % startup_options.len();
        let curr_startup_label = startup_options[curr_startup_idx].1;
        let is_startup_open = self.panels.settings.open_dropdown.as_deref() == Some("startup");

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
                        let startup_shell = startup_shell.clone();
                        move |_event, _window, cx| {
                            let _ = startup_shell.update(cx, |shell, cx| {
                                if shell.panels.settings.open_dropdown.as_deref() == Some("startup")
                                {
                                    shell.panels.settings.open_dropdown = None;
                                } else {
                                    shell.panels.settings.open_dropdown =
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
                    let item_shell = startup_shell.clone();

                    menu_items.push(
                        select_option(ElementId::Name(format!("startup-item-{}", idx).into()), c, d)
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
                                    shell.panels.settings.pref_startup_option = idx;
                                    shell.panels.settings.open_dropdown = None;
                                    cx.notify();
                                });
                            })
                            .into_any_element(),
                    );
                }

                startup_btn_wrap =
                    startup_btn_wrap.child(gpui::deferred(select_panel(c, d).children(menu_items)));
            }

            sec3_items.push(make_row(
                "On Startup",
                "Choose default document state when launching splitype editor",
                startup_btn_wrap.into_any_element(),
                theme,
                inner_border_color,
            ));
        }

        let sec3_shell = cx.entity().downgrade();
        sections.push(make_section(
            "pref-sec-startup",
            "Startup Behavior",
            is_sec3_expanded,
            Box::new(move |_event, _window, cx| {
                let _ = sec3_shell.update(cx, |shell, cx| {
                    shell.panels.settings.toggle_section(sec3_key);
                    cx.notify();
                });
            }),
            sec3_items,
            theme,
            panel_id,
        ));

        sections
    }
}
