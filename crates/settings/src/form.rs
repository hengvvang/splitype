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
use ui::stepper::{stepper_container, stepper_divider, stepper_step_button};


/// Settings row container — title/description label and a control on the right.
pub fn settings_row(border: Hsla, c: &ThemeColors, d: &ThemeDimensions) -> Div {
    div()
        .w_full()
        .min_w(px(0.0))
        .min_h(px(56.0))
        .py(px(10.0))
        .px(px(16.0))
        .rounded(px(d.settings_row_radius))
        .bg(c.dialog_surface)
        .border_1()
        .border_color(border)
        .hover(|this| this.bg(c.panel_row_hover))
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(16.0))
}

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

/// Click handler signature shared by all settings controls.
pub type SettingsClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
/// Key-down handler signature for inline numeric editing.
pub type SettingsKeyHandler = Box<dyn Fn(&KeyDownEvent, &mut Window, &mut App) + 'static>;
/// Dismiss handler signature for inline editing.
pub type SettingsDismissHandler =
    Box<dyn Fn(&platform_contracts::actions::DismissTransientUi, &mut Window, &mut App) + 'static>;
/// Paste handler signature for inline editing.
pub type SettingsPasteHandler =
    Box<dyn Fn(&platform_contracts::actions::Paste, &mut Window, &mut App) + 'static>;
/// Search-query handler signature for searchable pickers.
pub type SettingsSearchHandler = Box<dyn Fn(String, &mut Window, &mut App)>;
/// Option select handler: takes the option value and returns a click handler.
pub type SettingsOptionHandler<T> = Box<dyn Fn(T) -> SettingsClickHandler>;

/// Highlights occurrences of words in `query` within `text` using `StyledText` and `HighlightStyle`.
pub fn highlight_search_text(
    text: &str,
    query: &str,
    base_color: Hsla,
    highlight_bg: Hsla,
) -> AnyElement {
    let q = query.trim();
    if q.is_empty() || text.is_empty() {
        return div()
            .w_full()
            .min_w(px(0.0))
            .text_color(base_color)
            .child(text.to_string())
            .into_any_element();
    }

    let text_lower = text.to_lowercase();
    let words: Vec<String> = q.split_whitespace().map(|w| w.to_lowercase()).collect();
    if words.is_empty() {
        return div()
            .w_full()
            .min_w(px(0.0))
            .text_color(base_color)
            .child(text.to_string())
            .into_any_element();
    }

    let mut ranges = Vec::new();
    for word in &words {
        let mut start = 0;
        while let Some(idx) = text_lower[start..].find(word.as_str()) {
            let match_start = start + idx;
            let match_end = match_start + word.len();
            if text.is_char_boundary(match_start) && text.is_char_boundary(match_end) {
                ranges.push((match_start, match_end));
            }
            start = match_end.max(start + 1);
            if start >= text.len() {
                break;
            }
        }
    }

    if ranges.is_empty() {
        return div()
            .w_full()
            .min_w(px(0.0))
            .text_color(base_color)
            .child(text.to_string())
            .into_any_element();
    }

    ranges.sort_by_key(|r| r.0);
    let mut merged: Vec<std::ops::Range<usize>> = Vec::new();
    for (start, end) in ranges {
        if let Some(last) = merged.last_mut() {
            if start <= last.end {
                last.end = last.end.max(end);
                continue;
            }
        }
        merged.push(start..end);
    }

    let highlight_style = HighlightStyle {
        background_color: Some(highlight_bg),
        color: Some(base_color),
        font_weight: Some(FontWeight::SEMIBOLD),
        ..Default::default()
    };

    let styled = StyledText::new(text.to_string())
        .with_highlights(merged.into_iter().map(|range| (range, highlight_style)));

    div()
        .w_full()
        .min_w(px(0.0))
        .text_color(base_color)
        .child(styled)
        .into_any_element()
}

