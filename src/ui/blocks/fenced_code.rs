//! Fenced code block rendering.
// Migrated from blocks/render.rs

use gpui::*;

use crate::editor::block::Block;
use crate::services::i18n::I18nStrings;
use crate::ui::theme::Theme;

/// Render a fenced code block with optional toolbar and language picker.
pub(crate) fn render_fenced_code(
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
        block.code_toolbar_hovered || block.code_language_picker_open || code_language_focused;

    let editor_section =
        block.render_code_editor_section(show_toolbar, is_placeholder, theme, strings, cx);

    focused_base
        .relative()
        .on_hover(cx.listener(Block::on_code_block_hover))
        .bg(c.code_bg)
        .rounded(px(d.menu_item_radius))
        .child(editor_section)
        .into_any_element()
}
