//! Preview standalone image rendering — a lone image paragraph renders as a
//! self-contained image widget, mirroring the WYSIWYG image styles.

use gpui::*;

use crate::block::PreviewBlock;
use crate::render::{paragraph, preview_centered_column_width};
use markdown_parser::block::image::{ImageResolvedSource, parse_standalone_image};
use theme::Theme;

/// Renders a paragraph/list-item block that holds a lone image read-only.
/// Falls back to the paragraph renderer when the block is not an image.
pub(crate) fn render_preview_image(
    block: &PreviewBlock,
    base: Div,
    theme: &Theme,
    window: &Window,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;

    let plain = block.data.text.plain_text();
    let Some(syntax) = parse_standalone_image(&plain) else {
        return paragraph::render_preview_paragraph(block, base, theme);
    };
    let alt = syntax.alt.clone();
    let Some(handle) = block.image_handle_for_syntax(syntax) else {
        return render_preview_image_placeholder(&alt, &plain, base, theme);
    };

    let viewport_width = f32::from(window.viewport_size().width.max(px(1.0)));
    let max_width =
        (preview_centered_column_width(viewport_width, d) - d.block_padding_x * 2.0).max(160.0);

    let source = handle.resolved_source.clone();
    let image = match source {
        ImageResolvedSource::Local(path) => img(path),
        ImageResolvedSource::Remote(uri) => img(uri),
    }
    .max_w(px(max_width))
    .max_h(px(d.image_root_max_height))
    .object_fit(ObjectFit::Contain);

    let mut container = div()
        .w_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(d.image_caption_gap))
        .child(image);

    if let Some(title) = handle
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

    base.w_full()
        .p(relative(0.005))
        .flex()
        .items_center()
        .justify_center()
        .child(container)
        .into_any_element()
}

/// Read-only placeholder for an image that could not be resolved, mirroring
/// the WYSIWYG "Image Not Found" placeholder.
fn render_preview_image_placeholder(alt: &str, raw: &str, base: Div, theme: &Theme) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;

    let target_name = raw.trim();
    let title_text = if !alt.trim().is_empty() {
        format!("Image Not Found: {}", alt.trim())
    } else {
        "Image Not Found".to_string()
    };

    base.w_full()
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
