//! Tab / navigation style builder — settings navigation rows.
//!
//! Selected-state backgrounds and text styling stay at call sites.

use gpui::*;

use crate::infra::theme::{ThemeColors, ThemeDimensions};

/// Settings navigation tab row.
pub fn nav_tab(id: impl Into<ElementId>, c: &ThemeColors, d: &ThemeDimensions) -> Stateful<Div> {
    div()
        .id(id)
        .px(px(12.0))
        .py(px(8.0))
        .rounded(px(d.tab_radius))
        .flex()
        .items_center()
        .cursor_pointer()
        .hover(|this| this.bg(c.panel_row_hover))
}
