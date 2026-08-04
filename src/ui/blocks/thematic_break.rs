//! Thematic break (horizontal rule) rendering.
// Migrated from blocks/render.rs

use gpui::*;

use crate::ui::blocks::block_view::Block;
use crate::ui::theme::Theme;

/// Render a thematic break (horizontal rule) when the block is not focused.
pub(crate) fn render_thematic_break_unfocused(
    focused_base: Stateful<Div>,
    theme: &Theme,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;

    let line_slot_height = px(t.text_size * t.text_line_height);
    let line = div()
        .w_full()
        .border_b(px(d.separator_thickness))
        .border_color(c.separator_color);

    focused_base
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

/// Render a thematic break when the block is focused (shows editable text
/// alongside the separator line).
pub(crate) fn render_thematic_break_focused(
    block: &mut Block,
    focused: bool,
    is_placeholder: bool,
    focused_base: Stateful<Div>,
    theme: &Theme,
    cx: &mut Context<Block>,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;

    let line_slot_height = px(t.text_size * t.text_line_height);
    let line = div()
        .w_full()
        .border_b(px(d.separator_thickness))
        .border_color(c.separator_color);

    let text_input = block.render_text_or_mixed_inline_visuals(
        theme,
        focused,
        is_placeholder,
        None,
        None,
        c.text_default,
        t.text_size,
        FontWeight::NORMAL,
        cx,
    );

    focused_base
        .w_full()
        .h(line_slot_height)
        .text_size(px(t.text_size))
        .text_color(c.text_default)
        .line_height(rems(t.text_line_height))
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .child(div().flex_none().child(text_input))
        .child(
            div()
                .w(relative(0.70))
                .h_full()
                .flex()
                .items_center()
                .child(line),
        )
        .into_any_element()
}
