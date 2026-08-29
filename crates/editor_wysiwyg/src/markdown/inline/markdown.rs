//! Markdown-to-`BlockText` parser: tokenizes inline text, matches delimiter
//! pairs (bold, italic, links, footnotes, math, inline HTML), and rebuilds
//! the styled fragment tree with an input-to-normalized offset map.

use std::ops::Range;

use super::latex::{InlineLatex, InlineLatexDelimiter};
use super::link::InlineLink;
use super::serialize::{
    Delimiter, apply_delimiter_style, backtick_run_len, can_close_emphasis, emphasis_requires_body,
    has_closing_delimiter, match_open_delimiter,
};
use super::style::{InlineScript, InlineStyle};
use super::text::{BlockText, InlineAttributes, InlineFragment};
use crate::markdown::block::html::{
    HtmlAttr, HtmlNode, HtmlNodeKind, has_dangerous_attrs, is_inline_tag, parse_html_attrs,
    style_for_node,
};
use crate::markdown::block::link::{LinkReferenceDefinition, LinkReferenceDefinitions, parse_link_target};
use crate::markdown::inline::footnote::{InlineFootnoteReference, parse_inline_footnote_reference};
use crate::markdown::inline::html::HtmlInlineStyle;

// ---------------------------------------------------------------------------
// Normalizer: traverse CharTokens and rebuild the tree
// ---------------------------------------------------------------------------

/// Source character plus style and byte range used by inline parsing.
#[derive(Clone)]
pub(crate) struct CharToken {
    pub(crate) ch: char,
    pub(crate) style: InlineStyle,
    pub(crate) html_style: Option<HtmlInlineStyle>,
    pub(crate) source_range: Range<usize>,
}

/// Result of parsing a delimited inline region.
pub(crate) struct ParseResult {
    next_index: usize,
    closed: bool,
}

/// Builds the output fragments during normalization (marker parsing).
/// Keeps track of the input-to-normalized offset mapping so that
/// selections and cursors can be mapped to the normalized tree. The input
/// side is the normalizer's input text: Markdown when parsing, plain text
/// when editing.
pub struct NormalizeBuilder {
    pub(crate) fragments: Vec<InlineFragment>,
    pub(crate) visible_to_normalized: Vec<usize>,
    normalized_len: usize,
}

#[derive(Clone)]
pub(crate) struct NormalizeBuilderCheckpoint {
    fragments: Vec<InlineFragment>,
    visible_to_normalized: Vec<usize>,
    normalized_len: usize,
}

impl NormalizeBuilder {
    pub fn new(input_len: usize) -> Self {
        Self {
            fragments: Vec::new(),
            visible_to_normalized: vec![0; input_len + 1],
            normalized_len: 0,
        }
    }

    pub(crate) fn checkpoint(&self) -> NormalizeBuilderCheckpoint {
        NormalizeBuilderCheckpoint {
            fragments: self.fragments.clone(),
            visible_to_normalized: self.visible_to_normalized.clone(),
            normalized_len: self.normalized_len,
        }
    }

    pub(crate) fn restore(&mut self, checkpoint: NormalizeBuilderCheckpoint) {
        self.fragments = checkpoint.fragments;
        self.visible_to_normalized = checkpoint.visible_to_normalized;
        self.normalized_len = checkpoint.normalized_len;
    }

    fn drop_token(&mut self, token: &CharToken) {
        for boundary in token.source_range.start..=token.source_range.end {
            self.visible_to_normalized[boundary] = self.normalized_len;
        }
    }

    fn emit_token(
        &mut self,
        token: &CharToken,
        extra_style: InlineStyle,
        html_style: Option<HtmlInlineStyle>,
    ) {
        let mut style = token.style;
        if extra_style.bold {
            style.bold = true;
            style.bold_marker = extra_style.bold_marker;
        }
        if extra_style.italic {
            style.italic = true;
            style.italic_marker = extra_style.italic_marker;
        }
        if extra_style.bold && extra_style.italic {
            style.italic_outer = extra_style.italic_outer;
        }
        if extra_style.underline {
            style.underline = true;
        }
        if extra_style.strikethrough {
            style.strikethrough = true;
        }
        if extra_style.highlight {
            style.highlight = true;
        }
        if extra_style.code {
            style.code = true;
        }
        if extra_style.has_script() {
            style.script = extra_style.script;
        }
        let html_style = merge_html_styles(html_style, token.html_style);

        let text = token.ch.to_string();
        let start = self.normalized_len;
        for boundary in token.source_range.start..=token.source_range.end {
            self.visible_to_normalized[boundary] = start + (boundary - token.source_range.start);
        }
        self.normalized_len += text.len();

        if let Some(last) = self.fragments.last_mut()
            && last.style == style
            && last.html_style() == html_style
            && last.link().is_none()
            && last.footnote().is_none()
            && last.math().is_none()
        {
            last.text.push_str(&text);
            return;
        }

        self.fragments.push(InlineFragment::new(
            text,
            InlineAttributes {
                style,
                html_style,
                ..Default::default()
            },
        ));
    }

