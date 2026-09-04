//! Image and media placeholders, loading states, and container width budgets.

use gpui::*;

use crate::model::block::Block;
use crate::render::layout::centered_column_width;
use crate::render::visible_quote_guides;
use markdown_parser::block::image::{ImageHandle, ImageResolvedSource};
use markdown_parser::parse::BlockKind;
use theme::{Theme, ThemeDimensions};

pub fn render_image_placeholder(runtime: &ImageHandle, width: Length, theme: &Theme) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;

    let target_name = if !runtime.src.trim().is_empty() {
        runtime.src.trim()
    } else if !runtime.alt.trim().is_empty() {
        runtime.alt.trim()
    } else {
        "unnamed"
    };

    let title_text = if !runtime.alt.trim().is_empty() {
        format!("Image Not Found: {}", runtime.alt.trim())
    } else {
        "Image Not Found".to_string()
    };

    div()
        .w_full()
        .max_w(width)
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(4.0))
        .px(px(d.block_padding_x))
        .py(px(24.0))
        .child(
            div()
                .text_size(px(t.text_size))
                .font_weight(FontWeight::MEDIUM)
                .text_color(c.image_placeholder_text)
                .child(SharedString::from(title_text)),
        )
        .child(
            div()
                .text_size(px(t.code_size))
                .text_color(c.dialog_muted)
                .child(SharedString::from(format!("({})", target_name))),
        )
        .into_any_element()
}

pub fn render_loading_placeholder(
    runtime: &ImageHandle,
    width: Length,
    theme: &Theme,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;

    let target_name = if !runtime.src.trim().is_empty() {
        runtime.src.trim()
    } else {
        "image"
    };

    div()
        .w_full()
        .max_w(width)
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(4.0))
        .px(px(d.block_padding_x))
        .py(px(24.0))
        .child(
            div()
                .text_size(px(t.text_size))
                .font_weight(FontWeight::MEDIUM)
                .text_color(c.image_placeholder_text)
                .child(SharedString::from("Loading image...")),
        )
        .child(
            div()
                .text_size(px(t.code_size))
                .text_color(c.dialog_muted)
                .child(SharedString::from(format!("({})", target_name))),
        )
        .into_any_element()
}

pub fn effective_table_width(block: &Block, viewport_width: f32, d: &ThemeDimensions) -> f32 {
    let centered_width = centered_column_width(viewport_width, d);
    let visible_quote_guides = visible_quote_guides(block);
    let quote_inset = d.quote_padding_left * visible_quote_guides as f32;
    let callout_inset = if block.callout_depth > 0 {
        d.callout_padding_x * 2.0 + d.callout_border_width
    } else {
        0.0
    };

    (centered_width
        - d.block_padding_x * 2.0
        - d.table_append_button_extent
        - quote_inset
        - callout_inset)
        .max((d.table_cell_padding_x * 2.0 + 80.0).max(120.0))
}

fn container_image_width_budget(block: &Block, viewport_width: f32, d: &ThemeDimensions) -> f32 {
    let centered_width = centered_column_width(viewport_width, d);
    let visible_quote_guides = visible_quote_guides(block);
    let quote_inset = d.quote_padding_left * visible_quote_guides as f32;
    let callout_inset = if block.callout_depth > 0 {
        d.callout_padding_x * 2.0 + d.callout_border_width
    } else {
        0.0
    };

    centered_width - quote_inset - callout_inset
}

pub fn effective_image_width(block: &Block, viewport_width: f32, d: &ThemeDimensions) -> f32 {
    let list_inset = d.nested_block_indent * block.render_depth as f32;
    (container_image_width_budget(block, viewport_width, d) - d.block_padding_x * 2.0 - list_inset)
        .max(160.0)
}

pub fn effective_list_item_image_width(
    block: &Block,
    viewport_width: f32,
    d: &ThemeDimensions,
) -> f32 {
    let marker_width = match block.kind() {
        BlockKind::BulletListItem => d.list_marker_width,
        BlockKind::TaskListItem { .. } => d.list_marker_width.max(d.task_checkbox_size),
        BlockKind::NumberedListItem => d.ordered_list_marker_width,
        _ => 0.0,
    };
    let list_inset = d.nested_block_indent * block.render_depth as f32;

    (container_image_width_budget(block, viewport_width, d)
        - d.block_padding_x * 2.0
        - list_inset
        - marker_width
        - d.list_marker_gap)
        .max(160.0)
}

impl Block {
    pub fn render_image_content(
        &self,
        runtime: &ImageHandle,
        max_width: Length,
        max_height: Pixels,
        theme: &Theme,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let source = runtime.resolved_source.clone();
        let placeholder_theme = theme.clone();
        let loading_theme = theme.clone();
        let runtime_for_fallback = runtime.clone();
        let runtime_for_loading = runtime.clone();

        // Missing local files render their placeholder immediately: no
        // async load is attempted, so no asset error is logged and no
        // doomed background task is spawned.
        let image: AnyElement = if !source.is_loadable() {
            render_image_placeholder(runtime, max_width, theme)
        } else {
            (match source {
                ImageResolvedSource::Local(path) => img(path),
                ImageResolvedSource::Remote(uri) => img(uri),
            })
            .max_w(max_width)
            .max_h(max_height)
            .object_fit(ObjectFit::Contain)
            .with_fallback(move || {
                render_image_placeholder(&runtime_for_fallback, max_width, &placeholder_theme)
            })
            .with_loading(move || {
                render_loading_placeholder(&runtime_for_loading, max_width, &loading_theme)
            })
            .into_any_element()
        };

        let mut container = div()
            .w_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(d.image_caption_gap))
            .child(image);

        if let Some(title) = runtime
            .title
            .as_ref()
            .filter(|title| !title.trim().is_empty())
        {
            container = container.child(
                div()
                    .w_full()
                    .text_center()
                    .text_size(px(t.code_size))
                    .text_color(c.image_caption_text)
                    .child(SharedString::from(title.clone())),
            );
        }

        container.into_any_element()
    }
}
