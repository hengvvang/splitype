//! Source-offset mapping between canonical Markdown and rendered blocks.

pub(crate) mod block_kinds;
pub(crate) mod collector;
pub(crate) mod mapping_buffer;
pub(crate) mod table_cells;

use std::collections::HashMap;
use std::ops::Range;

use gpui::*;

use crate::editor::engine::controller::*;

impl Editor {
    pub(crate) fn build_source_target_mappings(&self, cx: &App) -> Vec<SourceTargetMapping> {
        self.build_source_target_mappings_with_block_ranges(cx).0
    }

    /// Like [`Self::build_source_target_mappings`], but also returns the source
    /// span of every block keyed by entity id. Atomic blocks (e.g. tables) have
    /// no per-block text mapping, so this is the only way to recover their full
    /// source extent for selection/deletion.
    pub(crate) fn build_source_target_mappings_with_block_ranges(
        &self,
        cx: &App,
    ) -> (Vec<SourceTargetMapping>, HashMap<EntityId, Range<usize>>) {
        let mut mappings = Vec::new();
        let mut block_ranges = HashMap::new();
        let mut absolute = 0usize;
        let mut first = true;

        for block in self.doc().root_blocks() {
            if !first {
                absolute += 1;
            }

            let len = self.collect_single_block_source_mappings(
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
}
