//! Settings-form primitives: row scaffolding, handler aliases, number fields,
//! and the searchable font picker.
//!
//! Components here take theme tokens by reference per render and never cache
//! colors, so every control recolors automatically when the theme switches.

use gpui::prelude::FluentBuilder;
use gpui::*;

use theme::{ThemeColors, ThemeDimensions};
use ui::SearchInput;
use ui::select::{select_option, select_panel, select_trigger};


pub use ui::components::card::{highlight_search_text, settings_card_row, settings_row};
pub use ui::components::number_box::{
    NumberBoxClickHandler as SettingsClickHandler,
    NumberBoxDismissHandler as SettingsDismissHandler,
    NumberBoxKeyHandler as SettingsKeyHandler,
    NumberBoxPasteHandler as SettingsPasteHandler,
    NumberBoxProps as NumberFieldProps,
    render_number_box as render_number_field,
};

/// Settings navigation tab row.
pub fn nav_tab(id: impl Into<ElementId>, c: &ThemeColors, d: &ThemeDimensions) -> Stateful<Div> {
    div()
        .id(id)
        .px(px(12.0))
        .py(px(8.0))
        .rounded(px(d.tab_radius))
        .flex()
        .items_center()
        .cursor_pointer()
        .hover(|this| this.bg(c.panel_row_hover))
}

/// Search-query handler signature for searchable pickers.
pub type SettingsSearchHandler = Box<dyn Fn(String, &mut Window, &mut App)>;
/// Option select handler: takes the option value and returns a click handler.
pub type SettingsOptionHandler<T> = Box<dyn Fn(T) -> SettingsClickHandler>;

/// Searchable font picker with a default option and live filtering.
pub struct SearchableFontPickerProps {
    pub id_prefix: String,
    pub current_font_name: String,
    pub default_label: String,
    pub is_open: bool,
    pub search_query: String,
    pub focus_handle: FocusHandle,
    pub on_toggle: SettingsClickHandler,
    pub on_search_change: SettingsSearchHandler,
    pub available_fonts: Vec<SharedString>,
    pub on_select_font: SettingsOptionHandler<String>,
}

pub fn render_searchable_font_picker(
    c: &ThemeColors,
    d: &ThemeDimensions,
    props: SearchableFontPickerProps,
) -> AnyElement {
    let id_prefix = props.id_prefix;
    let mut btn_wrap = div().relative().child(
        select_trigger(format!("{id_prefix}-btn"), c, d)
            .w(px(140.0))
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
                        .path("plugin://splitype.settings/chevron-up-down.svg")
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
                .when(is_default_selected, |this| this.bg(c.panel_row_hover))
                .text_size(px(12.0))
                .text_color(if is_default_selected {
                    c.dialog_primary_button_bg
                } else {
                    c.text_default
                })
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .truncate()
                        .child(props.default_label.clone()),
                )
                .child(if is_default_selected {
                    svg()
                        .path("plugin://splitype.settings/checkmark.svg")
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
                .when(is_selected, |this| this.bg(c.panel_row_hover))
                .text_size(px(12.0))
                .text_color(if is_selected {
                    c.dialog_primary_button_bg
                } else {
                    c.text_default
                })
                .child(div().flex_1().min_w(px(0.0)).truncate().child(f_name))
                .child(if is_selected {
                    svg()
                        .path("plugin://splitype.settings/checkmark.svg")
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

        // Popover header: Search box using unified SearchInput component
        let on_search_change = props.on_search_change;
        let search_box = div().mb(px(4.0)).child(
            SearchInput::new(
                ElementId::Name(format!("{id_prefix}-search").into()),
                props.search_query.clone(),
                props.focus_handle.clone(),
            )
            .placeholder("Search…")
            .autofocus(true)
            .colors(c.clone())
            .dimensions(d.clone())
            .on_change(move |query, window, cx| {
                on_search_change(query, window, cx);
            }),
        );


        // Popover body: Scrollable list of fonts
        let list_container = div()
            .id(ElementId::Name(format!("{id_prefix}-list").into()))
            .w_full()
            .max_h(px(240.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .children(menu_items);

        let panel = select_panel(c, d)
            .w(px(220.0))
            .max_h(px(290.0))
            .child(search_box)
            .child(list_container);

        btn_wrap = btn_wrap.child(gpui::deferred(panel));
    }

    btn_wrap.into_any_element()
}

