//! Preview Mermaid diagram block rendering — centered SVG with horizontal
//! scroll fallback, sized from the live viewport like the WYSIWYG panel.

use gpui::*;

use crate::editor::panels::preview::render::preview_centered_column_width;
use crate::editor::render::mermaid_render::render_mermaid_svg_for_display;
use crate::editor::tree::block::Block;
use crate::infra::theme::Theme;
use crate::model::syntax::mermaid::parse_mermaid_fence_source;

/// Renders a Mermaid diagram block read-only.
pub(crate) fn render_preview_mermaid_diagram(
    block: &Block,
    base: Div,
    theme: &Theme,
    window: &Window,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;
    let raw = block
        .record
        .raw_source
        .as_deref()
        .unwrap_or_else(|| block.display_text());

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
        crate::model::syntax::mermaid::MermaidSource {
            raw: raw.to_string(),
            body,
            info: "mermaid".to_string(),
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
                        format!("preview-mermaid-{}", block.record.id).into(),
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
                    .child(SharedString::from(format!("Mermaid render error: {err}"))),
            )
            .into_any_element(),
    }
}
