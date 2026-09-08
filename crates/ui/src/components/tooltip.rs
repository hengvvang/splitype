//! Tooltip component with WinUI 3 geometry (4px corner radius).

use gpui::*;
use theme::{ThemeColors, ThemeDimensions};

/// Standard floating tooltip container adhering to WinUI 3 ControlCornerRadius (4.0px).
pub fn tooltip_container(c: &ThemeColors, d: &ThemeDimensions) -> Div {
    div()
        .occlude()
        .max_w(px(420.0))
        .px(px(10.0))
        .py(px(6.0))
        .rounded(px(d.button_radius))
        .bg(c.dialog_surface)
        .border(px(1.0))
        .border_color(c.dialog_border)
        .shadow_md()
        .text_size(px(13.0))
        .text_color(c.dialog_muted)
        .line_height(relative(1.5))
}
