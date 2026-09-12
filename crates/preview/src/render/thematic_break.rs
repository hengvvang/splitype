//! Preview thematic break rendering — a horizontal rule.

use gpui::*;

use theme::Theme;

/// Renders a thematic break (horizontal rule) read-only.
pub(crate) fn render_preview_thematic_break(theme: &Theme) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;

    let line = div().w_full().h(px(d.separator_thickness)).bg(c.separator);

    div()
        .w_full()
        .my(px(14.0))
        .py(px(6.0))
        .flex()
        .flex_row()
        .items_center()
        .child(line)
        .into_any_element()
}
