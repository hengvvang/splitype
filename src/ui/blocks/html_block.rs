//! Raw HTML block rendering.
// Migrated from blocks/render.rs

use gpui::*;

use crate::editor::block::Block;
use crate::ui::theme::Theme;

/// Render a raw HTML block.
pub(crate) fn render_html_block(
    block: &mut Block,
    focused_base: Stateful<Div>,
    theme: &Theme,
    cx: &mut Context<Block>,
) -> AnyElement {
    let c = &theme.colors;
    let t = &theme.typography;

    let html = block.record.html.as_ref().cloned().unwrap_or_else(|| {
        crate::model::syntax::html::parse_html_document(
            block
                .record
                .raw_source
                .as_deref()
                .unwrap_or_else(|| block.display_text()),
        )
    });

    focused_base
        .text_size(px(t.text_size))
        .text_color(c.text_default)
        .line_height(rems(t.text_line_height))
        .child(block.render_html_document(&html, theme, cx))
        .into_any_element()
}
