//! Status bar container — the full-width bar shared by the editor status
//! bar and the per-panel status bars. Tinted like the area header and
//! borderless so the header/status bar read as one surface.

use gpui::*;

use crate::theme::ThemeColors;

/// Status bar container without a top border, tinted like the area header.
pub fn status_bar_container(c: &ThemeColors, height: f32, padding_x: f32) -> Div {
    div()
        .h(px(height))
        .w_full()
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_between()
        .px(px(padding_x))
        .bg(c.dialog_surface)
}
