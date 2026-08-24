//! Thematic break (horizontal rule) rendering.

use gpui::*;

use crate::editor::tree::block::Block;
use crate::infra::theme::Theme;

/// Render a thematic break (horizontal rule) when the block is not focused:
/// a full-width line (matching the editing column), vertically centered on
/// the row.
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
        .h(px(d.separator_thickness))
        .bg(c.separator);

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

/// Render a thematic break when the block is focused: the raw Markdown
/// source (`---` / `***` / `___`) becomes directly editable text.
pub(crate) fn render_thematic_break_focused(
    block: &mut Block,
    focused: bool,
    is_placeholder: bool,
    focused_base: Stateful<Div>,
    theme: &Theme,
    cx: &mut Context<Block>,
) -> AnyElement {
    let c = &theme.colors;
    let t = &theme.typography;

    let line_slot_height = px(t.text_size * t.text_line_height);
    let text_input = block.render_text_or_mixed_inline_visuals(
        theme,
        focused,
        is_placeholder,
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
        .child(text_input)
        .into_any_element()
}