    fn emit_inline_math(
        &mut self,
        tokens: &[CharToken],
        math: InlineLatex,
        extra_style: InlineStyle,
        extra_html_style: Option<HtmlInlineStyle>,
    ) {
        let source_start = tokens
            .first()
            .map(|token| token.source_range.start)
            .unwrap_or(0);
        let normalized_start = self.normalized_len;
        let source = math.source.clone();
        let plain_len = source.len();

        for token in tokens {
            let token_len = token.source_range.len();
            for delta in 0..=token_len {
                self.visible_to_normalized[token.source_range.start + delta] =
                    normalized_start + (token.source_range.start + delta - source_start);
            }
        }

        self.normalized_len += plain_len;
        self.fragments.push(InlineFragment::new(
            source,
            InlineAttributes {
                style: extra_style,
                html_style: extra_html_style,
                math: Some(math),
                ..Default::default()
            },
        ));
    }
}

pub(crate) fn flatten_tokens(fragments: &[InlineFragment]) -> Vec<CharToken> {
    let mut tokens = Vec::new();
    let mut plain_offset = 0;

    for fragment in fragments {
        for ch in fragment.text.chars() {
            let len = ch.len_utf8();
            tokens.push(CharToken {
                ch,
                style: fragment.style,
                html_style: fragment.html_style(),
                source_range: plain_offset..plain_offset + len,
            });
            plain_offset += len;
        }
    }

    tokens
}

/// Recursive-descent parser that consumes [`CharToken`]s and reconstructs
/// the normalized inline tree.  Matching delimiters are consumed (dropped);
/// unmatched ones are emitted as literal text.  Nested styles are handled by
/// recursive calls that accumulate `extra_style`.
pub(crate) fn parse_until(
    tokens: &[CharToken],
    mut index: usize,
    end_delimiter: Option<Delimiter>,
    extra_style: InlineStyle,
    extra_html_style: Option<HtmlInlineStyle>,
    builder: &mut NormalizeBuilder,
    inside_code: bool,
    reference_definitions: &LinkReferenceDefinitions,
) -> ParseResult {
    let body_start = index;
    while index < tokens.len() {
        // Check for closing delimiter.
        if let Some(ref end_delim) = end_delimiter {
            let mut closed = match end_delim {
                Delimiter::CodeMarkdown { run_len } => {
                    tokens[index].ch == '`' && backtick_run_len(tokens, index) == *run_len
                }
                Delimiter::SuperscriptMarkdown => {
                    tokens[index].ch == '^' && can_close_emphasis(tokens, index, 1, '^')
                }
                Delimiter::SubscriptMarkdown => {
                    is_single_tilde_delimiter(tokens, index)
                        && can_close_emphasis(tokens, index, 1, '~')
                }
                _ => {
                    let close_str = end_delim.close();
                    let close_len = close_str.chars().count();
                    let marker = close_str.chars().next().unwrap_or('*');
                    matches_sequence(tokens, index, &close_str)
                        && can_close_emphasis(tokens, index, close_len, marker)
                }
            };

            // Emphasis spans must enclose at least one character; reject a
            // close at the very start of the body so empty spans stay literal.
            if closed && index == body_start && emphasis_requires_body(*end_delim) {
                closed = false;
            }

            if closed {
                let close_len = end_delim.close().chars().count();
                for token in &tokens[index..index + close_len] {
                    builder.drop_token(token);
                }
                return ParseResult {
                    next_index: index + close_len,
                    closed: true,
                };
            }
        }

        if !inside_code
            && let Some(next_index) =
                parse_inline_math(tokens, index, extra_style, extra_html_style, builder)
        {
            index = next_index;
            continue;
        }

        if !inside_code
            && tokens[index].ch == '\\'
            && let Some(escaped_len) = escaped_sequence_token_len(tokens, index)
        {
            builder.drop_token(&tokens[index]);
            let escaped_start = index + 1;
            let escaped_end = escaped_start + escaped_len;
            for token in &tokens[escaped_start..escaped_end] {
                builder.emit_token(token, extra_style, extra_html_style);
            }
            index = escaped_end;
            continue;
        }

        // Inside a code span, all text (including markers) is literal.
        if !inside_code {
            if tokens[index].ch == '['
                && let Some(next_index) =
                    parse_footnote_reference(tokens, index, extra_style, extra_html_style, builder)
            {
                index = next_index;
                continue;
            }

            if (tokens[index].ch == '['
                || (tokens[index].ch == '!'
                    && tokens.get(index + 1).map(|t| t.ch) == Some('[')))
                && let Some(next_index) = parse_inline_link(
                    tokens,
                    index,
                    extra_style,
                    extra_html_style,
                    builder,
                    reference_definitions,
                )
            {
                index = next_index;
                continue;
            }

            if tokens[index].ch == '<'
                && let Some(next_index) = parse_inline_html_container(
                    tokens,
                    index,
                    extra_style,
                    extra_html_style,
                    builder,
                    reference_definitions,
                )
            {
                index = next_index;
                continue;
            }

            if tokens[index].ch == '<'
                && let Some(next_index) = parse_autolink(
                    tokens,
                    index,
                    extra_style,
                    extra_html_style,
                    builder,
                    reference_definitions,
                )
            {
                index = next_index;
                continue;
            }

            if let Some(delimiter) = match_open_delimiter(tokens, index) {
                if has_closing_delimiter(tokens, index, delimiter) {
                    let checkpoint = builder.checkpoint();
                    for token in &tokens[index..index + delimiter.token_len()] {
                        builder.drop_token(token);
                    }
                    let inner_start = index + delimiter.token_len();
                    let is_code_delim = matches!(delimiter, Delimiter::CodeMarkdown { .. });
                    let parsed = parse_until(
                        tokens,
                        inner_start,
                        Some(delimiter),
                        apply_delimiter_style(extra_style, delimiter),
                        extra_html_style,
                        builder,
                        is_code_delim,
                        reference_definitions,
                    );
                    if parsed.closed {
                        index = parsed.next_index;
                        continue;
                    }
                    builder.restore(checkpoint);
                } else if delimiter.token_len() > 1 {
                    // Keep an unclosed multi-character opener (`**`, `__`, `~~`,
                    // backtick run) literal as one unit. Emitting just its first
                    // char would let the rest open a shorter span (e.g. `**bold*`
                    // -> `*` + italic `bold`), which is committed on every
                    // keystroke and loses the intended bold.
                    for token in &tokens[index..index + delimiter.token_len()] {
                        builder.emit_token(token, extra_style, extra_html_style);
                    }
                    index += delimiter.token_len();
                    continue;
                }
            }
        }

        builder.emit_token(&tokens[index], extra_style, extra_html_style);
        index += 1;
    }

    ParseResult {
        next_index: tokens.len(),
        closed: false,
    }
}

