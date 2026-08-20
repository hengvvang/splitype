//! Shared UI helpers and building blocks across settings tab views.

use gpui::*;

use crate::infra::theme::{ThemeColors, ThemeDimensions};
use crate::settings::window::SettingsWindow;
use crate::ui::section::{section_card, section_header, settings_row};
use crate::ui::stepper::{stepper_container, stepper_divider, stepper_step_button, stepper_value};

pub(crate) type SettingsClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

pub(crate) fn make_row(
    inner_border_color: Hsla,
    c: &ThemeColors,
    d: &ThemeDimensions,
    title: &'static str,
    desc: &'static str,
    control: AnyElement,
) -> AnyElement {
    settings_row(inner_border_color, c, d)
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(c.text_default)
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(c.dialog_muted)
                        .child(desc),
                ),
        )
        .child(control)
        .into_any_element()
}

pub(crate) fn render_zed_stepper(
    c: &ThemeColors,
    d: &ThemeDimensions,
    id_dec: &'static str,
    id_inc: &'static str,
    val_num: String,
    unit_str: &'static str,
    is_editing: bool,
    on_dec: SettingsClickHandler,
    on_inc: SettingsClickHandler,
    on_click_center: SettingsClickHandler,
) -> AnyElement {
    let mut center_box = stepper_value()
        .id(ElementId::Name(format!("{}-center", id_dec).into()))
        .bg(if is_editing {
            c.dialog_surface
        } else {
            c.dialog_secondary_button_bg
        })
        .child(
            div()
                .text_size(px(12.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(c.text_default)
                .child(val_num),
        );

    if is_editing {
        center_box = center_box
            .border_1()
            .border_color(c.dialog_primary_button_bg)
            .child(div().w(px(1.5)).h(px(12.0)).bg(c.dialog_primary_button_bg));
    }

    if !unit_str.is_empty() {
        center_box = center_box.child(
            div()
                .text_size(px(11.0))
                .text_color(c.dialog_muted)
                .child(unit_str),
        );
    }

    let center_box = center_box.on_click(on_click_center);

    stepper_container(c, d)
        .child(
            stepper_step_button(id_dec, c)
                .id(id_dec)
                .child(
                    svg()
                        .path("icons/settings/minus.svg")
                        .size(px(12.0))
                        .text_color(c.dialog_secondary_button_text),
                )
                .on_click(on_dec),
        )
        .child(stepper_divider(c))
        .child(center_box)
        .child(stepper_divider(c))
        .child(
            stepper_step_button(id_inc, c)
                .id(id_inc)
                .child(
                    svg()
                        .path("icons/settings/plus.svg")
                        .size(px(12.0))
                        .text_color(c.dialog_secondary_button_text),
                )
                .on_click(on_inc),
        )
        .into_any_element()
}

pub(crate) fn make_section(
    c: &ThemeColors,
    d: &ThemeDimensions,
    id: &'static str,
    key: &'static str,
    title: &'static str,
    expanded: bool,
    toggle_ed: WeakEntity<SettingsWindow>,
    items: Vec<AnyElement>,
) -> AnyElement {
    let header = section_header()
        .id(ElementId::Name(format!("{}-header", id).into()))
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
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(c.text_default)
                .child(title),
        )
        .on_click(move |_event, _window, cx| {
            let _ = toggle_ed.update(cx, |this, cx| {
                if this.expanded_sections.contains(key) {
                    this.expanded_sections.remove(key);
                } else {
                    this.expanded_sections.insert(key.to_string());
                }
                cx.notify();
            });
        });

    let mut card = section_card(c, d).id(id).child(header);

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
