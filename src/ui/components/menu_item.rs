//! Menu item style builder — the standard floating-menu row.
//!
//! Used by context menus, the in-window menu bar, and settings navigation.
//! Conditional backgrounds (selected / hover-pinned items) can override the
//! default surface background by chaining `.bg(...)` at the call site.

use gpui::*;

use crate::theme::{ThemeColors, ThemeDimensions};

/// Standard floating-menu item row.
pub fn menu_item(id: impl Into<ElementId>, c: &ThemeColors, d: &ThemeDimensions) -> Stateful<Div> {
    div()
        .id(id)
        .h(px(d.menu_item_height))
        .px(px(d.menu_item_padding_x))
        .flex()
        .items_center()
        .rounded(px(d.menu_item_radius))
        .bg(c.dialog_surface)
        .hover(|this| this.bg(c.dialog_secondary_button_hover))
        .active(|this| this.opacity(0.92))
        .cursor_pointer()
}
