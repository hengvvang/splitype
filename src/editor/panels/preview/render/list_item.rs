//! Preview list item rendering — bullet, task, and numbered markers with
//! the same visuals as the WYSIWYG list items. Marker helpers are
//! re-implemented here so preview styling can diverge independently.

use gpui::*;

use crate::editor::panels::preview::render::inline;
use crate::editor::tree::block::Block;
use crate::infra::theme::Theme;

/// Bullet marker shapes by nesting depth (solid disc, hollow disc, square).
fn bullet_marker(depth: usize, color: Hsla) -> AnyElement {
    match depth % 3 {
        0 => div()
            .size(px(5.5))
            .rounded_full()
            .bg(color)
            .into_any_element(),
        1 => div()
            .size(px(5.5))
            .rounded_full()
            .border_1()
            .border_color(color)
            .into_any_element(),
        _ => div()
            .size(px(4.5))
            .rounded(px(0.5))
            .bg(color)
            .into_any_element(),
    }
}

/// List ordinal: numbers at depth 0, lowercase letters at depth 1, roman
/// numerals at depth 2+.
fn numbered_marker(depth: usize, ordinal: usize) -> String {
    match depth {
        0 => format!("{ordinal}."),
        1 => format!("{}.", alphabetic_marker(ordinal)),
        _ => format!("{}.", roman_marker(ordinal)),
    }
}

fn alphabetic_marker(ordinal: usize) -> String {
    let mut result = String::new();
    let mut value = ordinal;
    loop {
        let remainder = (value - 1) % 26;
        result.insert(0, (b'a' + remainder as u8) as char);
        if value <= 26 {
            break;
        }
        value = (value - 1) / 26;
    }
    result
}

fn roman_marker(ordinal: usize) -> String {
    const TOKENS: &[(usize, &str)] = &[
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    let mut value = ordinal;
    let mut out = String::new();
    for (amount, token) in TOKENS {
        while value >= *amount {
            out.push_str(token);
            value -= amount;
        }
    }
    out
}

/// Renders a bulleted list item content line read-only.
pub(crate) fn render_preview_bulleted_list_item(
    block: &Block,
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
                .child(bullet_marker(depth, c.text_default)),
            div()
                .min_w(px(0.0))
                .flex_grow()
                .child(inline::render_preview_inline(
                    &block.record.text,
                    c.text_default,
                    t.text_size,
                    FontWeight::NORMAL,
                    theme,
                )),
        ])
        .into_any_element()
}

/// Renders a task list item content line read-only.
pub(crate) fn render_preview_task_list_item(
    block: &Block,
    checked: bool,
    depth: usize,
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
    let _ = depth;

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
                .flex_grow()
                .child(inline::render_preview_inline(
                    &block.record.text,
                    text_color,
                    t.text_size,
                    FontWeight::NORMAL,
                    theme,
                )),
        ])
        .into_any_element()
}

/// Renders a numbered list item content line read-only.
pub(crate) fn render_preview_numbered_list_item(
    block: &Block,
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
                .child(SharedString::from(numbered_marker(
                    depth,
                    block.list_ordinal.unwrap_or(1),
                ))),
            div()
                .min_w(px(0.0))
                .flex_grow()
                .child(inline::render_preview_inline(
                    &block.record.text,
                    c.text_default,
                    t.text_size,
                    FontWeight::NORMAL,
                    theme,
                )),
        ])
        .into_any_element()
}
