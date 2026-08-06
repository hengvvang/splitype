//! Select style builders — the settings dropdown (trigger, panel, option).
//!
//! All three parts share one visual language across the theme, language,
//! image-paste, and startup dropdowns. Conditional option backgrounds
//! (selected state) stay at call sites.

use gpui::*;

use crate::theme::{ThemeColors, ThemeDimensions};

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
        .w(px(145.0))
        .h(px(28.0))
        .px(px(8.0))
        .rounded(px(d.menu_item_radius))
        .bg(c.dialog_secondary_button_bg)
        .hover(|this| this.bg(c.dialog_secondary_button_hover))
        .border_1()
        .border_color(c.dialog_border)
}

/// Dropdown panel anchored below the trigger.
pub fn select_panel(c: &ThemeColors) -> Div {
    div()
        .absolute()
        .top_full()
        .right_0()
        .mt(px(4.0))
        .w(px(160.0))
        .occlude()
        .bg(c.dialog_surface)
        .border_1()
        .border_color(c.dialog_border)
        .rounded(px(6.0))
        .shadow_lg()
        .p(px(4.0))
        .flex()
        .flex_col()
        .gap(px(2.0))
}

/// Dropdown option row (hoverable; selected background stays at call sites).
pub fn select_option(id: impl Into<ElementId>, c: &ThemeColors) -> Stateful<Div> {
    div()
        .id(id)
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_between()
        .px(px(10.0))
        .py(px(6.0))
        .rounded(px(4.0))
        .hover(|this| this.bg(c.panel_row_hover))
}
