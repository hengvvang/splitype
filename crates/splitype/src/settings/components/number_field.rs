//! Zed-identical NumberField component with inline text editing and steppers.
//!
//! Replaces mock cycle buttons with genuine inline typing, keyboard navigation (Up/Down/Enter/Esc),
//! Shift/Alt modifier step scaling, and clean unitless numeric presentation matching Zed.

use gpui::prelude::FluentBuilder;
use gpui::*;

use theme::{ThemeColors, ThemeDimensions};
use crate::settings::ui_helpers::SettingsClickHandler;
use ui::stepper::{stepper_container, stepper_divider, stepper_step_button};

pub struct NumberFieldProps {
    pub id_prefix: &'static str,
    pub value_text: String,
    pub is_editing: bool,
    pub edit_buffer: Option<String>,
    pub focus_handle: FocusHandle,
    pub on_dec: SettingsClickHandler,
    pub on_inc: SettingsClickHandler,
    pub on_start_edit: Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>,
    pub on_key_down: Box<dyn Fn(&KeyDownEvent, &mut Window, &mut App) + 'static>,
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
            this.border_y_1().border_color(c.app_menu_active)
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
                            .bg(c.app_menu_active),
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
