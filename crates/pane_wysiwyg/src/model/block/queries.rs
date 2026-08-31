//! Block text, inline style, footnote, and structural relationship queries.

use std::ops::Range;

use gpui::*;

use super::Block;
use super::state::CollapsedCaretAffinity;
use crate::model::block::footnotes::FootnoteMap;
use crate::markdown::block::link::LinkReferenceDefinitions;
use crate::markdown::inline::render_cache::{InlineRenderCache, InlineSpan};
use crate::markdown::inline::text::BlockText;
use crate::markdown::parse::BlockKind;
use std::sync::Arc;

impl Block {
    pub fn display_text(&self) -> &str {
        self.display_cache().text()
    }

    /// Returns the currently selected slice of display text.
    pub fn selected_text(&self) -> String {
        let text = self.display_text();
        if self.selected_range.is_empty() {
            return String::new();
        }
        let start = crate::markdown::inline::serialize::clamp_to_char_boundary(text, self.selected_range.start);
        let end = crate::markdown::inline::serialize::clamp_to_char_boundary(text, self.selected_range.end.max(start));
        text[start..end].to_string()
    }

    /// Cheap clone of the current display text as a `SharedString` (Arc bump)
    /// — avoids a fresh String allocation per render. The cached value is
    /// refreshed by [`Self::refresh_cached_display_text`] whenever the
    /// underlying text might have changed.
    pub fn shared_display_text(&self) -> SharedString {
        self.cached_display_text.clone()
    }

    pub fn refresh_cached_display_text(&mut self) {
        let current = self.display_cache().text();
        if self.cached_display_text.as_ref() != current {
            self.cached_display_text = SharedString::from(current.to_string());
        }
    }

    pub fn inline_tree_from_markdown_with_context(&self, markdown: &str) -> BlockText {
        BlockText::from_markdown_with_link_references(markdown, &self.link_reference_definitions)
    }

    pub fn inline_spans(&self) -> &[InlineSpan] {
        self.display_cache().spans()
    }

    #[cfg(test)]
    pub fn inline_style_at(&self, offset: usize) -> crate::markdown::inline::style::InlineStyle {
        self.display_cache().style_at(offset)
    }

    #[cfg(test)]
    pub fn inline_html_style_at(
        &self,
        offset: usize,
    ) -> Option<crate::markdown::inline::html::HtmlInlineStyle> {
        self.display_cache().html_style_at(offset)
    }

    #[cfg(test)]
    pub fn inline_link_at(&self, offset: usize) -> Option<&str> {
        self.display_cache().link_at(offset)
    }

    pub fn has_mixed_inline_visuals(&self) -> bool {
        self.data.text.has_mixed_inline_visuals()
    }

    pub fn footnote_definition_id(&self) -> Option<String> {
        self.kind().is_footnote_definition().then(|| {
            crate::markdown::block::footnote::split_footnote_definition_text(
                &self.data.text.plain_text(),
            )
            .0
            .to_string()
        })
    }

    pub fn has_footnote_definition_backref(&self) -> bool {
        self.footnote_definition_id().as_deref().is_some_and(|id| {
            self.footnote_registry
                .binding(id)
                .and_then(|binding| binding.first_reference.as_ref())
                .is_some()
        })
    }

    pub fn display_range_for_footnote_occurrence(
        &self,
        occurrence_index: usize,
    ) -> Option<Range<usize>> {
        if let Some(projection) = self.projection.as_ref() {
            for span in &projection.footnote_spans {
                if span.footnote.occurrence_index == occurrence_index {
                    let start = span.display_range.start + 2; // skip "[^"
                    let end = span.display_range.end.saturating_sub(1); // skip "]"
                    if start <= end {
                        return Some(start..end);
                    }
                }
            }
        }
        let mut plain_offset = 0usize;
        for fragment in &self.data.text.fragments {
            let len = fragment.text.len();
            if fragment
                .footnote()
                .is_some_and(|footnote| footnote.occurrence_index == occurrence_index)
            {
                return Some(self.plain_to_display_range(plain_offset..plain_offset + len));
            }
            plain_offset += len;
        }
        None
    }

    pub fn is_empty(&self) -> bool {
        self.display_text().is_empty()
    }

    pub fn is_direct_list_child(&self) -> bool {
        self.parent_is_list_item && !self.kind().is_list_item()
    }

    pub fn is_nested_list_item(&self) -> bool {
        self.parent_is_list_item && self.kind().is_list_item()
    }

    pub fn can_adjust_list_nesting(&self) -> bool {
        (self.kind().is_list_item() || self.parent_is_list_item) && !self.kind().is_code_block()
    }

    pub fn can_outdent_list_nesting(&self) -> bool {
        self.kind().is_list_item() || self.parent_is_list_item
    }

    pub fn display_len(&self) -> usize {
        self.display_cache().len()
    }

    pub fn split_text(&self, offset: usize) -> (BlockText, BlockText) {
        self.data
            .text
            .split_at(self.display_to_plain_offset(offset))
    }

    pub fn clear_vertical_motion(&mut self) {
        self.vertical_motion_x = None;
    }

