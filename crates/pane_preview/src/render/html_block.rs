//! Preview raw HTML block rendering — full semantic HTML document
//! rendering, mirroring the WYSIWYG HTML styles without any interactive
//! state (details always follows its `open` attribute, images render
//! directly, no hover or toggle handlers).

use gpui::*;

use crate::node::PreviewBlock;
use syntax_highlighter::render_helpers::{
    HtmlComputedStyle, HtmlNodeVisualStyle, html_children_text, html_node_visual_style,
};
use theme::Theme;
use markdown_parser::block::html::{
    HtmlNode, HtmlNodeKind, attr_value, parse_html_document, parse_html_image_block,
};
use markdown_parser::block::image::resolve_image_source;

/// Renders a raw HTML block read-only with the same visuals as the WYSIWYG
/// HTML document rendering.
pub(crate) fn render_preview_html_block(block: &PreviewBlock, base: Div, theme: &Theme) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;

    let raw_fallback;
    let raw = match block.data.raw_source.as_deref() {
        Some(s) => s,
        None => {
            raw_fallback = block.display_text();
            &raw_fallback
        }
    };
    let html = block.data.html.as_ref().cloned().unwrap_or_else(|| {
        parse_html_document(raw)
    });

    if !html.is_semantic() {
        return base
            .w_full()
            .rounded(px(d.code_block_radius))
            .bg(c.source_mode_block_bg)
            .px(px(d.block_padding_x))
            .py(px(d.block_padding_y))
            .text_size(px(t.code_size))
            .text_color(c.text_default)
            .child(SharedString::from(html.raw_source.clone()))
            .into_any_element();
    }

    base.w_full()
        .min_w(px(0.0))
        .flex()
        .flex_col()
        .gap(px(d.block_gap * 0.4))
        .children(
            html.nodes
                .iter()
                .map(|node| render_preview_html_node(node, theme, HtmlComputedStyle::root(theme))),
        )
        .into_any_element()
}

