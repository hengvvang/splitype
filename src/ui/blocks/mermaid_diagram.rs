//! Mermaid diagram block rendering.
// Migrated from blocks/render.rs

use gpui::*;

use crate::ui::blocks::block_view::Block;
use crate::services::i18n::I18nStrings;
use crate::ui::theme::Theme;

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

    let mermaid_preview = block.render_mermaid_content(theme, window);

    if !focused {
        block.last_layout = None;
        block.last_bounds = None;

        // Unfocused: outer rect (no border, no rounded, transparent)
        // with inner fitted diagram padded inside
        let outer = div()
            .w_full()
            .p(relative(0.005))
            .flex()
            .items_center()
            .justify_center()
            .child(mermaid_preview);

        focused_base.w_full().child(outer).into_any_element()
    } else {
        let show_toolbar =
            block.code_toolbar_hovered || block.code_language_picker_open || code_language_focused;
        let editor_section =
            block.render_code_editor_section(show_toolbar, is_placeholder, theme, strings, cx);

        let container = div()
            .w_full()
            .flex()
            .flex_col()
            .child(
                // Top: rendered diagram inside
                div()
                    .w_full()
                    .p(relative(0.005))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(mermaid_preview),
            )
            .child(editor_section);

        focused_base
            .relative()
            .on_hover(cx.listener(Block::on_code_block_hover))
            .bg(c.code_bg)
            .rounded(px(d.menu_item_radius))
            .w_full()
            .flex()
            .flex_col()
            .child(container)
            .into_any_element()
    }
}
