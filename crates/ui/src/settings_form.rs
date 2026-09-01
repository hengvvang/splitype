//! Settings-form primitives shared by every settings section renderer:
//! row/section scaffolding, handler aliases, number fields, and the
//! searchable font picker.
//!
//! Components here are business-free building blocks: they take theme
//! tokens by reference per render and never cache colors, so every control
//! recolors automatically when the theme switches.

use gpui::prelude::FluentBuilder;
use gpui::*;

use crate::section::{section_card, section_header, settings_row};
use crate::select::{select_option, select_panel, select_trigger};
use crate::stepper::{stepper_container, stepper_divider, stepper_step_button};
use theme::{ThemeColors, ThemeDimensions};

/// Click handler signature shared by all settings controls.
pub type SettingsClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
/// Key-down handler signature for inline numeric editing.
pub type SettingsKeyHandler = Box<dyn Fn(&KeyDownEvent, &mut Window, &mut App) + 'static>;
/// Search-query handler signature for searchable pickers.
pub type SettingsSearchHandler = Box<dyn Fn(String, &mut Window, &mut App)>;
/// Option select handler: takes the option value and returns a click handler.
pub type SettingsOptionHandler<T> = Box<dyn Fn(T) -> SettingsClickHandler>;

/// A settings row with a title, description, and control.
pub fn make_row(
    inner_border_color: Hsla,
    c: &ThemeColors,
    d: &ThemeDimensions,
    title: impl Into<SharedString>,
    desc: impl Into<SharedString>,
    control: AnyElement,
) -> AnyElement {
    make_row_with_reset(inner_border_color, c, d, title, desc, None, control)
}

/// A settings row with an optional reset-to-default button next to the title.
pub fn make_row_with_reset(
    inner_border_color: Hsla,
    c: &ThemeColors,
    d: &ThemeDimensions,
    title: impl Into<SharedString>,
    desc: impl Into<SharedString>,
    on_reset: Option<SettingsClickHandler>,
    control: AnyElement,
) -> AnyElement {
    let title = title.into();
    let desc = desc.into();
    let mut title_row = div().flex().items_center().gap(px(6.0)).child(
        div()
            .text_size(px(12.5))
            .font_weight(FontWeight::MEDIUM)
            .text_color(c.text_default)
            .child(title.clone()),
    );

    if let Some(reset_fn) = on_reset {
        let reset_id = ElementId::Name(format!("reset-{title}").into());
        title_row = title_row.child(
            div()
                .id(reset_id)
                .cursor_pointer()
                .p(px(2.0))
                .rounded(px(3.0))
                .hover(|s| s.bg(c.dialog_secondary_button_hover))
                .child(
                    svg()
                        .path("icons/settings/undo.svg")
                        .size(px(12.0))
                        .text_color(c.dialog_muted)
                        .hover(|s| s.text_color(c.text_default)),
                )
                .on_click(reset_fn),
        );
    }

    settings_row(inner_border_color, c, d)
        .child(
            div().flex().flex_col().gap(px(2.0)).child(title_row).child(
                div()
                    .text_size(px(11.0))
                    .text_color(c.dialog_muted)
                    .child(desc),
            ),
        )
        .child(control)
        .into_any_element()
}

/// A collapsible settings section card with a header and body rows.
pub fn make_section(
    c: &ThemeColors,
    d: &ThemeDimensions,
    id: impl Into<ElementId>,
    title: impl Into<SharedString>,
    expanded: bool,
    toggle_fn: SettingsClickHandler,
    items: Vec<AnyElement>,
) -> AnyElement {
    let title = title.into();
    let base_id = id.into();
    let header_id = ElementId::from((base_id.clone(), "header"));
    let card_id = ElementId::from((base_id, "card"));
    let header = section_header()
        .id(header_id)
        .child(
            svg()
                .path(if expanded {
                    "icons/settings/chevron-down.svg"
                } else {
                    "icons/settings/chevron-right.svg"
                })
                .size(px(16.0))
                .text_color(c.text_default),
        )
        .child(
            div()
                .text_size(px(13.0))
                .font_weight(FontWeight::BOLD)
                .text_color(c.text_default)
                .child(title),
        )
        .on_click(move |event, window, cx| toggle_fn(event, window, cx));

    let mut card = section_card(c, d).id(card_id).child(header);

    if expanded && !items.is_empty() {
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

    let center_box = div()
        .id(center_id)
        .key_context("NumberField")
        .track_focus(&props.focus_handle)
        .cursor_text()
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
            c.dialog_secondary_button_bg
        })
        .when(is_editing, |this| {
            this.border_y_1().border_color(c.focus_accent)
        })
        .on_click(props.on_start_edit)
        .on_key_down(props.on_key_down)
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
                    this.child(
                        div()
                            .w(px(1.5))
                            .h(px(13.0))
                            .ml(px(1.0))
                            .bg(c.focus_accent),
                    )
                }),
        );

    stepper_container(c, d)
        .child(
            stepper_step_button(dec_id, c)
                .child(
                    svg()
                        .path("icons/settings/minus.svg")
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
                        .path("icons/settings/plus.svg")
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

        // Popover header: Search box — an inline text filter driven by
        // key events, dispatching every change through `on_search_change`.
        let search_query = props.search_query.clone();
        let search_focus = props.focus_handle.clone();
        let search_focus_click = props.focus_handle.clone();
        let on_search_change = props.on_search_change;
        let search_box = div()
            .id(ElementId::Name(format!("{id_prefix}-search").into()))
            .key_context("SettingsSearch")
            .track_focus(&search_focus)
            .relative()
            .overflow_hidden()
            .cursor_text()
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
            .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                window.focus(&search_focus_click, cx);
            })
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
                        search_query.clone()
                    }),
            )
            .child(
                div()
                    .absolute()
                    .bottom_0()
                    .left_0()
                    .right_0()
                    .h(px(2.0))
                    .rounded_b(px(d.select_trigger_radius))
                    .bg(c.focus_accent),
            )
            .on_key_down(Box::new(
                move |event: &KeyDownEvent, _window: &mut Window, cx: &mut App| {
                    let mut query = search_query.clone();
                    match event.keystroke.key.as_str() {
                        "backspace" => {
                            query.pop();
                        }
                        "escape" => {}
                        "space" => {
                            query.push(' ');
                        }
                        _ => {
                            let text = event.keystroke.key_char.clone().unwrap_or_else(|| {
                                if event.keystroke.key.len() == 1 {
                                    event.keystroke.key.as_str().to_string()
                                } else {
                                    String::new()
                                }
                            });
                            if !text.is_empty() {
                                query.push_str(&text);
                            }
                        }
                    }
                    on_search_change(query, _window, cx);
                },
            ));

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