fn render_preview_html_node(
    node: &HtmlNode,
    theme: &Theme,
    inherited: HtmlComputedStyle,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;

    if node.kind == HtmlNodeKind::RawTextBlock {
        return div()
            .w_full()
            .rounded(px(d.code_block_radius))
            .bg(c.source_mode_block_bg)
            .px(px(d.block_padding_x * 0.6))
            .py(px(d.block_padding_y * 0.6))
            .text_size(px(inherited.font_size))
            .text_color(c.text_default)
            .child(SharedString::from(node.raw_source.clone()))
            .into_any_element();
    }

    if node.tag_name == "#text" {
        return div()
            .min_w(px(0.0))
            .text_size(px(inherited.font_size))
            .text_color(inherited.color)
            .child(SharedString::from(node.raw_source.clone()))
            .into_any_element();
    }

    let node_style = html_node_visual_style(node, inherited, theme);
    match node.tag_name.as_str() {
        "strong" | "b" => {
            render_preview_html_inline_container(node, theme, node_style, FontWeight::BOLD)
        }
        "em" | "i" | "span" | "abbr" | "dfn" | "time" | "u" | "ins" | "del" | "small" | "sup"
        | "sub" | "a" | "mark" => {
            render_preview_html_inline_container(node, theme, node_style, FontWeight::NORMAL)
        }
        "code" | "kbd" => {
            let mut element = div()
                .flex()
                .rounded(px(theme.dimensions.code_bg_radius))
                .px(px(4.0))
                .text_size(px(node_style.computed.font_size))
                .text_color(node_style.computed.color)
                .children(
                    node.children
                        .iter()
                        .map(|child| render_preview_html_node(child, theme, node_style.computed)),
                );
            if let Some(bg) = node_style.background {
                element = element.bg(bg);
            }
            element.into_any_element()
        }
        "q" => {
            let mut element = div()
                .flex()
                .text_size(px(node_style.computed.font_size))
                .text_color(node_style.computed.color)
                .children([
                    div().child("\u{201C}").into_any_element(),
                    div()
                        .children(node.children.iter().map(|child| {
                            render_preview_html_node(child, theme, node_style.computed)
                        }))
                        .into_any_element(),
                    div().child("\u{201D}").into_any_element(),
                ]);
            if let Some(bg) = node_style.background {
                element = element.bg(bg).rounded(px(theme.dimensions.code_bg_radius)).px(px(2.0));
            }
            element.into_any_element()
        }
        "br" => div().child("\n").into_any_element(),
        "hr" => div()
            .w_full()
            .h(px(d.separator_thickness))
            .my(px(d.separator_margin_y))
            .bg(c.separator)
            .rounded(px(theme::dimensions::FULL_CORNER_RADIUS))
            .into_any_element(),
        "blockquote" => {
            let mut element = div()
                .w_full()
                .pl(px(d.quote_padding_left))
                .border_l(px(d.quote_border_width))
                .border_color(c.border_quote)
                .text_size(px(node_style.computed.font_size))
                .text_color(node_style.computed.color)
                .children(
                    node.children
                        .iter()
                        .map(|child| render_preview_html_node(child, theme, node_style.computed)),
                );
            if let Some(bg) = node_style.background {
                element = element.bg(bg);
            }
            element.into_any_element()
        }
        "pre" => {
            let mut element = div()
                .w_full()
                .rounded(px(d.code_block_radius))
                .px(px(d.code_block_padding_x))
                .py(px(d.code_block_padding_y))
                .text_size(px(node_style.computed.font_size))
                .text_color(node_style.computed.color)
                .child(SharedString::from(html_children_text(node)));
            if let Some(bg) = node_style.background {
                element = element.bg(bg);
            }
            element.into_any_element()
        }
        "img" => render_preview_html_image(node, theme, node_style),
        "table" => {
            let mut element = div()
                .w_full()
                .border(px(1.0))
                .border_color(theme.colors.table_border)
                .text_size(px(node_style.computed.font_size))
                .text_color(node_style.computed.color)
                .children(
                    node.children
                        .iter()
                        .map(|child| render_preview_html_node(child, theme, node_style.computed)),
                );
            if let Some(bg) = node_style.background {
                element = element.bg(bg);
            }
            element.into_any_element()
        }
        "thead" | "tbody" | "tfoot" | "figure" => {
            let mut element = div()
                .w_full()
                .flex()
                .flex_col()
                .text_size(px(node_style.computed.font_size))
                .text_color(node_style.computed.color)
                .children(
                    node.children
                        .iter()
                        .map(|child| render_preview_html_node(child, theme, node_style.computed)),
                );
            if let Some(bg) = node_style.background {
                element = element.bg(bg);
            }
            element.into_any_element()
        }
        "tr" => {
            let mut element = div()
                .w_full()
                .flex()
                .text_size(px(node_style.computed.font_size))
                .text_color(node_style.computed.color)
                .children(
                    node.children
                        .iter()
                        .map(|child| render_preview_html_node(child, theme, node_style.computed)),
                );
            if let Some(bg) = node_style.background {
                element = element.bg(bg);
            }
            element.into_any_element()
        }
        "th" | "td" => {
            let mut element = div()
                .min_w(px(0.0))
                .flex_grow(1.0)
                .border(px(1.0))
                .border_color(c.table_border)
                .px(px(d.table_cell_padding_x))
                .py(px(d.table_cell_padding_y))
                .text_size(px(node_style.computed.font_size))
                .text_color(node_style.computed.color)
                .font_weight(if node.tag_name == "th" {
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::NORMAL
                })
                .children(
                    node.children
                        .iter()
                        .map(|child| render_preview_html_node(child, theme, node_style.computed)),
                );
            if let Some(bg) = node_style.background {
                element = element.bg(bg);
            }
            element.into_any_element()
        }
        "details" => render_preview_html_details(node, theme, node_style),
        "summary" => {
            let mut element = div()
                .w_full()
                .font_weight(FontWeight::SEMIBOLD)
                .text_size(px(node_style.computed.font_size))
                .text_color(node_style.computed.color)
                .children(
                    node.children
                        .iter()
                        .map(|child| render_preview_html_node(child, theme, node_style.computed)),
                );
            if let Some(bg) = node_style.background {
                element = element.bg(bg);
            }
            element.into_any_element()
        }
        "figcaption" => {
            let mut element = div()
                .w_full()
                .text_center()
                .text_size(px(node_style.computed.font_size))
                .text_color(node_style.computed.color)
                .children(
                    node.children
                        .iter()
                        .map(|child| render_preview_html_node(child, theme, node_style.computed)),
                );
            if let Some(bg) = node_style.background {
                element = element.bg(bg);
            }
            element.into_any_element()
        }
        _ => {
            let mut element = div()
                .w_full()
                .text_size(px(node_style.computed.font_size))
                .text_color(node_style.computed.color)
                .children(
                    node.children
                        .iter()
                        .map(|child| render_preview_html_node(child, theme, node_style.computed)),
                );
            if let Some(bg) = node_style.background {
                element = element.bg(bg);
            }
            element.into_any_element()
        }
    }
}

