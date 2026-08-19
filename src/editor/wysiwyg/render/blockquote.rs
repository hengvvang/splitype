//! Blockquote rendering.

use gpui::*;

use crate::editor::tree::block::Block;
use crate::infra::theme::Theme;

/// Render a blockquote.
pub(crate) fn render_blockquote(
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
        .text_color(c.text_quote)
        .line_height(rems(t.text_line_height))
        .child(block.render_text_or_mixed_inline_visuals(
            theme,
            focused,
            is_placeholder,
            None,
            None,
            c.text_quote,
            t.text_size,
            FontWeight::NORMAL,
            cx,
        ))
        .into_any_element()
}
