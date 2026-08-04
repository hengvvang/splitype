//! Footnote definition rendering.
// Migrated from blocks/render.rs

use gpui::*;

use crate::editor::tree::block::Block;
use crate::theme::Theme;

/// Render a footnote definition block.
pub(crate) fn render_footnote_definition(
    block: &mut Block,
    focused: bool,
    is_placeholder: bool,
    focused_base: Stateful<Div>,
    theme: &Theme,
    cx: &mut Context<Block>,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;

    let ordinal = block.footnote_definition_ordinal();
    let badge = ordinal
        .map(|ordinal| ordinal.to_string())
        .unwrap_or_else(|| "?".to_string());
    let badge_text_size = px((t.code_size - 1.0).max(10.0));

    let header = focused_base
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(d.list_marker_gap))
        .text_size(px(t.code_size))
        .text_color(c.text_quote)
        .child(
            div()
                .px(px(d.footnote_badge_padding_x))
                .py(px(d.footnote_badge_padding_y))
                .rounded(px(999.0))
                .bg(c.footnote_badge_bg)
                .text_size(badge_text_size)
                .text_color(c.footnote_badge_text)
                .font_weight(FontWeight::SEMIBOLD)
                .child(SharedString::from(badge)),
        )
        .child(
            div()
                .min_w(px(0.0))
                .flex_grow()
                .text_color(c.text_quote)
                .child(block.render_text_or_mixed_inline_visuals(
                    theme,
                    focused,
                    is_placeholder,
                    None,
                    None,
                    c.text_quote,
                    t.code_size,
                    FontWeight::NORMAL,
                    cx,
                )),
        );

    if block.footnote_definition_has_backref() {
        header
            .child(
                div()
                    .text_color(c.footnote_backref)
                    .hover(|this| this.text_color(c.text_link))
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(Block::on_footnote_backref_mouse_down),
                    )
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(Block::on_footnote_backref_mouse_up),
                    )
                    .child("\u{21A9}"),
            )
            .into_any_element()
    } else {
        header.into_any_element()
    }
}
