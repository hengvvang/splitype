//! Bottom bar container — the full-width bar at the bottom of every window
//! area that has one (editor / explorer).
//!
//! Shared by the per-area bottom bars so all areas render the same height,
//! tint and separator (dashed top edge).

use gpui::*;

use crate::infra::theme::ThemeColors;

/// Bottom bar container with a top separator line.
pub fn bottombar_container(c: &ThemeColors, height: f32, padding_x: f32) -> Div {
    div()
        .h(px(height))
        .w_full()
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_between()
        .px(px(padding_x))
        .bg(c.dialog_surface)
        .border_t_1()
        .border_color(c.dialog_border)
}
