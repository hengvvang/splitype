//! Top bar container — the full-width bar at the top of every window panel.
//!
//! Shared by the editor / explorer / settings area top bars so all three
//! panels render the same height, tint and separator (dashed bottom edge).

use gpui::*;

use crate::infra::theme::ThemeColors;

/// Top bar container with a bottom separator line.
pub fn topbar_container(c: &ThemeColors, height: f32, padding_x: f32) -> Div {
    div()
        .h(px(height))
        .w_full()
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_between()
        .px(px(padding_x))
        .bg(c.dialog_surface)
        .border_b_1()
        .border_color(c.dialog_border)
}
