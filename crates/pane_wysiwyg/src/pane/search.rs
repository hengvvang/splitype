//! WYSIWYG search matching, rich-text-safe replacement, and block highlight mapping.

use crate::model::Document;
use editor_contracts::{SearchMatch, SearchQuery};
use gpui::{App, EntityId};
use std::ops::Range;

/// Searches across all blocks in the WYSIWYG document.
pub fn search_in_document(doc: &Document, query: &SearchQuery, cx: &App) -> Vec<SearchMatch> {
    let mut results = Vec::new();
    let mut cumulative_line = 1;

    for entry in doc.blocks().iter() {
        let block = entry.entity.read(cx);
        let block_text = block.display_text().to_string();
        let entity_id = entry.entity.entity_id();
        let raw_matches = query.find_matches(&block_text, cumulative_line);

        for mat in raw_matches {
            results.push(SearchMatch {
                file_path: None,
                file_name: "Current File".to_string(),
                block_id: Some(block.data.id.0),
                entity_id: Some(entity_id),
                line_number: mat.line_number,
                column_number: mat.column_number,
                byte_range: mat.byte_range,
                preview_prefix: mat.preview_prefix,
                preview_match: mat.preview_match,
                preview_suffix: mat.preview_suffix,
            });
        }

        let lines_in_block = block_text.matches('\n').count().max(1);
        cumulative_line += lines_in_block;
    }

    results
}

/// Safely replaces a match within a block entity while preserving rich text formatting.
pub fn replace_in_block_entity(
    doc: &Document,
    entity_id: EntityId,
    range: Range<usize>,
    replacement: &str,
    cx: &mut App,
) {
    if let Some(block) = doc.block_entity_by_id(entity_id) {
        let replace_len = replacement.len();
        block.update(cx, |block, cx| {
            let inserted_attrs = block.data.text.attributes_for_insertion_at(range.start);
            let edit_res = block.data.text.replace_plain_range_verbatim(
                range.clone(),
                replacement,
                inserted_attrs,
            );
            block.data.text = edit_res.tree;
            block.selected_range = range.start..(range.start + replace_len);
            block.refresh_cached_display_text();
            block.sync_render_cache();
            cx.notify();
        });
    }
}

/// Clears all search match highlights from document blocks.
pub fn clear_document_search_highlights(doc: &Document, cx: &mut App) {
    for entry in doc.blocks() {
        entry.entity.update(cx, |block, cx| {
            if !block.search_matches.is_empty() {
                block.search_matches.clear();
                block.sync_render_cache();
                cx.notify();
            }
        });
    }
}
