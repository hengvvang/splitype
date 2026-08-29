//! Preview LaTeX math block rendering — centered SVG with fallback to the
//! raw source, mirroring the WYSIWYG math styles.

use gpui::*;

use crate::editor::plugins::latex_render::{display_math_font_size, render_display_math_svg};
use crate::editor::panes::preview::node::PreviewBlock;
use crate::infra::theme::Theme;
use crate::model::block::math::parse_display_math_source;

/// Renders a LaTeX math block read-only.
pub(crate) fn render_preview_latex_math(block: &PreviewBlock, base: Div, theme: &Theme) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;
    let raw_fallback;
    let raw = match block.data.raw_source.as_deref() {
        Some(s) => s,
        None => {
            raw_fallback = block.display_text();
            &raw_fallback
        }
    };

    let source = parse_display_math_source(raw).unwrap_or_else(|| {
        let body = raw
            .trim()
            .strip_prefix("$$")
            .unwrap_or(raw.trim())
            .strip_suffix("$$")
            .unwrap_or(raw.trim())
            .trim()
            .to_string();
        crate::model::block::math::DisplayMathSource {
            source: raw.to_string(),
            body,
        }
    });

    if source.body.is_empty() {
        return base
            .w_full()
            .child(crate::editor::panes::wysiwyg::render::embedded_preview::render_graphic_preview_box(
                crate::editor::panes::wysiwyg::render::graphic_state::render_empty_graphic_placeholder(
                    crate::editor::panes::wysiwyg::render::graphic_state::GraphicKind::LatexMath,
                    theme,
                ),
                theme,
            ))
            .into_any_element();
    }

    match render_display_math_svg(&source, c.text_default, display_math_font_size(t.text_size)) {
        Ok(rendered) => base
            .w_full()
            .flex()
            .justify_center()
            .py(px(d.block_padding_y.max(6.0)))
            .child(
                img(rendered.path.clone())
                    .max_w(Length::Definite(relative(1.0)))
                    .max_h(px(d.image_root_max_height))
                    .object_fit(ObjectFit::Contain),
            )
            .into_any_element(),
        Err(err) => base
            .w_full()
            .child(crate::editor::panes::wysiwyg::render::embedded_preview::render_graphic_preview_box(
                crate::editor::panes::wysiwyg::render::graphic_state::render_graphic_error_card(
                    crate::editor::panes::wysiwyg::render::graphic_state::GraphicKind::LatexMath,
                    &err.to_string(),
                    raw,
                    theme,
                ),
                theme,
            ))
            .into_any_element(),
    }
}
