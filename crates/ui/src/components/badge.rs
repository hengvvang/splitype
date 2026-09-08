//! InfoBadge component adhering to WinUI 3 geometry (4px corner radius or pill).

use gpui::*;
use theme::{ThemeColors, ThemeDimensions};

/// Standard badge container adhering to WinUI 3 badge_radius (4.0px).
pub fn info_badge(c: &ThemeColors, d: &ThemeDimensions) -> Div {
    div()
        .px(px(8.0))
        .py(px(2.0))
        .min_w(px(28.0))
        .flex()
        .items_center()
        .justify_center()
        .border(px(1.0))
        .border_color(c.dialog_border)
        .rounded(px(d.badge_radius))
        .bg(c.dialog_surface)
        .text_size(px(12.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(c.text_default)
}

/// Standard pill badge container with full corner radius (rounded_full).
pub fn info_badge_pill(c: &ThemeColors) -> Div {
    div()
        .px(px(8.0))
        .py(px(2.0))
        .min_w(px(24.0))
        .flex()
        .items_center()
        .justify_center()
        .border(px(1.0))
        .border_color(c.dialog_border)
        .rounded_full()
        .bg(c.dialog_surface)
        .text_size(px(11.5))
        .font_weight(FontWeight::MEDIUM)
        .text_color(c.text_default)
}
