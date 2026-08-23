//! Inline projection engine for editable Markdown delimiters.

use std::ops::Range;

use crate::model::inline::footnote::InlineFootnoteReference;
use crate::model::inline::link::InlineLink;
use crate::model::inline::render_cache::InlineRenderCache;
use crate::model::inline::serialize::can_use_markdown_script_delimiters;
use crate::model::inline::style::{InlineScript, InlineStyle};
use crate::model::inline::text::{BlockText, InlineFragment};

use crate::editor::tree::block::CollapsedCaretAffinity;

/// One displayed segment in an expanded inline projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExpandedInlineSegment {
    pub(crate) display_range: Range<usize>,
    pub(crate) plain_range: Range<usize>,
    pub(crate) fragment_index: usize,
    pub(crate) link_group: Option<usize>,
    pub(crate) kind: ExpandedInlineSegmentKind,
}

/// Inline construct whose Markdown delimiters can be projected for editing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExpandedInlineKind {
    /// Link label and target syntax.
    Link,
    /// Bold Markdown delimiters.
    BoldMarkdown { marker: char },
    /// Italic Markdown delimiters.
    ItalicMarkdown { marker: char },
    /// Strikethrough delimiters.
    Strikethrough,
    /// Code span backtick delimiters.
    Code,
    /// Superscript Markdown delimiters.
    SuperscriptMarkdown,
    /// Superscript HTML delimiters.
    SuperscriptHtml,
    /// Subscript Markdown delimiters.
    SubscriptMarkdown,
    /// Subscript HTML delimiters.
    SubscriptHtml,
}

impl ExpandedInlineKind {
    fn applies_to(self, style: InlineStyle) -> bool {
        match self {
            Self::Link => false,
            Self::BoldMarkdown { .. } => style.bold,
            Self::ItalicMarkdown { .. } => style.italic,
            Self::Strikethrough => style.strikethrough,
            Self::Code => style.code,
            Self::SuperscriptMarkdown | Self::SuperscriptHtml => {
                style.script == InlineScript::Superscript
            }
            Self::SubscriptMarkdown | Self::SubscriptHtml => {
                style.script == InlineScript::Subscript
            }
        }
    }

    fn open_marker(self) -> &'static str {
        match self {
            Self::Link => "[",
            Self::BoldMarkdown { marker: '*' } => "**",
            Self::BoldMarkdown { marker: '_' } => "__",
            Self::ItalicMarkdown { marker: '*' } => "*",
            Self::ItalicMarkdown { marker: '_' } => "_",
            Self::Strikethrough => "~~",
            Self::Code => "`",
            Self::SuperscriptMarkdown => "^",
            Self::SuperscriptHtml => "<sup>",
            Self::SubscriptMarkdown => "~",
            Self::SubscriptHtml => "<sub>",
            _ => "**",
        }
    }

    fn close_marker(self) -> &'static str {
        match self {
            Self::Link => ")",
            Self::SuperscriptHtml => "</sup>",
            Self::SubscriptHtml => "</sub>",
            _ => self.open_marker(),
        }
    }
}

/// Display role of one projected inline segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExpandedInlineSegmentKind {
    /// Editable block-level syntax such as an ATX heading prefix.
    BlockPrefix,
    /// Text with no projected inline syntax.
    PlainText,
    /// Text carrying projected style.
    StyledText,
    /// Opening delimiter such as `[` or backticks.
    OpeningDelimiter(ExpandedInlineKind),
    /// Middle delimiter such as `](` for links.
    MiddleDelimiter(ExpandedInlineKind),
    /// Editable link target text.
    LinkTargetText,
    /// Editable footnote id text.
    FootnoteIdText,
    /// Closing delimiter such as `)` or backticks.
    ClosingDelimiter(ExpandedInlineKind),
}

/// One projected link span spanning one or more inline fragments.
#[derive(Clone, Debug)]
pub(crate) struct ExpandedLinkSpan {
    pub(crate) link: InlineLink,
    pub(crate) start_fragment_index: usize,
    pub(crate) end_fragment_index: usize,
    pub(crate) plain_range: Range<usize>,
    pub(crate) display_range: Range<usize>,
    pub(crate) target_display_range: Range<usize>,
}

/// One projected footnote reference span.
#[derive(Clone, Debug)]
pub(crate) struct ExpandedFootnoteSpan {
    pub(crate) footnote: InlineFootnoteReference,
    pub(crate) plain_range: Range<usize>,
    pub(crate) display_range: Range<usize>,
}

