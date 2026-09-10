//! NumberBox component — inline numeric field with stepper buttons and keyboard editing.
//!
//! Conforms to WinUI 3 NumberBox design.

use gpui::prelude::FluentBuilder;
use gpui::*;
use theme::{ThemeColors, ThemeDimensions};

use super::stepper::{stepper_container, stepper_divider, stepper_step_button};

/// Click handler signature for number box actions.
pub type NumberBoxClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
/// Key-down handler signature for inline numeric editing.
pub type NumberBoxKeyHandler = Box<dyn Fn(&KeyDownEvent, &mut Window, &mut App) + 'static>;
/// Dismiss handler signature for inline editing.
pub type NumberBoxDismissHandler =
    Box<dyn Fn(&platform_contracts::actions::DismissTransientUi, &mut Window, &mut App) + 'static>;
/// Paste handler signature for inline editing.
pub type NumberBoxPasteHandler =
    Box<dyn Fn(&platform_contracts::actions::Paste, &mut Window, &mut App) + 'static>;

/// Properties for rendering a NumberBox control.
pub struct NumberBoxProps {
    pub id_prefix: String,
    pub value_text: String,
    pub is_editing: bool,
    pub edit_buffer: Option<String>,
    pub focus_handle: FocusHandle,
    pub on_dec: NumberBoxClickHandler,
    pub on_inc: NumberBoxClickHandler,
    pub on_start_edit: NumberBoxClickHandler,
    pub on_key_down: NumberBoxKeyHandler,
    pub on_dismiss: Option<NumberBoxDismissHandler>,
    pub on_paste: Option<NumberBoxPasteHandler>,
}

/// Renders a full interactive NumberBox control.
pub fn render_number_box(
    c: &ThemeColors,
    d: &ThemeDimensions,
    props: NumberBoxProps,
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
        center_box = center_box.on_action(
            move |action: &platform_contracts::actions::DismissTransientUi, window, cx| {
                cx.stop_propagation();
                on_dismiss(action, window, cx);
            },
        );
    }

    if let Some(on_paste) = props.on_paste {
        center_box = center_box.on_action(
            move |action: &platform_contracts::actions::Paste, window, cx| {
                cx.stop_propagation();
                on_paste(action, window, cx);
            },
        );
    }

    let center_box = center_box
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(12.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(if is_editing {
                    c.text_default
                } else {
                    c.dialog_secondary_button_text
                })
                .child(
                    div()
                        .max_w(px(80.0))
                        .overflow_hidden()
                        .whitespace_nowrap()
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
