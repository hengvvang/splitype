//! Markdown-to-editor-tree deserialization.
//!
//! Raw Markdown is parsed by `editor_wysiwyg::markdown::parse` into `BlockData` records,
//! then converted into the runtime tree of GPUI `Entity<Block>` values.

use std::collections::HashMap;

use gpui::*;

use crate::editor_scheduler::engine::controller::Editor;
use editor_wysiwyg::document::block::Block;
use editor_wysiwyg::markdown::parse::BlockData;

impl Editor {
    /// Parse a Markdown string into a tree of block entities using WYSIWYG (1:1 line) mode.
    pub(crate) fn parse_wysiwyg_document(
        cx: &mut Context<Self>,
        markdown: &str,
    ) -> Vec<Entity<Block>> {
        let blocks = editor_wysiwyg::markdown::parse::parser::parse_wysiwyg_document(markdown);
        blocks_to_entity_tree(blocks, cx)
    }

    /// Build runtime blocks from pre-split Markdown lines (WYSIWYG mode).
    pub(crate) fn build_wysiwyg_blocks_from_lines(
        cx: &mut Context<Self>,
        lines: &[String],
    ) -> Vec<Entity<Block>> {
        let blocks = editor_wysiwyg::markdown::parse::parser::build_wysiwyg_blocks_from_lines(lines);
        blocks_to_entity_tree(blocks, cx)
    }

    /// Replace the whole document with `markdown`.
    ///
    /// Model C: `text` is the authoritative session source, so a rebuild
    /// is just a text swap plus cache invalidation — the block tree is
    /// dropped and re-parsed lazily by `ensure_document`. When the WYSIWYG
    /// pane is active it re-parses immediately (the caller is about to
    /// keep interacting with the tree); otherwise parsing waits until the
    /// WYSIWYG world actually needs it.
    pub(crate) fn rebuild_document_from_markdown(
        &mut self,
        markdown: &str,
        cx: &mut Context<Self>,
    ) {
        self.tab_mut().text = markdown.to_string();
        self.tab_mut().document = None;
        self.tab_mut().text_stale = false;
        // The old tree's entities are abandoned; their subscriptions die
        // with the entities, so the bookkeeping set must not grow.
        self.subscribed_blocks.clear();
        if self.is_wysiwyg() {
            self.ensure_document(cx);
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
        let entity = cx.new(|cx| Block::with_data(cx, block.clone()));
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