/// Selection snapshot translated into an expanded link display range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProjectedLinkSelectionSnapshot {
    pub(crate) plain_range: Range<usize>,
    pub(crate) display_relative_range: Range<usize>,
    pub(crate) selection_reversed: bool,
}

/// Render cache and offset maps for an expanded inline projection.
#[derive(Clone, Debug)]
pub(crate) struct ExpandedInlineProjection {
    pub(crate) cache: InlineRenderCache,
    pub(crate) segments: Vec<ExpandedInlineSegment>,
    pub(crate) block_prefix_range: Option<Range<usize>>,
    pub(crate) plain_to_display_cursor: Vec<usize>,
    pub(crate) display_to_plain: Vec<usize>,
    pub(crate) link_spans: Vec<ExpandedLinkSpan>,
    pub(crate) footnote_spans: Vec<ExpandedFootnoteSpan>,
}

impl ExpandedInlineProjection {
    // Projection is a temporary editing view over plain inline fragments. It
    // exposes delimiters only for the fragment touched by the caret, selection,
    // or IME marked range, while preserving maps back to plain text offsets.

    pub(crate) fn build_with_prefix(
        fragments: &[InlineFragment],
        plain_selected: Range<usize>,
        plain_marked: Option<Range<usize>>,
        block_prefix: Option<&str>,
        footnote_head_len: Option<usize>,
    ) -> Option<Self> {
        let plain_len = fragments
            .iter()
            .map(|fragment| fragment.text.len())
            .sum::<usize>();
        let mut projected_fragments = Vec::new();
        let mut segments = Vec::new();
        let mut plain_to_display_cursor = vec![0; plain_len + 1];
        let mut display_to_plain = vec![0];
        let mut link_spans = Vec::new();
        let mut footnote_spans = Vec::new();
        let mut plain_cursor = 0usize;
        let mut display_cursor = 0usize;
        let mut any_expanded = false;
        let mut block_prefix_range = None;
        let mut fragment_index = 0usize;
        let mut footnote_head_closed = footnote_head_len.is_none();

        if let Some(prefix) = block_prefix.filter(|prefix| !prefix.is_empty()) {
            let prefix_len = prefix.len();
            projected_fragments.push(InlineFragment {
                text: prefix.to_string(),
                style: InlineStyle::default(),
                html_style: None,
                link: None,
                footnote: None,
                math: None,
            });
            segments.push(ExpandedInlineSegment {
                display_range: 0..prefix_len,
                plain_range: 0..0,
                fragment_index: 0,
                link_group: None,
                kind: ExpandedInlineSegmentKind::BlockPrefix,
            });
            display_to_plain.extend(std::iter::repeat_n(0, prefix_len));
            plain_to_display_cursor.fill(prefix_len);
            display_cursor = prefix_len;
            // `block_prefix_range` is the heading-prefix edit target. Footnote
            // definitions also use `block_prefix` for their `[^` marker, but
            // that marker must not be treated as a heading prefix, so leave
            // the range unset for them.
            if footnote_head_len.is_none() {
                block_prefix_range = Some(0..prefix_len);
            }
            any_expanded = true;
        }

        while fragment_index < fragments.len() {
            let fragment = &fragments[fragment_index];
            let fragment_len = fragment.text.len();
            if fragment_len == 0 {
                fragment_index += 1;
                continue;
            }

            if let Some(footnote) = fragment.footnote.as_ref() {
                let plain_range = plain_cursor..plain_cursor + fragment_len;
                let expand_footnote = true;
                let span_display_start = display_cursor;
                if expand_footnote {
                    any_expanded = true;
                    let open_marker = "[^".to_string();
                    let open_len = open_marker.len();
                    projected_fragments.push(InlineFragment {
                        text: open_marker,
                        style: fragment.style,
                        html_style: None,
                        link: None,
                        footnote: None,
                        math: None,
                    });
                    segments.push(ExpandedInlineSegment {
                        display_range: display_cursor..display_cursor + open_len,
                        plain_range: plain_range.start..plain_range.start,
                        fragment_index,
                        link_group: None,
                        kind: ExpandedInlineSegmentKind::OpeningDelimiter(ExpandedInlineKind::Link),
                    });
                    for _ in 0..open_len {
                        display_to_plain.push(plain_range.start);
                    }
                    display_cursor += open_len;

                    let id_text = footnote.id.clone();
                    let id_len = id_text.len();
                    projected_fragments.push(InlineFragment {
                        text: id_text,
                        style: fragment.style,
                        html_style: fragment.html_style,
                        link: None,
                        footnote: Some(footnote.clone()),
                        math: None,
                    });
                    segments.push(ExpandedInlineSegment {
                        display_range: display_cursor..display_cursor + id_len,
                        plain_range: plain_range.clone(),
                        fragment_index,
                        link_group: None,
                        kind: ExpandedInlineSegmentKind::FootnoteIdText,
                    });
                    for offset in 0..=fragment_len {
                        let mapped = if fragment_len == 0 {
                            0
                        } else {
                            (id_len * offset) / fragment_len
                        };
                        plain_to_display_cursor[plain_range.start + offset] =
                            display_cursor + mapped;
                    }
                    for offset in 1..=id_len {
                        let mapped = if id_len == 0 {
                            0
                        } else {
                            (fragment_len * offset) / id_len
                        };
                        display_to_plain.push(plain_range.start + mapped);
                    }
                    display_cursor += id_len;
                    let close_marker = "]".to_string();
                    let close_len = close_marker.len();
                    projected_fragments.push(InlineFragment {
                        text: close_marker,
                        style: fragment.style,
                        html_style: None,
                        link: None,
                        footnote: None,
                        math: None,
                    });
                    segments.push(ExpandedInlineSegment {
                        display_range: display_cursor..display_cursor + close_len,
                        plain_range: plain_range.end..plain_range.end,
                        fragment_index,
                        link_group: None,
                        kind: ExpandedInlineSegmentKind::ClosingDelimiter(ExpandedInlineKind::Link),
                    });
                    for _ in 0..close_len {
                        display_to_plain.push(plain_range.end);
                    }
                    display_cursor += close_len;

                    footnote_spans.push(ExpandedFootnoteSpan {
                        footnote: footnote.clone(),
                        plain_range: plain_range.clone(),
                        display_range: span_display_start..display_cursor,
                    });
                } else {
                    projected_fragments.push(fragment.clone());
                    segments.push(ExpandedInlineSegment {
                        display_range: display_cursor..display_cursor + fragment_len,
                        plain_range: plain_range.clone(),
                        fragment_index,
                        link_group: None,
                        kind: ExpandedInlineSegmentKind::PlainText,
                    });
                    for offset in 0..=fragment_len {
                        plain_to_display_cursor[plain_range.start + offset] =
                            display_cursor + offset;
                    }
                    for offset in 1..=fragment_len {
                        display_to_plain.push(plain_range.start + offset);
                    }
                    display_cursor += fragment_len;
                }

                plain_cursor = plain_range.end;
                fragment_index += 1;
                continue;
            }

            if let Some(link) = fragment.link.as_ref() {
                let span_start = fragment_index;
                let span_plain_start = plain_cursor;
                let mut span_end = fragment_index;
                let mut span_plain_end = plain_cursor;
                while span_end < fragments.len() {
                    let span_fragment = &fragments[span_end];
                    if span_fragment.link.as_ref() != Some(link) {
                        break;
                    }
                    span_plain_end += span_fragment.text.len();
                    span_end += 1;
                }

                let span_plain_range = span_plain_start..span_plain_end;
                let expand_link = Self::fragment_is_touched(
                    span_plain_range.clone(),
                    &plain_selected,
                    plain_marked.as_ref(),
                );
                let link_group = expand_link.then_some(link_spans.len());
                let span_display_start = display_cursor;
                if expand_link {
                    any_expanded = true;
                    let open_marker = link.open_marker().to_string();
                    let open_len = open_marker.len();
                    projected_fragments.push(InlineFragment {
                        text: open_marker,
                        style: InlineStyle::default(),
                        html_style: None,
                        link: None,
                        footnote: None,
                        math: None,
                    });
                    segments.push(ExpandedInlineSegment {
                        display_range: display_cursor..display_cursor + open_len,
                        plain_range: span_plain_start..span_plain_start,
                        fragment_index: span_start,
                        link_group,
                        kind: ExpandedInlineSegmentKind::OpeningDelimiter(ExpandedInlineKind::Link),
                    });
                    for _ in 0..open_len {
                        display_to_plain.push(span_plain_start);
                    }
                    display_cursor += open_len;
                }

                let mut local_plain_cursor = span_plain_start;
                for current_index in span_start..span_end {
                    let current_fragment = &fragments[current_index];
                    let current_len = current_fragment.text.len();
                    let current_plain_range = local_plain_cursor..local_plain_cursor + current_len;
                    // While the link is expanded, reveal each label fragment's
                    // own emphasis markers so anchor text edits like ordinary text.
                    let label_kinds = if expand_link {
                        Self::expanded_kinds_for_fragment(
                            fragments,
                            current_index,
                            current_fragment.style,
                            current_plain_range.clone(),
                            &plain_selected,
                            plain_marked.as_ref(),
                        )
                    } else {
                        Vec::new()
                    };
                    push_projected_fragment(
                        current_fragment,
                        current_index,
                        current_plain_range.clone(),
                        &label_kinds,
                        link_group,
                        expand_link,
                        &mut projected_fragments,
                        &mut segments,
                        &mut plain_to_display_cursor,
                        &mut display_to_plain,
                        &mut display_cursor,
                        &mut any_expanded,
                    );
                    local_plain_cursor = current_plain_range.end;
                }
                if expand_link {
                    if let Some(middle_marker) = link.middle_marker() {
                        let middle_len = middle_marker.len();
                        projected_fragments.push(InlineFragment::plain(middle_marker));
                        segments.push(ExpandedInlineSegment {
                            display_range: display_cursor..display_cursor + middle_len,
                            plain_range: span_plain_end..span_plain_end,
                            fragment_index: span_start,
                            link_group,
                            kind: ExpandedInlineSegmentKind::MiddleDelimiter(
                                ExpandedInlineKind::Link,
                            ),
                        });
                        for _ in 0..middle_len {
                            display_to_plain.push(span_plain_end);
                        }
                        display_cursor += middle_len;
                    }

                    let target_display_start = display_cursor;
                    if let Some(link_target) = link.editable_text() {
                        let target_len = link_target.len();
                        if target_len > 0 {
                            let mut target_fragment = InlineFragment::plain(link_target);
                            target_fragment.link = Some(link.clone());
                            projected_fragments.push(target_fragment);
                            segments.push(ExpandedInlineSegment {
                                display_range: display_cursor..display_cursor + target_len,
                                plain_range: span_plain_end..span_plain_end,
                                fragment_index: span_start,
                                link_group,
                                kind: ExpandedInlineSegmentKind::LinkTargetText,
                            });
                            for _ in 0..target_len {
                                display_to_plain.push(span_plain_end);
                            }
                            display_cursor += target_len;
                        }
                    }
                    let target_display_end = display_cursor;

                    let close_marker = link.close_marker().to_string();
                    let close_len = close_marker.len();
                    projected_fragments.push(InlineFragment::plain(close_marker));
                    segments.push(ExpandedInlineSegment {
                        display_range: display_cursor..display_cursor + close_len,
                        plain_range: span_plain_end..span_plain_end,
                        fragment_index: span_start,
                        link_group,
                        kind: ExpandedInlineSegmentKind::ClosingDelimiter(ExpandedInlineKind::Link),
                    });
                    for _ in 0..close_len {
                        display_to_plain.push(span_plain_end);
                    }
                    display_cursor += close_len;

                    link_spans.push(ExpandedLinkSpan {
                        link: link.clone(),
                        start_fragment_index: span_start,
                        end_fragment_index: span_end,
                        plain_range: span_plain_range.clone(),
                        display_range: span_display_start..display_cursor,
                        target_display_range: target_display_start..target_display_end,
                    });
                }

                plain_cursor = span_plain_end;
                fragment_index = span_end;
                continue;
            }

            let plain_range = plain_cursor..plain_cursor + fragment_len;

            // Footnote definitions split their leading `id:` fragment so the
            // projected `]` sits directly after the id. The id is always the
            // plain prefix up to `footnote_head_len` (the first `:`).
            let head_split = if !footnote_head_closed
                && let Some(head_len) = footnote_head_len
                && plain_range.start < head_len
                && head_len < plain_range.end
            {
                Some(head_len)
            } else {
                None
            };

            if let Some(head_len) = head_split {
                let split_offset = head_len - plain_range.start;
                let attrs = fragment.attributes();
                let head_fragment =
                    InlineFragment::with_attributes(&fragment.text[..split_offset], &attrs);
                let tail_fragment =
                    InlineFragment::with_attributes(&fragment.text[split_offset..], &attrs);

                push_projected_fragment(
                    &head_fragment,
                    fragment_index,
                    plain_range.start..head_len,
                    &[],
                    None,
                    false,
                    &mut projected_fragments,
                    &mut segments,
                    &mut plain_to_display_cursor,
                    &mut display_to_plain,
                    &mut display_cursor,
                    &mut any_expanded,
                );

                let suffix = "]";
                let suffix_len = suffix.len();
                projected_fragments.push(InlineFragment::plain(suffix));
                segments.push(ExpandedInlineSegment {
                    display_range: display_cursor..display_cursor + suffix_len,
                    plain_range: head_len..head_len,
                    fragment_index,
                    link_group: None,
                    kind: ExpandedInlineSegmentKind::BlockPrefix,
                });
                for _ in 0..suffix_len {
                    display_to_plain.push(head_len);
                }
                display_cursor += suffix_len;
                footnote_head_closed = true;
                any_expanded = true;

                push_projected_fragment(
                    &tail_fragment,
                    fragment_index,
                    head_len..plain_range.end,
                    &[],
                    None,
                    false,
                    &mut projected_fragments,
                    &mut segments,
                    &mut plain_to_display_cursor,
                    &mut display_to_plain,
                    &mut display_cursor,
                    &mut any_expanded,
                );

                plain_cursor = plain_range.end;
                fragment_index += 1;
                continue;
            }

            let expanded_kinds = Self::expanded_kinds_for_fragment(
                fragments,
                fragment_index,
                fragment.style,
                plain_range.clone(),
                &plain_selected,
                plain_marked.as_ref(),
            );

            push_projected_fragment(
                fragment,
                fragment_index,
                plain_range.clone(),
                &expanded_kinds,
                None,
                false,
                &mut projected_fragments,
                &mut segments,
                &mut plain_to_display_cursor,
                &mut display_to_plain,
                &mut display_cursor,
                &mut any_expanded,
            );

            plain_cursor = plain_range.end;
            fragment_index += 1;
        }

        if any_expanded {
            for segment in &segments {
                match segment.kind {
                    ExpandedInlineSegmentKind::OpeningDelimiter(
                        ExpandedInlineKind::BoldMarkdown { .. },
                    )
                    | ExpandedInlineSegmentKind::OpeningDelimiter(
                        ExpandedInlineKind::ItalicMarkdown { .. },
                    )
                    | ExpandedInlineSegmentKind::OpeningDelimiter(ExpandedInlineKind::Code)
                    | ExpandedInlineSegmentKind::OpeningDelimiter(
                        ExpandedInlineKind::Strikethrough,
                    )
                    | ExpandedInlineSegmentKind::OpeningDelimiter(
                        ExpandedInlineKind::SuperscriptMarkdown,
                    )
                    | ExpandedInlineSegmentKind::OpeningDelimiter(
                        ExpandedInlineKind::SubscriptMarkdown,
                    ) => {
                        plain_to_display_cursor[segment.plain_range.start] =
                            segment.display_range.end;
                    }
                    ExpandedInlineSegmentKind::ClosingDelimiter(
                        ExpandedInlineKind::BoldMarkdown { .. },
                    )
                    | ExpandedInlineSegmentKind::ClosingDelimiter(
                        ExpandedInlineKind::ItalicMarkdown { .. },
                    )
                    | ExpandedInlineSegmentKind::ClosingDelimiter(ExpandedInlineKind::Code)
                    | ExpandedInlineSegmentKind::ClosingDelimiter(
                        ExpandedInlineKind::Strikethrough,
                    )
                    | ExpandedInlineSegmentKind::ClosingDelimiter(
                        ExpandedInlineKind::SuperscriptMarkdown,
                    )
                    | ExpandedInlineSegmentKind::ClosingDelimiter(
                        ExpandedInlineKind::SubscriptMarkdown,
                    ) => {
                        plain_to_display_cursor[segment.plain_range.start] =
                            segment.display_range.start;
                    }
                    _ => {}
                }
            }
        }

        any_expanded.then(|| Self {
            cache: BlockText::from_fragments(projected_fragments).render_cache(),
            segments,
            block_prefix_range,
            plain_to_display_cursor,
            display_to_plain,
            link_spans,
            footnote_spans,
        })
    }

