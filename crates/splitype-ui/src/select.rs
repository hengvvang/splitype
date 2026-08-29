//! Select style builders — the settings dropdown (trigger, panel, option).
//!
//! All three parts share one visual language across the theme, language,
//! image-paste, and startup dropdowns. Conditional option backgrounds
//! (selected state) stay at call sites.

use gpui::*;

use splitype_infra::theme::{ThemeColors, ThemeDimensions};

/// Dropdown trigger button (bordered, hoverable).
pub fn select_trigger(
    id: impl Into<ElementId>,
    c: &ThemeColors,
    d: &ThemeDimensions,
) -> Stateful<Div> {
    div()
        .id(id)
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_between()
        .w(px(160.0))
        .h(px(28.0))
        .px(px(8.0))
        .rounded(px(d.select_trigger_radius))
        .bg(c.dialog_secondary_button_bg)
        .hover(|this| this.bg(c.dialog_secondary_button_hover))
        .border_1()
        .border_color(c.dialog_border)
}

/// Dropdown panel anchored below the trigger.
pub fn select_panel(c: &ThemeColors, d: &ThemeDimensions) -> Div {
    div()
        .absolute()
        .top_full()
        .right_0()
        .mt(px(4.0))
        .min_w(px(220.0))
        .max_w(px(360.0))
        .occlude()
        .bg(c.dialog_surface)
        .border_1()
        .border_color(c.dialog_border)
        .rounded(px(d.select_panel_radius))
        .shadow_lg()
        .p(px(4.0))
        .flex()
        .flex_col()
        .gap(px(2.0))
}

/// Dropdown option row (hoverable; selected background stays at call sites).
pub fn select_option(
    id: impl Into<ElementId>,
    c: &ThemeColors,
    d: &ThemeDimensions,
) -> Stateful<Div> {
    div()
        .id(id)
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_between()
        .whitespace_nowrap()
        .gap(px(8.0))
        .px(px(10.0))
        .py(px(6.0))
        .rounded(px(d.select_option_radius))
        .hover(|this| this.bg(c.panel_row_hover))
}
