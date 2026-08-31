//! LaTeX math block rendering.

use gpui::*;

use crate::model::block::Block;
use crate::render::embedded_preview::render_graphic_preview_box;
use config::language::I18nStrings;
use theme::Theme;

/// Render a LaTeX math block.
pub fn render_latex_math(
    block: &mut Block,
    focused: bool,
    is_placeholder: bool,
    code_language_focused: bool,
    focused_base: Stateful<Div>,
    theme: &Theme,
    strings: &I18nStrings,
    cx: &mut Context<Block>,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;

    let (math_preview, is_rendered_graphic) = block.render_math_content(theme);
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
                .child(math_preview)
        } else {
            render_graphic_preview_box(math_preview, theme)
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
            .child(render_graphic_preview_box(math_preview, theme));

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
