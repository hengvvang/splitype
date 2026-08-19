//! Preview footnote definition rendering — real-id text plus content.

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
                .min_w(px(0.0))
                .flex_grow()
                .text_color(c.text_quote)
                .child(inline::render_preview_inline(
                    &block.data.text,
                    c.text_quote,
                    t.code_size,
                    FontWeight::NORMAL,
                    theme,
                )),
        );

    if block.has_footnote_definition_backref() {
        header = header.child(
            div()
                .text_color(c.footnote_backref)
                .hover(|this| this.underline().text_color(c.text_link))
                .cursor_pointer()
                .child("\u{21A9}"),
        );
    }

    header.into_any_element()
}

/// Renders the collected GitHub-style footnotes section: a top divider line
/// followed by every footnote definition in document order.
pub(crate) fn render_preview_footnotes_section(
    footnotes: &[Entity<Block>],
    theme: &Theme,
    window: &mut Window,
    cx: &App,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;

    let rows: Vec<AnyElement> = footnotes
        .iter()
        .map(|entity| super::render_preview_block(entity.read(cx), 0, 0, theme, window, cx))
        .collect();

    div()
        .w_full()
        .flex_shrink_0()
        .mt(px(d.block_gap * 2.0))
        .pt(px(8.0))
        .border_t(px(1.0))
        .border_color(c.footnote_border)
        .children(rows)
        .into_any_element()
}
