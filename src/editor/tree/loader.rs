//! Markdown-to-editor-tree deserialization.
//!
//! Raw Markdown is parsed by `model::parse` into `BlockData` records, then
//! converted into the runtime tree of GPUI `Entity<Block>` values.

use std::collections::HashMap;

use gpui::*;

use crate::editor::controller::Editor;
use crate::model::block::BlockData;
use crate::editor::tree::block::Block;

impl Editor {
    /// Parse a Markdown string into a tree of block entities.
    ///
    /// Delegates to the pure parser in `model::parse` and converts the
    /// resulting `BlockData` records into GPUI `Entity<Block>` values.
    pub(crate) fn parse_document(cx: &mut Context<Self>, markdown: &str) -> Vec<Entity<Block>> {
        let records = crate::model::parse::parser::parse_document(markdown);
        records_to_entity_blocks(records, cx)
    }

    /// Build runtime blocks from pre-split Markdown lines.
    ///
    /// Delegates to `model::parse::parser::build_blocks_from_lines`.
    pub(crate) fn build_blocks_from_lines(
        cx: &mut Context<Self>,
        lines: &[String],
    ) -> Vec<Entity<Block>> {
        let records = crate::model::parse::parser::build_blocks_from_lines(lines);
        records_to_entity_blocks(records, cx)
    }
}

/// Convert a flat list of `BlockData` into a tree of `Entity<Block>`.
///
/// Parent-child relationships encoded in `BlockData.parent` / `BlockData.children`
/// are reconstructed as `Block.children` vectors.
fn records_to_entity_blocks(
    records: Vec<BlockData>,
    cx: &mut Context<Editor>,
) -> Vec<Entity<Block>> {
    // Create GPUI entities for every record.
    let mut entities: HashMap<uuid::Uuid, Entity<Block>> = HashMap::new();
    for record in &records {
        let entity = Editor::new_block(cx, record.clone());
        entities.insert(record.id.0, entity);
    }

    // Wire up child relationships on each parent entity.
    for record in &records {
        if record.children.is_empty() {
            continue;
        }
        if let Some(parent_entity) = entities.get(&record.id.0) {
            let child_entities: Vec<Entity<Block>> = record
                .children
                .iter()
                .filter_map(|child_id| entities.get(&child_id.0).cloned())
                .collect();

            if !child_entities.is_empty() {
                parent_entity.update(cx, |parent, _cx| {
                    parent.children.extend(child_entities);
                });
            }
        }
    }

    // Return only root records (those without a parent).
    records
        .iter()
        .filter(|record| record.parent.is_none())
        .filter_map(|record| entities.get(&record.id.0).cloned())
        .collect()
}
