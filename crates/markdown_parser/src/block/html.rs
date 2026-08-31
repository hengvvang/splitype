//! Native-safe HTML classification for Markdown raw HTML blocks.
//!
//! The parser keeps the original source as the serialization truth and builds
//! a conservative semantic tree only for tags that can be rendered safely in
//! GPUI. Anything risky, unknown, malformed, or ambiguous becomes raw text.

use std::ops::Range;

#[cfg(feature = "html-native")]
use tree_sitter::Parser;

use crate::inline::html::{HtmlInlineStyle, css_number, parse_css_number, parse_inline_style};

/// Active fenced code block while scanning for image/link reference definitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FenceInfo {
    pub(crate) ch: char,
    pub(crate) len: usize,
}

/// HTML block start that suppresses reference-definition scanning.
pub(crate) enum HtmlBlockStart {
    /// HTML comment beginning with `<!--`.
    Comment,
    /// HTML tag block whose closing behavior depends on the tag.
    Tag {
        name: String,
        self_closing: bool,
        closes_same_line: bool,
    },
}

/// Safety classification for an HTML fragment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HtmlSafetyClass {
    /// The fragment has at least one safe semantic node.
    Semantic,
    /// The entire fragment must be shown and stored as plain raw text.
    RawTextBlock,
}

/// Broad rendering category of a parsed HTML node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HtmlNodeKind {
    /// Safe inline tag or text that can be represented with text runs.
    InlineSemantic,
    /// Safe block tag that maps to a native block-like GPUI element.
    BlockSemantic,
    /// Opaque raw source that must not be interpreted as HTML.
    RawTextBlock,
}

/// One source attribute from an HTML tag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlAttr {
    /// Lowercase attribute name used for safety checks.
    pub name: String,
    /// Parsed attribute value without surrounding quotes.
    pub value: Option<String>,
    /// Exact attribute source text.
    pub raw_source: String,
}

/// Safe data extracted from a standalone HTML `<img>` block.
#[derive(Clone, Debug, PartialEq)]
pub struct HtmlImageBlock {
    pub src: String,
    pub alt: String,
    pub zoom: f32,
}

impl HtmlImageBlock {
    pub fn zoom_factor(&self) -> f32 {
        self.zoom.clamp(0.1, 3.0)
    }

    pub fn to_sanitized_html_with_src(&self, src: &str) -> String {
        let mut html = format!("<img src=\"{}\"", escape_html_attr(src));
        if !self.alt.is_empty() {
            html.push_str(" alt=\"");
            html.push_str(&escape_html_attr(&self.alt));
            html.push('"');
        }
        if (self.zoom_factor() - 1.0).abs() > f32::EPSILON {
            html.push_str(" style=\"zoom: ");
            html.push_str(&css_number(self.zoom_factor() * 100.0));
            html.push_str("%;\"");
        }
        html.push('>');
        html
    }
}

/// A classified HTML node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlNode {
    /// Rendering category selected by the safety policy.
    pub kind: HtmlNodeKind,
    /// Lowercase tag name, or `#text` for text nodes.
    pub tag_name: String,
    /// Safe attributes retained as semantic data.
    pub attrs: Vec<HtmlAttr>,
    /// Classified child nodes. Empty for raw text nodes.
    pub children: Vec<HtmlNode>,
    /// Exact source text covered by this node.
    pub raw_source: String,
    /// Byte range in the original HTML fragment.
    pub source_range: Range<usize>,
}

/// Classified HTML fragment plus its preserved source text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlDocument {
    /// Exact source string used for serialization and raw editing.
    pub raw_source: String,
    /// Root-level classified nodes.
    pub nodes: Vec<HtmlNode>,
    /// Overall fragment safety.
    pub safety: HtmlSafetyClass,
}

impl HtmlDocument {
    pub fn raw(raw_source: impl Into<String>) -> Self {
        let raw_source = raw_source.into();
        Self {
            nodes: vec![raw_node(&raw_source, 0..raw_source.len())],
            safety: HtmlSafetyClass::RawTextBlock,
            raw_source,
        }
    }

