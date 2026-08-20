//! Common UI helpers and building blocks for in-editor settings panel.

use gpui::*;

use crate::infra::theme::Theme;
use crate::ui::section::{section_card, section_header, settings_row};
use crate::ui::stepper::{stepper_container, stepper_divider, stepper_step_button, stepper_value};

pub(crate) type SettingsClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

pub(crate) fn make_row(
    title: &'static str,
    desc: &'static str,
    control: AnyElement,
    theme: &Theme,
    border_col: Hsla,
) -> AnyElement {
    let tc = &theme.colors;
    let td = &theme.dimensions;
    settings_row(border_col, tc, td)
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(tc.text_default)
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(tc.dialog_muted)
                        .child(desc),
                ),
        )
        .child(control)
        .into_any_element()
}

pub(crate) fn render_zed_stepper(
    id_dec: &'static str,
    id_inc: &'static str,
    val_num: String,
    unit_str: &'static str,
    is_editing: bool,
    on_dec: SettingsClickHandler,
    on_inc: SettingsClickHandler,
    on_click_center: SettingsClickHandler,
    theme: &Theme,
    panel_id: usize,
) -> AnyElement {
    let tc = &theme.colors;
    let td = &theme.dimensions;

    let mut center_box = stepper_value()
        .id(ElementId::Name(
            format!("{}-center-{}", id_dec, panel_id).into(),
        ))
        .bg(if is_editing {
            tc.dialog_surface
        } else {
            tc.dialog_secondary_button_bg
        })
        .child(
            div()
                .text_size(px(12.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(tc.text_default)
                .child(val_num),
        );

    if is_editing {
        center_box = center_box
            .border_1()
            .border_color(tc.dialog_primary_button_bg)
            .child(div().w(px(1.5)).h(px(12.0)).bg(tc.dialog_primary_button_bg));
    }

    if !unit_str.is_empty() {
        center_box = center_box.child(
            div()
                .text_size(px(11.0))
                .text_color(tc.dialog_muted)
                .child(unit_str),
        );
    }

    let center_box = center_box.on_click(on_click_center);

    stepper_container(tc, td)
        .child(
            stepper_step_button(id_dec, tc)
                .id((id_dec, panel_id))
                .child(
                    svg()
                        .path("icons/settings/minus.svg")
                        .size(px(12.0))
                        .text_color(tc.dialog_secondary_button_text),
                )
                .on_click(on_dec),
        )
        .child(stepper_divider(tc))
        .child(center_box)
        .child(stepper_divider(tc))
        .child(
            stepper_step_button(id_inc, tc)
                .id((id_inc, panel_id))
                .child(
                    svg()
                        .path("icons/settings/plus.svg")
                        .size(px(12.0))
                        .text_color(tc.dialog_secondary_button_text),
                )
                .on_click(on_inc),
        )
        .into_any_element()
}

pub(crate) fn make_section(
    sec_id: &'static str,
    title: &'static str,
    is_expanded: bool,
    toggle_fn: SettingsClickHandler,
    items: Vec<AnyElement>,
    theme: &Theme,
    panel_id: usize,
) -> AnyElement {
    let tc = &theme.colors;
    let td = &theme.dimensions;

    let header = section_header()
        .id((sec_id, panel_id))
        .child(
            svg()
                .path(if is_expanded {
                    "icons/settings/chevron-down.svg"
                } else {
                    "icons/settings/chevron-right.svg"
                })
                .size(px(16.0))
                .text_color(tc.text_default),
        )
        .child(
            div()
                .text_size(px(13.0))
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(tc.text_default)
                .child(title),
        )
        .on_click(move |event, window, cx| toggle_fn(event, window, cx));

    let mut card = section_card(tc, td).child(header);

    if is_expanded && !items.is_empty() {
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
