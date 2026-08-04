//! Status bar container — the full-width bar with a top border shared by
//! the editor status bar and the per-panel status bars.

use gpui::*;

use crate::theme::ThemeColors;

/// Status bar container with top border.
pub fn status_bar_container(c: &ThemeColors, height: f32, padding_x: f32) -> Div {
    div()
        .h(px(height))
        .w_full()
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_between()
        .px(px(padding_x))
        .bg(c.status_bar_background)
        .border_t(px(1.0))
        .border_color(c.dialog_border)
}
