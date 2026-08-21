//! Source-offset mapping between canonical Markdown and rendered blocks.

pub(crate) mod block_kinds;
pub(crate) mod collector;
pub(crate) mod mapping_buffer;
pub(crate) mod table_cells;

use std::collections::HashMap;
use std::ops::Range;

use gpui::*;

use crate::editor::controller::*;

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
        let mut pending_empty_roots = 0usize;
        let mut wrote_non_empty_root = false;
        let mut previous_was_list_item = false;

        for block in self.doc().root_blocks() {
            let (is_empty_root, current_is_list_item, current_is_footnote) = {
                let block_ref = block.read(cx);
                (
                    Self::is_empty_root_paragraph(block_ref),
                    block_ref.kind().is_list_item(),
                    block_ref.kind() == BlockKind::FootnoteDefinition,
                )
            };
            if is_empty_root {
                // Empty roots carry no text mapping, but they still need a source
                // span so a cross-block selection whose boundary lands on one can
                // be resolved (otherwise deletion of the selection aborts). A
                // zero-width anchor at the current cursor is the right position:
                // 0 for a leading empty root, source end for a trailing one.
                block_ranges.insert(block.entity_id(), absolute..absolute);
                pending_empty_roots += 1;
                continue;
            }

            if wrote_non_empty_root {
                let separator_count = if previous_was_list_item && current_is_list_item {
                    pending_empty_roots
                } else if current_is_footnote && pending_empty_roots == 0 {
                    // Mirrors collect_root_markdown_lines: a footnote definition
                    // directly after another block stays tight (no blank line).
                    0
                } else {
                    pending_empty_roots + 1
                };
                absolute += separator_count;
            } else if pending_empty_roots > 0 {
                absolute += pending_empty_roots;
            }

            absolute += self.collect_single_block_source_mappings(
                block,
                0,
                0,
                absolute,
                &mut mappings,
                &mut block_ranges,
                cx,
            );

            wrote_non_empty_root = true;
            pending_empty_roots = 0;
            previous_was_list_item = current_is_list_item;
            absolute += 1;
        }

        (mappings, block_ranges)
    }
}
