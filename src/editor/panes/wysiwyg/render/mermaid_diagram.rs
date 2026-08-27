//! Mermaid diagram block rendering.

use gpui::*;

use crate::editor::document::block::Block;
use crate::editor::panes::wysiwyg::render::embedded_preview::render_graphic_preview_box;
use crate::infra::i18n::I18nStrings;
use crate::infra::theme::Theme;

/// Render a Mermaid diagram block.
pub(crate) fn render_mermaid_diagram(
    block: &mut Block,
    focused: bool,
    is_placeholder: bool,
    code_language_focused: bool,
    focused_base: Stateful<Div>,
    theme: &Theme,
    strings: &I18nStrings,
    window: &mut Window,
    cx: &mut Context<Block>,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;

    let (mermaid_preview, is_rendered_graphic) = block.render_mermaid_content(theme, window);
    let is_editing = focused || code_language_focused || block.code_toolbar.picker.is_open;

    if !is_editing {
        block.last_paints.clear();

        let outer = if is_rendered_graphic {
            div()
                .w_full()
                .p(relative(0.005))
                .flex()
                .items_center()
                .justify_center()
                .child(mermaid_preview)
        } else {
            render_graphic_preview_box(mermaid_preview, theme)
        };

        focused_base.w_full().child(outer).into_any_element()
    } else {
        let show_toolbar = block.code_toolbar.is_hovered
            || block.code_toolbar.picker.is_open
            || code_language_focused;
        let editor_section =
            block.render_code_editor_section(show_toolbar, is_placeholder, theme, strings, cx);

        let container = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .w_full()
                    .bg(c.code_bg)
                    .rounded(px(d.code_block_radius))
                    .child(editor_section),
            )
            .child(render_graphic_preview_box(mermaid_preview, theme));

        focused_base
            .relative()
            .on_hover(cx.listener(Block::on_code_block_hover))
            .w_full()
            .flex()
            .flex_col()
            .child(container)
            .into_any_element()
    }
}