// ---------------------------------------------------------------------------
// Inline math parser
// ---------------------------------------------------------------------------

pub(crate) fn parse_inline_math(
    tokens: &[CharToken],
    index: usize,
    extra_style: InlineStyle,
    extra_html_style: Option<HtmlInlineStyle>,
    builder: &mut NormalizeBuilder,
) -> Option<usize> {
    let (body_start, close_start, close_end, delimiter) = if tokens.get(index)?.ch == '$' {
        if matches_sequence(tokens, index, "$$") || token_is_backslash_escaped(tokens, index) {
            return None;
        }
        let close = locate_inline_dollar_math_close(tokens, index + 1)?;
        (index + 1, close, close, InlineLatexDelimiter::Dollar)
    } else if matches_sequence(tokens, index, "\\(") {
        let close = locate_inline_paren_math_close(tokens, index + 2)?;
        (index + 2, close, close + 1, InlineLatexDelimiter::Paren)
    } else {
        return None;
    };

    if body_start >= close_start {
        return None;
    }
    if tokens[body_start..close_start]
        .iter()
        .any(|token| token.ch == '\n' || token.ch == '\r')
    {
        return None;
    }
    if tokens[body_start].ch.is_whitespace() || tokens[close_start - 1].ch.is_whitespace() {
        return None;
    }

    let source = tokens_to_string(&tokens[index..=close_end]);
    let body = tokens_to_string(&tokens[body_start..close_start]);
    if looks_like_obvious_currency(tokens, index, close_end, &body) {
        return None;
    }

    let math = InlineLatex {
        source,
        body,
        delimiter,
    };
    builder.emit_inline_math(
        &tokens[index..=close_end],
        math,
        extra_style,
        extra_html_style,
    );
    Some(close_end + 1)
}

fn locate_inline_dollar_math_close(tokens: &[CharToken], mut cursor: usize) -> Option<usize> {
    while cursor < tokens.len() {
        let token = &tokens[cursor];
        if token.ch == '\n' || token.ch == '\r' {
            return None;
        }
        if token.ch == '$'
            && !token_is_backslash_escaped(tokens, cursor)
            && !matches_sequence(tokens, cursor, "$$")
        {
            return Some(cursor);
        }
        cursor += 1;
    }
    None
}

fn locate_inline_paren_math_close(tokens: &[CharToken], mut cursor: usize) -> Option<usize> {
    while cursor + 1 < tokens.len() {
        if tokens[cursor].ch == '\n' || tokens[cursor].ch == '\r' {
            return None;
        }
        if matches_sequence(tokens, cursor, "\\)") {
            return Some(cursor);
        }
        cursor += 1;
    }
    None
}

pub(crate) fn token_is_backslash_escaped(tokens: &[CharToken], index: usize) -> bool {
    if index == 0 {
        return false;
    }
    let mut cursor = index;
    let mut slash_count = 0usize;
    while cursor > 0 && tokens[cursor - 1].ch == '\\' {
        slash_count += 1;
        cursor -= 1;
    }
    slash_count % 2 == 1
}

fn looks_like_obvious_currency(
    tokens: &[CharToken],
    open_index: usize,
    close_index: usize,
    body: &str,
) -> bool {
    let prev_is_digit = open_index
        .checked_sub(1)
        .and_then(|idx| tokens.get(idx))
        .is_some_and(|token| token.ch.is_ascii_digit());
    let next_is_digit = tokens
        .get(close_index + 1)
        .is_some_and(|token| token.ch.is_ascii_digit());
    if prev_is_digit || next_is_digit {
        return true;
    }

    body.chars()
        .all(|ch| ch.is_ascii_digit() || matches!(ch, '.' | ',' | '_'))
        && body.chars().any(|ch| ch.is_ascii_digit())
        && body.len() > 1
}

