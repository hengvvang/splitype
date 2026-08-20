//! Editing settings tab: Typography, Markdown & Assets, Startup & Windows.

use gpui::*;

use crate::infra::config::settings::{EditorSettings, ImagePasteBehavior, StartupOpenSetting};
use crate::infra::theme::Theme;
use crate::settings::tabs::common::{make_row, make_section, render_zed_stepper};
use crate::settings::window::SettingsWindow;
use crate::ui::select::{select_option, select_panel, select_trigger};
use crate::ui::switch::Switch;

impl SettingsWindow {
    pub(crate) fn render_editing_tab(
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

        // Section 1: Typography & Formatting
        let sec1_key = "typography";
        let mut sec1_items = Vec::new();

        let font_dec = cx.entity().downgrade();
        let font_inc = cx.entity().downgrade();
        let font_ctr = cx.entity().downgrade();
        let curr_size = self.font_size;
        let is_editing_font = self.editing_stepper.as_deref() == Some("font");

        let ctrl_font = render_zed_stepper(
            c,
            d,
            "win-font-dec",
            "win-font-inc",
            format!("{}", curr_size),
            "px",
            is_editing_font,
            Box::new(move |_event, _window, cx| {
                let _ = font_dec.update(cx, |this, cx| {
                    this.editing_stepper = None;
                    if this.font_size > 8 {
                        this.font_size -= 1;
                        cx.notify();
                    }
                });
            }),
            Box::new(move |_event, _window, cx| {
                let _ = font_inc.update(cx, |this, cx| {
                    this.editing_stepper = None;
                    if this.font_size < 48 {
                        this.font_size += 1;
                        cx.notify();
                    }
                });
            }),
            Box::new(move |_event, _window, cx| {
                let _ = font_ctr.update(cx, |this, cx| {
                    this.editing_stepper = Some("font".to_string());
                    this.font_size = match this.font_size {
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
        );

        sec1_items.push(make_row(
            inner_border_color,
            c,
            d,
            "Editor Font Size",
            "Baseline font size in pixels for text editor content",
            ctrl_font,
        ));

        let lh_dec = cx.entity().downgrade();
        let lh_inc = cx.entity().downgrade();
        let lh_ctr = cx.entity().downgrade();
        let curr_lh = self.line_height;
        let is_editing_lh = self.editing_stepper.as_deref() == Some("line_height");

        let ctrl_lh = render_zed_stepper(
            c,
            d,
            "win-lh-dec",
            "win-lh-inc",
            format!("{:.1}", curr_lh),
            "",
            is_editing_lh,
            Box::new(move |_event, _window, cx| {
                let _ = lh_dec.update(cx, |this, cx| {
                    this.editing_stepper = None;
                    if this.line_height > 1.05 {
                        this.line_height = (this.line_height - 0.1).max(1.0);
                        cx.notify();
                    }
                });
            }),
            Box::new(move |_event, _window, cx| {
                let _ = lh_inc.update(cx, |this, cx| {
                    this.editing_stepper = None;
                    if this.line_height < 3.0 {
                        this.line_height = (this.line_height + 0.1).min(3.0);
                        cx.notify();
                    }
                });
            }),
            Box::new(move |_event, _window, cx| {
                let _ = lh_ctr.update(cx, |this, cx| {
                    this.editing_stepper = Some("line_height".to_string());
                    this.line_height = if (this.line_height - 1.2).abs() < 0.05 {
                        1.4
                    } else if (this.line_height - 1.4).abs() < 0.05 {
                        1.6
                    } else if (this.line_height - 1.6).abs() < 0.05 {
                        1.8
                    } else if (this.line_height - 1.8).abs() < 0.05 {
                        2.0
                    } else {
                        1.2
                    };
                    cx.notify();
                });
            }),
        );

        sec1_items.push(make_row(
            inner_border_color,
            c,
            d,
            "Line Height Multiplier",
            "Adjust vertical line spacing ratio for reading comfort",
            ctrl_lh,
        ));

        sections.push(make_section(
            c,
            d,
            "win-sec-typo",
            sec1_key,
            "Typography & Formatting",
            self.expanded_sections.contains(sec1_key),
            toggle_section_ed.clone(),
            sec1_items,
        ));

        // Section 2: Markdown & Assets
        let sec2_key = "markdown";
        let mut sec2_items = Vec::new();

        let tbl_ed = cx.entity().downgrade();
        let show_tb_headers = EditorSettings::show_table_headers(cx);
        let ctrl_tbl = Switch::new("win-switch-table-headers")
            .checked(show_tb_headers)
            .on_click(move |_event, _window, cx| {
                let _ = tbl_ed.update(cx, |_this, cx| {
                    let new_val = !EditorSettings::show_table_headers(cx);
                    EditorSettings::set_show_table_headers(cx, new_val);
                    cx.notify();
                });
            })
            .into_any_element();

        sec2_items.push(make_row(
            inner_border_color,
            c,
            d,
            "Table Column Headers",
            "Automatically render header row when formatting markdown tables",
            ctrl_tbl,
        ));

        let image_label = match self.image_paste_behavior {
            ImagePasteBehavior::CopyToAssetsFolder => "Save to Local Assets",
            ImagePasteBehavior::CopyToDocumentFolder => "Copy to Document Folder",
            ImagePasteBehavior::CopyToNamedAssetsFolder => "Insert Direct Link",
            ImagePasteBehavior::None => "None",
        };
        let image_btn_ed = cx.entity().downgrade();
        let mut image_btn_wrap = div().relative().child(
            select_trigger("pref-btn-win-image", c, d)
                .text_size(px(12.0))
                .text_color(c.text_default)
                .child(div().flex_1().min_w(px(0.0)).truncate().child(image_label))
                .child(
                    div().flex_shrink_0().pl(px(4.0)).child(
                        svg()
                            .path("icons/settings/select-chevron.svg")
                            .size(px(16.0))
                            .text_color(c.dialog_muted),
                    ),
                )
                .on_click(move |_event, _window, cx| {
                    let _ = image_btn_ed.update(cx, |this, cx| {
                        this.image_dropdown_open = !this.image_dropdown_open;
                        this.startup_dropdown_open = false;
                        this.theme_dropdown_open = false;
                        this.lang_dropdown_open = false;
                        cx.notify();
                    });
                }),
        );

        if self.image_dropdown_open {
            let image_opts = [
                (
                    ImagePasteBehavior::CopyToAssetsFolder,
                    "Save to Local Assets",
                ),
                (
                    ImagePasteBehavior::CopyToDocumentFolder,
                    "Copy to Document Folder",
                ),
                (
                    ImagePasteBehavior::CopyToNamedAssetsFolder,
                    "Insert Direct Link",
                ),
            ];
            let mut menu_items = Vec::new();
            for (pref, label) in image_opts {
                let is_selected = pref == self.image_paste_behavior;
                let item_ed = cx.entity().downgrade();

                menu_items.push(
                    select_option(
                        ElementId::Name(format!("win-image-item-{:?}", pref).into()),
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
                    .on_click(move |event, window, cx| {
                        let _ = item_ed.update(cx, |this, cx| {
                            this.image_paste_behavior = pref;
                            this.image_dropdown_open = false;
                            this.save(event, window, cx);
                        });
                    })
                    .into_any_element(),
                );
            }

            image_btn_wrap =
                image_btn_wrap.child(gpui::deferred(select_panel(c).children(menu_items)));
        }

        sec2_items.push(make_row(
            inner_border_color,
            c,
            d,
            "Image Paste Action",
            "Default storage location when pasting images into document",
            image_btn_wrap.into_any_element(),
        ));

        sections.push(make_section(
            c,
            d,
            "win-sec-markdown",
            sec2_key,
            "Markdown & Assets",
            self.expanded_sections.contains(sec2_key),
            toggle_section_ed.clone(),
            sec2_items,
        ));

        // Section 3: Startup Options
        let sec3_key = "startup";
        let mut sec3_items = Vec::new();

        let startup_label = match self.startup_open {
            StartupOpenSetting::NewFile => "New Blank Document",
            StartupOpenSetting::LastOpenedFile => "Open Last Opened File",
        };
        let startup_btn_ed = cx.entity().downgrade();
        let mut startup_btn_wrap = div().relative().child(
            select_trigger("pref-btn-win-startup", c, d)
                .text_size(px(12.0))
                .text_color(c.text_default)
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .truncate()
                        .child(startup_label),
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
                    let _ = startup_btn_ed.update(cx, |this, cx| {
                        this.startup_dropdown_open = !this.startup_dropdown_open;
                        this.image_dropdown_open = false;
                        this.theme_dropdown_open = false;
                        this.lang_dropdown_open = false;
                        cx.notify();
                    });
                }),
        );

        if self.startup_dropdown_open {
            let startup_opts = [
                (StartupOpenSetting::NewFile, "New Blank Document"),
                (StartupOpenSetting::LastOpenedFile, "Open Last Opened File"),
            ];
            let mut menu_items = Vec::new();
            for (pref, label) in startup_opts {
                let is_selected = pref == self.startup_open;
                let item_ed = cx.entity().downgrade();

                menu_items.push(
                    select_option(
                        ElementId::Name(format!("win-startup-item-{:?}", pref).into()),
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
                    .on_click(move |event, window, cx| {
                        let _ = item_ed.update(cx, |this, cx| {
                            this.startup_open = pref;
                            this.startup_dropdown_open = false;
                            this.save(event, window, cx);
                        });
                    })
                    .into_any_element(),
                );
            }

            startup_btn_wrap =
                startup_btn_wrap.child(gpui::deferred(select_panel(c).children(menu_items)));
        }

        sec3_items.push(make_row(
            inner_border_color,
            c,
            d,
            "On Startup",
            "Choose default document state when launching splitype editor",
            startup_btn_wrap.into_any_element(),
        ));

        sections.push(make_section(
            c,
            d,
            "win-sec-startup",
            sec3_key,
            "Startup Options",
            self.expanded_sections.contains(sec3_key),
            toggle_section_ed,
            sec3_items,
        ));

        sections
    }
}
