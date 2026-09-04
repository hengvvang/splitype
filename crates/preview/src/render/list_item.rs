use gpui::*;

use crate::node::PreviewBlock;
use crate::render::inline;
use syntax_highlighter::graphics::markup::{numbered_list_marker, render_custom_bullet_marker};
use theme::Theme;

/// Renders a bulleted list item content line read-only.
pub(crate) fn render_preview_bulleted_list_item(
    block: &PreviewBlock,
    depth: usize,
    base: Div,
    theme: &Theme,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;
    let first_line_height = t.text_size * t.text_line_height;

    base.text_size(px(t.text_size))
        .text_color(c.text_default)
        .line_height(rems(t.text_line_height))
        .w_full()
        .flex()
        .flex_row()
        .items_start()
        .gap(px(d.list_marker_gap))
        .children([
            div()
                .min_w(px(d.list_marker_width))
                .h(px(first_line_height))
                .flex()
                .items_center()
                .justify_center()
                .child(render_custom_bullet_marker(depth, c.text_default)),
            div()
                .min_w(px(0.0))
                .flex_grow(1.0)
                .child(inline::render_preview_inline(
                    &block.data.text,
                    c.text_default,
                    t.text_size,
                    FontWeight::NORMAL,
                    theme,
                    &block.search_matches,
                )),
        ])
        .into_any_element()
}

/// Renders a task list item content line read-only.
pub(crate) fn render_preview_task_list_item(
    block: &PreviewBlock,
    checked: bool,
    base: Div,
    theme: &Theme,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;
    let marker_width = d.list_marker_width.max(d.task_checkbox_size);
    let first_line_height = t.text_size * t.text_line_height;
    let text_color = if checked {
        c.text_quote
    } else {
        c.text_default
    };

    base.text_size(px(t.text_size))
        .text_color(text_color)
        .line_height(rems(t.text_line_height))
        .w_full()
        .flex()
        .flex_row()
        .items_start()
        .gap(px(d.list_marker_gap))
        .children([
            div()
                .min_w(px(marker_width))
                .h(px(first_line_height))
                .flex()
                .items_center()
                .child(
                    svg()
                        .path(if checked {
                            "icons/editor/preview/checkbox-checked.svg"
                        } else {
                            "icons/editor/preview/checkbox.svg"
                        })
                        .size(px(d.task_checkbox_size))
                        .text_color(if checked {
                            c.task_checkbox_checked_bg
                        } else {
                            c.task_checkbox_border
                        }),
                ),
            div()
                .min_w(px(0.0))
                .flex_grow(1.0)
                .child(inline::render_preview_inline(
                    &block.data.text,
                    text_color,
                    t.text_size,
                    FontWeight::NORMAL,
                    theme,
                    &block.search_matches,
                )),
        ])
        .into_any_element()
}

/// Renders a numbered list item content line read-only.
pub(crate) fn render_preview_numbered_list_item(
    block: &PreviewBlock,
    depth: usize,
    base: Div,
    theme: &Theme,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;

    base.text_size(px(t.text_size))
        .text_color(c.text_default)
        .line_height(rems(t.text_line_height))
        .w_full()
        .flex()
        .flex_row()
        .items_start()
        .gap(px(d.list_marker_gap))
        .children([
            div()
                .min_w(px(d.ordered_list_marker_width))
                .child(SharedString::from(numbered_list_marker(
                    depth,
                    block.list_ordinal.unwrap_or(1),
                ))),
            div()
                .min_w(px(0.0))
                .flex_grow(1.0)
                .child(inline::render_preview_inline(
                    &block.data.text,
                    c.text_default,
                    t.text_size,
                    FontWeight::NORMAL,
                    theme,
                    &block.search_matches,
                )),
        ])
        .into_any_element()
}
