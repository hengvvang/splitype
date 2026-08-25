//! Editing settings components: Typography, Markdown & Assets, Startup.

use gpui::*;

use crate::infra::theme::{ThemeColors, ThemeDimensions};
use crate::settings::ui_helpers::{SettingsClickHandler, make_row, make_section, render_zed_stepper};
use crate::ui::select::{select_option, select_panel, select_trigger};
use crate::ui::switch::Switch;

pub(crate) struct TypographyProps {
    pub font_size: u32,
    pub is_editing_font: bool,
    pub on_font_dec: SettingsClickHandler,
    pub on_font_inc: SettingsClickHandler,
    pub on_font_cycle: SettingsClickHandler,

    pub line_height: f32,
    pub is_editing_lh: bool,
    pub on_lh_dec: SettingsClickHandler,
    pub on_lh_inc: SettingsClickHandler,
    pub on_lh_cycle: SettingsClickHandler,
}

pub(crate) fn render_typography_section(
    c: &ThemeColors,
    d: &ThemeDimensions,
    id: impl Into<ElementId>,
    expanded: bool,
    toggle_fn: SettingsClickHandler,
    props: TypographyProps,
) -> AnyElement {
    let mut inner_border_color = c.dialog_border;
    inner_border_color.a *= 0.4;

    let mut rows = Vec::new();

    if expanded {
        let ctrl_font = render_zed_stepper(
            c,
            d,
            "pref-font-dec",
            "pref-font-inc",
            format!("{}", props.font_size),
            "px",
            props.is_editing_font,
            props.on_font_dec,
            props.on_font_inc,
            props.on_font_cycle,
        );

        rows.push(make_row(
            inner_border_color,
            c,
            d,
            "Editor Font Size",
            "Baseline font size in pixels for text editor content",
            ctrl_font,
        ));

        let ctrl_lh = render_zed_stepper(
            c,
            d,
            "pref-lh-dec",
            "pref-lh-inc",
            format!("{:.1}", props.line_height),
            "",
            props.is_editing_lh,
            props.on_lh_dec,
            props.on_lh_inc,
            props.on_lh_cycle,
        );

        rows.push(make_row(
            inner_border_color,
            c,
            d,
            "Line Spacing Multiplier",
            "Proportional line height for comfortable reading",
            ctrl_lh,
        ));
    }

    make_section(c, d, id, "Typography & Formatting", expanded, toggle_fn, rows)
}

pub(crate) struct MarkdownProps {
    pub show_table_headers: bool,
    pub on_toggle_table_headers: SettingsClickHandler,

    pub image_paste_action: usize,
    pub is_image_paste_open: bool,
    pub on_toggle_image_paste: SettingsClickHandler,
    pub on_select_image_paste: Box<dyn Fn(usize) -> SettingsClickHandler>,
}

