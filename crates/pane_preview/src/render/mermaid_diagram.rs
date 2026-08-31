//! Preview Mermaid diagram block rendering — centered SVG with horizontal
//! scroll fallback, sized from the live viewport like the WYSIWYG panel.

use gpui::*;

use crate::node::PreviewBlock;
use crate::render::preview_centered_column_width;
use syntax_highlighter::mermaid::render_mermaid_svg_for_display;
use theme::Theme;
use markdown_parser::block::mermaid::parse_mermaid_fence_source;

/// Renders a Mermaid diagram block read-only.
pub(crate) fn render_preview_mermaid_diagram(
    block: &PreviewBlock,
    base: Div,
    theme: &Theme,
    window: &Window,
) -> AnyElement {
    let d = &theme.dimensions;
    let raw_fallback;
    let raw = match block.data.raw_source.as_deref() {
        Some(s) => s,
        None => {
            raw_fallback = block.display_text();
            &raw_fallback
        }
    };

    let source = parse_mermaid_fence_source(raw).unwrap_or_else(|| {
        let trimmed = raw.trim();
        let body = if let Some(rest) = trimmed
            .strip_prefix("```mermaid")
            .or_else(|| trimmed.strip_prefix("```"))
        {
            rest.strip_suffix("```").unwrap_or(rest).trim().to_string()
        } else {
            trimmed.to_string()
        };
        markdown_parser::block::mermaid::MermaidSource {
            source: raw.to_string(),
            body,
            info: "mermaid".to_string(),
        }
    });

    if source.body.is_empty() {
        return base
            .w_full()
            .child(syntax_highlighter::graphics::render_graphic_preview_box(
                syntax_highlighter::graphics::render_empty_graphic_placeholder(
                    syntax_highlighter::graphics::GraphicKind::Mermaid,
                    theme,
                ),
                theme,
            ))
            .into_any_element();
    }

    let viewport_width = f32::from(window.viewport_size().width.max(px(1.0)));
    let available_width =
        (preview_centered_column_width(viewport_width, d) - d.block_padding_x * 2.0).max(160.0);

    match render_mermaid_svg_for_display(&source, available_width, viewport_width) {
        Ok(rendered) => {
            let display_width = rendered.display_width.max(1.0);
            let display_height = rendered.display_height.max(1.0);
            let image_path = rendered.path.clone();
            let image = move || {
                img(image_path.clone())
                    .w(px(display_width))
                    .h(px(display_height))
            };
            let content = if display_width <= available_width + 0.5 {
                div()
                    .w_full()
                    .flex()
                    .justify_center()
                    .child(image())
                    .into_any_element()
            } else {
                div()
                    .id(ElementId::Name(
                        format!("preview-mermaid-{}", block.data.id).into(),
                    ))
                    .w_full()
                    .overflow_x_scroll()
                    .scrollbar_width(px(0.0))
                    .child(div().w(px(display_width)).child(image()))
                    .into_any_element()
            };

            base.w_full()
                .py(px(d.block_padding_y.max(6.0)))
                .child(content)
                .into_any_element()
        }
        Err(err) => base
            .w_full()
            .child(syntax_highlighter::graphics::render_graphic_preview_box(
                syntax_highlighter::graphics::render_graphic_error_card(
                    syntax_highlighter::graphics::GraphicKind::Mermaid,
                    &err.to_string(),
                    raw,
                    theme,
                ),
                theme,
            ))
            .into_any_element(),
    }
}