// ---------------------------------------------------------------------------
// Footnote reference parser
// ---------------------------------------------------------------------------

pub(crate) fn parse_footnote_reference(
    tokens: &[CharToken],
    index: usize,
    extra_style: InlineStyle,
    extra_html_style: Option<HtmlInlineStyle>,
    builder: &mut NormalizeBuilder,
) -> Option<usize> {
    if tokens.get(index)?.ch != '[' || tokens.get(index + 1)?.ch != '^' {
        return None;
    }

    let mut cursor = index + 2;
    let end_index = loop {
        let token = tokens.get(cursor)?;
        if token.ch == '\\' {
            cursor += 2;
            continue;
        }
        if token.ch == ']' {
            break cursor;
        }
        cursor += 1;
    };

    let raw_markdown = tokens_to_string(&tokens[index..=end_index]);
    let id = parse_inline_footnote_reference(&raw_markdown)?;
    let fragments = vec![InlineFragment::new(
        id.clone(),
        InlineAttributes {
            style: InlineStyle {
                script: InlineScript::Superscript,
                ..extra_style
            },
            html_style: extra_html_style,
            footnote: Some(InlineFootnoteReference {
                id: id.clone(),
                occurrence_index: 0,
            }),
            ..Default::default()
        },
    )];

    let normalized_start = builder.normalized_len;
    let plain_len = id.len();
    let normalized_end = normalized_start + plain_len;
    for token in &tokens[index..=end_index] {
        let token_len = token.source_range.len();
        for delta in 0..=token_len {
            let offset = token.source_range.start + delta - tokens[index].source_range.start;
            let mapped = if raw_markdown.is_empty() {
                0
            } else {
                (plain_len * offset) / raw_markdown.len()
            };
            builder.visible_to_normalized[token.source_range.start + delta] =
                normalized_start + mapped.min(plain_len);
        }
    }

    for fragment in fragments {
        builder.normalized_len += fragment.text.len();
        if let Some(last) = builder.fragments.last_mut()
            && last.style == fragment.style
            && last.extra == fragment.extra
            && last.math().is_none()
            && fragment.math().is_none()
        {
            last.text.push_str(&fragment.text);
        } else {
            builder.fragments.push(fragment);
        }
    }

    for boundary in tokens[end_index].source_range.end..=tokens[end_index].source_range.end {
        builder.visible_to_normalized[boundary] = normalized_end;
    }

    Some(end_index + 1)
}

// ---------------------------------------------------------------------------
// Inline link parser
// ---------------------------------------------------------------------------

pub(crate) fn parse_inline_link(
    tokens: &[CharToken],
    index: usize,
    extra_style: InlineStyle,
    extra_html_style: Option<HtmlInlineStyle>,
    builder: &mut NormalizeBuilder,
    reference_definitions: &LinkReferenceDefinitions,
) -> Option<usize> {
    let located = locate_inline_link(tokens, index, reference_definitions)?;
    let label_end = located.label_end;
    let bracket_index = if located.link.is_image() { index + 1 } else { index };
    let label_tokens = &tokens[bracket_index + 1..label_end];
    let label_markdown = tokens_to_string(label_tokens);
    let mut label_result = BlockText::plain(label_markdown)
        .normalize_inline_syntax_with_link_references(reference_definitions);
    apply_extra_style_to_fragments(
        &mut label_result.tree.fragments,
        extra_style,
        extra_html_style,
    );
    let link = located.link;

    let normalized_start = builder.normalized_len;
    let label_len = label_result.tree.plain_len();

    for token in &tokens[index..=bracket_index] {
        for boundary in token.source_range.start..=token.source_range.end {
            builder.visible_to_normalized[boundary] = normalized_start;
        }
    }

    let mut local_boundary = 0usize;
    for token in label_tokens {
        let token_len = token.source_range.len();
        for delta in 0..=token_len {
            builder.visible_to_normalized[token.source_range.start + delta] =
                normalized_start + label_result.visible_to_normalized[local_boundary + delta];
        }
        local_boundary += token_len;
    }

    let normalized_end = normalized_start + label_len;
    for token in &tokens[label_end..=located.end_index] {
        for boundary in token.source_range.start..=token.source_range.end {
            builder.visible_to_normalized[boundary] = normalized_end;
        }
    }

    if label_result.tree.fragments.is_empty() {
        builder.fragments.push(InlineFragment::new(
            String::new(),
            InlineAttributes {
                style: extra_style,
                html_style: extra_html_style,
                link: Some(link),
                ..Default::default()
            },
        ));
    } else {
        for mut fragment in label_result.tree.fragments {
            fragment.set_link(Some(link.clone()));
            fragment.set_footnote(None);
            fragment.set_math(None);
            builder.normalized_len += fragment.text.len();
            if let Some(last) = builder.fragments.last_mut()
                && last.style == fragment.style
                && last.extra == fragment.extra
                && last.math().is_none()
                && fragment.math().is_none()
            {
                last.text.push_str(&fragment.text);
            } else {
                builder.fragments.push(fragment);
            }
        }
    }

    Some(located.end_index + 1)
}

