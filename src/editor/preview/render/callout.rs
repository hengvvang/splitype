//! Preview callout rendering — read-only mirror of the WYSIWYG callout
//! styles.

use gpui::*;

use crate::editor::preview::render::inline;
use crate::editor::tree::block::Block;
use crate::infra::theme::Theme;
use crate::model::block::CalloutKind;

/// Renders a callout (admonition) block read-only.
pub(crate) fn render_preview_callout(
    block: &Block,
    variant: CalloutKind,
    _depth: usize,
    base: Div,
    theme: &Theme,
) -> AnyElement {
    let style = theme.callout_style(variant);
    let accent = style.border_color;
    let text_is_empty = block.data.text.plain_text().is_empty();
    let header_label = SharedString::from(variant.label());

    let header_text = if text_is_empty {
        div()
            .text_size(px(theme.typography.text_size))
            .font_weight(FontWeight::LIGHT)
            .text_color(accent)
            .child(header_label)
            .into_any_element()
    } else {
        div()
            .min_w(px(0.0))
            .flex_grow()
            .text_size(px(theme.typography.text_size))
            .font_weight(FontWeight::LIGHT)
            .text_color(accent)
            .child(inline::render_preview_inline(
                &block.data.text,
                accent,
                theme.typography.text_size,
                FontWeight::LIGHT,
                theme,
            ))
            .into_any_element()
    };

    base.w_full().child(header_text).into_any_element()
}
