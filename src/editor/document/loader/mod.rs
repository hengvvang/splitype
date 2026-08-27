//! Markdown-to-editor-tree deserialization.
//!
//! Raw Markdown is parsed by `model::parse` into `BlockData` records, then
//! converted into the runtime tree of GPUI `Entity<Block>` values.

use std::collections::HashMap;

use gpui::*;

use crate::editor::engine::controller::{Editor, EditorPaneKind};
use crate::editor::document::block::Block;
use crate::model::parse::BlockData;

impl Editor {
    /// Parse a Markdown string into a tree of block entities using WYSIWYG (1:1 line) mode.
    pub(crate) fn parse_wysiwyg_document(
        cx: &mut Context<Self>,
        markdown: &str,
    ) -> Vec<Entity<Block>> {
        let blocks = crate::model::parse::parser::parse_wysiwyg_document(markdown);
        blocks_to_entity_tree(blocks, cx)
    }

    /// Parse a Markdown string into a tree of block entities using Preview (CommonMark merged) mode.
    pub(crate) fn parse_preview_document(
        cx: &mut Context<Self>,
        markdown: &str,
    ) -> Vec<Entity<Block>> {
        let blocks = crate::model::parse::parser::parse_preview_document(markdown);
        blocks_to_entity_tree(blocks, cx)
    }

    /// Parse a Markdown string into a tree of block entities (defaults to WYSIWYG mode).
    pub(crate) fn parse_document(cx: &mut Context<Self>, markdown: &str) -> Vec<Entity<Block>> {
        Self::parse_wysiwyg_document(cx, markdown)
    }

    /// Build runtime blocks from pre-split Markdown lines (WYSIWYG mode).
    pub(crate) fn build_blocks_from_lines(
        cx: &mut Context<Self>,
        lines: &[String],
    ) -> Vec<Entity<Block>> {
        let blocks = crate::model::parse::parser::build_wysiwyg_blocks_from_lines(lines);
        blocks_to_entity_tree(blocks, cx)
    }

    /// Replace the whole document with a fresh parse of `markdown`,
    /// rebuilding table/image handles and bumping the document revision.
    ///
    /// Mode-dependent: Wysiwyg parses 1:1 line blocks; Preview parses CommonMark AST;
    /// SourceCode rebuilds a single raw block.
    pub(crate) fn rebuild_document_from_markdown(
        &mut self,
        markdown: &str,
        cx: &mut Context<Self>,
    ) {
        match self.tab().mode {
            EditorPaneKind::Wysiwyg => {
                let mut roots = Self::parse_wysiwyg_document(cx, markdown);
                if roots.is_empty() {
                    roots.push(Self::new_block(cx, BlockData::paragraph(String::new())));
                }
                self.doc_mut().replace_blocks(roots, cx);
                self.rebuild_table_grids(cx);
                self.rebuild_reference_registries(cx);
            }
            EditorPaneKind::Preview | EditorPaneKind::Outline => {
                let mut roots = Self::parse_preview_document(cx, markdown);
                if roots.is_empty() {
                    roots.push(Self::new_block(cx, BlockData::paragraph(String::new())));
                }
                self.doc_mut().replace_blocks(roots, cx);
                self.rebuild_table_grids(cx);
                self.rebuild_reference_registries(cx);
            }
            EditorPaneKind::SourceCode => {
                let block = Self::new_block(cx, BlockData::paragraph(markdown.to_string()));
                block.update(cx, |block, _cx| block.set_source_document_mode());
                self.doc_mut().replace_blocks(vec![block], cx);
                self.tab_mut().tables.cells.clear();
            }
        }
        self.bump_document_revision();
    }
}

/// Convert a flat list of `BlockData` into a tree of `Entity<Block>`.
///
/// Parent-child relationships encoded in `BlockData.parent` / `BlockData.children`
/// are reconstructed as `Block.children` vectors.
fn blocks_to_entity_tree(data: Vec<BlockData>, cx: &mut Context<Editor>) -> Vec<Entity<Block>> {
    let block_count = data.len();
    // Pre-allocate hash map to prevent re-allocations and re-hashing during load.
    let mut entities: HashMap<uuid::Uuid, Entity<Block>> = HashMap::with_capacity(block_count);
    for block in &data {
        let entity = Editor::new_block(cx, block.clone());
        entities.insert(block.id.0, entity);
    }

    // Wire up child relationships on each parent entity.
    for block in &data {
        if block.children.is_empty() {
            continue;
        }
        if let Some(parent_entity) = entities.get(&block.id.0) {
            let mut child_entities: Vec<Entity<Block>> = Vec::with_capacity(block.children.len());
            for child_id in &block.children {
                if let Some(child_entity) = entities.get(&child_id.0) {
                    child_entities.push(child_entity.clone());
                }
            }

            if !child_entities.is_empty() {
                parent_entity.update(cx, |parent, _cx| {
                    parent.children.extend(child_entities);
                });
            }
        }
    }

    // Return only root blocks (those without a parent).
    data.iter()
        .filter(|block| block.parent.is_none())
        .filter_map(|block| entities.get(&block.id.0).cloned())
        .collect()
}

#[cfg(test)]
mod tests;