// ---------------------------------------------------------------------------
// Autolink parser
// ---------------------------------------------------------------------------

pub(crate) fn parse_autolink(
    tokens: &[CharToken],
    index: usize,
    extra_style: InlineStyle,
    extra_html_style: Option<HtmlInlineStyle>,
    builder: &mut NormalizeBuilder,
    _reference_definitions: &LinkReferenceDefinitions,
) -> Option<usize> {
    let end_index = locate_autolink(tokens, index)?;
    let target_tokens = &tokens[index + 1..end_index];
    let target = tokens_to_string(target_tokens);
    let fragments = vec![InlineFragment::new(
        target.clone(),
        InlineAttributes {
            style: extra_style,
            html_style: extra_html_style,
            link: Some(InlineLink::Autolink {
                destination: target.clone(),
            }),
            ..Default::default()
        },
    )];

    let normalized_start = builder.normalized_len;
    let target_len = target.len();

    for boundary in tokens[index].source_range.start..=tokens[index].source_range.end {
        builder.visible_to_normalized[boundary] = normalized_start;
    }

    let mut local_boundary = 0usize;
    for token in target_tokens {
        let token_len = token.source_range.len();
        for delta in 0..=token_len {
            builder.visible_to_normalized[token.source_range.start + delta] =
                normalized_start + local_boundary + delta;
        }
        local_boundary += token_len;
    }

    let normalized_end = normalized_start + target_len;
    for boundary in tokens[end_index].source_range.start..=tokens[end_index].source_range.end {
        builder.visible_to_normalized[boundary] = normalized_end;
    }

    for fragment in fragments {
        builder.normalized_len += fragment.text.len();
        if let Some(last) = builder.fragments.last_mut()
            && last.style == fragment.style
            && last.extra == fragment.extra
            && last.math().is_none()
            && fragment.math().is_none()
        {
            last.text.push_str(&fragment.text);
        } else {
            builder.fragments.push(fragment);
        }
    }

    Some(end_index + 1)
}

// ---------------------------------------------------------------------------
// Inline HTML container parser
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct InlineHtmlTag {
    name: String,
    attrs: Vec<HtmlAttr>,
    end_index: usize,
    self_closing: bool,
}

pub(crate) fn parse_inline_html_container(
    tokens: &[CharToken],
    index: usize,
    extra_style: InlineStyle,
    extra_html_style: Option<HtmlInlineStyle>,
    builder: &mut NormalizeBuilder,
    reference_definitions: &LinkReferenceDefinitions,
) -> Option<usize> {
    let tag = locate_inline_html_open_tag(tokens, index)?;
    if tag.self_closing || !is_inline_tag(&tag.name) || has_dangerous_attrs(&tag.attrs) {
        return None;
    }

    let (close_start, close_end) =
        locate_matching_inline_html_close(tokens, tag.end_index + 1, &tag.name)?;
    let tag_style = inline_html_semantic_style(&tag.name, extra_style);
    let html_style = merge_html_styles(extra_html_style, inline_html_style(&tag));
    if tag_style == extra_style && html_style == extra_html_style {
        return None;
    }

    for token in &tokens[index..=tag.end_index] {
        builder.drop_token(token);
    }
    let _ = parse_until(
        &tokens[tag.end_index + 1..close_start],
        0,
        None,
        tag_style,
        html_style,
        builder,
        false,
        reference_definitions,
    );
    for token in &tokens[close_start..=close_end] {
        builder.drop_token(token);
    }

    Some(close_end + 1)
}

fn inline_html_semantic_style(name: &str, style: InlineStyle) -> InlineStyle {
    match name {
        "strong" | "b" => style.with_bold(),
        "em" | "i" => style.with_italic(),
        "u" | "ins" => style.with_underline(),
        "del" => style.with_strikethrough(),
        "code" | "kbd" => style.with_code(),
        "sup" => style.with_superscript(),
        "sub" => style.with_subscript(),
        _ => style,
    }
}

fn inline_html_style(tag: &InlineHtmlTag) -> Option<HtmlInlineStyle> {
    let node = HtmlNode {
        kind: HtmlNodeKind::InlineSemantic,
        tag_name: tag.name.clone(),
        attrs: tag.attrs.clone(),
        children: Vec::new(),
        raw_source: String::new(),
        source_range: 0..0,
    };
    let style = style_for_node(&node);
    (!style.is_empty()).then_some(style)
}

fn merge_html_styles(
    parent: Option<HtmlInlineStyle>,
    child: Option<HtmlInlineStyle>,
) -> Option<HtmlInlineStyle> {
    let mut merged = parent.unwrap_or_default();
    if let Some(child) = child {
        if child.color.is_some() {
            merged.color = child.color;
        }
        if child.background_color.is_some() {
            merged.background_color = child.background_color;
        }
        if child.font_size.is_some() {
            merged.font_size = child.font_size;
        }
    }

    (!merged.is_empty()).then_some(merged)
}

// ---------------------------------------------------------------------------
// Link locating
// ---------------------------------------------------------------------------

