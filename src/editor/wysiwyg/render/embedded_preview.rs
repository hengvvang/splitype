//! Dynamic embedded previews for LaTeX math expressions and Mermaid diagrams.

use gpui::*;

use crate::editor::render::latex_render::{display_math_font_size, render_display_math_svg};
use crate::editor::render::mermaid_render::{
    mermaid_content_fingerprint, render_mermaid_svg_for_display,
};
use crate::editor::tree::block::Block;
use crate::editor::wysiwyg::render::media_placeholder::effective_image_width;
use crate::infra::theme::{Theme, ThemeDimensions};
use crate::model::block::math::parse_display_math_source;
use crate::model::block::mermaid::parse_mermaid_fence_source;

impl Block {
    pub(crate) fn render_math_content(&self, theme: &Theme) -> AnyElement {
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
            crate::model::block::math::DisplayMathSource {
                source: raw.to_string(),
                body,
            }
        });

        if source.body.is_empty() {
            return div()
                .w_full()
                .text_size(px(t.text_size))
                .line_height(rems(t.text_line_height))
                .text_color(c.text_default)
                .child(SharedString::from(raw.to_string()))
                .into_any_element();
        }

        match render_display_math_svg(&source, c.text_default, display_math_font_size(t.text_size))
        {
            Ok(rendered) => div()
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
            Err(err) => div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .rounded(px(d.code_block_radius))
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

    pub(crate) fn render_mermaid_content(&mut self, theme: &Theme, window: &Window) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
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
            crate::model::block::mermaid::MermaidSource {
                source: raw.to_string(),
                body,
                info: "mermaid".to_string(),
            }
        });

        if source.body.is_empty() {
            return div()
                .w_full()
                .text_size(px(t.text_size))
                .line_height(rems(t.text_line_height))
                .text_color(c.text_default)
                .child(SharedString::from(raw.to_string()))
                .into_any_element();
        }

        let viewport_width = f32::from(window.viewport_size().width.max(px(1.0)));
        let available_width = effective_image_width(self, viewport_width, d);

        // The display render is content-addressed, so the per-block cache
        // stays valid until the diagram source or the layout parameters
        // change; cursor-blink repaints then skip the SVG file reads.
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
            Some(rendered) => Self::render_mermaid_svg_element(rendered, available_width, self, d),
            None => {
                match render_mermaid_svg_for_display(&source, available_width, viewport_width) {
                    Ok(rendered) => {
                        self.mermaid_render_cache =
                            Some((fingerprint, width_key, viewport_key, rendered.clone()));
                        Self::render_mermaid_svg_element(rendered, available_width, self, d)
                    }
                    Err(err) => div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .rounded(px(d.code_block_radius))
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
        }
    }

    pub(crate) fn render_mermaid_svg_element(
        rendered: crate::editor::render::mermaid_render::MermaidSvgRender,
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