fn render_preview_html_inline_container(
    node: &HtmlNode,
    theme: &Theme,
    node_style: HtmlNodeVisualStyle,
    weight: FontWeight,
) -> AnyElement {
    let mut element = div()
        .flex()
        .min_w(px(0.0))
        .text_size(px(node_style.computed.font_size))
        .text_color(node_style.computed.color)
        .font_weight(weight)
        .children(
            node.children
                .iter()
                .map(|child| render_preview_html_node(child, theme, node_style.computed)),
        );
    if let Some(bg) = node_style.background {
        element = element.bg(bg).rounded(px(theme.dimensions.code_bg_radius)).px(px(2.0));
    }
    match node.tag_name.as_str() {
        "sup" => {
            element = element
                .relative()
                .top(px(-node_style.computed.font_size * 0.28))
        }
        "sub" => {
            element = element
                .relative()
                .top(px(node_style.computed.font_size * 0.22))
        }
        _ => {}
    }
    element.into_any_element()
}

fn render_preview_html_image(
    node: &HtmlNode,
    theme: &Theme,
    node_style: HtmlNodeVisualStyle,
) -> AnyElement {
    let parsed_image = parse_html_image_block(&node.raw_source);
    let src = parsed_image
        .as_ref()
        .map(|image| image.src.as_str())
        .or_else(|| attr_value(node, "src"))
        .filter(|src| !src.trim().is_empty());

    let Some(src) = src else {
        let mut element = div()
            .text_size(px(node_style.computed.font_size))
            .text_color(node_style.computed.color)
            .child(SharedString::from(node.raw_source.clone()));
        if let Some(bg) = node_style.background {
            element = element.bg(bg);
        }
        return element.into_any_element();
    };

    let zoom = parsed_image
        .as_ref()
        .map(|image| image.zoom_factor())
        .unwrap_or(1.0);

    let image = match resolve_image_source(src, None) {
        markdown_parser::block::image::ImageResolvedSource::Local(path) => img(path),
        markdown_parser::block::image::ImageResolvedSource::Remote(uri) => img(uri),
    }
    .max_w(Length::Definite(relative(zoom)))
    .max_h(px(theme.dimensions.image_root_max_height * zoom))
    .object_fit(ObjectFit::Contain);

    if let Some(bg) = node_style.background {
        div().w_full().bg(bg).child(image).into_any_element()
    } else {
        image.into_any_element()
    }
}

fn render_preview_html_details(
    node: &HtmlNode,
    theme: &Theme,
    node_style: HtmlNodeVisualStyle,
) -> AnyElement {
    let is_open = attr_value(node, "open").is_some();
    let summary = node
        .children
        .iter()
        .find(|child| child.tag_name == "summary");
    let body = node
        .children
        .iter()
        .filter(|child| child.tag_name != "summary");

    let mut container =
        div()
            .w_full()
            .rounded(px(theme.dimensions.code_block_radius))
            .border(px(1.0))
            .border_color(theme.colors.table_border)
            .px(px(theme.dimensions.block_padding_x))
            .py(px(theme.dimensions.block_padding_y))
            .text_size(px(node_style.computed.font_size))
            .text_color(node_style.computed.color)
            .child(
                div()
                    .w_full()
                    .flex()
                    .gap(px(theme.dimensions.list_marker_gap))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(if is_open { "\u{25BE}" } else { "\u{25B8}" })
                    .children(summary.into_iter().map(|summary| {
                        render_preview_html_node(summary, theme, node_style.computed)
                    })),
            );
    if let Some(bg) = node_style.background {
        container = container.bg(bg);
    }

    if is_open {
        container = container.child(
            div()
                .w_full()
                .pt(px(theme.dimensions.block_padding_y))
                .children(
                    body.map(|child| render_preview_html_node(child, theme, node_style.computed)),
                ),
        );
    }

    container.into_any_element()
}


