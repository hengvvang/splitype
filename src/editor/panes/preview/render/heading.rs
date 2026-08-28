//! Preview heading rendering (H1–H6) — read-only mirror of the WYSIWYG
//! heading styles.

use gpui::*;

use crate::editor::panes::preview::render::inline;
use crate::editor::document::block::Block;
use crate::infra::theme::Theme;

/// Renders a heading block at the given level (1–6).
pub(crate) fn render_preview_heading(
    block: &Block,
    level: u8,
    base: Div,
    theme: &Theme,
) -> AnyElement {
    let style = theme.heading_style(level);

    let element = base
        .text_size(px(style.font_size))
        .font_weight(style.font_weight)
        .text_color(style.text_color);

    let mut inner = div().w_full();
    if style.padding_bottom > 0.0 {
        inner = inner.pb(px(style.padding_bottom));
    }
    if style.margin_bottom > 0.0 {
        inner = inner.mb(px(style.margin_bottom));
    }
    if style.border_width > 0.0 {
        inner = inner.border_b(px(style.border_width));
    }
    if let Some(border_color) = style.border_color {
        inner = inner.border_color(border_color);
    }

    let text_content = inline::render_preview_inline_with_matches(
        &block.data.text,
        style.text_color,
        style.font_size,
        style.font_weight,
        theme,
        &block.search_matches,
    );

    element.child(inner.child(text_content)).into_any_element()
}