    pub fn sync_render_cache(&mut self) {
        let plain_selected = self.display_to_plain_range(self.selected_range.clone());
        let plain_marked = self
            .marked_range
            .clone()
            .map(|range| self.display_to_plain_range(range));
        let (plain_anchor, plain_focus) = self.plain_selection_anchor_focus();
        let (anchor_affinity, focus_affinity) = self.selection_endpoint_affinities();
        let collapsed_affinity = self.display_collapsed_caret_affinity();
        let keep_projection =
            self.projection.is_some() && self.edit_mode.supports_inline_projection();
        self.render_cache = if let BlockKind::Callout(variant) = self.kind()
            && self.data.text.plain_text().is_empty()
        {
            InlineRenderCache::plain(format!("[!{}]", variant.marker_lower()))
        } else {
            self.data.text.render_cache()
        };
        self.sync_code_highlight();
        self.sync_image_handle();
        self.projection = None;
        self.projection_cache_key = None;
        if keep_projection {
            self.rebuild_inline_projection(plain_selected.clone(), plain_marked.clone());
            if plain_selected.is_empty() {
                let offset = self.plain_to_display_cursor_offset_with_affinity(
                    plain_selected.start,
                    collapsed_affinity,
                );
                self.assign_collapsed_selection_offset(offset, collapsed_affinity, None);
            } else {
                self.set_selection_from_plain_anchor_focus(
                    plain_anchor,
                    plain_focus,
                    anchor_affinity,
                    focus_affinity,
                );
                self.collapsed_caret_affinity = CollapsedCaretAffinity::Default;
            }
            self.marked_range = plain_marked.map(|range| self.plain_to_display_range(range));
        } else {
            self.set_selection_from_anchor_focus(plain_anchor, plain_focus);
            self.marked_range = plain_marked;
            self.collapsed_caret_affinity = CollapsedCaretAffinity::Default;
        }
        self.refresh_cached_display_text();
    }

    pub fn sync_link_reference_definitions(
        &mut self,
        link_reference_definitions: Arc<LinkReferenceDefinitions>,
    ) -> bool {
        if self.link_reference_definitions == link_reference_definitions {
            return false;
        }

        let selected_source = (!self.edits_verbatim_text())
            .then(|| self.display_range_to_source_range(self.selected_range.clone()));
        let marked_source = (!self.edits_verbatim_text())
            .then(|| {
                self.marked_range
                    .clone()
                    .map(|range| self.display_range_to_source_range(range))
            })
            .flatten();
        let selection_reversed = self.selection_reversed;
        let collapsed_affinity = self.display_collapsed_caret_affinity();
        let had_projection = self.projection.is_some();

        self.link_reference_definitions = link_reference_definitions;
        if self.edits_verbatim_text() {
            return true;
        }

        let markdown = self.data.text.serialize_markdown();
        let next_text = BlockText::from_markdown_with_link_references(
            &markdown,
            &self.link_reference_definitions,
        );
        if self.data.text == next_text {
            return true;
        }

        self.data.set_text(next_text);
        self.sync_edit_mode_from_kind();
        self.sync_render_cache();

        if let Some(selected_source) = selected_source {
            let restored = self.source_range_to_display_range(selected_source);
            if restored.is_empty() {
                self.assign_collapsed_selection_offset(
                    restored.start,
                    collapsed_affinity,
                    self.vertical_motion_x,
                );
            } else {
                self.selected_range = restored;
                self.selection_reversed = selection_reversed;
                self.collapsed_caret_affinity = CollapsedCaretAffinity::Default;
            }
        }

        self.marked_range = marked_source.map(|range| self.source_range_to_display_range(range));

        if had_projection {
            self.sync_inline_projection_for_focus(true);
        }
        true
    }

    pub fn sync_footnote_registry(&mut self, footnote_registry: Arc<FootnoteMap>) -> bool {
        if self.footnote_registry == footnote_registry {
            return false;
        }

        let selected_source = (!self.edits_verbatim_text())
            .then(|| self.display_range_to_source_range(self.selected_range.clone()));
        let marked_source = (!self.edits_verbatim_text())
            .then(|| {
                self.marked_range
                    .clone()
                    .map(|range| self.display_range_to_source_range(range))
            })
            .flatten();
        let selection_reversed = self.selection_reversed;
        let collapsed_affinity = self.display_collapsed_caret_affinity();
        let had_projection = self.projection.is_some();

        self.footnote_registry = footnote_registry;
        if self.edits_verbatim_text() || !self.data.text.has_footnote_references() {
            return true;
        }

        let mut next_text = self.data.text.clone();
        let mut occurrence_iter = self
            .footnote_registry
            .occurrences_for_block(self.data.id)
            .unwrap_or(&[])
            .iter();
        next_text.apply_footnote_reference_state(|id| {
            if self.footnote_registry.binding(id).is_none() {
                return None;
            }
            let occurrence = occurrence_iter.next()?;
            if occurrence.id != id {
                return None;
            }
            Some(occurrence.occurrence_index)
        });
        if self.data.text == next_text {
            return true;
        }

        self.data.set_text(next_text);
        self.sync_edit_mode_from_kind();
        self.sync_render_cache();

        if let Some(selected_source) = selected_source {
            let restored = self.source_range_to_display_range(selected_source);
            if restored.is_empty() {
                self.assign_collapsed_selection_offset(
                    restored.start,
                    collapsed_affinity,
                    self.vertical_motion_x,
                );
            } else {
                self.selected_range = restored;
                self.selection_reversed = selection_reversed;
                self.collapsed_caret_affinity = CollapsedCaretAffinity::Default;
            }
        }

        self.marked_range = marked_source.map(|range| self.source_range_to_display_range(range));

        if had_projection {
            self.sync_inline_projection_for_focus(true);
        }
        true
    }
}


