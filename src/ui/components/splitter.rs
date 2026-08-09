//! Splitter bar style builders — the draggable dividers between tiled
//! panes. Horizontal bars resize rows, vertical bars resize columns.

use gpui::*;

use crate::infra::theme::ThemeColors;

/// Horizontal splitter bar (resizes rows).
pub fn splitter_bar_h(id: impl Into<ElementId>, c: &ThemeColors) -> Stateful<Div> {
    div()
        .id(id)
        .w(px(2.0))
        .h_full()
        .flex_shrink_0()
        .cursor_col_resize()
        .bg(c.dialog_border)
        .hover(|this| this.bg(c.selection))
}

/// Vertical splitter bar (resizes columns).
pub fn splitter_bar_v(id: impl Into<ElementId>, c: &ThemeColors) -> Stateful<Div> {
    div()
        .id(id)
        .h(px(2.0))
        .w_full()
        .flex_shrink_0()
        .cursor_row_resize()
        .bg(c.dialog_border)
        .hover(|this| this.bg(c.selection))
}
