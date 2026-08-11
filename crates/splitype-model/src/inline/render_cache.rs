//! Pre-computed view of a [`BlockText`] tree optimized for rendering.
//!
//! The render cache flattens the fragment tree into a single text
//! string plus a list of [`InlineSpan`]s.

use std::ops::Range;

use crate::inline::footnote::InlineFootnoteReference;
use crate::inline::latex::InlineLatex;
use crate::inline::link::InlineLink;
use crate::inline::style::InlineStyle;
use crate::inline::text::BlockText;
use crate::syntax::html::HtmlInlineStyle;

/// A text range with its associated [`InlineStyle`], used by
/// the render cache to build styled text runs for the text system.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InlineSpan {
    pub range: Range<usize>,
    pub style: InlineStyle,
    pub html_style: Option<HtmlInlineStyle>,
    pub link: Option<crate::inline::link::InlineLinkHit>,
    pub footnote: Option<crate::inline::footnote::InlineFootnoteHit>,
    pub math: Option<InlineLatex>,
}

/// Pre-computed view of an [`BlockText`] optimized for rendering.
///
/// Flattens the fragment tree into a text string plus a list of
/// [`InlineSpan`]s.
#[derive(Clone, Debug, Default)]
pub struct InlineRenderCache {
    text: String,
    spans: Vec<InlineSpan>,
}

impl InlineRenderCache {
    pub fn from_tree(tree: &BlockText) -> Self {
        let mut text = String::new();
        let mut spans = Vec::new();
        let mut plain_offset = 0;

        for fragment in &tree.fragments {
            let fragment_start = plain_offset;
            text.push_str(&fragment.text);
            let fragment_len = fragment.text.len();
            if fragment_len > 0 {
                spans.push(InlineSpan {
                    range: fragment_start..fragment_start + fragment_len,
                    style: fragment.style,
                    html_style: fragment.html_style,
                    link: fragment.link.as_ref().map(InlineLink::hit),
                    footnote: fragment
                        .footnote
                        .as_ref()
                        .and_then(InlineFootnoteReference::hit),
                    math: fragment.math.clone(),
                });
            }

            plain_offset += fragment_len;
        }

        Self {
            text,
            spans,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn spans(&self) -> &[InlineSpan] {
        &self.spans
    }

    pub fn len(&self) -> usize {
        self.text.len()
    }

    pub fn style_at(&self, offset: usize) -> InlineStyle {
        self.spans
            .iter()
            .find(|span| span.range.start <= offset && offset < span.range.end)
            .map(|span| span.style)
            .unwrap_or_default()
    }

    pub fn html_style_at(&self, offset: usize) -> Option<HtmlInlineStyle> {
        self.spans
            .iter()
            .find(|span| span.range.start <= offset && offset < span.range.end)
            .and_then(|span| span.html_style)
    }

    pub fn link_at(&self, offset: usize) -> Option<&str> {
        self.spans
            .iter()
            .find(|span| span.range.start <= offset && offset < span.range.end)
            .and_then(|span| span.link.as_ref())
            .map(|hit| hit.open_target.as_str())
    }

    pub fn footnote_hit_at(
        &self,
        offset: usize,
    ) -> Option<&crate::inline::footnote::InlineFootnoteHit> {
        self.spans
            .iter()
            .find(|span| span.range.start <= offset && offset < span.range.end)
            .and_then(|span| span.footnote.as_ref())
    }
}