    pub(crate) fn collapsed_affinity_for_display_offset(
        &self,
        offset: usize,
    ) -> CollapsedCaretAffinity {
        for segment in &self.segments {
            match segment.kind {
                ExpandedInlineSegmentKind::OpeningDelimiter(_)
                    if offset == segment.display_range.start =>
                {
                    return CollapsedCaretAffinity::OuterStart;
                }
                ExpandedInlineSegmentKind::ClosingDelimiter(_)
                    if offset == segment.display_range.end =>
                {
                    return CollapsedCaretAffinity::OuterEnd;
                }
                _ => {}
            }
        }
        CollapsedCaretAffinity::Default
    }

    /// Whether `plain` sits at the start of a projected closing delimiter (the
    /// end boundary of a styled span). Used to place the caret after a
    /// just-typed closing marker.
    pub(crate) fn caret_closes_span_at_plain(&self, plain: usize) -> bool {
        self.segments.iter().any(|segment| {
            matches!(segment.kind, ExpandedInlineSegmentKind::ClosingDelimiter(_))
                && segment.plain_range.start == plain
        })
    }

    pub(crate) fn display_offset_for_plain_cursor(
        &self,
        plain: usize,
        affinity: CollapsedCaretAffinity,
    ) -> Option<usize> {
        match affinity {
            CollapsedCaretAffinity::Default => self
                .plain_to_display_cursor
                .get(plain.min(self.plain_to_display_cursor.len().saturating_sub(1)))
                .copied(),
            CollapsedCaretAffinity::OuterStart => self
                .segments
                .iter()
                .find_map(|segment| match segment.kind {
                    ExpandedInlineSegmentKind::OpeningDelimiter(_)
                        if segment.plain_range.start == plain =>
                    {
                        Some(segment.display_range.start)
                    }
                    _ => None,
                })
                .or_else(|| {
                    self.plain_to_display_cursor
                        .get(plain.min(self.plain_to_display_cursor.len().saturating_sub(1)))
                        .copied()
                }),
            CollapsedCaretAffinity::OuterEnd => self
                .segments
                .iter()
                .find_map(|segment| match segment.kind {
                    ExpandedInlineSegmentKind::ClosingDelimiter(_)
                        if segment.plain_range.start == plain =>
                    {
                        Some(segment.display_range.end)
                    }
                    _ => None,
                })
                .or_else(|| {
                    self.plain_to_display_cursor
                        .get(plain.min(self.plain_to_display_cursor.len().saturating_sub(1)))
                        .copied()
                }),
        }
    }

