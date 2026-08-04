//! Callout / admonition block rendering.
// Migrated from blocks/render.rs

use gpui::*;

use crate::editor::block::Block;
use crate::model::block::CalloutKind;
use crate::ui::blocks::render::callout_accent_and_background;
use crate::ui::theme::Theme;

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
    let (accent, _) = callout_accent_and_background(variant, theme);
    let title_is_empty = block.record.text.visible_text().is_empty();
    let show_static_default_label = title_is_empty && !focused;
    let header_label = SharedString::from(variant.label());

    let header_text = if show_static_default_label {
        div()
            .text_size(px(theme.typography.text_size))
            .font_weight(FontWeight::LIGHT)
            .text_color(accent)
            .child(header_label.clone())
            .into_any_element()
    } else {
        div()
            .min_w(px(0.0))
            .flex_grow()
            .text_size(px(theme.typography.text_size))
            .font_weight(FontWeight::LIGHT)
            .text_color(accent)
            .child(block.render_text_or_mixed_inline_visuals(
                theme,
                focused,
                is_placeholder,
                Some(header_label),
                Some(accent),
                accent,
                theme.typography.text_size,
                FontWeight::LIGHT,
                cx,
            ))
            .into_any_element()
    };

    focused_base
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .child(header_text)
        .into_any_element()
}
