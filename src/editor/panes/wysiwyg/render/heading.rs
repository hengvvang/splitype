//! Heading block rendering (H1–H6).

use gpui::*;

use crate::editor::document::block::Block;
use crate::infra::theme::Theme;

/// Render a heading block at the given level (1–6).
pub(crate) fn render_heading(
    block: &mut Block,
    level: u8,
    focused: bool,
    is_placeholder: bool,
    focused_base: Stateful<Div>,
    theme: &Theme,
    cx: &mut Context<Block>,
) -> AnyElement {
    let style = theme.heading_style(level);

    let element = focused_base
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

    let text_content = block.render_text_or_mixed_inline_visuals(
        theme,
        focused,
        is_placeholder,
        style.text_color,
        style.font_size,
        style.font_weight,
        cx,
    );

    element.child(inner.child(text_content)).into_any_element()
}
