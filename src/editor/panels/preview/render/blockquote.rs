//! Preview blockquote rendering — quoted text color.

use gpui::*;

use crate::editor::tree::block::Block;
use crate::editor::panels::preview::render::inline;
use crate::infra::theme::Theme;

/// Renders a blockquote's own content line read-only. Nested children are
/// rendered by the dispatcher with an extra indent level.
pub(crate) fn render_preview_blockquote(
    block: &Block,
    _depth: usize,
    base: Div,
    theme: &Theme,
) -> AnyElement {
    let c = &theme.colors;
    let t = &theme.typography;

    base.text_size(px(t.text_size))
        .text_color(c.text_quote)
        .line_height(rems(t.text_line_height))
        .child(inline::render_preview_inline(
            &block.record.text,
            c.text_quote,
            t.text_size,
            FontWeight::NORMAL,
            theme,
        ))
        .into_any_element()
}
