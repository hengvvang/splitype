//! Source-offset mapping between canonical Markdown and rendered blocks.

pub mod block_kinds;
pub mod collector;
pub mod mapping_buffer;
pub mod table_cells;

use std::collections::HashMap;
use std::ops::Range;

use gpui::*;

use crate::document::Document;
pub use crate::source_map::collector::collect_single_block_source_mappings;
pub use crate::source_map::mapping_buffer::{
    build_code_block_content_mapping, build_prefixed_content_mapping,
};
use crate::state::SourceTargetMapping;

/// Builds source target mappings for the whole document.
pub fn build_source_target_mappings(
    doc: &Document,
    cx: &App,
) -> Vec<SourceTargetMapping> {
    build_source_target_mappings_with_block_ranges(doc, cx).0
}

/// Builds source target mappings along with the per-block source ranges.
pub fn build_source_target_mappings_with_block_ranges(
    doc: &Document,
    cx: &App,
) -> (Vec<SourceTargetMapping>, HashMap<EntityId, Range<usize>>) {
    let mut mappings = Vec::new();
    let mut block_ranges = HashMap::new();
    let mut absolute = 0usize;
    let mut first = true;

    for block in doc.root_blocks() {
        if !first {
            absolute += 1;
        }

        let len = collect_single_block_source_mappings(
            block,
            0,
            0,
            absolute,
            &mut mappings,
            &mut block_ranges,
            cx,
        );

        absolute += len;
        first = false;
    }

    (mappings, block_ranges)
}
