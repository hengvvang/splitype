//! Read-only inline rendering for the preview panel.
//!
//! Walks a [`RichText`] render cache and produces plain styled text runs —
//! bold, italic, underline, strikethrough, script, inline code, links,
//! footnotes, and inline math — with no caret, selection, or projection
//! logic. Styles mirror the WYSIWYG inline rendering.

use gpui::*;

use crate::editor::render::latex_render::{inline_math_font_size, render_inline_math_svg};
use crate::infra::theme::Theme;
use crate::model::inline::render_cache::InlineSpan;
use crate::model::inline::style::InlineScript;
use crate::model::inline::text::RichText;

/// Renders the inline content of `text` with `base_color` and `font_size`,
/// mirroring the WYSIWYG inline segment styling without any editing state.
pub(crate) fn render_preview_inline(
    text: &RichText,
    base_color: Hsla,
    font_size: f32,
    font_weight: FontWeight,
    theme: &Theme,
) -> AnyElement {
    let cache = text.render_cache();
    let plain = cache.visible_text();

    let mut elements: Vec<AnyElement> = Vec::new();
    for span in cache.spans() {
        let segment = &plain[span.range.clone()];
        if segment.is_empty() {
            continue;
        }
        elements.push(render_preview_span(
            segment,
            span,
            base_color,
            font_size,
            font_weight,
            theme,
        ));
    }
    if elements.is_empty() {
        elements.push(div().into_any_element());
    }

    div()
        .flex()
        .flex_wrap()
        .children(elements)
        .into_any_element()
}

/// Renders a single styled inline span as a plain (non-interactive) element.
pub(crate) fn render_preview_span(
    text: &str,
    span: &InlineSpan,
    base_color: Hsla,
    font_size: f32,
    font_weight: FontWeight,
    theme: &Theme,
) -> AnyElement {
    let color = if span.link.is_some() || span.footnote.is_some() {
        theme.colors.text_link
    } else {
        base_color
    };

    let script_offset = match span.style.script {
        InlineScript::Normal => 0.0,
        InlineScript::Superscript => -font_size * 0.28,
        InlineScript::Subscript => font_size * 0.22,
    };
    let display_font_size = if span.style.has_script() {
        (font_size * 0.72).max(6.0)
    } else {
        font_size
    };

    let mut element = div()
        .min_w(px(0.0))
        .text_size(px(display_font_size))
        .line_height(rems(theme.typography.text_line_height))
        .text_color(color)
        .font_weight(if span.style.bold {
            FontWeight::BOLD
        } else {
            font_weight
        })
        .child(SharedString::from(text.to_string()));

    if script_offset != 0.0 {
        element = element.relative().top(px(script_offset));
    }

    if span.style.underline || span.link.is_some() || span.footnote.is_some() {
        element = element.underline();
    }
    if span.style.italic {
        element = element.italic();
    }
    if span.style.strikethrough {
        element = element.line_through();
    }
    if span.style.code {
        element = element
            .rounded(px(theme.dimensions.code_bg_radius))
            .px(px(theme.dimensions.code_bg_pad_x))
            .py(px(theme.dimensions.code_bg_pad_y))
            .bg(theme.colors.code_bg);
    }

    // Inline math renders as a small SVG replacing the math span.
    if let Some(math) = &span.math {
        let math_size = inline_math_font_size(font_size);
        if let Ok(rendered) = render_inline_math_svg(&math.body, color, math_size) {
            return div()
                .flex()
                .items_center()
                .h(px(math_size * 1.65))
                .child(
                    img(rendered.path.clone())
                        .max_h(px(math_size * 1.65))
                        .object_fit(ObjectFit::Contain),
                )
                .into_any_element();
        }
        // Fall back to rendering the raw LaTeX source as styled text.
        element = element.child(SharedString::from(math.source.clone()));
        return element.into_any_element();
    }

    element.into_any_element()
}
