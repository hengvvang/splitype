//! Stepper style builders — the numeric stepper shared by the settings
//! window and the in-editor settings panel.

use gpui::*;

use splitype_infra::theme::{ThemeColors, ThemeDimensions};

/// Stepper container (decrement | value | increment).
pub fn stepper_container(c: &ThemeColors, d: &ThemeDimensions) -> Div {
    div()
        .flex()
        .items_center()
        .w(px(145.0))
        .h(px(28.0))
        .rounded(px(d.stepper_radius))
        .border_1()
        .border_color(c.dialog_border)
        .bg(c.dialog_secondary_button_bg)
        .overflow_hidden()
}

/// Stepper increment/decrement button.
pub fn stepper_step_button(id: impl Into<ElementId>, c: &ThemeColors) -> Stateful<Div> {
    div()
        .id(id)
        .cursor_pointer()
        .h_full()
        .w(px(28.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .hover(|this| this.bg(c.dialog_secondary_button_hover))
        .text_size(px(13.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(c.text_default)
}

/// Thin divider between stepper sections.
pub fn stepper_divider(c: &ThemeColors) -> Div {
    div().w(px(1.0)).h_full().flex_shrink_0().bg(c.dialog_border)
}

/// Stepper value display (editing background stays at call sites).
pub fn stepper_value() -> Div {
    div()
        .h_full()
        .flex_1()
        .min_w(px(0.0))
        .px(px(6.0))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(3.0))
}
