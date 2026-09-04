//! Preview paragraph rendering — plain text with inline styles.

use gpui::*;

use crate::block::PreviewBlock;
use crate::render::inline;
use theme::Theme;

/// Renders a plain paragraph (and HTML-comment fallback) read-only.
pub(crate) fn render_preview_paragraph(
    block: &PreviewBlock,
    base: Div,
    theme: &Theme,
) -> AnyElement {
    let c = &theme.colors;
    let t = &theme.typography;

    base.text_size(px(t.text_size))
        .text_color(c.text_default)
        .line_height(rems(t.text_line_height))
        .child(inline::render_preview_inline(
            &block.data.text,
            c.text_default,
            t.text_size,
            FontWeight::NORMAL,
            theme,
            &block.search_matches,
        ))
        .into_any_element()
}

/// Renders a raw Markdown fallback block read-only (identical to paragraph).
pub(crate) fn render_preview_raw_markdown(
    block: &PreviewBlock,
    base: Div,
    theme: &Theme,
) -> AnyElement {
    render_preview_paragraph(block, base, theme)
}