    pub fn is_semantic(&self) -> bool {
        self.safety == HtmlSafetyClass::Semantic
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TagKind {
    Open,
    Close,
    CommentLike,
}

#[derive(Clone, Debug)]
struct TagToken {
    kind: TagKind,
    name: String,
    attrs: Vec<HtmlAttr>,
    self_closing: bool,
    source_range: Range<usize>,
}

/// Parses and classifies a raw HTML fragment. The returned document always
/// preserves `raw_source` exactly, even when semantic parsing succeeds.
pub fn parse_html_document(raw_source: &str) -> HtmlDocument {
    if raw_source.trim().is_empty() {
        return HtmlDocument::raw(raw_source);
    }

    if tree_sitter_reports_error(raw_source) {
        return HtmlDocument::raw(raw_source);
    }

    let (nodes, index, ok) = parse_nodes(raw_source, 0, None);
    if !ok || index < raw_source.len() || nodes.is_empty() {
        return HtmlDocument::raw(raw_source);
    }

    if nodes
        .iter()
        .all(|node| matches!(node.kind, HtmlNodeKind::RawTextBlock))
    {
        return HtmlDocument::raw(raw_source);
    }

    HtmlDocument {
        raw_source: raw_source.to_string(),
        nodes,
        safety: HtmlSafetyClass::Semantic,
    }
}

/// Rewrites an HTML fragment for document export: safe semantic nodes keep
/// their HTML shape, while raw text nodes are escaped so browsers cannot
/// execute or interpret them.
pub fn sanitize_html_for_export(raw_source: &str) -> String {
    if let Some(image) = parse_html_image_block(raw_source) {
        return image.to_sanitized_html_with_src(&image.src);
    }

    let document = parse_html_document(raw_source);
    if !document.is_semantic() {
        return format!(
            "<pre class=\"vlt-raw-html\">{}</pre>",
            escape_html(raw_source)
        );
    }

    document
        .nodes
        .iter()
        .map(sanitize_node_for_export)
        .collect::<String>()
}

/// Parses the safe visual subset of a semantic node's `style` attribute.
pub fn style_for_node(node: &HtmlNode) -> HtmlInlineStyle {
    if node.kind == HtmlNodeKind::RawTextBlock {
        return HtmlInlineStyle::default();
    }

    let Some(style) = attr_value(node, "style") else {
        return HtmlInlineStyle::default();
    };

    parse_inline_style(style)
}

fn sanitize_node_for_export(node: &HtmlNode) -> String {
    if node.kind == HtmlNodeKind::RawTextBlock {
        return format!(
            "<span class=\"vlt-raw-html\">{}</span>",
            escape_html(&node.raw_source)
        );
    }

    if node.tag_name == "#text" {
        return node.raw_source.clone();
    }

    if is_void_tag(&node.tag_name) {
        return sanitized_open_tag(node);
    }

    let Some(_open_end) = node.raw_source.find('>').map(|index| index + 1) else {
        return escape_html(&node.raw_source);
    };
    let close_start =
        find_closing_tag_start(&node.raw_source, &node.tag_name).unwrap_or(node.raw_source.len());
    let close = &node.raw_source[close_start..];
    let children = node
        .children
        .iter()
        .map(sanitize_node_for_export)
        .collect::<String>();
    format!("{}{children}{close}", sanitized_open_tag(node))
}

fn sanitized_open_tag(node: &HtmlNode) -> String {
    if node.tag_name == "img"
        && let Some(image) = parse_html_image_block(&node.raw_source)
    {
        return image.to_sanitized_html_with_src(&image.src);
    }

    let mut open = format!("<{}", node.tag_name);
    for attr in &node.attrs {
        if attr.name == "style" {
            continue;
        }
        open.push(' ');
        open.push_str(&attr.raw_source);
    }
    if let Some(style) = style_for_node(node).to_css() {
        open.push_str(" style=\"");
        open.push_str(&escape_html_attr(&style));
        open.push('"');
    }
    open.push('>');
    open
}

fn find_closing_tag_start(raw_source: &str, tag_name: &str) -> Option<usize> {
    let needle = format!("</{tag_name}");
    raw_source.to_ascii_lowercase().rfind(&needle)
}

pub(crate) fn escape_html_attr(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '"' => escaped.push_str("&quot;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn escape_html(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn parse_nodes(
    raw: &str,
    mut index: usize,
    closing_tag: Option<&str>,
) -> (Vec<HtmlNode>, usize, bool) {
    let mut nodes = Vec::new();
    while index < raw.len() {
        let Some(tag_start_relative) = raw[index..].find('<') else {
            if closing_tag.is_some() {
                push_text_node(raw, index..raw.len(), &mut nodes);
            } else {
                push_text_node(raw, index..raw.len(), &mut nodes);
            }
            return (nodes, raw.len(), closing_tag.is_none());
        };

        let tag_start = index + tag_start_relative;
        if tag_start > index {
            push_text_node(raw, index..tag_start, &mut nodes);
        }

        let Some(token) = parse_tag_token(raw, tag_start) else {
            push_text_node(raw, tag_start..tag_start + 1, &mut nodes);
            index = tag_start + 1;
            continue;
        };

        match token.kind {
            TagKind::Close => {
                if closing_tag == Some(token.name.as_str()) {
                    return (nodes, token.source_range.end, true);
                }
                nodes.push(raw_node(raw, token.source_range.clone()));
                index = token.source_range.end;
            }
            TagKind::CommentLike => {
                nodes.push(raw_node(raw, token.source_range.clone()));
                index = token.source_range.end;
            }
            TagKind::Open => {
                let class = classify_open_tag(&token);
                if class == HtmlSafetyClass::RawTextBlock {
                    let raw_end = raw_region_end(raw, &token).unwrap_or(token.source_range.end);
                    nodes.push(raw_node(raw, token.source_range.start..raw_end));
                    index = raw_end;
                    continue;
                }

                if token.self_closing || is_void_tag(&token.name) {
                    nodes.push(semantic_node(raw, token, Vec::new()));
                    index = nodes
                        .last()
                        .map(|node| node.source_range.end)
                        .unwrap_or(index);
                    continue;
                }

                let (children, child_end, closed) =
                    parse_nodes(raw, token.source_range.end, Some(&token.name));
                if !closed {
                    nodes.push(raw_node(raw, token.source_range.start..raw.len()));
                    return (nodes, raw.len(), closing_tag.is_none());
                }

                let mut node = semantic_node(raw, token, children);
                node.source_range.end = child_end;
                node.raw_source = raw[node.source_range.clone()].to_string();
                nodes.push(node);
                index = child_end;
            }
        }
    }

    (nodes, index, closing_tag.is_none())
}

fn parse_tag_token(raw: &str, start: usize) -> Option<TagToken> {
    let rest = raw.get(start..)?;
    if !rest.starts_with('<') {
        return None;
    }

    if rest.starts_with("<!--") {
        let end = rest.find("-->").map(|offset| start + offset + 3)?;
        return Some(TagToken {
            kind: TagKind::CommentLike,
            name: "#comment".into(),
            attrs: Vec::new(),
            self_closing: true,
            source_range: start..end,
        });
    }

    if rest.starts_with("<!") || rest.starts_with("<?") {
        let end = rest.find('>').map(|offset| start + offset + 1)?;
        return Some(TagToken {
            kind: TagKind::CommentLike,
            name: "#raw".into(),
            attrs: Vec::new(),
            self_closing: true,
            source_range: start..end,
        });
    }

    let bytes = raw.as_bytes();
    let mut index = start + 1;
    let closing = bytes.get(index) == Some(&b'/');
    if closing {
        index += 1;
    }

    let name_start = index;
    while index < raw.len() {
        let ch = raw[index..].chars().next()?;
        if ch.is_ascii_alphanumeric() || ch == '-' {
            index += ch.len_utf8();
        } else {
            break;
        }
    }
    if index == name_start {
        return None;
    }

    let name = raw[name_start..index].to_ascii_lowercase();
    let attrs_start = index;
    let mut quote: Option<char> = None;
    while index < raw.len() {
        let ch = raw[index..].chars().next()?;
        if let Some(active_quote) = quote {
            if ch == active_quote {
                quote = None;
            }
            index += ch.len_utf8();
            continue;
        }

        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            index += ch.len_utf8();
            continue;
        }

        if ch == '>' {
            let source_range = start..index + 1;
            let attrs_source = &raw[attrs_start..index];
            let self_closing = attrs_source.trim_end().ends_with('/');
            return Some(TagToken {
                kind: if closing {
                    TagKind::Close
                } else {
                    TagKind::Open
                },
                name,
                attrs: if closing {
                    Vec::new()
                } else {
                    parse_html_attrs(attrs_source)
                },
                self_closing,
                source_range,
            });
        }

        index += ch.len_utf8();
    }

    None
}

/// Peek the next char at `index` without advancing. Returns `None` at EOF.
#[inline]
fn peek_char(source: &str, index: usize) -> Option<char> {
    source[index..].chars().next()
}

/// Advance `index` past the next char and return it. Returns `None` at EOF.
/// Encapsulates the byte-index ↔ UTF-8-boundary invariant so callers that
/// don't need the char's value can't drift into a panic by hand-incrementing
/// `index` by anything other than `ch.len_utf8()`. Loops that *do* need the
/// char for a check should peek with [`peek_char`], inspect the value, and
/// then advance with `index += ch.len_utf8()` — see [`parse_html_attrs`] —
/// so the char is read only once per iteration.
#[inline]
fn advance_char(source: &str, index: &mut usize) -> Option<char> {
    let ch = source[*index..].chars().next()?;
    *index += ch.len_utf8();
    Some(ch)
}

pub fn parse_html_attrs(source: &str) -> Vec<HtmlAttr> {
    let mut attrs = Vec::new();
    let mut index = 0usize;
    while index < source.len() {
        while let Some(ch) = peek_char(source, index).filter(|c| c.is_whitespace() || *c == '/') {
            index += ch.len_utf8();
        }
        if index >= source.len() {
            break;
        }

        let start = index;
        while let Some(ch) = peek_char(source, index) {
            if ch.is_whitespace() || ch == '=' || ch == '/' {
                break;
            }
            index += ch.len_utf8();
        }
        let name_end = index;
        if name_end == start {
            // Lone separator we couldn't classify — consume one char and retry.
            advance_char(source, &mut index);
            continue;
        }

        while let Some(ch) = peek_char(source, index).filter(|c| c.is_whitespace()) {
            index += ch.len_utf8();
        }

        let mut value = None;
        if source[index..].starts_with('=') {
            index += 1;
            while let Some(ch) = peek_char(source, index).filter(|c| c.is_whitespace()) {
                index += ch.len_utf8();
            }

            if let Some(quote) = peek_char(source, index).filter(|c| *c == '"' || *c == '\'') {
                index += quote.len_utf8();
                let value_start = index;
                while let Some(ch) = peek_char(source, index) {
                    if ch == quote {
                        break;
                    }
                    index += ch.len_utf8();
                }
                value = Some(source[value_start..index].to_string());
                if index < source.len() {
                    index += quote.len_utf8();
                }
            } else if peek_char(source, index).is_some() {
                let value_start = index;
                while let Some(ch) = peek_char(source, index) {
                    if ch.is_whitespace() || ch == '/' {
                        break;
                    }
                    index += ch.len_utf8();
                }
                value = Some(source[value_start..index].to_string());
            }
        }

        attrs.push(HtmlAttr {
            name: source[start..name_end].to_ascii_lowercase(),
            value,
            raw_source: source[start..index].to_string(),
        });
    }

    attrs
}

fn classify_open_tag(token: &TagToken) -> HtmlSafetyClass {
    if !is_safe_tag(&token.name) || has_dangerous_attrs(&token.attrs) {
        HtmlSafetyClass::RawTextBlock
    } else {
        HtmlSafetyClass::Semantic
    }
}

fn semantic_node(raw: &str, token: TagToken, children: Vec<HtmlNode>) -> HtmlNode {
    HtmlNode {
        kind: if is_inline_tag(&token.name) {
            HtmlNodeKind::InlineSemantic
        } else {
            HtmlNodeKind::BlockSemantic
        },
        tag_name: token.name,
        attrs: token.attrs,
        children,
        raw_source: raw[token.source_range.clone()].to_string(),
        source_range: token.source_range,
    }
}

fn push_text_node(raw: &str, range: Range<usize>, nodes: &mut Vec<HtmlNode>) {
    if range.is_empty() {
        return;
    }
    nodes.push(HtmlNode {
        kind: HtmlNodeKind::InlineSemantic,
        tag_name: "#text".into(),
        attrs: Vec::new(),
        children: Vec::new(),
        raw_source: raw[range.clone()].to_string(),
        source_range: range,
    });
}

fn raw_node(raw: &str, range: Range<usize>) -> HtmlNode {
    HtmlNode {
        kind: HtmlNodeKind::RawTextBlock,
        tag_name: "#raw".into(),
        attrs: Vec::new(),
        children: Vec::new(),
        raw_source: raw[range.clone()].to_string(),
        source_range: range,
    }
}

fn raw_region_end(raw: &str, token: &TagToken) -> Option<usize> {
    if token.self_closing || is_void_tag(&token.name) {
        return Some(token.source_range.end);
    }

    let close = format!("</{}>", token.name);
    let close_upper = close.to_ascii_uppercase();
    let rest = &raw[token.source_range.end..];
    let lower = rest.to_ascii_lowercase();
    let upper = rest.to_ascii_uppercase();
    lower
        .find(&close)
        .or_else(|| upper.find(&close_upper))
        .map(|offset| token.source_range.end + offset + close.len())
        .or(Some(raw.len()))
}

pub fn has_dangerous_attrs(attrs: &[HtmlAttr]) -> bool {
    attrs.iter().any(|attr| {
        attr.name.starts_with("on")
            || attr.value.as_deref().is_some_and(|value| {
                let normalized = value
                    .chars()
                    .filter(|ch| !ch.is_whitespace() && *ch != '\0')
                    .collect::<String>()
                    .to_ascii_lowercase();
                matches!(
                    attr.name.as_str(),
                    "href" | "src" | "action" | "formaction" | "xlink:href"
                ) && normalized.starts_with("javascript:")
            })
    })
}

pub fn attr_value<'a>(node: &'a HtmlNode, name: &str) -> Option<&'a str> {
    node.attrs
        .iter()
        .find(|attr| attr.name == name)
        .and_then(|attr| attr.value.as_deref())
}

