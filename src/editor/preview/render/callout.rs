//! Preview callout rendering — read-only mirror of the WYSIWYG callout
//! styles. Deliberately re-implements the accent colors so preview styling
//! can diverge independently.

use gpui::*;

use crate::editor::preview::render::inline;
use crate::editor::tree::block::Block;
use crate::infra::theme::Theme;
use crate::model::block::CalloutKind;

/// Accent and background colors for a callout variant, mirroring the
/// WYSIWYG callout styles.
fn callout_accent(variant: CalloutKind, theme: &Theme) -> Hsla {
    let c = &theme.colors;
    match variant {
        CalloutKind::Note => c.callout_note_border,
        CalloutKind::Tip => c.callout_tip_border,
        CalloutKind::Important => c.callout_important_border,
        CalloutKind::Warning => c.callout_warning_border,
        CalloutKind::Caution => c.callout_caution_border,
    }
}

/// Renders a callout (admonition) block read-only.
pub(crate) fn render_preview_callout(
    block: &Block,
    variant: CalloutKind,
    _depth: usize,
    base: Div,
    theme: &Theme,
) -> AnyElement {
    let accent = callout_accent(variant, theme);
    let title_is_empty = block.record.text.visible_text().is_empty();
    let header_label = SharedString::from(variant.label());

    let header_text = if title_is_empty {
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
                &block.record.text,
                accent,
                theme.typography.text_size,
                FontWeight::LIGHT,
                theme,
            ))
            .into_any_element()
    };

    base.w_full().child(header_text).into_any_element()
}