/// Located inline link syntax inside the token stream.
#[derive(Clone)]
struct LocatedInlineLink {
    label_end: usize,
    end_index: usize,
    link: InlineLink,
}

fn locate_inline_link(
    tokens: &[CharToken],
    index: usize,
    reference_definitions: &LinkReferenceDefinitions,
) -> Option<LocatedInlineLink> {
    let (is_image, bracket_index) = if tokens.get(index)?.ch == '!'
        && tokens.get(index + 1).map(|token| token.ch) == Some('[')
    {
        (true, index + 1)
    } else if tokens.get(index)?.ch == '[' {
        if index > 0 && matches!(tokens[index - 1].ch, '!' | ']') {
            return None;
        }
        (false, index)
    } else {
        return None;
    };

    let mut label_depth = 0usize;
    let mut cursor = bracket_index + 1;
    let label_end = loop {
        let token = tokens.get(cursor)?;
        if token.ch == '\\' {
            cursor += 2;
            continue;
        }

        match token.ch {
            '[' => label_depth += 1,
            ']' if label_depth == 0 => break cursor,
            ']' => label_depth = label_depth.saturating_sub(1),
            _ => {}
        }
        cursor += 1;
    };

    match tokens.get(label_end + 1).map(|token| token.ch) {
        Some('(') => {
            let url_start = label_end + 2;
            let mut paren_depth = 0usize;
            cursor = url_start;
            let url_end = loop {
                let token = tokens.get(cursor)?;
                if token.ch == '\\' {
                    cursor += 2;
                    continue;
                }

                match token.ch {
                    '(' => paren_depth += 1,
                    ')' if paren_depth == 0 => break cursor,
                    ')' => paren_depth = paren_depth.saturating_sub(1),
                    _ => {}
                }
                cursor += 1;
            };

            // An empty destination such as in `[label]()` is a valid link, but the
            // target parser rejects an empty string. Recognizing it keeps the caret
            // inside the projected link while the destination is filled in.
            let (destination, title) = if url_start == url_end {
                (String::new(), None)
            } else {
                parse_link_target(&tokens_to_string(&tokens[url_start..url_end]))?
            };
            Some(LocatedInlineLink {
                label_end,
                end_index: url_end,
                link: InlineLink::Inline {
                    destination,
                    title,
                    is_image,
                },
            })
        }
        Some('[') => {
            let reference_start = label_end + 2;
            cursor = reference_start;
            let reference_end = loop {
                let token = tokens.get(cursor)?;
                if token.ch == '\\' {
                    cursor += 2;
                    continue;
                }
                if token.ch == ']' {
                    break cursor;
                }
                cursor += 1;
            };

            let raw_label = tokens_to_string(&tokens[reference_start..reference_end]);
            let link_label = if raw_label.is_empty() {
                tokens_to_string(&tokens[bracket_index + 1..label_end])
            } else {
                raw_label
            };
            let normalized_label = crate::markdown::block::image::normalize_reference_label(&link_label)?;
            let LinkReferenceDefinition { destination, .. } =
                reference_definitions.get(&normalized_label)?.clone();
            Some(LocatedInlineLink {
                label_end,
                end_index: reference_end,
                link: InlineLink::Reference {
                    label: link_label,
                    destination,
                    is_image,
                },
            })
        }
        _ => {
            let raw_label = tokens_to_string(&tokens[bracket_index + 1..label_end]);
            let normalized_label = crate::markdown::block::image::normalize_reference_label(&raw_label)?;
            let LinkReferenceDefinition { destination, .. } =
                reference_definitions.get(&normalized_label)?.clone();
            Some(LocatedInlineLink {
                label_end,
                end_index: label_end,
                link: InlineLink::Reference {
                    label: raw_label,
                    destination,
                    is_image,
                },
            })
        }
    }
}

fn locate_autolink(tokens: &[CharToken], index: usize) -> Option<usize> {
    if tokens.get(index)?.ch != '<' {
        return None;
    }

    let mut cursor = index + 1;
    let end_index = loop {
        let token = tokens.get(cursor)?;
        if token.ch == '\\' {
            cursor += 2;
            continue;
        }
        if token.ch == '>' {
            break cursor;
        }
        cursor += 1;
    };

    let target = tokens_to_string(&tokens[index + 1..end_index]);
    (!target.is_empty() && !looks_like_non_autolink_html_tag(tokens, end_index, &target))
        .then_some(end_index)
}

fn tokens_to_string(tokens: &[CharToken]) -> String {
    tokens.iter().map(|token| token.ch).collect()
}

// ---------------------------------------------------------------------------
// HTML tag locating
// ---------------------------------------------------------------------------

