//! List item block rendering (unordered, ordered, task list).
// Migrated from blocks/render.rs

use gpui::*;

use crate::editor::block::Block;
use crate::ui::blocks::render::{
    effective_list_item_image_width, numbered_list_marker, render_custom_bullet_marker,
};
use crate::ui::theme::Theme;

/// Render an unordered (bulleted) list item.
pub(crate) fn render_bulleted_list_item(
    block: &mut Block,
    focused: bool,
    is_placeholder: bool,
    showing_rendered_image: bool,
    focused_base: Stateful<Div>,
    theme: &Theme,
    window: &mut Window,
    cx: &mut Context<Block>,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;
    let first_line_height = t.text_size * t.text_line_height;

    focused_base
        .text_size(px(t.text_size))
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
                .child(render_custom_bullet_marker(
                    block.render_depth,
                    c.text_default,
                )),
            if showing_rendered_image {
                let viewport_width = f32::from(window.viewport_size().width.max(px(1.0)));
                let max_width = px(effective_list_item_image_width(block, viewport_width, d));
                if let Some(runtime) = block.image_runtime() {
                    div().flex_grow().child(block.render_image_content(
                        runtime,
                        max_width.into(),
                        px(d.image_root_max_height),
                        px(d.image_root_placeholder_height),
                        theme,
                        &strings_from_context(cx),
                    ))
                } else {
                    div().min_w(px(0.0)).flex_grow().child(
                        block.render_text_or_mixed_inline_visuals(
                            theme,
                            focused,
                            is_placeholder,
                            None,
                            None,
                            c.text_default,
                            t.text_size,
                            FontWeight::NORMAL,
                            cx,
                        ),
                    )
                }
            } else {
                div()
                    .min_w(px(0.0))
                    .flex_grow()
                    .child(block.render_text_or_mixed_inline_visuals(
                        theme,
                        focused,
                        is_placeholder,
                        None,
                        None,
                        c.text_default,
                        t.text_size,
                        FontWeight::NORMAL,
                        cx,
                    ))
            },
        ])
        .into_any_element()
}

/// Render a task list item (checkbox).
pub(crate) fn render_task_list_item(
    block: &mut Block,
    checked: bool,
    focused: bool,
    is_placeholder: bool,
    showing_rendered_image: bool,
    focused_base: Stateful<Div>,
    theme: &Theme,
    window: &mut Window,
    cx: &mut Context<Block>,
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

    focused_base
        .text_size(px(t.text_size))
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
                    div()
                        .size(px(d.task_checkbox_size))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(d.task_checkbox_radius))
                        .border(px(d.task_checkbox_border_width))
                        .border_color(if checked {
                            c.task_checkbox_checked_bg
                        } else {
                            c.task_checkbox_border
                        })
                        .bg(if checked {
                            c.task_checkbox_checked_bg
                        } else {
                            c.task_checkbox_bg
                        })
                        .cursor_pointer()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(Block::on_task_checkbox_mouse_down),
                        )
                        .on_mouse_up(
                            MouseButton::Left,
                            cx.listener(Block::on_task_checkbox_mouse_up),
                        )
                        .children(if checked {
                            Some(
                                svg()
                                    .path("icon/task_check.svg")
                                    .size(px(d.task_checkbox_check_size))
                                    .text_color(c.task_checkbox_check),
                            )
                        } else {
                            None
                        }),
                ),
            if showing_rendered_image {
                let viewport_width = f32::from(window.viewport_size().width.max(px(1.0)));
                let max_width = px(effective_list_item_image_width(block, viewport_width, d));
                if let Some(runtime) = block.image_runtime() {
                    div().flex_grow().child(block.render_image_content(
                        runtime,
                        max_width.into(),
                        px(d.image_root_max_height),
                        px(d.image_root_placeholder_height),
                        theme,
                        &strings_from_context(cx),
                    ))
                } else {
                    div().min_w(px(0.0)).flex_grow().child(
                        block.render_text_or_mixed_inline_visuals(
                            theme,
                            focused,
                            is_placeholder,
                            None,
                            None,
                            text_color,
                            t.text_size,
                            FontWeight::NORMAL,
                            cx,
                        ),
                    )
                }
            } else {
                div()
                    .min_w(px(0.0))
                    .flex_grow()
                    .child(block.render_text_or_mixed_inline_visuals(
                        theme,
                        focused,
                        is_placeholder,
                        None,
                        None,
                        text_color,
                        t.text_size,
                        FontWeight::NORMAL,
                        cx,
                    ))
            },
        ])
        .into_any_element()
}

/// Render a numbered (ordered) list item.
pub(crate) fn render_numbered_list_item(
    block: &mut Block,
    focused: bool,
    is_placeholder: bool,
    showing_rendered_image: bool,
    focused_base: Stateful<Div>,
    theme: &Theme,
    window: &mut Window,
    cx: &mut Context<Block>,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;

    focused_base
        .text_size(px(t.text_size))
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
                    block.render_depth,
                    block.list_ordinal.unwrap_or(1),
                ))),
            if showing_rendered_image {
                let viewport_width = f32::from(window.viewport_size().width.max(px(1.0)));
                let max_width = px(effective_list_item_image_width(block, viewport_width, d));
                if let Some(runtime) = block.image_runtime() {
                    div().flex_grow().child(block.render_image_content(
                        runtime,
                        max_width.into(),
                        px(d.image_root_max_height),
                        px(d.image_root_placeholder_height),
                        theme,
                        &strings_from_context(cx),
                    ))
                } else {
                    div().min_w(px(0.0)).flex_grow().child(
                        block.render_text_or_mixed_inline_visuals(
                            theme,
                            focused,
                            is_placeholder,
                            None,
                            None,
                            c.text_default,
                            t.text_size,
                            FontWeight::NORMAL,
                            cx,
                        ),
                    )
                }
            } else {
                div()
                    .min_w(px(0.0))
                    .flex_grow()
                    .child(block.render_text_or_mixed_inline_visuals(
                        theme,
                        focused,
                        is_placeholder,
                        None,
                        None,
                        c.text_default,
                        t.text_size,
                        FontWeight::NORMAL,
                        cx,
                    ))
            },
        ])
        .into_any_element()
}

/// Helper to get I18n strings from the global context.
fn strings_from_context(cx: &mut Context<Block>) -> crate::services::i18n::I18nStrings {
    cx.global::<crate::services::i18n::I18nManager>()
        .strings_arc()
        .as_ref()
        .clone()
}
