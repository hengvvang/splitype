//! Dynamic embedded previews for LaTeX math expressions and Mermaid diagrams.

use gpui::*;

use crate::model::block::Block;
use crate::render::media_placeholder::effective_image_width;
use markdown_parser::block::math::parse_display_math_source;
use markdown_parser::block::mermaid::parse_mermaid_fence_source;
use syntax_highlighter::graphics::{
    GraphicKind, render_empty_graphic_placeholder, render_graphic_error_card,
};
use syntax_highlighter::latex::{display_math_font_size, render_display_math_svg};
use syntax_highlighter::mermaid::{mermaid_content_fingerprint, render_mermaid_svg_for_display};
use theme::{Theme, ThemeDimensions};

impl Block {
    pub fn render_math_content(&self, theme: &Theme) -> (AnyElement, bool) {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let raw = self
            .data
            .raw_source
            .as_deref()
            .unwrap_or_else(|| self.display_text());

        let source = parse_display_math_source(raw).unwrap_or_else(|| {
            let body = raw
                .trim()
                .strip_prefix("$$")
                .unwrap_or(raw.trim())
                .strip_suffix("$$")
                .unwrap_or(raw.trim())
                .trim()
                .to_string();
            markdown_parser::block::math::DisplayMathSource {
                source: raw.to_string(),
                body,
            }
        });

        if source.body.is_empty() {
            return (
                render_empty_graphic_placeholder(GraphicKind::LatexMath, theme),
                false,
            );
        }

        match render_display_math_svg(&source, c.text_default, display_math_font_size(t.text_size))
        {
            Ok(rendered) => (
                div()
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
                true,
            ),
            Err(err) => (
                render_graphic_error_card(GraphicKind::LatexMath, &err.to_string(), theme),
                false,
            ),
        }
    }

    pub fn render_mermaid_content(&mut self, theme: &Theme, window: &Window) -> (AnyElement, bool) {
        let d = &theme.dimensions;
        let raw = self
            .data
            .raw_source
            .as_deref()
            .unwrap_or_else(|| self.display_text());

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
            return (
                render_empty_graphic_placeholder(GraphicKind::Mermaid, theme),
                false,
            );
        }

        let viewport_width = f32::from(window.viewport_size().width.max(px(1.0)));
        let available_width = effective_image_width(self, viewport_width, d);

        let fingerprint = mermaid_content_fingerprint(&source.body);
        let width_key = available_width.to_bits();
        let viewport_key = viewport_width.to_bits();
        let cached = self
            .mermaid_render_cache
            .as_ref()
            .filter(|(cached_fingerprint, cached_width, cached_viewport, _)| {
                *cached_fingerprint == fingerprint
                    && *cached_width == width_key
                    && *cached_viewport == viewport_key
            })
            .map(|(_, _, _, rendered)| rendered.clone());

        match cached {
            Some(rendered) => (
                Self::render_mermaid_svg_element(rendered, available_width, self, d),
                true,
            ),
            None => {
                match render_mermaid_svg_for_display(&source, available_width, viewport_width) {
                    Ok(rendered) => {
                        self.mermaid_render_cache =
                            Some((fingerprint, width_key, viewport_key, rendered.clone()));
                        (
                            Self::render_mermaid_svg_element(rendered, available_width, self, d),
                            true,
                        )
                    }
                    Err(err) => (
                        render_graphic_error_card(GraphicKind::Mermaid, &err.to_string(), theme),
                        false,
                    ),
                }
            }
        }
    }

    pub fn render_mermaid_svg_element(
        rendered: syntax_highlighter::mermaid::MermaidSvgRender,
        available_width: f32,
        block: &Block,
        d: &ThemeDimensions,
    ) -> AnyElement {
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
                    format!("mermaid-scroll-{}", block.data.id).into(),
                ))
                .w_full()
                .overflow_x_scroll()
                .scrollbar_width(px(0.0))
                .child(div().w(px(display_width)).child(image()))
                .into_any_element()
        };

        div()
            .w_full()
            .py(px(d.block_padding_y.max(6.0)))
            .child(content)
            .into_any_element()
    }
}
