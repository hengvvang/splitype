//! Empty state style builder — the centered placeholder container used by
//! empty editor panels and the explorer sidebar.

use gpui::*;

/// Centered empty-state container.
pub fn empty_state_container() -> Div {
    div()
        .w_full()
        .h_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .text_align(TextAlign::Center)
}
