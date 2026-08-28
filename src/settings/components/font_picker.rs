//! Searchable Font Picker component for settings, modeled after Zed's font_picker.
//!
//! Provides a trigger button with up-down chevron and a searchable popover list
//! with real-time filtering across all installed and available fonts on the system.

use gpui::*;

use crate::infra::theme::{ThemeColors, ThemeDimensions};
use crate::settings::ui_helpers::SettingsClickHandler;
use crate::ui::select::{select_option, select_panel, select_trigger};

pub(crate) struct SearchableFontPickerProps {
    pub id_prefix: &'static str,
    pub current_font_name: String,
    pub default_label: String,
    pub is_open: bool,
    pub search_query: String,
    pub on_toggle: SettingsClickHandler,
    pub on_search_change: Box<dyn Fn(String, &mut Window, &mut App)>,
    pub available_fonts: Vec<SharedString>,
    pub on_select_font: Box<dyn Fn(String) -> SettingsClickHandler>,
}

pub(crate) fn render_searchable_font_picker(
    c: &ThemeColors,
    d: &ThemeDimensions,
    props: SearchableFontPickerProps,
) -> AnyElement {
    let id_prefix = props.id_prefix;
    let mut btn_wrap = div().relative().child(
        select_trigger(format!("{id_prefix}-btn"), c, d)
            .w(px(160.0))
            .text_size(px(12.0))
            .text_color(c.text_default)
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .truncate()
                    .child(props.current_font_name.clone()),
            )
            .child(
                div().flex_shrink_0().pl(px(4.0)).child(
                    svg()
                        .path("icons/settings/chevron-up-down.svg")
                        .size(px(14.0))
                        .text_color(c.dialog_muted),
                ),
            )
            .on_click(props.on_toggle),
    );

    if props.is_open {
        let query_lower = props.search_query.trim().to_lowercase();
        let mut menu_items = Vec::new();

        // 1. Default option (always at top)
        let is_default_selected = props.current_font_name == props.default_label
            || props.current_font_name.starts_with("Default");
        let default_matches = query_lower.is_empty()
            || props.default_label.to_lowercase().contains(&query_lower)
            || "default".contains(&query_lower);
        if default_matches {
            menu_items.push(
                select_option(
                    ElementId::Name(format!("{id_prefix}-item-default").into()),
                    c,
                    d,
                )
                .bg(if is_default_selected {
                    c.panel_row_selected
                } else {
                    c.dialog_surface
                })
                .text_size(px(12.0))
                .text_color(c.text_default)
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .truncate()
                        .child(props.default_label.clone()),
                )
                .child(if is_default_selected {
                    svg()
                        .path("icons/settings/checkmark.svg")
                        .size(px(14.0))
                        .text_color(c.dialog_primary_button_bg)
                        .into_any_element()
                } else {
                    div().w(px(14.0)).into_any_element()
                })
                .on_click((props.on_select_font)("default".to_string()))
                .into_any_element(),
            );
        }

        // 2. Filtered system fonts
        for font in &props.available_fonts {
            let font_str = font.as_ref();
            if !query_lower.is_empty() && !font_str.to_lowercase().contains(&query_lower) {
                continue;
            }

            let is_selected = font_str == props.current_font_name;
            let f_id = font_str.to_string();
            let f_name = font_str.to_string();

            menu_items.push(
                select_option(
                    ElementId::Name(format!("{id_prefix}-item-{f_id}").into()),
                    c,
                    d,
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
                        .flex_1()
                        .min_w(px(0.0))
                        .truncate()
                        .child(f_name),
                )
                .child(if is_selected {
                    svg()
                        .path("icons/settings/checkmark.svg")
                        .size(px(14.0))
                        .text_color(c.dialog_primary_button_bg)
                        .into_any_element()
                } else {
                    div().w(px(14.0)).into_any_element()
                })
                .on_click((props.on_select_font)(f_id))
                .into_any_element(),
            );
        }

        let search_query = props.search_query.clone();
        let _ = props.on_search_change;

        // Popover header: Search box
        let search_box = div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .w_full()
            .h(px(28.0))
            .px(px(8.0))
            .mb(px(4.0))
            .rounded(px(d.select_trigger_radius))
            .bg(c.dialog_secondary_button_bg)
            .border_1()
            .border_color(c.dialog_border)
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .text_size(px(12.0))
                    .text_color(if search_query.is_empty() {
                        c.dialog_muted
                    } else {
                        c.text_default
                    })
                    .child(if search_query.is_empty() {
                        "Search fonts…".to_string()
                    } else {
                        search_query
                    }),
            );

        // Popover body: Scrollable list of fonts
        let list_container = div()
            .id(ElementId::Name(format!("{id_prefix}-list").into()))
            .w_full()
            .max_h(px(220.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .children(menu_items);

        let panel = select_panel(c, d)
            .w(px(210.0))
            .max_h(px(280.0))
            .child(search_box)
            .child(list_container);

        btn_wrap = btn_wrap.child(gpui::deferred(panel));
    }

    btn_wrap.into_any_element()
}