pub fn parse_html_image_block(raw_source: &str) -> Option<HtmlImageBlock> {
    let trimmed = raw_source.trim();
    if trimmed.is_empty() {
        return None;
    }

    let token = parse_tag_token(trimmed, 0)?;
    if token.kind != TagKind::Open
        || token.name != "img"
        || token.source_range != (0..trimmed.len())
    {
        return None;
    }
    if has_dangerous_attrs(&token.attrs) {
        return None;
    }

    let src = attr_value_in_attrs(&token.attrs, "src")?.trim().to_string();
    if src.is_empty() {
        return None;
    }

    let alt = attr_value_in_attrs(&token.attrs, "alt")
        .unwrap_or_default()
        .to_string();
    let zoom = attr_value_in_attrs(&token.attrs, "style")
        .and_then(parse_html_zoom)
        .unwrap_or(1.0);

    Some(HtmlImageBlock { src, alt, zoom })
}

fn attr_value_in_attrs<'a>(attrs: &'a [HtmlAttr], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|attr| attr.name == name)
        .and_then(|attr| attr.value.as_deref())
}

pub fn parse_html_zoom(style: &str) -> Option<f32> {
    for declaration in style.split(';') {
        let Some((property, value)) = declaration.split_once(':') else {
            continue;
        };
        if !property.trim().eq_ignore_ascii_case("zoom") {
            continue;
        }

        let value = value.trim();
        let parsed = if let Some(percent) = value.strip_suffix('%') {
            parse_css_number(percent)? / 100.0
        } else {
            parse_css_number(value)?
        };
        return Some(parsed.clamp(0.1, 3.0));
    }
    None
}

