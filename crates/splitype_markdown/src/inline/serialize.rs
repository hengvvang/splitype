//! Markdown serialization for `BlockText`: reconstructs delimiter markers
//! from fragment styles using a Viterbi-like DP that avoids ambiguous runs.

use super::markdown::{
    CharToken, escaped_sequence_token_len, is_single_tilde_delimiter, locate_script_close,
    matches_sequence, token_is_backslash_escaped,
};
use super::offsets::SourceOffsetMap;
use super::style::{InlineScript, InlineStyle};
use super::text::InlineFragment;
use crate::inline::html::HtmlInlineStyle;

// ---------------------------------------------------------------------------
// Serializer helpers
// ---------------------------------------------------------------------------

pub(crate) fn serialize_fragment_run_with_offset_map(
    fragments: &[InlineFragment],
) -> SourceOffsetMap {
    if fragments.is_empty() {
        return SourceOffsetMap {
            source: String::new(),
            plain_to_source: vec![0],
            source_to_plain: vec![0],
        };
    }

    let stacks = choose_fragment_stacks(fragments);
    let mut output = String::new();
    let total_plain_len = fragments
        .iter()
        .map(|fragment| fragment.text.len())
        .sum::<usize>();
    let mut plain_to_source = vec![0; total_plain_len + 1];
    let mut source_to_plain = vec![0];
    let mut current_stack: Vec<Delimiter> = Vec::new();
    let mut current_html_style: Option<HtmlInlineStyle> = None;
    let mut plain_cursor = 0usize;

    for (fragment, next_stack) in fragments.iter().zip(stacks.iter()) {
        if current_html_style != fragment.html_style() {
            let transition = stack_transition_string(&current_stack, &[]);
            push_markdown_marker(&mut output, &mut source_to_plain, plain_cursor, &transition);
            current_stack.clear();

            if current_html_style.is_some() {
                push_markdown_marker(&mut output, &mut source_to_plain, plain_cursor, "</span>");
            }
            if let Some(style) = fragment.html_style()
                && let Some(marker) = html_style_open_marker(style)
            {
                push_markdown_marker(&mut output, &mut source_to_plain, plain_cursor, &marker);
            }
            current_html_style = fragment.html_style();
        }

        let transition = stack_transition_string(&current_stack, next_stack);
        let transition_start = output.len();
        output.push_str(&transition);
        source_to_plain.resize(output.len() + 1, plain_cursor);
        for local in 0..=transition.len() {
            source_to_plain[transition_start + local] = plain_cursor;
        }

        let escaped = if let Some(math) = fragment.math() {
            identity_text_with_offset_map(&math.source)
        } else if fragment.style.code {
            escape_code_span_text_with_offset_map(&fragment.text)
        } else {
            identity_text_with_offset_map(&fragment.text)
        };
        let escaped_start = output.len();
        output.push_str(escaped.source());
        for local_plain in 0..=fragment.text.len() {
            plain_to_source[plain_cursor + local_plain] =
                escaped_start + escaped.plain_to_source_offset(local_plain);
        }
        source_to_plain.resize(output.len() + 1, plain_cursor);
        for local_source in 0..=escaped.source().len() {
            source_to_plain[escaped_start + local_source] =
                plain_cursor + escaped.source_to_plain_offset(local_source);
        }
        plain_cursor += fragment.text.len();
        current_stack = next_stack.clone();
    }

    let transition = stack_transition_string(&current_stack, &[]);
    push_markdown_marker(&mut output, &mut source_to_plain, plain_cursor, &transition);
    if current_html_style.is_some() {
        push_markdown_marker(&mut output, &mut source_to_plain, plain_cursor, "</span>");
    }

    SourceOffsetMap {
        source: output,
        plain_to_source,
        source_to_plain,
    }
}

fn push_markdown_marker(
    output: &mut String,
    source_to_plain: &mut Vec<usize>,
    plain_cursor: usize,
    marker: &str,
) {
    if marker.is_empty() {
        return;
    }
    let marker_start = output.len();
    output.push_str(marker);
    source_to_plain.resize(output.len() + 1, plain_cursor);
    for local in 0..=marker.len() {
        source_to_plain[marker_start + local] = plain_cursor;
    }
}