    pub(crate) fn move_left_target(
        &self,
        offset: usize,
    ) -> Option<(usize, CollapsedCaretAffinity)> {
        for segment in &self.segments {
            match segment.kind {
                ExpandedInlineSegmentKind::OpeningDelimiter(_)
                    if offset == segment.display_range.end =>
                {
                    return Some((
                        segment.display_range.start,
                        CollapsedCaretAffinity::OuterStart,
                    ));
                }
                ExpandedInlineSegmentKind::ClosingDelimiter(_)
                    if offset == segment.display_range.end =>
                {
                    return Some((segment.display_range.start, CollapsedCaretAffinity::Default));
                }
                _ => {}
            }
        }
        None
    }

    pub(crate) fn move_right_target(
        &self,
        offset: usize,
    ) -> Option<(usize, CollapsedCaretAffinity)> {
        for segment in &self.segments {
            match segment.kind {
                ExpandedInlineSegmentKind::OpeningDelimiter(_)
                    if offset == segment.display_range.start =>
                {
                    return Some((segment.display_range.end, CollapsedCaretAffinity::Default));
                }
                ExpandedInlineSegmentKind::ClosingDelimiter(_)
                    if offset == segment.display_range.start =>
                {
                    return Some((segment.display_range.end, CollapsedCaretAffinity::OuterEnd));
                }
                _ => {}
            }
        }
        None
    }

