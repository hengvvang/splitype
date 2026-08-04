//! Section style builders — collapsible settings cards.

use gpui::*;

use crate::theme::{ThemeColors, ThemeDimensions};

/// Collapsible section header row (chevron + title stay at call sites).
pub fn section_header() -> Div {
    div()
        .w_full()
        .px(px(14.0))
        .py(px(10.0))
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(8.0))
}

/// Card container holding a section header and its body.
pub fn section_card(c: &ThemeColors, d: &ThemeDimensions) -> Div {
    div()
        .relative()
        .w_full()
        .rounded(px(d.menu_panel_radius))
        .bg(c.dialog_surface)
        .border_1()
        .border_color(c.dialog_border)
        .flex()
        .flex_col()
}
