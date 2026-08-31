//! Fenced code block rendering.

use gpui::*;

use crate::model::block::Block;
use config::language::I18nStrings;
use theme::Theme;

/// Render a fenced code block with optional toolbar and language picker.
pub fn render_fenced_code(
    block: &mut Block,
    is_placeholder: bool,
    code_language_focused: bool,
    focused_base: Stateful<Div>,
    theme: &Theme,
    strings: &I18nStrings,
    cx: &mut Context<Block>,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;

    let show_toolbar =
        block.code_toolbar.is_hovered || block.code_toolbar.picker.is_open || code_language_focused;

    let editor_section =
        block.render_code_editor_section(show_toolbar, is_placeholder, theme, strings, cx);

    focused_base
        .relative()
        .on_hover(cx.listener(Block::on_code_block_hover))
        .child(
            div()
                .w_full()
                .bg(c.code_bg)
                .rounded(px(d.code_block_radius))
                .child(editor_section),
        )
        .into_any_element()
}