    fn expanded_kinds_for_fragment(
        fragments: &[InlineFragment],
        fragment_index: usize,
        style: InlineStyle,
        fragment_range: Range<usize>,
        plain_selected: &Range<usize>,
        plain_marked: Option<&Range<usize>>,
    ) -> Vec<ExpandedInlineKind> {
        let mut kinds = Vec::new();
        let script_kind = Self::script_projection_kind(fragments, fragment_index);
        let bold_kind = style.bold.then_some(ExpandedInlineKind::BoldMarkdown {
            marker: style.bold_marker.char(),
        });
        let italic_kind = style.italic.then_some(ExpandedInlineKind::ItalicMarkdown {
            marker: style.italic_marker.char(),
        });
        let (first_emphasis, second_emphasis) = if style.italic_outer {
            (italic_kind, bold_kind)
        } else {
            (bold_kind, italic_kind)
        };
        for kind in [
            first_emphasis,
            Some(ExpandedInlineKind::Strikethrough),
            script_kind,
            second_emphasis,
            Some(ExpandedInlineKind::Code),
        ]
        .into_iter()
        .flatten()
        {
            if kind.applies_to(style) {
                let always_expanded = matches!(
                    kind,
                    ExpandedInlineKind::Strikethrough
                        | ExpandedInlineKind::SuperscriptMarkdown
                        | ExpandedInlineKind::SuperscriptHtml
                        | ExpandedInlineKind::SubscriptMarkdown
                        | ExpandedInlineKind::SubscriptHtml
                );
                if always_expanded
                    || Self::fragment_is_touched(fragment_range.clone(), plain_selected, plain_marked)
                {
                    kinds.push(kind);
                }
            }
        }
        kinds
    }

