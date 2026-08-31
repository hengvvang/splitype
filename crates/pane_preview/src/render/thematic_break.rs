//! Preview thematic break rendering — a horizontal rule.

use gpui::*;

use theme::Theme;

/// Renders a thematic break (horizontal rule) read-only.
pub(crate) fn render_preview_thematic_break(theme: &Theme) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;

    let line_slot_height = px(t.text_size * t.text_line_height);
    let line = div().w_full().h(px(d.separator_thickness)).bg(c.separator);

    div()
        .w_full()
        .h(line_slot_height)
        .text_size(px(t.text_size))
        .text_color(c.text_default)
        .line_height(rems(t.text_line_height))
        .flex()
        .flex_row()
        .items_center()
        .child(line)
        .into_any_element()
}
