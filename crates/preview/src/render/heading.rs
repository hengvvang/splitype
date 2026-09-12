//! Preview heading rendering (H1–H6) — read-only mirror of the WYSIWYG
//! heading styles.

use gpui::*;

use crate::block::PreviewBlock;
use crate::render::inline;
use theme::Theme;

/// Renders a heading block at the given level (1–6).
pub(crate) fn render_preview_heading(
    block: &PreviewBlock,
    level: u8,
    base: Div,
    is_first: bool,
    theme: &Theme,
) -> AnyElement {
    let style = theme.heading_style(level);

    // Ergonomic vertical hierarchy:
    // Heading margin-top creates clear visual separation from preceding sections.
    // The document-initial heading (is_first) gets zero top margin to avoid awkward whitespace.
    let top_margin = if is_first {
        0.0
    } else {
        match level {
            1 => 32.0,
            2 => 24.0,
            3 => 18.0,
            4 => 14.0,
            5 => 12.0,
            6 => 10.0,
            _ => 10.0,
        }
    };

    let heading_line_height = match level {
        1 => 1.30,
        2 => 1.35,
        3 => 1.40,
        _ => 1.45,
    };

    let element = base
        .text_size(px(style.font_size))
        .font_weight(style.font_weight)
        .text_color(style.text_color)
        .line_height(rems(heading_line_height));

    let mut inner = div().w_full();
    if top_margin > 0.0 {
        inner = inner.mt(px(top_margin));
    }
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

    let text_content = inline::render_preview_inline(
        &block.data.text,
        style.text_color,
        style.font_size,
        style.font_weight,
        theme,
        &block.search_matches,
    );

    element.child(inner.child(text_content)).into_any_element()
}