fn locate_inline_html_open_tag(tokens: &[CharToken], index: usize) -> Option<InlineHtmlTag> {
    if tokens.get(index)?.ch != '<' {
        return None;
    }

    let mut cursor = index + 1;
    if !tokens.get(cursor)?.ch.is_ascii_alphabetic() {
        return None;
    }
    let name_start = cursor;
    while cursor < tokens.len() && is_html_tag_name_char(tokens[cursor].ch) {
        cursor += 1;
    }
    let name = tokens_to_string(&tokens[name_start..cursor]).to_ascii_lowercase();

    match tokens.get(cursor).map(|token| token.ch) {
        Some(ch) if ch.is_whitespace() || ch == '>' || ch == '/' => {}
        _ => return None,
    }

    let attrs_start = cursor;
    let mut quote = None;
    while cursor < tokens.len() {
        let ch = tokens[cursor].ch;
        if let Some(active_quote) = quote {
            if ch == active_quote {
                quote = None;
            }
            cursor += 1;
            continue;
        }

        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            cursor += 1;
            continue;
        }

        if ch == '>' {
            let attrs_source = tokens_to_string(&tokens[attrs_start..cursor]);
            let self_closing = attrs_source.trim_end().ends_with('/');
            return Some(InlineHtmlTag {
                name,
                attrs: parse_html_attrs(&attrs_source),
                end_index: cursor,
                self_closing,
            });
        }

        cursor += 1;
    }

    None
}

fn locate_inline_html_close_tag(
    tokens: &[CharToken],
    index: usize,
    expected_name: &str,
) -> Option<usize> {
    if tokens.get(index)?.ch != '<' || tokens.get(index + 1)?.ch != '/' {
        return None;
    }

    let mut cursor = index + 2;
    while tokens
        .get(cursor)
        .is_some_and(|token| token.ch.is_whitespace())
    {
        cursor += 1;
    }
    let name_start = cursor;
    while cursor < tokens.len() && is_html_tag_name_char(tokens[cursor].ch) {
        cursor += 1;
    }
    if name_start == cursor {
        return None;
    }
    let name = tokens_to_string(&tokens[name_start..cursor]).to_ascii_lowercase();
    if name != expected_name {
        return None;
    }
    while tokens
        .get(cursor)
        .is_some_and(|token| token.ch.is_whitespace())
    {
        cursor += 1;
    }
    (tokens.get(cursor)?.ch == '>').then_some(cursor)
}

fn locate_matching_inline_html_close(
    tokens: &[CharToken],
    mut cursor: usize,
    name: &str,
) -> Option<(usize, usize)> {
    let mut depth = 1usize;
    while cursor < tokens.len() {
        if tokens[cursor].ch != '<' {
            cursor += 1;
            continue;
        }

        if let Some(close_end) = locate_inline_html_close_tag(tokens, cursor, name) {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some((cursor, close_end));
            }
            cursor = close_end + 1;
            continue;
        }

        if let Some(open) = locate_inline_html_open_tag(tokens, cursor) {
            if open.name == name && !open.self_closing {
                depth += 1;
            }
            cursor = open.end_index + 1;
            continue;
        }

        cursor += 1;
    }

    None
}

fn looks_like_non_autolink_html_tag(tokens: &[CharToken], end_index: usize, target: &str) -> bool {
    let target = target.trim();
    if target.starts_with('/') {
        let rest = target.trim_start_matches('/').trim();
        return html_tag_name_with_attrs(rest).is_some();
    }

    if let Some((_tag_name, has_attrs_or_slash)) = html_tag_name_with_attrs(target)
        && has_attrs_or_slash
    {
        return true;
    }

    let Some((tag_name, _)) = html_tag_name_with_attrs(target) else {
        return false;
    };
    let rest = tokens_to_string(&tokens[end_index + 1..]).to_ascii_lowercase();
    let tag_name = tag_name.to_ascii_lowercase();
    rest.contains(&format!("</{tag_name}>"))
}

fn html_tag_name_with_attrs(target: &str) -> Option<(&str, bool)> {
    if target.is_empty() {
        return None;
    }

    let first = target.as_bytes()[0];
    if !first.is_ascii_alphabetic() {
        return None;
    }

    let mut end = 0usize;
    for (index, ch) in target.char_indices() {
        if is_html_tag_name_char(ch) {
            end = index + ch.len_utf8();
        } else {
            break;
        }
    }
    if end == 0 {
        return None;
    }

    let raw_rest = &target[end..];
    let rest = raw_rest.trim();
    if rest.is_empty() {
        return Some((&target[..end], false));
    }
    (raw_rest.chars().next().is_some_and(|ch| ch.is_whitespace())
        || rest == "/"
        || rest.starts_with('/'))
    .then_some((&target[..end], true))
}

fn is_html_tag_name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_')
}

fn apply_extra_style_to_fragments(
    fragments: &mut [InlineFragment],
    extra_style: InlineStyle,
    extra_html_style: Option<HtmlInlineStyle>,
) {
    for fragment in fragments {
        if extra_style.bold {
            fragment.style.bold = true;
            fragment.style.bold_marker = extra_style.bold_marker;
        }
        if extra_style.italic {
            fragment.style.italic = true;
            fragment.style.italic_marker = extra_style.italic_marker;
        }
        if extra_style.bold && extra_style.italic {
            fragment.style.italic_outer = extra_style.italic_outer;
        }
        if extra_style.underline {
            fragment.style.underline = true;
        }
        if extra_style.strikethrough {
            fragment.style.strikethrough = true;
        }
        if extra_style.code {
            fragment.style.code = true;
        }
        if extra_style.has_script() {
            fragment.style.script = extra_style.script;
        }
        fragment.set_html_style(merge_html_styles(extra_html_style, fragment.html_style()));
    }
}
pub(crate) fn locate_script_close(
    tokens: &[CharToken],
    mut cursor: usize,
    marker: char,
) -> Option<usize> {
    let body_start = cursor;
    while cursor < tokens.len() {
        if tokens[cursor].ch == '\\'
            && let Some(escaped_len) = escaped_sequence_token_len(tokens, cursor)
        {
            cursor += 1 + escaped_len;
            continue;
        }

        let is_close = if marker == '~' {
            is_single_tilde_delimiter(tokens, cursor)
        } else {
            tokens[cursor].ch == marker
        };
        if is_close {
            return valid_script_body(tokens, body_start, cursor).then_some(cursor);
        }

        cursor += 1;
    }

    None
}

