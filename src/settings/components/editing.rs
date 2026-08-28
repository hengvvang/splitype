//! Editing settings components: Typography, Markdown & Assets, Startup.

use gpui::*;

use crate::infra::theme::{ThemeColors, ThemeDimensions};
use crate::settings::components::font_picker::{SearchableFontPickerProps, render_searchable_font_picker};
use crate::settings::ui_helpers::{
    SettingsClickHandler, make_row, make_row_with_reset, make_section, render_zed_stepper,
};
use crate::ui::select::{select_option, select_panel, select_trigger};
use crate::ui::switch::Switch;

pub(crate) struct TypographyProps {
    pub ui_font_name: String,
    pub is_ui_font_open: bool,
    pub search_query_ui_font: String,
    pub on_toggle_ui_font: SettingsClickHandler,
    pub on_search_ui_font: Box<dyn Fn(String, &mut Window, &mut App)>,
    pub on_select_ui_font: Box<dyn Fn(String) -> SettingsClickHandler>,
    pub on_reset_ui_font: Option<SettingsClickHandler>,

    pub prose_font_name: String,
    pub is_prose_font_open: bool,
    pub search_query_prose_font: String,
    pub on_toggle_prose_font: SettingsClickHandler,
    pub on_search_prose_font: Box<dyn Fn(String, &mut Window, &mut App)>,
    pub on_select_prose_font: Box<dyn Fn(String) -> SettingsClickHandler>,
    pub on_reset_prose_font: Option<SettingsClickHandler>,

    pub code_font_name: String,
    pub is_code_font_open: bool,
    pub search_query_code_font: String,
    pub on_toggle_code_font: SettingsClickHandler,
    pub on_search_code_font: Box<dyn Fn(String, &mut Window, &mut App)>,
    pub on_select_code_font: Box<dyn Fn(String) -> SettingsClickHandler>,
    pub on_reset_code_font: Option<SettingsClickHandler>,

    pub available_fonts: Vec<SharedString>,

    pub font_size: u32,
    pub is_editing_font: bool,
    pub on_font_dec: SettingsClickHandler,
    pub on_font_inc: SettingsClickHandler,
    pub on_font_cycle: SettingsClickHandler,
    pub on_reset_font_size: Option<SettingsClickHandler>,

    pub line_height: f32,
    pub is_editing_lh: bool,
    pub on_lh_dec: SettingsClickHandler,
    pub on_lh_inc: SettingsClickHandler,
    pub on_lh_cycle: SettingsClickHandler,
    pub on_reset_line_height: Option<SettingsClickHandler>,
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
        // 1. Interface Font Row
        let ui_picker = render_searchable_font_picker(
            c,
            d,
            SearchableFontPickerProps {
                id_prefix: "pref-ui-font",
                current_font_name: props.ui_font_name,
                is_open: props.is_ui_font_open,
                search_query: props.search_query_ui_font,
                on_toggle: props.on_toggle_ui_font,
                on_search_change: props.on_search_ui_font,
                available_fonts: props.available_fonts.clone(),
                on_select_font: props.on_select_ui_font,
            },
        );
        rows.push(make_row_with_reset(
            inner_border_color,
            c,
            d,
            "Interface Font",
            "Font used for menus, explorer sidebar, and application chrome",
            props.on_reset_ui_font,
            ui_picker,
        ));

        // 2. Prose Text Font Row
        let prose_picker = render_searchable_font_picker(
            c,
            d,
            SearchableFontPickerProps {
                id_prefix: "pref-prose-font",
                current_font_name: props.prose_font_name,
                is_open: props.is_prose_font_open,
                search_query: props.search_query_prose_font,
                on_toggle: props.on_toggle_prose_font,
                on_search_change: props.on_search_prose_font,
                available_fonts: props.available_fonts.clone(),
                on_select_font: props.on_select_prose_font,
            },
        );
        rows.push(make_row_with_reset(
            inner_border_color,
            c,
            d,
            "Prose Text Font",
            "Font used for Markdown prose, headings, and tables",
            props.on_reset_prose_font,
            prose_picker,
        ));

        // 3. Code Block Font Row
        let code_picker = render_searchable_font_picker(
            c,
            d,
            SearchableFontPickerProps {
                id_prefix: "pref-code-font",
                current_font_name: props.code_font_name,
                is_open: props.is_code_font_open,
                search_query: props.search_query_code_font,
                on_toggle: props.on_toggle_code_font,
                on_search_change: props.on_search_code_font,
                available_fonts: props.available_fonts,
                on_select_font: props.on_select_code_font,
            },
        );
        rows.push(make_row_with_reset(
            inner_border_color,
            c,
            d,
            "Code Block Font",
            "Monospace font used for code blocks and inline code",
            props.on_reset_code_font,
            code_picker,
        ));

        // 4. Document Font Size Row
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

        rows.push(make_row_with_reset(
            inner_border_color,
            c,
            d,
            "Document Font Size",
            "Baseline font size in pixels for document prose content",
            props.on_reset_font_size,
            ctrl_font,
        ));

        // 5. Line Spacing Multiplier Row
        let ctrl_lh = render_zed_stepper(
            c,
            d,
            "pref-lh-dec",
            "pref-lh-inc",
            format!("{:.2}", props.line_height),
            "",
            props.is_editing_lh,
            props.on_lh_dec,
            props.on_lh_inc,
            props.on_lh_cycle,
        );

        rows.push(make_row_with_reset(
            inner_border_color,
            c,
            d,
            "Line Spacing Multiplier",
            "Proportional line height for comfortable reading",
            props.on_reset_line_height,
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
