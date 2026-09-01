//! Preview blockquote rendering — quoted text color.

use gpui::*;

use crate::node::PreviewBlock;
use crate::render::inline;
use theme::Theme;

use std::ops::Range;

/// Renders a blockquote's own content line read-only. Nested children are
/// rendered by the dispatcher with an extra indent level.
pub(crate) fn render_preview_blockquote(
    block: &PreviewBlock,
    _depth: usize,
    selection_range: Option<Range<usize>>,
    base: Div,
    theme: &Theme,
) -> AnyElement {
    let c = &theme.colors;
    let t = &theme.typography;

    base.text_size(px(t.text_size))
        .text_color(c.text_quote)
        .line_height(rems(t.text_line_height))
        .child(inline::render_preview_inline_with_selection(
            &block.data.text,
            c.text_quote,
            t.text_size,
            FontWeight::NORMAL,
            theme,
            &block.search_matches,
            selection_range,
        ))
        .into_any_element()
}
