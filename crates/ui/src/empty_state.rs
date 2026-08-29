//! Empty state style builder — the centered placeholder container used by
//! empty editor panels and the explorer sidebar.

use gpui::*;

/// Centered empty-state container that scrolls on small viewports without top-clipping.
pub fn empty_state_container(id: impl Into<ElementId>) -> Stateful<Div> {
    div()
        .id(id)
        .w_full()
        .h_full()
        .flex()
        .flex_col()
        .items_center()
        .overflow_y_scroll()
        .text_align(TextAlign::Center)
}
