//! Searchable combobox / dropdown picker (WinUI 3 AutoSuggestBox / ComboBox).
//!
//! Provides a standardized trigger, floating panel with integrated search input,
//! and scrollable option list.

use gpui::prelude::FluentBuilder;
use gpui::*;
use theme::{ThemeColors, ThemeDimensions};

use super::input::SearchInput;
use super::popover::menu_panel;
use super::select::select_option;

/// Single selectable option item in a searchable combobox.
#[derive(Clone, Debug)]
pub struct ComboboxOption {
    pub id: String,
    pub label: String,
    pub is_selected: bool,
}

/// Renders a standardized floating searchable combobox panel anchored below a trigger.
pub fn render_searchable_combobox_panel<F>(
    panel_id: impl Into<ElementId>,
    search_input_id: impl Into<ElementId>,
    query: &str,
    focus_handle: &FocusHandle,
    options: &[ComboboxOption],
    c: &ThemeColors,
    d: &ThemeDimensions,
    on_search_change: impl Fn(String, &mut Window, &mut App) + 'static,
    on_select: F,
) -> AnyElement
where
    F: Fn(&str, &mut Window, &mut App) + 'static + Clone,
{
    let panel_id = panel_id.into();
    let search_input_id = search_input_id.into();

    let search_box = div().px(px(2.0)).pt(px(2.0)).pb(px(4.0)).child(
        SearchInput::new(search_input_id, query, focus_handle.clone())
            .placeholder("Search…")
            .autofocus(true)
            .on_change(on_search_change),
    );

    let mut option_elements: Vec<AnyElement> = Vec::new();
    for opt in options {
        let opt_id = opt.id.clone();
        let on_select_opt = on_select.clone();
        let is_selected = opt.is_selected;

        let row = select_option((panel_id.clone(), opt_id.as_str()), c, d)
            .when(is_selected, |this| this.bg(c.panel_row_hover))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .truncate()
                    .text_size(px(12.0))
                    .text_color(if is_selected {
                        c.text_default
                    } else {
                        c.dialog_muted
                    })
                    .child(opt.label.clone()),
            )
            .when(is_selected, |this| {
                this.child(
                    div()
                        .text_size(px(11.0))
                        .text_color(c.focus_accent)
                        .child("✓"),
                )
            })
            .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                cx.stop_propagation();
                on_select_opt(&opt_id, window, cx);
            });

        option_elements.push(row.into_any_element());
    }

    let option_list = div()
        .id((panel_id.clone(), "combobox-option-list"))
        .max_h(px(220.0))
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .children(option_elements);

    menu_panel(c, d)
        .id(panel_id)
        .absolute()
        .top_full()
        .right_0()
        .mt(px(4.0))
        .w(px(220.0))
        .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
            cx.stop_propagation();
        })
        .child(search_box)
        .child(option_list)
        .into_any_element()
}
