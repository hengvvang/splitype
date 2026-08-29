//! Popover style builders — floating menu panel containers.
//!
//! Positioning (`.absolute().left().top().w()`) and event handlers stay at
//! call sites; this builder supplies the shared surface styling.

use gpui::*;

use splitype_infra::theme::{ThemeColors, ThemeDimensions};

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

/// Full-window overlay layer — covers the editor viewport.
///
/// Event handling (`on_mouse_down`), occlude, and centering stay at call
/// sites; this builder supplies the four-corner full-screen geometry.
pub fn overlay() -> Div {
    div().absolute().top_0().left_0().right_0().bottom_0()
}
