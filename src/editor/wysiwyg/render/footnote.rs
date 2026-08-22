//! Footnote definition rendering.

use gpui::*;

use crate::editor::tree::block::Block;
use crate::infra::theme::Theme;

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

    let header = focused_base
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(d.list_marker_gap))
        .text_size(px(t.code_size))
        .text_color(c.text_default)
        .child(
            div()
                .min_w(px(0.0))
                .flex_grow()
                .text_color(c.text_default)
                .child(block.render_text_or_mixed_inline_visuals(
                    theme,
                    focused,
                    is_placeholder,
                    None,
                    None,
                    c.text_default,
                    t.code_size,
                    FontWeight::NORMAL,
                    cx,
                )),
        );

    if block.has_footnote_definition_backref() {
        header
            .child(
                div()
                    .text_color(c.footnote_backref)
                    .hover(|this| this.underline().text_color(c.text_link))
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
