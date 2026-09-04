//! Read-only inline rendering for the preview panel.
//!
//! Walks a [`BlockText`] render cache and produces plain styled text runs —
//! bold, italic, underline, strikethrough, script, inline code, links,
//! footnotes, and inline math — with no caret, selection, or projection
//! logic. Styles mirror the WYSIWYG inline rendering.

use gpui::*;

use markdown_parser::inline::render_cache::InlineSpan;
use markdown_parser::inline::style::InlineScript;
use markdown_parser::inline::text::BlockText;
use syntax_highlighter::graphics::latex::{inline_math_font_size, render_inline_math_svg};
use theme::Theme;

use std::ops::Range;

#[inline]
fn safe_str_slice(s: &str, start: usize, end: usize) -> &str {
    if start >= end || start >= s.len() {
        return "";
    }
    let end = end.min(s.len());
    let mut s_idx = start;
    while s_idx > 0 && !s.is_char_boundary(s_idx) {
        s_idx -= 1;
    }
    let mut e_idx = end;
    while e_idx > s_idx && !s.is_char_boundary(e_idx) {
        e_idx -= 1;
    }
    if s_idx >= e_idx {
        return "";
    }
    &s[s_idx..e_idx]
}

/// Renders the inline content of `text` with `base_color` and `font_size`,
/// mirroring the WYSIWYG inline segment styling without any editing state.
pub(crate) fn render_preview_inline(
    text: &BlockText,
    base_color: Hsla,
    font_size: f32,
    font_weight: FontWeight,
    theme: &Theme,
    search_matches: &[(Range<usize>, bool)],
) -> AnyElement {
    let cache = text.render_cache();
    let plain = cache.text();

    let mut elements: Vec<AnyElement> = Vec::new();
    for span in cache.spans() {
        let span_start = span.range.start;
        let span_end = span.range.end;
        if span_start >= span_end || span_start >= plain.len() {
            continue;
        }

        let mut relevant_highlights: Vec<(Range<usize>, Hsla, Option<Hsla>)> = Vec::new();

        for (m_range, is_active) in search_matches {
            let overlap_start = m_range.start.max(span_start);
            let overlap_end = m_range.end.min(span_end);
            if overlap_start < overlap_end {
                let bg_color = if *is_active {
                    theme.colors.app_menu_active.opacity(0.65)
                } else {
                    theme.colors.app_menu_active.opacity(0.25)
                };
                let border = if *is_active {
                    Some(theme.colors.app_menu_active)
                } else {
                    None
                };
                relevant_highlights.push((overlap_start..overlap_end, bg_color, border));
            }
        }

        if relevant_highlights.is_empty() {
            let segment = safe_str_slice(plain, span_start, span_end);
            if !segment.is_empty() {
                elements.push(render_preview_span(
                    segment,
                    span,
                    base_color,
                    font_size,
                    font_weight,
                    theme,
                    None,
                ));
            }
        } else {
            let mut curr = span_start;
            for (range, bg_color, _border) in relevant_highlights {
                if curr < range.start {
                    let seg = safe_str_slice(plain, curr, range.start);
                    if !seg.is_empty() {
                        elements.push(render_preview_span(
                            seg,
                            span,
                            base_color,
                            font_size,
                            font_weight,
                            theme,
                            None,
                        ));
                    }
                }
                let seg = safe_str_slice(plain, range.start, range.end);
                if !seg.is_empty() {
                    let span_el = render_preview_span(
                        seg,
                        span,
                        base_color,
                        font_size,
                        font_weight,
                        theme,
                        Some(bg_color),
                    );
                    elements.push(span_el);
                }
                curr = range.end;
            }
            if curr < span_end {
                let seg = safe_str_slice(plain, curr, span_end);
                if !seg.is_empty() {
                    elements.push(render_preview_span(
                        seg,
                        span,
                        base_color,
                        font_size,
                        font_weight,
                        theme,
                        None,
                    ));
                }
            }
        }
    }

    div()
        .w_full()
        .min_w(px(0.0))
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(0.0))
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
    bg_override: Option<Hsla>,
) -> AnyElement {
    let mut color = if span.link.is_some() {
        theme.colors.text_link
    } else if span.footnote.is_some() {
        theme.colors.footnote_backref
    } else {
        base_color
    };
    if let Some(style) = span.html_style
        && let Some(html_color) = style.color
    {
        color = syntax_highlighter::graphics::markup::html_css_color_to_hsla(html_color, color);
    }

    let display_font_size = if span.style.has_script() || span.footnote.is_some() {
        (font_size * 0.70).max(6.0)
    } else {
        font_size
    };
    let script_offset = match span.style.script {
        InlineScript::Normal => 0.0,
        InlineScript::Superscript => -font_size * 0.20,
        InlineScript::Subscript => font_size * 0.16,
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

    if span.style.underline || span.link.is_some() {
        element = element.underline();
    }
    if span.style.italic {
        element = element.italic();
    }
    if span.style.strikethrough {
        element = element.line_through();
    }

    if let Some(bg) = bg_override {
        element = element.bg(bg);
    } else if span.style.code {
        element = element
            .font(theme::TypographyStore::default_font(
                theme::TypographyScope::Code,
            ))
            .rounded(px(theme.dimensions.code_bg_radius))
            .px(px(theme.dimensions.code_bg_pad_x))
            .py(px(theme.dimensions.code_bg_pad_y))
            .bg(theme.colors.code_bg);
    } else if span.style.highlight {
        element = element
            .rounded(px(theme.dimensions.code_bg_radius))
            .px(px(2.0))
            .bg(theme.colors.text_highlight_bg);
    } else if let Some(style) = span.html_style
        && let Some(bg_color) = style.background_color
    {
        element = element
            .bg(syntax_highlighter::graphics::markup::html_css_color_to_hsla(bg_color, color));
    }

    if let Some(link) = &span.link {
        let open_target = link.open_target.clone();
        element = element
            .cursor_pointer()
            .hover(|style| style.underline().text_color(theme.colors.text_link))
            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                cx.stop_propagation();
            })
            .on_mouse_up(MouseButton::Left, move |_event, _window, cx| {
                if !open_target.is_empty() {
                    cx.open_url(&open_target);
                }
                cx.stop_propagation();
            });
    }

    if let Some(footnote) = &span.footnote {
        let _footnote_id = footnote.id.clone();
        element = element
            .cursor_pointer()
            .hover(|style| style.underline().text_color(theme.colors.text_link))
            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                cx.stop_propagation();
            })
            .on_mouse_up(MouseButton::Left, move |_event, _window, cx| {
                cx.stop_propagation();
            });
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
