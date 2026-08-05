//! Preview raw HTML block rendering — full semantic HTML document
//! rendering, mirroring the WYSIWYG HTML styles without any interactive
//! state (details always follows its `open` attribute, images render
//! directly, no hover or toggle handlers).

use gpui::*;

use crate::editor::tree::block::Block;
use crate::model::syntax::html::{
    HtmlCssColor, HtmlNode, HtmlNodeKind, attr_value, parse_html_document, parse_html_image_block,
    style_for_node,
};
use crate::model::syntax::image::resolve_image_source;
use crate::theme::Theme;

/// Renders a raw HTML block read-only with the same visuals as the WYSIWYG
/// HTML document rendering.
pub(crate) fn render_preview_html_block(
    block: &Block,
    base: Div,
    theme: &Theme,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;

    let html = block.record.html.as_ref().cloned().unwrap_or_else(|| {
        parse_html_document(
            block
                .record
                .raw_source
                .as_deref()
                .unwrap_or_else(|| block.display_text()),
        )
    });

    if !html.is_semantic() {
        return base
            .w_full()
            .rounded_sm()
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

#[derive(Clone, Copy, Debug)]
struct HtmlComputedStyle {
    color: Hsla,
    font_size: f32,
    root_font_size: f32,
}

#[derive(Clone, Copy, Debug)]
struct HtmlNodeVisualStyle {
    computed: HtmlComputedStyle,
    background: Option<Hsla>,
}

impl HtmlComputedStyle {
    fn root(theme: &Theme) -> Self {
        Self {
            color: theme.colors.text_default,
            font_size: theme.typography.text_size,
            root_font_size: theme.typography.text_size,
        }
    }
}

fn html_css_color_to_hsla(color: HtmlCssColor, current_color: Hsla) -> Hsla {
    match color {
        HtmlCssColor::CurrentColor => current_color,
        HtmlCssColor::Rgba(color) => Hsla::from(Rgba {
            r: color.red as f32 / 255.0,
            g: color.green as f32 / 255.0,
            b: color.blue as f32 / 255.0,
            a: color.alpha.clamp(0.0, 1.0),
        }),
    }
}

fn html_node_visual_style(
    node: &HtmlNode,
    parent: HtmlComputedStyle,
    theme: &Theme,
) -> HtmlNodeVisualStyle {
    let c = &theme.colors;
    let t = &theme.typography;
    let mut computed = parent;
    let mut background = None;

    match node.tag_name.as_str() {
        "a" => computed.color = c.text_link,
        "blockquote" => computed.color = c.text_quote,
        "code" | "kbd" | "pre" => {
            computed.color = c.code_text;
            computed.font_size = t.code_size;
            background = Some(c.code_bg);
        }
        "mark" => background = Some(c.comment_bg),
        "figcaption" => {
            computed.color = c.image_caption_text;
            computed.font_size = t.code_size;
        }
        "small" | "sup" | "sub" => computed.font_size = (computed.font_size * 0.8).max(6.0),
        "th" => background = Some(c.table_header_bg),
        "td" => background = Some(c.table_cell_bg),
        _ => {}
    }

    let inline_style = style_for_node(node);
    if let Some(color) = inline_style.color {
        computed.color = html_css_color_to_hsla(color, computed.color);
    }
    if let Some(font_size) = inline_style.font_size {
        computed.font_size = font_size.resolve(computed.font_size, computed.root_font_size);
    }
    if let Some(color) = inline_style.background_color {
        background = Some(html_css_color_to_hsla(color, computed.color));
    }

    HtmlNodeVisualStyle {
        computed,
        background,
    }
}

fn html_children_text(node: &HtmlNode) -> String {
    if node.children.is_empty() {
        return node.raw_source.clone();
    }

    let mut text = String::new();
    for child in &node.children {
        if child.tag_name == "br" {
            text.push('\n');
        } else {
            text.push_str(&html_children_text(child));
        }
    }
    text
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
            .rounded_sm()
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
        "em" | "i" | "span" | "abbr" | "dfn" | "time" | "u" | "ins" | "del" | "small"
        | "sup" | "sub" | "a" | "mark" => {
            render_preview_html_inline_container(node, theme, node_style, FontWeight::NORMAL)
        }
        "code" | "kbd" => {
            let mut element = div()
                .flex()
                .rounded(px(4.0))
                .px(px(4.0))
                .text_size(px(node_style.computed.font_size))
                .text_color(node_style.computed.color)
                .children(node.children.iter().map(|child| {
                    render_preview_html_node(child, theme, node_style.computed)
                }));
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
                element = element.bg(bg).rounded(px(3.0)).px(px(2.0));
            }
            element.into_any_element()
        }
        "br" => div().child("\n").into_any_element(),
        "hr" => div()
            .w_full()
            .h(px(d.separator_thickness))
            .my(px(d.separator_margin_y))
            .bg(c.separator_color)
            .rounded(px(999.0))
            .into_any_element(),
        "blockquote" => {
            let mut element = div()
                .w_full()
                .pl(px(d.quote_padding_left))
                .border_l(px(d.quote_border_width))
                .border_color(c.border_quote)
                .text_size(px(node_style.computed.font_size))
                .text_color(node_style.computed.color)
                .children(node.children.iter().map(|child| {
                    render_preview_html_node(child, theme, node_style.computed)
                }));
            if let Some(bg) = node_style.background {
                element = element.bg(bg);
            }
            element.into_any_element()
        }
        "pre" => {
            let mut element = div()
                .w_full()
                .rounded_sm()
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
                .children(node.children.iter().map(|child| {
                    render_preview_html_node(child, theme, node_style.computed)
                }));
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
                .children(node.children.iter().map(|child| {
                    render_preview_html_node(child, theme, node_style.computed)
                }));
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
                .children(node.children.iter().map(|child| {
                    render_preview_html_node(child, theme, node_style.computed)
                }));
            if let Some(bg) = node_style.background {
                element = element.bg(bg);
            }
            element.into_any_element()
        }
        "th" | "td" => {
            let mut element = div()
                .min_w(px(0.0))
                .flex_grow()
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
                .children(node.children.iter().map(|child| {
                    render_preview_html_node(child, theme, node_style.computed)
                }));
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
                .children(node.children.iter().map(|child| {
                    render_preview_html_node(child, theme, node_style.computed)
                }));
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
                .children(node.children.iter().map(|child| {
                    render_preview_html_node(child, theme, node_style.computed)
                }));
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
                .children(node.children.iter().map(|child| {
                    render_preview_html_node(child, theme, node_style.computed)
                }));
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
        element = element.bg(bg).rounded(px(3.0)).px(px(2.0));
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
        crate::model::syntax::image::ImageResolvedSource::Local(path) => img(path),
        crate::model::syntax::image::ImageResolvedSource::Remote(uri) => img(uri),
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

    let mut container = div()
        .w_full()
        .rounded_sm()
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
                .children(body.map(|child| {
                    render_preview_html_node(child, theme, node_style.computed)
                })),
        );
    }

    container.into_any_element()
}
