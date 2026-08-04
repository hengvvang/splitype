//! Button style builders — dialog action buttons and toolbar icon buttons.
//!
//! Two size families exist to preserve existing visuals:
//! - standard (`primary_button`, `secondary_button`): 36px height, large
//!   radius, used by the table-insert dialog and info dialogs;
//! - compact (`compact_*`): 32px height, small radius, used by the
//!   unsaved-changes / drop-replace dialogs.
//!
//! Builders take the element id and return a stateful element so press
//! feedback (`.active`) works. Text styling (size, weight, color) stays at
//! call sites because it comes from different typography tokens per family.

use gpui::*;

use crate::theme::{ThemeColors, ThemeDimensions};

/// Standard primary action button (36px height, large radius).
pub fn primary_button(
    id: impl Into<ElementId>,
    c: &ThemeColors,
    d: &ThemeDimensions,
) -> Stateful<Div> {
    action_base(
        id,
        d,
        d.dialog_button_height,
        (d.dialog_radius - 4.0).max(0.0),
    )
    .bg(c.dialog_primary_button_bg)
    .text_color(c.dialog_primary_button_text)
    .hover(|this| this.bg(c.dialog_primary_button_hover))
}

/// Standard secondary action button (36px height, large radius).
pub fn secondary_button(
    id: impl Into<ElementId>,
    c: &ThemeColors,
    d: &ThemeDimensions,
) -> Stateful<Div> {
    action_base(
        id,
        d,
        d.dialog_button_height,
        (d.dialog_radius - 4.0).max(0.0),
    )
    .border(px(d.dialog_border_width))
    .border_color(c.dialog_border)
    .bg(c.dialog_secondary_button_bg)
    .text_color(c.dialog_secondary_button_text)
    .hover(|this| this.bg(c.dialog_secondary_button_hover))
}

/// Compact primary action button (32px height, small radius).
pub fn compact_primary_button(
    id: impl Into<ElementId>,
    c: &ThemeColors,
    d: &ThemeDimensions,
) -> Stateful<Div> {
    action_base(id, d, 32.0, d.menu_item_radius)
        .bg(c.dialog_primary_button_bg)
        .text_color(c.dialog_primary_button_text)
        .hover(|this| this.bg(c.dialog_primary_button_hover))
}

/// Compact secondary action button (32px height, small radius).
pub fn compact_secondary_button(
    id: impl Into<ElementId>,
    c: &ThemeColors,
    d: &ThemeDimensions,
) -> Stateful<Div> {
    action_base(id, d, 32.0, d.menu_item_radius)
        .border(px(d.dialog_border_width))
        .border_color(c.dialog_border)
        .bg(c.dialog_secondary_button_bg)
        .text_color(c.dialog_secondary_button_text)
        .hover(|this| this.bg(c.dialog_secondary_button_hover))
}

/// Compact destructive action button (32px height, small radius).
pub fn compact_danger_button(
    id: impl Into<ElementId>,
    c: &ThemeColors,
    d: &ThemeDimensions,
) -> Stateful<Div> {
    action_base(id, d, 32.0, d.menu_item_radius)
        .border(px(d.dialog_border_width))
        .border_color(c.dialog_border)
        .bg(c.dialog_danger_button_bg)
        .text_color(c.dialog_danger_button_text)
        .hover(|this| this.bg(c.dialog_danger_button_hover))
}

/// Compact toolbar icon button (26px wide, full height).
pub fn icon_button(
    id: impl Into<ElementId>,
    c: &ThemeColors,
    d: &ThemeDimensions,
) -> Stateful<Div> {
    div()
        .id(id)
        .relative()
        .w(px(26.0))
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(d.menu_item_radius - 2.0))
        .hover(|this| this.bg(c.dialog_secondary_button_hover))
        .active(|this| this.opacity(0.9))
        .cursor_pointer()
}

/// Minimal icon chip (padding-only) for toolbar and header actions.
/// Call sites add `.id(...)` when the element needs interactivity state.
pub fn icon_chip_button(c: &ThemeColors, d: &ThemeDimensions) -> Div {
    div()
        .p(px(3.0))
        .rounded(px(d.menu_item_radius))
        .hover(|this| this.bg(c.dialog_secondary_button_hover))
        .cursor_pointer()
}

/// Small pill button with a secondary background (area headers, status
/// bars). Call sites may add `.id(...)` and override the background for
/// selected states.
pub fn small_pill_button(c: &ThemeColors, d: &ThemeDimensions) -> Div {
    div()
        .h(px(22.0))
        .px(px(8.0))
        .flex()
        .items_center()
        .gap(px(4.0))
        .rounded(px(d.menu_item_radius))
        .bg(c.dialog_secondary_button_bg)
        .hover(|this| this.bg(c.dialog_secondary_button_hover))
        .cursor_pointer()
}

/// Top-level menu bar button. Width is label-driven and set at call sites;
/// the open/selected background overrides the default transparent one.
pub fn menu_bar_button(
    id: impl Into<ElementId>,
    c: &ThemeColors,
    d: &ThemeDimensions,
) -> Stateful<Div> {
    div()
        .id(id)
        .h(px(d.menu_bar_button_height))
        .px(px(5.0))
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .rounded(px(d.menu_bar_button_radius))
        .hover(|this| this.bg(c.dialog_secondary_button_hover))
        .active(|this| this.opacity(0.92))
        .cursor_pointer()
}

fn action_base(
    id: impl Into<ElementId>,
    d: &ThemeDimensions,
    height: f32,
    radius: f32,
) -> Stateful<Div> {
    div()
        .id(id)
        .h(px(height))
        .px(px(d.dialog_button_padding_x))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(radius))
        .active(|this| this.opacity(0.92))
        .cursor_pointer()
}
