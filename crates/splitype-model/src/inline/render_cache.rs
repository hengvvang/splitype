//! Pre-computed view of a [`RichText`] tree optimized for rendering.
//!
//! The render cache flattens the fragment tree into a single visible text
//! string plus a list of [`InlineSpan`]s.

use std::ops::Range;

use crate::inline::footnote::InlineFootnoteReference;
use crate::inline::latex::InlineLatex;
use crate::inline::link::InlineLink;
use crate::inline::style::InlineStyle;
use crate::inline::text::RichText;
use crate::syntax::html::HtmlInlineStyle;

/// A visible-text range with its associated [`InlineStyle`], used by
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

/// Pre-computed view of an [`RichText`] optimized for rendering.
///
/// Flattens the fragment tree into a visible text string plus a list of
/// [`InlineSpan`]s.
#[derive(Clone, Debug, Default)]
pub struct InlineRenderCache {
    visible_text: String,
    spans: Vec<InlineSpan>,
}

impl InlineRenderCache {
    pub fn from_tree(tree: &RichText) -> Self {
        let mut visible_text = String::new();
        let mut spans = Vec::new();
        let mut visible_offset = 0;

        for fragment in &tree.fragments {
            let fragment_start = visible_offset;
            visible_text.push_str(&fragment.text);
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

            visible_offset += fragment_len;
        }

        Self {
            visible_text,
            spans,
        }
    }

    pub fn visible_text(&self) -> &str {
        &self.visible_text
    }

    pub fn spans(&self) -> &[InlineSpan] {
        &self.spans
    }

    pub fn visible_len(&self) -> usize {
        self.visible_text.len()
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
