//! Preview heading rendering (H1–H6) — read-only mirror of the WYSIWYG
//! heading styles.

use gpui::*;

use crate::editor::preview::render::inline;
use crate::editor::tree::block::Block;
use crate::infra::theme::Theme;

/// Renders a heading block at the given level (1–6).
pub(crate) fn render_preview_heading(
    block: &Block,
    level: u8,
    base: Div,
    theme: &Theme,
) -> AnyElement {
    let style = theme.heading_style(level);

    let mut element = base
        .text_size(px(style.font_size))
        .font_weight(style.font_weight)
        .text_color(style.text_color);

    if style.padding_bottom > 0.0 {
        element = element.pb(px(style.padding_bottom));
    }
    if style.margin_bottom > 0.0 {
        element = element.mb(px(style.margin_bottom));
    }
    if style.border_width > 0.0 {
        element = element.border_b(px(style.border_width));
    }
    if let Some(border_color) = style.border_color {
        element = element.border_color(border_color);
    }

    element
        .child(inline::render_preview_inline(
            &block.data.text,
            style.text_color,
            style.font_size,
            style.font_weight,
            theme,
        ))
        .into_any_element()
}
