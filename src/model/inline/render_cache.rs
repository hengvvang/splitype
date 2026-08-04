//! Pre-computed view of a [`RichText`] tree optimized for rendering.
//!
//! The render cache flattens the fragment tree into a single visible text
//! string plus a list of [`InlineSpan`]s.  It also maintains bidirectional
//! mapping tables between visible offsets and fragment positions, used by
//! the IME subsystem.

use std::ops::Range;

use crate::model::inline::footnote::InlineFootnoteReference;
use crate::model::inline::link::InlineLink;
use crate::model::inline::latex::InlineLatex;
use crate::model::inline::style::InlineStyle;
use crate::model::inline::text::{RichText, TextCursor};
use crate::model::syntax::html::HtmlInlineStyle;

/// A visible-text range with its associated [`InlineStyle`], used by
/// the render cache to build styled text runs for the text system.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InlineSpan {
    pub range: Range<usize>,
    pub style: InlineStyle,
    pub html_style: Option<HtmlInlineStyle>,
    pub link: Option<crate::model::inline::link::InlineLinkHit>,
    pub footnote: Option<crate::model::inline::footnote::InlineFootnoteHit>,
    pub math: Option<InlineLatex>,
}

/// Pre-computed view of an [`RichText`] optimized for rendering.
///
/// Flattens the fragment tree into a visible text string plus a list of
/// [`InlineSpan`]s.  Also maintains bidirectional mapping tables between
/// visible offsets and fragment positions, used by the IME subsystem.
#[derive(Clone, Debug, Default)]
pub struct InlineRenderCache {
    visible_text: String,
    spans: Vec<InlineSpan>,
    #[allow(dead_code)]
    visible_to_tree: Vec<TextCursor>,
    #[allow(dead_code)]
    tree_to_visible: Vec<usize>,
}

impl InlineRenderCache {
    pub fn from_tree(tree: &RichText) -> Self {
        let mut visible_text = String::new();
        let mut spans = Vec::new();
        let mut visible_to_tree = vec![TextCursor::default(); tree.visible_len() + 1];
        let mut tree_to_visible = Vec::with_capacity(tree.fragments.len() + 1);
        let mut visible_offset = 0;

        for (fragment_index, fragment) in tree.fragments.iter().enumerate() {
            tree_to_visible.push(visible_offset);
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

            for byte_offset in 0..=fragment_len {
                visible_to_tree[fragment_start + byte_offset] = TextCursor {
                    fragment_index,
                    byte_offset,
                };
            }

            visible_offset += fragment_len;
        }

        tree_to_visible.push(visible_offset);
        if tree.fragments.is_empty() {
            visible_to_tree[0] = TextCursor::default();
        }

        Self {
            visible_text,
            spans,
            visible_to_tree,
            tree_to_visible,
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

    #[allow(dead_code)]
    pub fn html_style_at(&self, offset: usize) -> Option<HtmlInlineStyle> {
        self.spans
            .iter()
            .find(|span| span.range.start <= offset && offset < span.range.end)
            .and_then(|span| span.html_style)
    }

    #[allow(dead_code)]
    pub fn link_at(&self, offset: usize) -> Option<&str> {
        self.link_hit_at(offset).map(|hit| hit.open_target.as_str())
    }

    pub fn link_hit_at(&self, offset: usize) -> Option<&crate::model::inline::link::InlineLinkHit> {
        self.spans
            .iter()
            .find(|span| span.range.start <= offset && offset < span.range.end)
            .and_then(|span| span.link.as_ref())
    }

    #[allow(dead_code)]
    pub fn footnote_hit_at(&self, offset: usize) -> Option<&crate::model::inline::footnote::InlineFootnoteHit> {
        self.spans
            .iter()
            .find(|span| span.range.start <= offset && offset < span.range.end)
            .and_then(|span| span.footnote.as_ref())
    }

    #[allow(dead_code)]
    pub fn inline_math_at(&self, offset: usize) -> Option<&InlineLatex> {
        self.spans
            .iter()
            .find(|span| span.range.start <= offset && offset < span.range.end)
            .and_then(|span| span.math.as_ref())
    }
}
