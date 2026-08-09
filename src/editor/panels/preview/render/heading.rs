//! Preview heading rendering (H1–H6) — read-only mirror of the WYSIWYG
//! heading styles.

use gpui::*;

use crate::editor::tree::block::Block;
use crate::editor::panels::preview::render::inline;
use crate::infra::theme::Theme;

/// Renders a heading block at the given level (1–6).
pub(crate) fn render_preview_heading(
    block: &Block,
    level: u8,
    base: Div,
    theme: &Theme,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;

    let (text_color, font_size, font_weight) = match level {
        1 => (c.text_h1, t.h1_size, t.h1_weight.to_font_weight()),
        2 => (c.text_h2, t.h2_size, t.h2_weight.to_font_weight()),
        3 => (c.text_h3, t.h3_size, t.h3_weight.to_font_weight()),
        4 => (c.text_h4, t.h4_size, t.h4_weight.to_font_weight()),
        5 => (c.text_h5, t.h5_size, t.h5_weight.to_font_weight()),
        6 => (c.text_h6, t.h6_size, t.h6_weight.to_font_weight()),
        _ => (c.text_default, t.text_size, FontWeight::NORMAL),
    };

    let mut element = base
        .text_size(px(font_size))
        .font_weight(font_weight)
        .text_color(text_color)
        .child(inline::render_preview_inline(
            &block.record.text,
            text_color,
            font_size,
            font_weight,
            theme,
        ));

    if level == 1 {
        element = element
            .pb(px(d.h1_padding_bottom))
            .mb(px(d.h1_margin_bottom))
            .border_b(px(d.h1_border_width))
            .border_color(c.border_h1);
    }

    element.into_any_element()
}
