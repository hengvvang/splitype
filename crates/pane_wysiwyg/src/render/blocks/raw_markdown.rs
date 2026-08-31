//! Raw Markdown block rendering (passthrough for unrecognized content).

use gpui::*;

use crate::model::block::Block;
use theme::Theme;

/// Render a raw Markdown fallback block (identical to paragraph rendering).
pub fn render_raw_markdown(
    block: &mut Block,
    focused: bool,
    is_placeholder: bool,
    focused_base: Stateful<Div>,
    theme: &Theme,
    cx: &mut Context<Block>,
) -> AnyElement {
    let c = &theme.colors;
    let t = &theme.typography;

    focused_base
        .text_size(px(t.text_size))
        .text_color(c.text_default)
        .line_height(rems(t.text_line_height))
        .child(block.render_text_or_mixed_inline_visuals(
            theme,
            focused,
            is_placeholder,
            c.text_default,
            t.text_size,
            FontWeight::NORMAL,
            cx,
        ))
        .into_any_element()
}