fn is_safe_tag(name: &str) -> bool {
    is_inline_tag(name) || is_block_tag(name)
}

pub fn is_inline_tag(name: &str) -> bool {
    matches!(
        name,
        "a" | "strong"
            | "em"
            | "b"
            | "i"
            | "u"
            | "mark"
            | "del"
            | "ins"
            | "code"
            | "kbd"
            | "sup"
            | "sub"
            | "small"
            | "abbr"
            | "dfn"
            | "time"
            | "q"
            | "span"
    )
}

fn is_block_tag(name: &str) -> bool {
    matches!(
        name,
        "div"
            | "p"
            | "blockquote"
            | "hr"
            | "br"
            | "details"
            | "summary"
            | "figure"
            | "figcaption"
            | "table"
            | "thead"
            | "tbody"
            | "tfoot"
            | "tr"
            | "th"
            | "td"
            | "img"
            | "pre"
    )
}

fn is_void_tag(name: &str) -> bool {
    matches!(name, "br" | "hr" | "img")
}

#[cfg(feature = "html-native")]
fn tree_sitter_reports_error(raw_source: &str) -> bool {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_html::LANGUAGE.into())
        .is_err()
    {
        return true;
    }
    parser
        .parse(raw_source, None)
        .is_none_or(|tree| tree.root_node().has_error())
}

#[cfg(not(feature = "html-native"))]
fn tree_sitter_reports_error(_: &str) -> bool {
    true
}