    fn script_projection_kind(
        fragments: &[InlineFragment],
        fragment_index: usize,
    ) -> Option<ExpandedInlineKind> {
        let fragment = fragments.get(fragment_index)?;
        match fragment.style.script {
            InlineScript::Normal => None,
            InlineScript::Superscript => {
                // Prefer compact Markdown markers only when serialization can
                // round-trip them safely; standalone script spans use HTML.
                if can_use_markdown_script_delimiters(
                    fragment_index
                        .checked_sub(1)
                        .and_then(|index| fragments.get(index)),
                    fragment,
                ) {
                    Some(ExpandedInlineKind::SuperscriptMarkdown)
                } else {
                    Some(ExpandedInlineKind::SuperscriptHtml)
                }
            }
            InlineScript::Subscript => {
                // A strikethrough subscript would serialize ambiguously around
                // `~`, so it also uses the HTML marker projection.
                if !fragment.style.strikethrough
                    && can_use_markdown_script_delimiters(
                        fragment_index
                            .checked_sub(1)
                            .and_then(|index| fragments.get(index)),
                        fragment,
                    )
                {
                    Some(ExpandedInlineKind::SubscriptMarkdown)
                } else {
                    Some(ExpandedInlineKind::SubscriptHtml)
                }
            }
        }
    }