fn identity_text_with_offset_map(text: &str) -> SourceOffsetMap {
    SourceOffsetMap {
        source: text.to_string(),
        plain_to_source: (0..=text.len()).collect(),
        source_to_plain: (0..=text.len()).collect(),
    }
}

fn html_style_open_marker(style: HtmlInlineStyle) -> Option<String> {
    style
        .to_css()
        .map(|css| format!("<span style=\"{}\">", escape_html_attr(&css)))
}

fn escape_html_attr(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '"' => escaped.push_str("&quot;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}
// ---------------------------------------------------------------------------
// Delimiter matching
// ---------------------------------------------------------------------------

/// Ordered preference of delimiter variants used by the DP serializer.
/// Lower rank = more preferred.  Markdown delimiters are preferred over HTML
/// because they are shorter and more idiomatic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Delimiter {
    /// Markdown bold marker using either `*` or `_`.
    BoldMarkdown { marker: char },
    /// Markdown italic marker using either `*` or `_`.
    ItalicMarkdown { marker: char },
    /// Markdown strikethrough marker `~~`.
    StrikethroughMarkdown,
    /// Markdown highlight marker `==`.
    HighlightMarkdown,
    /// Markdown superscript marker `^`.
    SuperscriptMarkdown,
    /// Markdown subscript marker `~`.
    SubscriptMarkdown,
    /// HTML underline marker `<u>`.
    Underline,
    /// HTML highlight marker `<mark>`.
    HighlightHtml,
    /// HTML superscript marker `<sup>`.
    SuperscriptHtml,
    /// HTML subscript marker `<sub>`.
    SubscriptHtml,
    /// HTML bold marker `<strong>`.
    BoldHtml,
    /// HTML italic marker `<em>`.
    ItalicHtml,
    /// Markdown code span marker using a selected backtick run length.
    CodeMarkdown { run_len: usize },
}

impl Delimiter {
    /// Returns the opening marker string.  For code spans this is `run_len`
    /// backticks; for emphasis it's `**`, `*`, `<u>`, etc.
    pub(crate) fn open(self) -> String {
        match self {
            Self::BoldMarkdown { marker } => marker.to_string().repeat(2),
            Self::ItalicMarkdown { marker } => marker.to_string(),
            Self::StrikethroughMarkdown => "~~".into(),
            Self::HighlightMarkdown => "==".into(),
            Self::SuperscriptMarkdown => "^".into(),
            Self::SubscriptMarkdown => "~".into(),
            Self::Underline => "<u>".into(),
            Self::HighlightHtml => "<mark>".into(),
            Self::SuperscriptHtml => "<sup>".into(),
            Self::SubscriptHtml => "<sub>".into(),
            Self::BoldHtml => "<strong>".into(),
            Self::ItalicHtml => "<em>".into(),
            Self::CodeMarkdown { run_len } => "`".repeat(run_len),
        }
    }

    pub(crate) fn close(self) -> String {
        match self {
            Self::BoldMarkdown { marker } => marker.to_string().repeat(2),
            Self::ItalicMarkdown { marker } => marker.to_string(),
            Self::StrikethroughMarkdown => "~~".into(),
            Self::HighlightMarkdown => "==".into(),
            Self::SuperscriptMarkdown => "^".into(),
            Self::SubscriptMarkdown => "~".into(),
            Self::Underline => "</u>".into(),
            Self::HighlightHtml => "</mark>".into(),
            Self::SuperscriptHtml => "</sup>".into(),
            Self::SubscriptHtml => "</sub>".into(),
            Self::BoldHtml => "</strong>".into(),
            Self::ItalicHtml => "</em>".into(),
            Self::CodeMarkdown { run_len } => "`".repeat(run_len),
        }
    }

    pub(crate) fn token_len(self) -> usize {
        match self {
            Self::CodeMarkdown { run_len } => run_len,
            other => other.open().chars().count(),
        }
    }

    fn preference_rank(self) -> u8 {
        match self {
            Self::BoldMarkdown { .. } => 0,
            Self::Underline => 1,
            Self::StrikethroughMarkdown => 2,
            Self::HighlightMarkdown => 3,
            Self::SuperscriptMarkdown | Self::SubscriptMarkdown => 4,
            Self::ItalicMarkdown { .. } => 5,
            Self::HighlightHtml => 6,
            Self::SuperscriptHtml | Self::SubscriptHtml => 7,
            Self::BoldHtml => 8,
            Self::ItalicHtml => 9,
            Self::CodeMarkdown { .. } => 10,
        }
    }

    pub(crate) fn is_html(self) -> bool {
        matches!(
            self,
            Self::BoldHtml
                | Self::ItalicHtml
                | Self::HighlightHtml
                | Self::SuperscriptHtml
                | Self::SubscriptHtml
        )
    }
}

pub(crate) fn match_open_delimiter(tokens: &[CharToken], index: usize) -> Option<Delimiter> {
    if matches_sequence(tokens, index, "<strong>") {
        Some(Delimiter::BoldHtml)
    } else if matches_sequence(tokens, index, "<em>") {
        Some(Delimiter::ItalicHtml)
    } else if matches_sequence(tokens, index, "<u>") {
        Some(Delimiter::Underline)
    } else if matches_sequence(tokens, index, "<mark>") {
        Some(Delimiter::HighlightHtml)
    } else if matches_sequence(tokens, index, "~~") {
        Some(Delimiter::StrikethroughMarkdown)
    } else if matches_sequence(tokens, index, "==") && can_open_emphasis(tokens, index, 2, '=') {
        Some(Delimiter::HighlightMarkdown)
    } else if matches_sequence(tokens, index, "^") && can_open_script(tokens, index, '^') {
        Some(Delimiter::SuperscriptMarkdown)
    } else if is_single_tilde_delimiter(tokens, index) && can_open_script(tokens, index, '~') {
        Some(Delimiter::SubscriptMarkdown)
    } else if matches_sequence(tokens, index, "**") && can_open_emphasis(tokens, index, 2, '*') {
        Some(Delimiter::BoldMarkdown { marker: '*' })
    } else if matches_sequence(tokens, index, "__") && can_open_emphasis(tokens, index, 2, '_') {
        Some(Delimiter::BoldMarkdown { marker: '_' })
    } else if matches_sequence(tokens, index, "*") && can_open_emphasis(tokens, index, 1, '*') {
        Some(Delimiter::ItalicMarkdown { marker: '*' })
    } else if matches_sequence(tokens, index, "_") && can_open_emphasis(tokens, index, 1, '_') {
        Some(Delimiter::ItalicMarkdown { marker: '_' })
    } else if tokens[index].ch == '`' {
        // Count the run of consecutive backticks.
        let run_len = backtick_run_len(tokens, index);
        // A backtick run is only a valid opener if it is NOT immediately
        // followed by another backtick (no double-counting).
        if run_len > 0 {
            Some(Delimiter::CodeMarkdown { run_len })
        } else {
            None
        }
    } else {
        None
    }
}

/// Returns the length of the consecutive backtick run starting at `index`.
pub(crate) fn backtick_run_len(tokens: &[CharToken], index: usize) -> usize {
    let mut len = 0;
    while index + len < tokens.len() && tokens[index + len].ch == '`' {
        len += 1;
    }
    // A backtick run is only valid if it's not immediately preceded by an
    // additional backtick (the run must start at `index`).
    if index > 0 && tokens[index - 1].ch == '`' {
        return 0;
    }
    len
}

pub(crate) fn has_closing_delimiter(
    tokens: &[CharToken],
    index: usize,
    delimiter: Delimiter,
) -> bool {
    let skip = delimiter.token_len();
    let close_str = delimiter.close();

    // For code spans we look for a matching-length backtick run;
    // for emphasis we just scan for the close string.
    if let Delimiter::CodeMarkdown { .. } = delimiter {
        let mut cursor = index + skip;
        while cursor < tokens.len() {
            if tokens[cursor].ch == '\\'
                && let Some(escaped_len) = escaped_sequence_token_len(tokens, cursor)
            {
                cursor += 1 + escaped_len;
                continue;
            }

            if tokens[cursor].ch == '`' && backtick_run_len(tokens, cursor) == skip {
                return true;
            }

            cursor += 1;
        }
        return false;
    }

    if matches!(
        delimiter,
        Delimiter::SuperscriptMarkdown | Delimiter::SubscriptMarkdown
    ) {
        let marker = match delimiter {
            Delimiter::SuperscriptMarkdown => '^',
            Delimiter::SubscriptMarkdown => '~',
            _ => unreachable!(),
        };
        return locate_script_close(tokens, index + skip, marker).is_some();
    }

    let body_start = index + skip;
    let requires_body = emphasis_requires_body(delimiter);
    let mut cursor = body_start;
    while cursor < tokens.len() {
        if tokens[cursor].ch == '\\'
            && let Some(escaped_len) = escaped_sequence_token_len(tokens, cursor)
        {
            cursor += 1 + escaped_len;
            continue;
        }

        if matches_sequence(tokens, cursor, &close_str) {
            // Emphasis spans must enclose at least one character; a close
            // sitting immediately after the open (e.g. `**` or `*` `*`) is an
            // empty span and is treated as literal text instead.
            if requires_body && cursor == body_start {
                cursor += 1;
                continue;
            }
            let close_len = close_str.chars().count();
            let marker = close_str.chars().next().unwrap_or('*');
            if can_close_emphasis(tokens, cursor, close_len, marker) {
                return true;
            }
        }

        cursor += 1;
    }

    false
}

/// Whether `delimiter` requires a non-empty body. Emphasis and strikethrough
/// markers must enclose at least one character; code spans may be empty and
/// script markers already constrain their bodies elsewhere.
pub(crate) fn emphasis_requires_body(delimiter: Delimiter) -> bool {
    matches!(
        delimiter,
        Delimiter::BoldMarkdown { .. }
            | Delimiter::ItalicMarkdown { .. }
            | Delimiter::StrikethroughMarkdown
            | Delimiter::BoldHtml
            | Delimiter::ItalicHtml
            | Delimiter::Underline
    )
}

fn escape_code_span_text_with_offset_map(text: &str) -> SourceOffsetMap {
    let needs_padding = !text.is_empty()
        && !text.chars().all(|ch| ch == ' ')
        && (text.starts_with([' ', '`']) || text.ends_with([' ', '`']));
    let leading_padding = usize::from(needs_padding);

    let mut source = String::new();
    if needs_padding {
        source.push(' ');
    }
    source.push_str(text);
    if needs_padding {
        source.push(' ');
    }

    let mut plain_to_source = vec![0; text.len() + 1];
    for (plain, source_offset) in plain_to_source.iter_mut().enumerate() {
        *source_offset = leading_padding + plain;
    }

    let content_start = leading_padding;
    let content_end = leading_padding + text.len();
    let mut source_to_plain = vec![0; source.len() + 1];
    for (source_offset, plain) in source_to_plain.iter_mut().enumerate() {
        *plain = if source_offset <= content_start {
            0
        } else if source_offset >= content_end {
            text.len()
        } else {
            source_offset - content_start
        };
    }

    SourceOffsetMap {
        source,
        plain_to_source,
        source_to_plain,
    }
}

// ---------------------------------------------------------------------------
// DP optimization for delimiter choice
// ---------------------------------------------------------------------------

/// Viterbi-like DP that picks the optimal delimiter stack for each fragment.
///
/// Each fragment's style can be expressed with either Markdown or HTML
/// delimiters.  We minimize the total number of delimiter characters written
/// plus a penalty for HTML variants.  A large penalty is added when a
/// transition would produce 4+ consecutive `*` characters (Markdown ambiguity).
fn choose_fragment_stacks(fragments: &[InlineFragment]) -> Vec<Vec<Delimiter>> {
    // Enumerate the 1-2 possible delimiter stacks for each fragment's style.
    let variants = fragments
        .iter()
        .enumerate()
        .map(|(index, fragment)| {
            stack_variants(
                fragment,
                index.checked_sub(1).and_then(|i| fragments.get(i)),
            )
        })
        .collect::<Vec<_>>();

    // DP table: costs[fragment_index][choice_index]
    let mut costs: Vec<Vec<usize>> = variants
        .iter()
        .map(|choices| vec![usize::MAX; choices.len()])
        .collect();
    let mut previous_choice: Vec<Vec<Option<usize>>> = variants
        .iter()
        .map(|choices| vec![None; choices.len()])
        .collect();

    // Initial fragment: cost from empty stack to each variant.
    for (choice_index, stack) in variants[0].iter().enumerate() {
        costs[0][choice_index] = stack_transition_cost(&[], stack) + stack_variant_penalty(stack);
    }

    // Forward pass: compute minimum cost for each fragment's choices.
    for fragment_index in 1..variants.len() {
        for (choice_index, stack) in variants[fragment_index].iter().enumerate() {
            for (prev_index, prev_stack) in variants[fragment_index - 1].iter().enumerate() {
                let prev_cost = costs[fragment_index - 1][prev_index];
                if prev_cost == usize::MAX {
                    continue;
                }

                let cost = prev_cost
                    + stack_transition_cost(prev_stack, stack)
                    + stack_variant_penalty(stack);
                if cost < costs[fragment_index][choice_index] {
                    costs[fragment_index][choice_index] = cost;
                    previous_choice[fragment_index][choice_index] = Some(prev_index);
                }
            }
        }
    }

    // Backtrack: choose the best final stack and trace back through the DP.
    let last_fragment_index = variants.len() - 1;
    let (mut best_choice, _) = variants[last_fragment_index]
        .iter()
        .enumerate()
        .map(|(choice_index, stack)| {
            (
                choice_index,
                costs[last_fragment_index][choice_index] + stack_transition_cost(stack, &[]),
            )
        })
        .min_by(|(left_index, left_cost), (right_index, right_cost)| {
            left_cost.cmp(right_cost).then_with(|| {
                stack_preference_key(&variants[last_fragment_index][*left_index]).cmp(
                    &stack_preference_key(&variants[last_fragment_index][*right_index]),
                )
            })
        })
        .unwrap_or((0, 0));

    let mut chosen = vec![Vec::new(); variants.len()];
    for fragment_index in (0..variants.len()).rev() {
        chosen[fragment_index] = variants[fragment_index][best_choice].clone();
        if let Some(prev_index) = previous_choice[fragment_index][best_choice] {
            best_choice = prev_index;
        }
    }

    chosen
}

fn stack_variants(
    fragment: &InlineFragment,
    previous_fragment: Option<&InlineFragment>,
) -> Vec<Vec<Delimiter>> {
    let style = fragment.style;
    let code_run_len = style.code.then(|| code_delimiter_run_len(&fragment.text));
    let mut markdown_stack = Vec::new();

    let bold_delim = Delimiter::BoldMarkdown {
        marker: style.bold_marker.char(),
    };
    let italic_delim = Delimiter::ItalicMarkdown {
        marker: style.italic_marker.char(),
    };

    if style.italic_outer {
        if style.italic {
            markdown_stack.push(italic_delim);
        }
        if style.underline {
            markdown_stack.push(Delimiter::Underline);
        }
        if style.strikethrough {
            markdown_stack.push(Delimiter::StrikethroughMarkdown);
        }
        if style.highlight {
            markdown_stack.push(Delimiter::HighlightMarkdown);
        }
        match style.script {
            InlineScript::Normal => {}
            InlineScript::Superscript
                if can_use_markdown_script_delimiters(previous_fragment, fragment) =>
            {
                markdown_stack.push(Delimiter::SuperscriptMarkdown)
            }
            InlineScript::Superscript => markdown_stack.push(Delimiter::SuperscriptHtml),
            InlineScript::Subscript
                if style.strikethrough
                    || !can_use_markdown_script_delimiters(previous_fragment, fragment) =>
            {
                markdown_stack.push(Delimiter::SubscriptHtml)
            }
            InlineScript::Subscript => markdown_stack.push(Delimiter::SubscriptMarkdown),
        }
        if style.bold {
            markdown_stack.push(bold_delim);
        }
    } else {
        if style.bold {
            markdown_stack.push(bold_delim);
        }
        if style.underline {
            markdown_stack.push(Delimiter::Underline);
        }
        if style.strikethrough {
            markdown_stack.push(Delimiter::StrikethroughMarkdown);
        }
        if style.highlight {
            markdown_stack.push(Delimiter::HighlightMarkdown);
        }
        match style.script {
            InlineScript::Normal => {}
            InlineScript::Superscript
                if can_use_markdown_script_delimiters(previous_fragment, fragment) =>
            {
                markdown_stack.push(Delimiter::SuperscriptMarkdown)
            }
            InlineScript::Superscript => markdown_stack.push(Delimiter::SuperscriptHtml),
            InlineScript::Subscript
                if style.strikethrough
                    || !can_use_markdown_script_delimiters(previous_fragment, fragment) =>
            {
                markdown_stack.push(Delimiter::SubscriptHtml)
            }
            InlineScript::Subscript => markdown_stack.push(Delimiter::SubscriptMarkdown),
        }
        if style.italic {
            markdown_stack.push(italic_delim);
        }
    }
    // Code is always the innermost delimiter so it nests inside emphasis.
    if let Some(run_len) = code_run_len {
        markdown_stack.push(Delimiter::CodeMarkdown { run_len });
    }

    let has_emphasis = style.bold || style.italic;
    if !has_emphasis {
        return vec![markdown_stack];
    }

    let mut html_stack = Vec::new();
    if style.italic_outer {
        if style.italic {
            html_stack.push(Delimiter::ItalicHtml);
        }
        if style.underline {
            html_stack.push(Delimiter::Underline);
        }
        if style.strikethrough {
            html_stack.push(Delimiter::StrikethroughMarkdown);
        }
        if style.highlight {
            html_stack.push(Delimiter::HighlightHtml);
        }
        match style.script {
            InlineScript::Normal => {}
            InlineScript::Superscript => html_stack.push(Delimiter::SuperscriptHtml),
            InlineScript::Subscript => html_stack.push(Delimiter::SubscriptHtml),
        }
        if style.bold {
            html_stack.push(Delimiter::BoldHtml);
        }
    } else {
        if style.bold {
            html_stack.push(Delimiter::BoldHtml);
        }
        if style.underline {
            html_stack.push(Delimiter::Underline);
        }
        if style.strikethrough {
            html_stack.push(Delimiter::StrikethroughMarkdown);
        }
        if style.highlight {
            html_stack.push(Delimiter::HighlightHtml);
        }
        match style.script {
            InlineScript::Normal => {}
            InlineScript::Superscript => html_stack.push(Delimiter::SuperscriptHtml),
            InlineScript::Subscript => html_stack.push(Delimiter::SubscriptHtml),
        }
        if style.italic {
            html_stack.push(Delimiter::ItalicHtml);
        }
    }
    if let Some(run_len) = code_run_len {
        html_stack.push(Delimiter::CodeMarkdown { run_len });
    }

    vec![markdown_stack, html_stack]
}

pub fn can_use_markdown_script_delimiters(
    previous_fragment: Option<&InlineFragment>,
    fragment: &InlineFragment,
) -> bool {
    // This guard is shared by serialization and inline projection. Markdown
    // script markers need a plain ASCII owner immediately before the script
    // fragment; otherwise we fall back to <sup>/<sub> so the next parse sees
    // the same style boundary.
    let Some(previous) = previous_fragment else {
        return false;
    };
    if previous.style.has_script() {
        return false;
    }
    previous
        .text
        .chars()
        .next_back()
        .is_some_and(|ch| ch.is_ascii_alphanumeric())
        && previous.extra == fragment.extra
        && previous.footnote().is_none()
        && fragment.footnote().is_none()
        && previous.math().is_none()
        && fragment.math().is_none()
        && styles_match_ignoring_script(previous.style, fragment.style)
}

fn styles_match_ignoring_script(left: InlineStyle, right: InlineStyle) -> bool {
    left.bold == right.bold
        && left.bold_marker == right.bold_marker
        && left.italic == right.italic
        && left.italic_marker == right.italic_marker
        && left.italic_outer == right.italic_outer
        && left.underline == right.underline
        && left.strikethrough == right.strikethrough
        && left.highlight == right.highlight
        && left.code == right.code
}

fn code_delimiter_run_len(text: &str) -> usize {
    let mut longest = 0usize;
    let mut current = 0usize;
    for ch in text.chars() {
        if ch == '`' {
            current += 1;
            longest = longest.max(current);
        } else {
            current = 0;
        }
    }
    longest + 1
}

fn stack_transition_len(from: &[Delimiter], to: &[Delimiter]) -> usize {
    let common = common_prefix_len(from, to);
    let close_len = from[common..]
        .iter()
        .rev()
        .map(|delimiter| delimiter.close().len())
        .sum::<usize>();
    let open_len = to[common..]
        .iter()
        .map(|delimiter| delimiter.open().len())
        .sum::<usize>();
    close_len + open_len
}

/// Cost of closing `from` delimiters and opening `to` delimiters in sequence.
/// Adds a heavy penalty if the resulting string would contain 4+ consecutive
/// `*` characters, which Markdown parsers may interpret ambiguously.
fn stack_transition_cost(from: &[Delimiter], to: &[Delimiter]) -> usize {
    let marker_len = stack_transition_len(from, to);
    let marker_string = stack_transition_string(from, to);
    let ambiguity_penalty =
        if !from.is_empty() && !to.is_empty() && longest_star_run(&marker_string) >= 4 {
            1_000
        } else {
            0
        };
    marker_len + ambiguity_penalty
}

fn stack_variant_penalty(stack: &[Delimiter]) -> usize {
    if stack.iter().any(|delimiter| delimiter.is_html()) {
        64
    } else {
        0
    }
}

fn write_stack_transition(output: &mut String, from: &[Delimiter], to: &[Delimiter]) {
    let common = common_prefix_len(from, to);
    for delimiter in from[common..].iter().rev() {
        output.push_str(&delimiter.close());
    }
    for delimiter in &to[common..] {
        output.push_str(&delimiter.open());
    }
}

fn stack_transition_string(from: &[Delimiter], to: &[Delimiter]) -> String {
    let mut output = String::new();
    write_stack_transition(&mut output, from, to);
    output
}

fn common_prefix_len(left: &[Delimiter], right: &[Delimiter]) -> usize {
    let mut index = 0;
    while index < left.len() && index < right.len() && left[index] == right[index] {
        index += 1;
    }
    index
}

fn stack_preference_key(stack: &[Delimiter]) -> Vec<u8> {
    stack
        .iter()
        .map(|delimiter| delimiter.preference_rank())
        .collect()
}

fn longest_star_run(text: &str) -> usize {
    let mut max_run = 0;
    let mut current_run = 0;
    for ch in text.chars() {
        if ch == '*' {
            current_run += 1;
            max_run = max_run.max(current_run);
        } else {
            current_run = 0;
        }
    }
    max_run
}

// ---------------------------------------------------------------------------
// Common helpers
// ---------------------------------------------------------------------------

pub fn clamp_to_char_boundary(text: &str, offset: usize) -> usize {
    let clamped = offset.min(text.len());
    if text.is_char_boundary(clamped) {
        return clamped;
    }

    let mut boundary = clamped;
    while boundary > 0 && !text.is_char_boundary(boundary) {
        boundary -= 1;
    }
    boundary
}

/// Returns true if `ch` is a Unicode punctuation character (CommonMark §2.1).
pub(crate) fn is_unicode_punctuation(ch: char) -> bool {
    ch.is_ascii_punctuation()
        || matches!(
            ch,
            '\u{00A1}'..='\u{00BF}'
                | '\u{2000}'..='\u{206F}'
                | '\u{2E00}'..='\u{2E7F}'
                | '\u{3000}'..='\u{303F}'
                | '\u{FF00}'..='\u{FFEF}'
        )
}

/// A delimiter run is left-flanking (CommonMark §6.2) iff it is not followed by
/// Unicode whitespace, and either:
/// 1. not followed by a Unicode punctuation character, OR
/// 2. followed by a Unicode punctuation character and preceded by Unicode whitespace or punctuation.
pub(crate) fn is_left_flanking(prev_char: Option<char>, next_char: Option<char>) -> bool {
    let Some(next) = next_char else {
        return false;
    };
    if next.is_whitespace() {
        return false;
    }
    if !is_unicode_punctuation(next) {
        return true;
    }
    match prev_char {
        None => true,
        Some(prev) => prev.is_whitespace() || is_unicode_punctuation(prev),
    }
}

/// A delimiter run is right-flanking (CommonMark §6.2) iff it is not preceded by
/// Unicode whitespace, and either:
/// 1. not preceded by a Unicode punctuation character, OR
/// 2. preceded by a Unicode punctuation character and followed by Unicode whitespace or punctuation.
pub(crate) fn is_right_flanking(prev_char: Option<char>, next_char: Option<char>) -> bool {
    let Some(prev) = prev_char else {
        return false;
    };
    if prev.is_whitespace() {
        return false;
    }
    if !is_unicode_punctuation(prev) {
        return true;
    }
    match next_char {
        None => true,
        Some(next) => next.is_whitespace() || is_unicode_punctuation(next),
    }
}

/// Determines whether a delimiter run starting at `index` of length `len` with `marker`
/// can open emphasis according to CommonMark §6.2.
pub(crate) fn can_open_emphasis(
    tokens: &[CharToken],
    index: usize,
    len: usize,
    marker: char,
) -> bool {
    let prev_char = if index > 0 {
        tokens.get(index - 1).map(|t| t.ch)
    } else {
        None
    };
    let next_char = tokens.get(index + len).map(|t| t.ch);

    let left = is_left_flanking(prev_char, next_char);
    let right = is_right_flanking(prev_char, next_char);

    if marker == '*' {
        left
    } else if marker == '_' {
        // A _ delimiter run can open emphasis iff it is left-flanking and:
        // not right-flanking, OR (right-flanking and preceded by a Unicode punctuation character).
        left && (!right || prev_char.is_some_and(is_unicode_punctuation))
    } else {
        left
    }
}

pub(crate) fn can_open_script(tokens: &[CharToken], index: usize, marker: char) -> bool {
    if token_is_backslash_escaped(tokens, index) {
        return false;
    }

    if marker == '~' && !is_single_tilde_delimiter(tokens, index) {
        return false;
    }

    index > 0
        && tokens[index - 1].ch.is_ascii_alphanumeric()
        && tokens
            .get(index + 1)
            .is_some_and(|token| token.ch.is_ascii_alphanumeric())
}

/// Determines whether a delimiter run at `index` with length `len` and `marker`
/// can close emphasis according to CommonMark §6.2.
pub(crate) fn can_close_emphasis(
    tokens: &[CharToken],
    index: usize,
    len: usize,
    marker: char,
) -> bool {
    let prev_char = if index > 0 {
        tokens.get(index - 1).map(|t| t.ch)
    } else {
        None
    };
    let next_char = tokens.get(index + len).map(|t| t.ch);

    let left = is_left_flanking(prev_char, next_char);
    let right = is_right_flanking(prev_char, next_char);

    if marker == '*' {
        right
    } else if marker == '_' {
        // A _ delimiter run can close emphasis iff it is right-flanking and:
        // not left-flanking, OR (left-flanking and followed by a Unicode punctuation character).
        right && (!left || next_char.is_some_and(is_unicode_punctuation))
    } else {
        right
    }
}

pub(crate) fn apply_delimiter_style(style: InlineStyle, delimiter: Delimiter) -> InlineStyle {
    match delimiter {
        Delimiter::BoldMarkdown { marker } => {
            let mut s = style.with_bold_char(marker);
            if style.italic {
                s.italic_outer = true;
            }
            s
        }
        Delimiter::BoldHtml => style.with_bold(),
        Delimiter::ItalicMarkdown { marker } => {
            let mut s = style.with_italic_char(marker);
            if !style.bold {
                s.italic_outer = true;
            }
            s
        }
        Delimiter::ItalicHtml => style.with_italic(),
        Delimiter::Underline => style.with_underline(),
        Delimiter::StrikethroughMarkdown => style.with_strikethrough(),
        Delimiter::HighlightMarkdown | Delimiter::HighlightHtml => style.with_highlight(),
        Delimiter::CodeMarkdown { .. } => style.with_code(),
        Delimiter::SuperscriptMarkdown | Delimiter::SuperscriptHtml => style.with_superscript(),
        Delimiter::SubscriptMarkdown | Delimiter::SubscriptHtml => style.with_subscript(),
    }
}

