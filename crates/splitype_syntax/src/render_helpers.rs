use gpui::*;
use theme::Theme;
use splitype_markdown::inline::html::HtmlCssColor;
use splitype_markdown::block::html::HtmlNode;
use splitype_markdown::block::CalloutKind;

/// Converts an HtmlCssColor to GPUI's Hsla.
pub fn html_css_color_to_hsla(color: HtmlCssColor, current_color: Hsla) -> Hsla {
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

pub fn html_children_text(node: &HtmlNode) -> String {
    if node.children.is_empty() {
        return node.raw_source.clone();
    }
    let mut out = String::new();
    for child in &node.children {
        out.push_str(&html_children_text(child));
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HtmlComputedStyle {
    pub color: Hsla,
    pub font_size: f32,
    pub root_font_size: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HtmlNodeVisualStyle {
    pub computed: HtmlComputedStyle,
    pub background: Option<Hsla>,
}

impl HtmlComputedStyle {
    pub fn root(theme: &Theme) -> Self {
        Self {
            color: theme.colors.text_default,
            font_size: theme.typography.text_size,
            root_font_size: theme.typography.text_size,
        }
    }
}

pub fn html_node_visual_style(
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

    let inline_style = splitype_markdown::block::html::style_for_node(node);
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

pub fn render_custom_bullet_marker(depth: usize, color: Hsla) -> AnyElement {
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

pub fn numbered_list_marker(depth: usize, ordinal: usize) -> String {
    match depth {
        0 => format!("{ordinal}."),
        1 => format!("{}.", alphabetic_list_marker(ordinal)),
        _ => format!("{}.", roman_list_marker(ordinal)),
    }
}

fn alphabetic_list_marker(ordinal: usize) -> String {
    const ALPHABET: &[u8; 26] = b"abcdefghijklmnopqrstuvwxyz";

    let ordinal = ordinal.max(1);
    if ordinal <= ALPHABET.len() {
        return char::from(ALPHABET[ordinal - 1]).to_string();
    }

    let wrapped = ordinal - (ALPHABET.len() + 1);
    let letter = char::from(ALPHABET[wrapped % ALPHABET.len()]);
    let suffix = wrapped + 1;
    format!("{letter}{suffix}")
}

fn roman_list_marker(ordinal: usize) -> String {
    const ROMAN_LOOKUP: &[(usize, &str)] = &[
        (1000, "M"), (900, "CM"), (500, "D"), (400, "CD"),
        (100, "C"), (90, "XC"), (50, "L"), (40, "XL"),
        (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I"),
    ];

    let mut num = ordinal.max(1);
    let mut out = String::new();
    for (val, sym) in ROMAN_LOOKUP {
        while num >= *val {
            out.push_str(sym);
            num -= *val;
        }
    }
    out.to_lowercase()
}

pub fn callout_colors(variant: CalloutKind, theme: &Theme) -> (Hsla, Hsla) {
    let style = variant.callout_style(theme);
    (style.border_color, style.background_color)
}
