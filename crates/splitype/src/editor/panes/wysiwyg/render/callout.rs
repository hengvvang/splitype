//! Callout / admonition block rendering.

use gpui::*;

use crate::editor::document::block::Block;
use crate::editor::panes::wysiwyg::render::layout::callout_colors;
use theme::Theme;
use primitives::CalloutKind;

/// Render a callout (admonition) block.
pub(crate) fn render_callout(
    block: &mut Block,
    variant: CalloutKind,
    focused: bool,
    is_placeholder: bool,
    focused_base: Stateful<Div>,
    theme: &Theme,
    cx: &mut Context<Block>,
) -> AnyElement {
    let (accent, _) = callout_colors(variant, theme);
    let t = &theme.typography;

    focused_base
        .text_size(px(t.text_size))
        .text_color(accent)
        .line_height(rems(t.text_line_height))
        .child(block.render_text_or_mixed_inline_visuals(
            theme,
            focused,
            is_placeholder,
            accent,
            t.text_size,
            FontWeight::NORMAL,
            cx,
        ))
        .into_any_element()
}
