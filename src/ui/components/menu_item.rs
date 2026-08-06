//! Menu item style builder — the standard floating-menu row.
//!
//! Used by context menus, the in-window menu bar, and settings navigation.
//! Conditional backgrounds (selected / hover-pinned items) can override the
//! default surface background by chaining `.bg(...)` at the call site.

use gpui::*;

use crate::theme::{ThemeColors, ThemeDimensions};

/// Menu item row structure (no interaction states).
///
/// Used by menu entries that must stay inert (disabled items, system
/// placeholders). Interactive rows use [`menu_item`].
pub fn menu_item_row(c: &ThemeColors, d: &ThemeDimensions) -> Div {
    div()
        .h(px(d.menu_item_height))
        .px(px(d.menu_item_padding_x))
        .flex()
        .items_center()
        .rounded(px(d.menu_item_radius))
        .bg(c.dialog_surface)
}

/// Standard interactive floating-menu item row.
pub fn menu_item(id: impl Into<ElementId>, c: &ThemeColors, d: &ThemeDimensions) -> Stateful<Div> {
    menu_item_row(c, d)
        .id(id)
        .hover(|this| this.bg(c.panel_row_hover))
        .active(|this| this.opacity(0.92))
        .cursor_pointer()
}
