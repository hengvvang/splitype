//! Raw HTML block rendering.

use gpui::*;

use crate::model::block::Block;
use theme::Theme;

/// Render a raw HTML block.
pub fn render_html_block(
    block: &mut Block,
    focused_base: Stateful<Div>,
    theme: &Theme,
    cx: &mut Context<Block>,
) -> AnyElement {
    let c = &theme.colors;
    let t = &theme.typography;

    let html = block.data.html.as_ref().cloned().unwrap_or_else(|| {
        markdown_parser::block::html::parse_html_document(
            block
                .data
                .raw_source
                .as_deref()
                .unwrap_or_else(|| block.display_text()),
        )
    });

    let hover_bg = match theme.appearance {
        theme::Appearance::Light => Hsla::from(rgba(0x0f172a07)),
        _ => Hsla::from(rgba(0xffffff07)),
    };
    focused_base
        .text_size(px(t.text_size))
        .text_color(c.text_default)
        .line_height(rems(t.text_line_height))
        .hover(move |this| this.bg(hover_bg))
        .child(block.render_html_document(&html, theme, cx))
        .into_any_element()
}