fn valid_script_body(tokens: &[CharToken], start: usize, end: usize) -> bool {
    start < end
        && tokens[start..end]
            .iter()
            .all(|token| token.ch.is_ascii_alphanumeric())
}

pub(crate) fn is_single_tilde_delimiter(tokens: &[CharToken], index: usize) -> bool {
    tokens.get(index).is_some_and(|token| token.ch == '~')
        && index
            .checked_sub(1)
            .and_then(|prev| tokens.get(prev))
            .is_none_or(|token| token.ch != '~')
        && tokens.get(index + 1).is_none_or(|token| token.ch != '~')
}

pub(crate) fn matches_sequence(tokens: &[CharToken], index: usize, sequence: &str) -> bool {
    sequence
        .chars()
        .enumerate()
        .all(|(offset, ch)| tokens.get(index + offset).is_some_and(|t| t.ch == ch))
}

pub(crate) fn escaped_sequence_token_len(tokens: &[CharToken], index: usize) -> Option<usize> {
    let next_index = index + 1;
    if next_index >= tokens.len() {
        return None;
    }

    if matches_sequence(tokens, next_index, "</strong>") {
        Some(9)
    } else if matches_sequence(tokens, next_index, "<strong>") {
        Some(8)
    } else if matches_sequence(tokens, next_index, "</em>") {
        Some(5)
    } else if matches_sequence(tokens, next_index, "<em>") {
        Some(4)
    } else if matches_sequence(tokens, next_index, "</u>") {
        Some(4)
    } else if matches_sequence(tokens, next_index, "<u>") {
        Some(3)
    } else if tokens[next_index].ch.is_ascii_punctuation() {
        Some(1)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_intraword_underscores_as_literal_text() {
        let text = BlockText::from_markdown("foo_bar_baz");
        assert_eq!(text.plain_text(), "foo_bar_baz");
        assert!(!text.fragments[0].style.italic);
    }

    #[test]
    fn parses_standalone_underscore_as_italic() {
        let text = BlockText::from_markdown("_hello world_");
        assert_eq!(text.plain_text(), "hello world");
        assert!(text.fragments[0].style.italic);
    }

    #[test]
    fn parses_asterisk_emphasis_and_strong() {
        let text = BlockText::from_markdown("**bold** and *italic*");
        assert_eq!(text.plain_text(), "bold and italic");
        assert!(text.fragments[0].style.bold);
        assert!(!text.fragments[1].style.bold);
        assert!(text.fragments[2].style.italic);
    }

    #[test]
    fn parses_intraword_asterisk_as_emphasis() {
        // CommonMark §6.2 Example 365: foo*bar*baz -> foo<em>bar</em>baz
        let text = BlockText::from_markdown("foo*bar*baz");
        assert_eq!(text.plain_text(), "foobarbaz");
        assert_eq!(text.fragments[0].text, "foo");
        assert!(!text.fragments[0].style.italic);
        assert_eq!(text.fragments[1].text, "bar");
        assert!(text.fragments[1].style.italic);
        assert_eq!(text.fragments[2].text, "baz");
        assert!(!text.fragments[2].style.italic);
    }

    #[test]
    fn parses_all_ascii_punctuation_escapes() {
        let text = BlockText::from_markdown(r"\# \! \. \- \> \( \) \[ \] \~ \* \_ \\");
        assert_eq!(text.plain_text(), "# ! . - > ( ) [ ] ~ * _ \\");
        for fragment in &text.fragments {
            assert!(!fragment.style.bold);
            assert!(!fragment.style.italic);
        }
    }

    #[test]
    fn parses_highlight_markdown_and_mark_html() {
        let text = BlockText::from_markdown("这是 ==高亮文本== 结束");
        assert_eq!(text.plain_text(), "这是 高亮文本 结束");
        assert!(!text.fragments[0].style.highlight);
        assert!(text.fragments[1].style.highlight);
        assert_eq!(text.fragments[1].text, "高亮文本");
        assert!(!text.fragments[2].style.highlight);
        assert_eq!(text.serialize_markdown(), "这是 ==高亮文本== 结束");

        let mark_text = BlockText::from_markdown("这是 <mark>标记文本</mark> 结束");
        assert_eq!(mark_text.plain_text(), "这是 标记文本 结束");
        assert!(mark_text.fragments[1].style.highlight);
        assert_eq!(mark_text.fragments[1].text, "标记文本");
    }
}