    fn fragment_is_touched(
        fragment_range: Range<usize>,
        plain_selected: &Range<usize>,
        plain_marked: Option<&Range<usize>>,
    ) -> bool {
        if let Some(marked_range) = plain_marked
            && !marked_range.is_empty()
            && Self::ranges_overlap(&fragment_range, marked_range)
        {
            return true;
        }

        if !plain_selected.is_empty() {
            return Self::ranges_overlap(&fragment_range, plain_selected);
        }

        let cursor = plain_selected.start;
        fragment_range.start <= cursor && cursor <= fragment_range.end
    }

    fn ranges_overlap(left: &Range<usize>, right: &Range<usize>) -> bool {
        left.start < right.end && right.start < left.end
    }

    pub(crate) fn link_span_fully_covering_range(
        &self,
        range: &Range<usize>,
    ) -> Option<&ExpandedLinkSpan> {
        self.link_spans.iter().find(|span| {
            span.display_range.start <= range.start && range.end <= span.display_range.end
        })
    }

    pub(crate) fn link_span_for_plain_range(
        &self,
        plain_range: &Range<usize>,
    ) -> Option<&ExpandedLinkSpan> {
        self.link_spans
            .iter()
            .find(|span| span.plain_range == *plain_range)
    }

    pub(crate) fn footnote_span_fully_covering_range(
        &self,
        range: &Range<usize>,
    ) -> Option<&ExpandedFootnoteSpan> {
        self.footnote_spans.iter().find(|span| {
            span.display_range.start <= range.start && range.end <= span.display_range.end
        })
    }

    /// Display ranges of projected Markdown delimiter markers (`**`, `~`, `^`,
    /// `[^`, `]`, backticks, heading prefixes). The shaped-text editor colors
    /// these runs distinctly so revealed source reads as syntax-highlighted
    /// Markdown instead of plain text.
    pub(crate) fn delimiter_ranges(&self) -> Vec<Range<usize>> {
        let mut ranges = Vec::new();
        for segment in &self.segments {
            match &segment.kind {
                ExpandedInlineSegmentKind::OpeningDelimiter(_)
                | ExpandedInlineSegmentKind::ClosingDelimiter(_)
                | ExpandedInlineSegmentKind::MiddleDelimiter(_) => {
                    ranges.push(segment.display_range.clone());
                }
                ExpandedInlineSegmentKind::BlockPrefix => {
                    let prefix_text = &self.cache.text()[segment.display_range.clone()];
                    if prefix_text.starts_with("[!") && let Some(bracket_end) = prefix_text.find(']') {
                        ranges.push(segment.display_range.start..segment.display_range.start + 2);
                        ranges.push(
                            segment.display_range.start + bracket_end
                                ..segment.display_range.start + bracket_end + 1,
                        );
                    } else {
                        ranges.push(segment.display_range.clone());
                    }
                }
                _ => {}
            }
        }
        ranges
    }
}

