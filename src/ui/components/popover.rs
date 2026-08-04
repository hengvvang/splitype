//! Popover style builders — floating menu panel containers.
//!
//! Positioning (`.absolute().left().top().w()`) and event handlers stay at
//! call sites; this builder supplies the shared surface styling.

use gpui::*;

use crate::theme::{ThemeColors, ThemeDimensions};

/// Floating menu panel container.
pub fn menu_panel(c: &ThemeColors, d: &ThemeDimensions) -> Div {
    div()
        .occlude()
        .bg(c.dialog_surface)
        .border(px(d.dialog_border_width))
        .border_color(c.dialog_border)
        .rounded(px(d.menu_panel_radius))
        .shadow_lg()
        .p(px(d.menu_panel_padding))
        .flex()
        .flex_col()
        .gap(px(d.menu_panel_gap))
}
