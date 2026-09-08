//! WinUI 3 vertical selection indicator pill.
//!
//! A 3px-wide capsule (`rounded_full`) positioned along the left edge
//! of active list items, tree nodes, or navigation tabs.

use gpui::*;

/// Renders a vertical selection indicator pill (WinUI 3 standard 3px pill).
pub fn selection_indicator(color: Hsla, top: Pixels, bottom: Pixels) -> Div {
    div()
        .absolute()
        .left_0()
        .top(top)
        .bottom(bottom)
        .w(px(3.0))
        .rounded_full()
        .bg(color)
}