fn marker_style_for_projection(style: InlineStyle, kind: ExpandedInlineKind) -> InlineStyle {
    // Delimiters keep the fragment's own style so editing a script still
    // shows its `^…^` / `~…~` markers at the superscript/subscript size and
    // vertical offset instead of popping back to normal text.
    // However, code delimiters (`...`) do not carry code style so the
    // background pill highlight remains restricted to the inner content.
    let mut style = style;
    if matches!(kind, ExpandedInlineKind::Code) {
        style.code = false;
    }
    style
}

/// Emit one inline fragment, wrapped in the projected emphasis delimiters for
/// `kinds`. Shared by standalone and link-label fragments so anchor text reveals
/// its bold/italic/code markers like ordinary text. `force_styled` keeps a
/// marker-less fragment styled (link labels while a link span is expanded).
#[allow(clippy::too_many_arguments)]
fn push_projected_fragment(
    fragment: &InlineFragment,
    fragment_index: usize,
    plain_range: Range<usize>,
    kinds: &[ExpandedInlineKind],
    link_group: Option<usize>,
    force_styled: bool,
    projected_fragments: &mut Vec<InlineFragment>,
    segments: &mut Vec<ExpandedInlineSegment>,
    plain_to_display_cursor: &mut [usize],
    display_to_plain: &mut Vec<usize>,
    display_cursor: &mut usize,
    any_expanded: &mut bool,
) {
    let fragment_len = fragment.text.len();

    for kind in kinds {
        *any_expanded = true;
        let marker = kind.open_marker().to_string();
        let marker_len = marker.len();
        let marker_style = marker_style_for_projection(fragment.style, *kind);
        projected_fragments.push(InlineFragment {
            text: marker,
            style: marker_style,
            html_style: fragment.html_style,
            link: None,
            footnote: None,
            math: None,
        });
        segments.push(ExpandedInlineSegment {
            display_range: *display_cursor..*display_cursor + marker_len,
            plain_range: plain_range.start..plain_range.start,
            fragment_index,
            link_group,
            kind: ExpandedInlineSegmentKind::OpeningDelimiter(*kind),
        });
        for _ in 0..marker_len {
            display_to_plain.push(plain_range.start);
        }
        *display_cursor += marker_len;
    }

    let text_segment_kind = if kinds.is_empty() && !force_styled {
        ExpandedInlineSegmentKind::PlainText
    } else {
        ExpandedInlineSegmentKind::StyledText
    };
    projected_fragments.push(fragment.clone());
    segments.push(ExpandedInlineSegment {
        display_range: *display_cursor..*display_cursor + fragment_len,
        plain_range: plain_range.clone(),
        fragment_index,
        link_group,
        kind: text_segment_kind,
    });
    for offset in 0..=fragment_len {
        plain_to_display_cursor[plain_range.start + offset] = *display_cursor + offset;
    }
    for offset in 1..=fragment_len {
        display_to_plain.push(plain_range.start + offset);
    }
    *display_cursor += fragment_len;

    for kind in kinds.iter().rev() {
        let marker = kind.close_marker().to_string();
        let marker_len = marker.len();
        let marker_style = marker_style_for_projection(fragment.style, *kind);
        projected_fragments.push(InlineFragment {
            text: marker,
            style: marker_style,
            html_style: fragment.html_style,
            link: None,
            footnote: None,
            math: None,
        });
        segments.push(ExpandedInlineSegment {
            display_range: *display_cursor..*display_cursor + marker_len,
            plain_range: plain_range.end..plain_range.end,
            fragment_index,
            link_group,
            kind: ExpandedInlineSegmentKind::ClosingDelimiter(*kind),
        });
        for _ in 0..marker_len {
            display_to_plain.push(plain_range.end);
        }
        *display_cursor += marker_len;
    }
}
