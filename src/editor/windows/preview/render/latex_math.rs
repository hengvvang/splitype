//! Preview LaTeX math block rendering — centered SVG with fallback to the
//! raw source, mirroring the WYSIWYG math styles.

use gpui::*;

use crate::editor::render::latex_render::{
    display_math_font_size, render_display_math_svg,
};
use crate::editor::tree::block::Block;
use crate::model::syntax::math::parse_display_math_source;
use crate::theme::Theme;

/// Renders a LaTeX math block read-only.
pub(crate) fn render_preview_latex_math(
    block: &Block,
    base: Div,
    theme: &Theme,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;
    let raw = block
        .record
        .raw_source
        .as_deref()
        .unwrap_or_else(|| block.display_text());

    let source = parse_display_math_source(raw).unwrap_or_else(|| {
        let body = raw
            .trim()
            .strip_prefix("$$")
            .unwrap_or(raw.trim())
            .strip_suffix("$$")
            .unwrap_or(raw.trim())
            .trim()
            .to_string();
        crate::model::syntax::math::DisplayMathSource {
            raw: raw.to_string(),
            body,
        }
    });

    if source.body.is_empty() {
        return base
            .w_full()
            .text_size(px(t.text_size))
            .line_height(rems(t.text_line_height))
            .text_color(c.text_default)
            .child(SharedString::from(raw.to_string()))
            .into_any_element();
    }

    match render_display_math_svg(&source, c.text_default, display_math_font_size(t.text_size)) {
        Ok(rendered) => base
            .w_full()
            .flex()
            .justify_center()
            .py(px(d.block_padding_y.max(6.0)))
            .child(
                img(rendered.path)
                    .max_w(Length::Definite(relative(1.0)))
                    .max_h(px(d.image_root_max_height))
                    .object_fit(ObjectFit::Contain),
            )
            .into_any_element(),
        Err(err) => base
            .w_full()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .rounded_sm()
            .bg(c.source_mode_block_bg)
            .px(px(d.block_padding_x))
            .py(px(d.block_padding_y))
            .text_size(px(t.text_size))
            .line_height(rems(t.text_line_height))
            .text_color(c.text_default)
            .child(SharedString::from(raw.to_string()))
            .child(
                div()
                    .text_size(px(t.code_size))
                    .text_color(c.dialog_muted)
                    .child(SharedString::from(format!("LaTeX render error: {err}"))),
            )
            .into_any_element(),
    }
}
