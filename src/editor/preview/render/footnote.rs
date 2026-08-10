//! Preview footnote definition rendering — ordinal badge plus content.

use gpui::*;

use crate::editor::preview::render::inline;
use crate::editor::tree::block::Block;
use crate::infra::theme::Theme;

/// Renders a footnote definition block read-only.
pub(crate) fn render_preview_footnote_definition(
    block: &Block,
    _depth: usize,
    base: Div,
    theme: &Theme,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;

    let ordinal = block
        .footnote_definition_ordinal()
        .map(|ordinal| ordinal.to_string())
        .unwrap_or_else(|| "?".to_string());
    let badge_text_size = px((t.code_size - 1.0).max(10.0));

    let mut header = base
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
                .child(SharedString::from(ordinal)),
        )
        .child(
            div()
                .min_w(px(0.0))
                .flex_grow()
                .text_color(c.text_quote)
                .child(inline::render_preview_inline(
                    &block.record.text,
                    c.text_quote,
                    t.code_size,
                    FontWeight::NORMAL,
                    theme,
                )),
        );

    if block.footnote_definition_has_backref() {
        header = header.child(div().text_color(c.footnote_backref).child("\u{21A9}"));
    }

    header.into_any_element()
}
