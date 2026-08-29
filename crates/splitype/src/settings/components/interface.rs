//! Interface settings components: Theme, Language, and Status Bar.

use gpui::*;

use splitype_infra::theme::{ThemeColors, ThemeDimensions};
use crate::settings::ui_helpers::{SettingsClickHandler, make_row, make_section};
use splitype_ui::select::{select_option, select_panel, select_trigger};
use splitype_ui::switch::Switch;

pub(crate) struct ThemeLangProps {
    pub current_theme_name: String,
    pub is_theme_dropdown_open: bool,
    pub on_toggle_theme_dropdown: SettingsClickHandler,
    pub available_themes: Vec<(String, String)>, // (id, display_name)
    pub on_select_theme: Box<dyn Fn(String) -> SettingsClickHandler>,

    pub current_lang_name: String,
    pub is_lang_dropdown_open: bool,
    pub on_toggle_lang_dropdown: SettingsClickHandler,
    pub lang_options: Vec<(&'static str, &'static str)>, // (code, display_name)
    pub on_select_lang: Box<dyn Fn(&'static str) -> SettingsClickHandler>,
}

pub(crate) fn render_theme_and_language_section(
    c: &ThemeColors,
    d: &ThemeDimensions,
    id: impl Into<ElementId>,
    expanded: bool,
    toggle_fn: SettingsClickHandler,
    props: ThemeLangProps,
) -> AnyElement {
    let mut inner_border_color = c.dialog_border;
    inner_border_color.a *= 0.4;

    let mut rows = Vec::new();

    if expanded {
        // Theme selector row
        let theme_icon_path = if props.current_theme_name == "Light" {
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
                                .child(props.current_theme_name.clone()),
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
                .on_click(props.on_toggle_theme_dropdown),
        );

        if props.is_theme_dropdown_open {
            let mut menu_items = Vec::new();
            for (t_id, display_label) in props.available_themes {
                let is_selected = display_label == props.current_theme_name;
                menu_items.push(
                    select_option(ElementId::Name(format!("theme-item-{}", t_id).into()), c, d)
                        .bg(if is_selected {
                            c.panel_row_selected
                        } else {
                            c.dialog_surface
                        })
                        .text_size(px(12.0))
                        .text_color(c.text_default)
                        .child(display_label)
                        .child(if is_selected {
                            svg()
                                .path("icons/settings/checkmark.svg")
                                .size(px(15.0))
                                .text_color(c.dialog_primary_button_bg)
                                .into_any_element()
                        } else {
                            div().w(px(13.0)).into_any_element()
                        })
                        .on_click((props.on_select_theme)(t_id))
                        .into_any_element(),
                );
            }

            theme_btn_wrap =
                theme_btn_wrap.child(gpui::deferred(select_panel(c, d).children(menu_items)));
        }

        rows.push(make_row(
            inner_border_color,
            c,
            d,
            "Color Theme",
            "Select the overall color theme for the editor and panels",
            theme_btn_wrap.into_any_element(),
        ));

        // Language selector row
        let mut lang_btn_wrap = div().relative().child(
            select_trigger("pref-btn-lang", c, d)
                .text_size(px(12.0))
                .text_color(c.text_default)
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .truncate()
                        .child(props.current_lang_name.clone()),
                )
                .child(
                    div().flex_shrink_0().pl(px(4.0)).child(
                        svg()
                            .path("icons/settings/select-chevron.svg")
                            .size(px(16.0))
                            .text_color(c.dialog_muted),
                    ),
                )
                .on_click(props.on_toggle_lang_dropdown),
        );

        if props.is_lang_dropdown_open {
            let mut menu_items = Vec::new();
            for (code, label) in props.lang_options {
                let is_selected = label == props.current_lang_name;
                menu_items.push(
                    select_option(ElementId::Name(format!("lang-item-{}", code).into()), c, d)
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
                        .on_click((props.on_select_lang)(code))
                        .into_any_element(),
                );
            }

            lang_btn_wrap =
                lang_btn_wrap.child(gpui::deferred(select_panel(c, d).children(menu_items)));
        }

        rows.push(make_row(
            inner_border_color,
            c,
            d,
            "Display Language",
            "Select preferred language for editor UI and dialogs",
            lang_btn_wrap.into_any_element(),
        ));
    }

    make_section(c, d, id, "Visual Theme & Language", expanded, toggle_fn, rows)
}

pub(crate) struct StatusBarProps {
    pub show_status_bar: bool,
    pub on_toggle_status_bar: SettingsClickHandler,
    pub show_word_count: bool,
    pub on_toggle_word_count: SettingsClickHandler,
    pub show_cursor_pos: bool,
    pub on_toggle_cursor_pos: SettingsClickHandler,
    pub show_character_count: bool,
    pub on_toggle_character_count: SettingsClickHandler,
    pub show_reading_time: bool,
    pub on_toggle_reading_time: SettingsClickHandler,
}

pub(crate) fn render_status_bar_section(
    c: &ThemeColors,
    d: &ThemeDimensions,
    id: impl Into<ElementId>,
    expanded: bool,
    toggle_fn: SettingsClickHandler,
    props: StatusBarProps,
) -> AnyElement {
    let mut inner_border_color = c.dialog_border;
    inner_border_color.a *= 0.4;

    let mut rows = Vec::new();

    if expanded {
        let ctrl_sb_main = Switch::new("switch-sb-main")
            .checked(props.show_status_bar)
            .on_click(props.on_toggle_status_bar)
            .into_any_element();

        rows.push(make_row(
            inner_border_color,
            c,
            d,
            "Status Bar Visibility",
            "Show or hide the persistent bottom status bar across window",
            ctrl_sb_main,
        ));

        let ctrl_sb_words = Switch::new("switch-sb-words")
            .checked(props.show_word_count)
            .on_click(props.on_toggle_word_count)
            .into_any_element();

        rows.push(make_row(
            inner_border_color,
            c,
            d,
            "Word Count Badge",
            "Display real-time document word count in status bar",
            ctrl_sb_words,
        ));

        let ctrl_sb_pos = Switch::new("switch-sb-pos")
            .checked(props.show_cursor_pos)
            .on_click(props.on_toggle_cursor_pos)
            .into_any_element();

        rows.push(make_row(
            inner_border_color,
            c,
            d,
            "Cursor Position Badge",
            "Display line and column coordinates in status bar",
            ctrl_sb_pos,
        ));

        let ctrl_sb_chars = Switch::new("switch-sb-chars")
            .checked(props.show_character_count)
            .on_click(props.on_toggle_character_count)
            .into_any_element();

        rows.push(make_row(
            inner_border_color,
            c,
            d,
            "Character Count Badge",
            "Display total character count in status bar",
            ctrl_sb_chars,
        ));

        let ctrl_sb_time = Switch::new("switch-sb-time")
            .checked(props.show_reading_time)
            .on_click(props.on_toggle_reading_time)
            .into_any_element();

        rows.push(make_row(
            inner_border_color,
            c,
            d,
            "Estimated Reading Time",
            "Display estimated reading duration in status bar",
            ctrl_sb_time,
        ));
    }

    make_section(c, d, id, "Status Bar Options", expanded, toggle_fn, rows)
}

