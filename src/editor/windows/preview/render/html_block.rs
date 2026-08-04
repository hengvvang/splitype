//! Preview raw HTML block rendering.
//!
//! First-generation preview renders the raw HTML source as plain styled
//! text; a full HTML document renderer can be added later without touching
//! the WYSIWYG side.

use gpui::*;

use crate::editor::tree::block::Block;
use crate::theme::Theme;

/// Renders a raw HTML block read-only.
pub(crate) fn render_preview_html_block(
    block: &Block,
    base: Div,
    theme: &Theme,
) -> AnyElement {
    let c = &theme.colors;
    let t = &theme.typography;

    let raw = block
        .record
        .raw_source
        .as_deref()
        .unwrap_or_else(|| block.display_text());

    base.text_size(px(t.text_size))
        .text_color(c.text_default)
        .line_height(rems(t.text_line_height))
        .child(SharedString::from(raw.to_string()))
        .into_any_element()
}
