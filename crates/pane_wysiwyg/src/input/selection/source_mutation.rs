//! Cross-block markdown serialization, text replacement, and source coordinate mapping.

use std::collections::HashMap;
use std::ops::Range;

use gpui::*;

use crate::model::Document;
use crate::input::selection::state::NormalizedCrossBlockSelection;
use crate::state::SourceTargetMapping;

/// Slices a string safely on UTF-8 character boundaries.
pub fn safe_source_slice(source: &str, range: Range<usize>) -> &str {
    let (start, end) = (range.start.min(range.end), range.start.max(range.end));
    let start = start.min(source.len());
    let end = end.min(source.len());
    let start = if source.is_char_boundary(start) {
        start
    } else {
        source.floor_char_boundary(start)
    };
    let end = if source.is_char_boundary(end) {
        end
    } else {
        source.ceil_char_boundary(end)
    };
    let end = end.min(source.len());
    if start <= end {
        &source[start..end]
    } else {
        ""
    }
}

/// Serializes the selected cross-block region into Markdown format.
pub fn cross_block_selected_markdown(
    doc: &Document,
    selection: &NormalizedCrossBlockSelection,
    source: &str,
    mappings: &HashMap<EntityId, &SourceTargetMapping>,
    cx: &App,
) -> Option<String> {
    let entries = doc.blocks();
    let mut result = String::new();
    let mut wrote_chunk = false;

    for index in selection.block_index_range() {
        let entity = entries.get(index)?.entity.clone();
        let block = entity.read(cx);
        let len = block.display_len();
        let range = if selection.is_single_block() {
            selection.start.offset.min(len)..selection.end.offset.min(len)
        } else if index == selection.start_index {
            selection.start.offset.min(len)..len
        } else if index == selection.end_index {
            0..selection.end.offset.min(len)
        } else {
            0..len
        };
        let full_block =
            range.start == 0 && range.end == len && (!selection.is_single_block() || len > 0);
        let include_atomic = len == 0 && !selection.is_single_block();
        if range.is_empty() && !include_atomic && !Document::is_empty_root_paragraph(block) {
            continue;
        }

        if wrote_chunk {
            result.push('\n');
        }

        let chunk = if let Some(mapping) = mappings.get(&entity.entity_id()) {
            if full_block || include_atomic {
                safe_source_slice(source, mapping.full_source_range.clone()).to_string()
            } else {
                let s_start = mapping.content_to_source.get(range.start).copied().unwrap_or(0);
                let s_end = mapping.content_to_source.get(range.end).copied().unwrap_or(source.len());
                safe_source_slice(source, s_start..s_end).to_string()
            }
        } else {
            block.display_text()[range].to_string()
        };

        result.push_str(&chunk);
        wrote_chunk = true;
    }

    Some(result)
}

