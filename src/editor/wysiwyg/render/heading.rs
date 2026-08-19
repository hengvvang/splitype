//! Heading block rendering (H1–H6).

use gpui::*;

use crate::editor::tree::block::Block;
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
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;

    match level {
        1 => focused_base
            .text_size(px(t.h1_size))
            .font_weight(t.h1_weight.to_font_weight())
            .text_color(c.text_h1)
            .pb(px(d.h1_padding_bottom))
            .mb(px(d.h1_margin_bottom))
            .border_b(px(d.h1_border_width))
            .border_color(c.border_h1)
            .child(block.render_text_or_mixed_inline_visuals(
                theme,
                focused,
                is_placeholder,
                None,
                None,
                c.text_h1,
                t.h1_size,
                t.h1_weight.to_font_weight(),
                cx,
            ))
            .into_any_element(),
        2 => focused_base
            .text_size(px(t.h2_size))
            .font_weight(t.h2_weight.to_font_weight())
            .text_color(c.text_h2)
            .pb(px(d.h1_padding_bottom))
            .mb(px(d.h1_margin_bottom))
            .border_b(px(d.h1_border_width))
            .border_color(c.border_h2)
            .child(block.render_text_or_mixed_inline_visuals(
                theme,
                focused,
                is_placeholder,
                None,
                None,
                c.text_h2,
                t.h2_size,
                t.h2_weight.to_font_weight(),
                cx,
            ))
            .into_any_element(),
        3 => focused_base
            .text_size(px(t.h3_size))
            .font_weight(t.h3_weight.to_font_weight())
            .text_color(c.text_h3)
            .child(block.render_text_or_mixed_inline_visuals(
                theme,
                focused,
                is_placeholder,
                None,
                None,
                c.text_h3,
                t.h3_size,
                t.h3_weight.to_font_weight(),
                cx,
            ))
            .into_any_element(),
        4 => focused_base
            .text_size(px(t.h4_size))
            .font_weight(t.h4_weight.to_font_weight())
            .text_color(c.text_h4)
            .child(block.render_text_or_mixed_inline_visuals(
                theme,
                focused,
                is_placeholder,
                None,
                None,
                c.text_h4,
                t.h4_size,
                t.h4_weight.to_font_weight(),
                cx,
            ))
            .into_any_element(),
        5 => focused_base
            .text_size(px(t.h5_size))
            .font_weight(t.h5_weight.to_font_weight())
            .text_color(c.text_h5)
            .child(block.render_text_or_mixed_inline_visuals(
                theme,
                focused,
                is_placeholder,
                None,
                None,
                c.text_h5,
                t.h5_size,
                t.h5_weight.to_font_weight(),
                cx,
            ))
            .into_any_element(),
        6 => focused_base
            .text_size(px(t.h6_size))
            .font_weight(t.h6_weight.to_font_weight())
            .text_color(c.text_h6)
            .child(block.render_text_or_mixed_inline_visuals(
                theme,
                focused,
                is_placeholder,
                None,
                None,
                c.text_h6,
                t.h6_size,
                t.h6_weight.to_font_weight(),
                cx,
            ))
            .into_any_element(),
        _ => focused_base
            .text_size(px(t.text_size))
            .text_color(c.text_default)
            .line_height(rems(t.text_line_height))
            .child(block.render_text_or_mixed_inline_visuals(
                theme,
                focused,
                is_placeholder,
                None,
                None,
                c.text_default,
                t.text_size,
                FontWeight::NORMAL,
                cx,
            ))
            .into_any_element(),
    }
}