pub(crate) fn render_markdown_section(
    c: &ThemeColors,
    d: &ThemeDimensions,
    id: impl Into<ElementId>,
    expanded: bool,
    toggle_fn: SettingsClickHandler,
    props: MarkdownProps,
) -> AnyElement {
    let mut inner_border_color = c.dialog_border;
    inner_border_color.a *= 0.4;

    let mut rows = Vec::new();

    if expanded {
        let ctrl_tbl = Switch::new("switch-tbl-headers")
            .checked(props.show_table_headers)
            .on_click(props.on_toggle_table_headers)
            .into_any_element();

        rows.push(make_row(
            inner_border_color,
            c,
            d,
            "Persistent Table Headers",
            "Keep table column headers visible when editing table blocks",
            ctrl_tbl,
        ));

        let paste_options = [
            "Save image to local .assets/ folder",
            "Embed inline as Base64 data URI",
            "Upload to remote cloud storage",
        ];
        let current_paste_label = paste_options
            .get(props.image_paste_action)
            .copied()
            .unwrap_or(paste_options[0]);

        let mut paste_btn_wrap = div().relative().child(
            select_trigger("pref-btn-img-paste", c, d)
                .text_size(px(12.0))
                .text_color(c.text_default)
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .truncate()
                        .child(current_paste_label),
                )
                .child(
                    div().flex_shrink_0().pl(px(4.0)).child(
                        svg()
                            .path("icons/settings/select-chevron.svg")
                            .size(px(16.0))
                            .text_color(c.dialog_muted),
                    ),
                )
                .on_click(props.on_toggle_image_paste),
        );

        if props.is_image_paste_open {
            let mut menu_items = Vec::new();
            for (idx, opt_label) in paste_options.iter().enumerate() {
                let is_selected = idx == props.image_paste_action;
                menu_items.push(
                    select_option(ElementId::Name(format!("paste-item-{}", idx).into()), c, d)
                        .bg(if is_selected {
                            c.panel_row_selected
                        } else {
                            c.dialog_surface
                        })
                        .text_size(px(12.0))
                        .text_color(c.text_default)
                        .child(*opt_label)
                        .child(if is_selected {
                            svg()
                                .path("icons/settings/checkmark.svg")
                                .size(px(15.0))
                                .text_color(c.dialog_primary_button_bg)
                                .into_any_element()
                        } else {
                            div().w(px(13.0)).into_any_element()
                        })
                        .on_click((props.on_select_image_paste)(idx))
                        .into_any_element(),
                );
            }

            paste_btn_wrap =
                paste_btn_wrap.child(gpui::deferred(select_panel(c, d).children(menu_items)));
        }

        rows.push(make_row(
            inner_border_color,
            c,
            d,
            "Clipboard Image Paste Action",
            "Target location and format when pasting images into editor",
            paste_btn_wrap.into_any_element(),
        ));
    }

    make_section(c, d, id, "Markdown & Document Elements", expanded, toggle_fn, rows)
}

pub(crate) struct StartupProps {
    pub startup_option: usize,
    pub is_startup_open: bool,
    pub on_toggle_startup: SettingsClickHandler,
    pub on_select_startup: Box<dyn Fn(usize) -> SettingsClickHandler>,
}

pub(crate) fn render_startup_section(
    c: &ThemeColors,
    d: &ThemeDimensions,
    id: impl Into<ElementId>,
    expanded: bool,
    toggle_fn: SettingsClickHandler,
    props: StartupProps,
) -> AnyElement {
    let mut inner_border_color = c.dialog_border;
    inner_border_color.a *= 0.4;

    let mut rows = Vec::new();

    if expanded {
        let startup_options = [
            "Open New Blank Document",
            "Reopen Last Opened Files",
        ];
        let current_startup_label = startup_options
            .get(props.startup_option)
            .copied()
            .unwrap_or(startup_options[0]);

        let mut startup_btn_wrap = div().relative().child(
            select_trigger("pref-btn-startup", c, d)
                .text_size(px(12.0))
                .text_color(c.text_default)
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .truncate()
                        .child(current_startup_label),
                )
                .child(
                    div().flex_shrink_0().pl(px(4.0)).child(
                        svg()
                            .path("icons/settings/select-chevron.svg")
                            .size(px(16.0))
                            .text_color(c.dialog_muted),
                    ),
                )
                .on_click(props.on_toggle_startup),
        );

        if props.is_startup_open {
            let mut menu_items = Vec::new();
            for (idx, opt_label) in startup_options.iter().enumerate() {
                let is_selected = idx == props.startup_option;
                menu_items.push(
                    select_option(ElementId::Name(format!("startup-item-{}", idx).into()), c, d)
                        .bg(if is_selected {
                            c.panel_row_selected
                        } else {
                            c.dialog_surface
                        })
                        .text_size(px(12.0))
                        .text_color(c.text_default)
                        .child(*opt_label)
                        .child(if is_selected {
                            svg()
                                .path("icons/settings/checkmark.svg")
                                .size(px(15.0))
                                .text_color(c.dialog_primary_button_bg)
                                .into_any_element()
                        } else {
                            div().w(px(13.0)).into_any_element()
                        })
                        .on_click((props.on_select_startup)(idx))
                        .into_any_element(),
                );
            }

            startup_btn_wrap =
                startup_btn_wrap.child(gpui::deferred(select_panel(c, d).children(menu_items)));
        }

        rows.push(make_row(
            inner_border_color,
            c,
            d,
            "App Startup Behavior",
            "Default workspace and document state upon launching Splitype",
            startup_btn_wrap.into_any_element(),
        ));
    }

    make_section(c, d, id, "Application Startup", expanded, toggle_fn, rows)
}
