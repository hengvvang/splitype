//! Dialog card style builder — the shared dialog surface container.
//!
//! Width defaults to `dialog_width` and can be overridden at call sites
//! (e.g. the table-insert dialog uses a narrower maximum).

use gpui::*;

use theme::{ThemeColors, ThemeDimensions};

/// Dialog card container.
pub fn dialog_card(c: &ThemeColors, d: &ThemeDimensions) -> Div {
    div()
        .w(px(d.dialog_width))
        .max_w(relative(1.0))
        .flex()
        .flex_col()
        .gap(px(d.dialog_gap))
        .p(px(d.dialog_padding))
        .rounded(px(d.dialog_radius))
        .border(px(d.dialog_border_width))
        .border_color(c.dialog_border)
        .shadow_lg()
        .bg(c.dialog_surface)
}