/// A unified settings row designed after Windows 11 SettingsCard with dynamic multi-line
/// description wrapping, optional icon, optional chevron, and non-shrinking right control.
pub fn settings_card_row(
    c: &ThemeColors,
    d: &ThemeDimensions,
    icon: Option<&'static str>,
    title: &str,
    desc: &str,
    query: &str,
    on_reset: Option<SettingsClickHandler>,
    control: AnyElement,
    has_chevron: bool,
) -> AnyElement {
    let has_query = !query.trim().is_empty();

    let title_element = if has_query {
        div()
            .min_w(px(0.0))
            .text_size(px(13.0))
            .font_weight(FontWeight::MEDIUM)
            .child(highlight_search_text(title, query, c.text_default, c.text_highlight_bg))
            .into_any_element()
    } else {
        div()
            .min_w(px(0.0))
            .text_size(px(13.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(c.text_default)
            .child(title.to_string())
            .into_any_element()
    };

    let mut title_row = div()
        .min_w(px(0.0))
        .flex()
        .items_center()
        .gap(px(6.0))
        .child(title_element);

    if let Some(reset_fn) = on_reset {
        let reset_id = ElementId::Name(format!("reset-{title}").into());
        title_row = title_row.child(
            div()
                .id(reset_id)
                .flex_shrink_0()
                .cursor_pointer()
                .p(px(2.0))
                .rounded(px(2.0))
                .hover(|s| s.bg(c.panel_row_hover))
                .child(
                    svg()
                        .path("plugin://splitype.settings/undo.svg")
                        .size(px(12.0))
                        .text_color(c.dialog_muted),
                )
                .on_click(reset_fn),
        );
    }

    let has_desc = !desc.is_empty();
    let label_column = if has_desc {
        let desc_element = if has_query {
            div()
                .w_full()
                .min_w(px(0.0))
                .text_size(px(11.5))
                .line_height(relative(1.35))
                .child(highlight_search_text(desc, query, c.dialog_muted, c.text_highlight_bg))
                .into_any_element()
        } else {
            div()
                .w_full()
                .min_w(px(0.0))
                .text_size(px(11.5))
                .line_height(relative(1.35))
                .text_color(c.dialog_muted)
                .child(desc.to_string())
                .into_any_element()
        };
        div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .gap(px(3.0))
            .child(title_row)
            .child(desc_element)
    } else {
        div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .child(title_row)
    };

    let left_side = if let Some(icon_path) = icon {
        div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .items_center()
            .gap(px(14.0))
            .child(
                div()
                    .size(px(24.0))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        svg()
                            .path(icon_path)
                            .size(px(18.0))
                            .text_color(c.text_default),
                    ),
            )
            .child(label_column)
            .into_any_element()
    } else {
        label_column.into_any_element()
    };

    let mut control_area = div()
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(control);

    if has_chevron {
        control_area = control_area.child(
            svg()
                .path("plugin://splitype.settings/chevron-right.svg")
                .size(px(14.0))
                .text_color(c.dialog_muted)
                .flex_shrink_0(),
        );
    }

    settings_row(c.dialog_border, c, d)
        .child(left_side)
        .child(control_area)
        .into_any_element()
}

/// Inline numeric field with steppers and keyboard editing.
pub struct NumberFieldProps {
    pub id_prefix: String,
    pub value_text: String,
    pub is_editing: bool,
    pub edit_buffer: Option<String>,
    pub focus_handle: FocusHandle,
    pub on_dec: SettingsClickHandler,
    pub on_inc: SettingsClickHandler,
    pub on_start_edit: SettingsClickHandler,
    pub on_key_down: SettingsKeyHandler,
    pub on_dismiss: Option<SettingsDismissHandler>,
    pub on_paste: Option<SettingsPasteHandler>,
}

pub fn render_number_field(
    c: &ThemeColors,
    d: &ThemeDimensions,
    props: NumberFieldProps,
) -> AnyElement {
    let is_editing = props.is_editing;
    let text_to_show = if is_editing {
        props
            .edit_buffer
            .clone()
            .unwrap_or_else(|| props.value_text.clone())
    } else {
        props.value_text.clone()
    };

    let id_prefix = props.id_prefix;
    let dec_id = ElementId::Name(format!("{id_prefix}-dec").into());
    let inc_id = ElementId::Name(format!("{id_prefix}-inc").into());
    let center_id = ElementId::Name(format!("{id_prefix}-center").into());

    let bottom_indicator = div()
        .absolute()
        .bottom_0()
        .left_0()
        .right_0()
        .h(if is_editing { px(2.0) } else { px(1.5) })
        .bg(if is_editing {
            c.focus_accent
        } else {
            c.focus_accent.opacity(0.4)
        });

    let mut center_box = div()
        .id(center_id)
        .key_context("NumberField")
        .track_focus(&props.focus_handle)
        .cursor_text()
        .relative()
        .overflow_hidden()
        .h_full()
        .flex_1()
        .min_w(px(0.0))
        .px(px(6.0))
        .flex()
        .items_center()
        .justify_center()
        .bg(if is_editing {
            c.dialog_surface
        } else {
            c.dialog_secondary_button_bg.opacity(0.55)
        })
        .when(!is_editing, |this| {
            this.hover(|this| this.bg(c.panel_row_hover))
        })
        .on_click(props.on_start_edit)
        .on_key_down(props.on_key_down);

    if let Some(on_dismiss) = props.on_dismiss {
        center_box = center_box.on_action(move |action: &platform_contracts::actions::DismissTransientUi, window, cx| {
            cx.stop_propagation();
            on_dismiss(action, window, cx);
        });
    }

    if let Some(on_paste) = props.on_paste {
        center_box = center_box.on_action(move |action: &platform_contracts::actions::Paste, window, cx| {
            cx.stop_propagation();
            on_paste(action, window, cx);
        });
    }

    let center_box = center_box
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_size(px(12.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(c.text_default)
                        .child(text_to_show),
                )
                .when(is_editing, |this| {
                    this.child(div().w(px(1.5)).h(px(13.0)).ml(px(1.0)).bg(c.focus_accent))
                }),
        )
        .child(bottom_indicator);

    stepper_container(c, d)
        .child(
            stepper_step_button(dec_id, c)
                .child(
                    svg()
                        .path("plugin://splitype.settings/minus.svg")
                        .size(px(12.0))
                        .text_color(c.dialog_secondary_button_text),
                )
                .on_click(props.on_dec),
        )
        .child(stepper_divider(c))
        .child(center_box)
        .child(stepper_divider(c))
        .child(
            stepper_step_button(inc_id, c)
                .child(
                    svg()
                        .path("plugin://splitype.settings/plus.svg")
                        .size(px(12.0))
                        .text_color(c.dialog_secondary_button_text),
                )
                .on_click(props.on_inc),
        )
        .into_any_element()
}

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
                .bg(c.dialog_surface)
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
                .bg(c.dialog_surface)
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
            .placeholder("Search fonts…")
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

