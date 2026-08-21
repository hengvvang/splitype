//! Document tree structure mutations: insertion, deletion, normalization, and metadata sync.

use gpui::*;

use super::Document;
use super::index::{BlockEntry, BlockIndex, BlockLocation, TreeInheritanceScope};
use crate::editor::controller::Editor;
use crate::editor::tree::block::{Block, BlockStructureContext};
use crate::model::parse::BlockKind;

impl Document {
    pub(crate) fn insert_blocks_at(
        &mut self,
        parent: Option<Entity<Block>>,
        index: usize,
        blocks: Vec<Entity<Block>>,
        cx: &mut Context<Editor>,
    ) {
        self.with_structure_mutation(cx, move |tree, cx| {
            tree.insert_blocks_unindexed(parent, index, blocks, cx);
        });
    }

    /// Runs a tree mutation and then eagerly rebuilds metadata and the entries
    /// snapshot exactly once for that mutation batch.
    pub(crate) fn with_structure_mutation<R>(
        &mut self,
        cx: &mut Context<Editor>,
        mutate: impl FnOnce(&mut Self, &mut Context<Editor>) -> R,
    ) -> R {
        let result = mutate(self, cx);
        self.structure_version += 1;
        self.rebuild_metadata_and_snapshot(cx);
        result
    }

    /// Rebuilds tree metadata and cached flattened-order data from the current
    /// roots.
    ///
    /// The pass first normalizes impossible runtime-only shapes by hoisting
    /// children out of leaf blocks. It then performs one DFS to update parent
    /// UUIDs, child UUID lists, render depth, numbered-list ordinals, and the
    /// entries snapshot.
    pub(crate) fn rebuild_metadata_and_snapshot(&mut self, cx: &mut Context<Editor>) {
        Self::normalize_block_list(&mut self.roots, cx);
        self.index.clear();
        Self::sync_block_list(
            &self.roots.clone(),
            &TreeInheritanceScope::root(),
            cx,
            &mut self.index,
        );
        self.metadata_rebuild_version = self.structure_version;
    }

    pub(crate) fn take_children(
        block: &Entity<Block>,
        cx: &mut Context<Editor>,
    ) -> Vec<Entity<Block>> {
        let mut children = Vec::new();
        block.update(cx, |block, _cx| {
            children = std::mem::take(&mut block.children);
        });
        children
    }

    pub(crate) fn insert_blocks_unindexed(
        &mut self,
        parent: Option<Entity<Block>>,
        index: usize,
        blocks: Vec<Entity<Block>>,
        cx: &mut Context<Editor>,
    ) {
        if blocks.is_empty() {
            return;
        }

        if let Some(parent) = parent {
            parent.update(cx, move |parent, _cx| {
                for (offset, block) in blocks.iter().cloned().enumerate() {
                    parent.children.insert(index + offset, block);
                }
            });
        } else {
            for (offset, block) in blocks.into_iter().enumerate() {
                self.roots.insert(index + offset, block);
            }
        }
    }

    pub(crate) fn remove_block_unindexed(
        &mut self,
        entity_id: EntityId,
        cx: &mut Context<Editor>,
    ) -> Option<(Entity<Block>, BlockLocation)> {
        let location = self.find_block_location(entity_id)?;
        let removed = if let Some(parent) = location.parent.clone() {
            let mut removed = None;
            parent.update(cx, |parent, _cx| {
                removed = Some(parent.children.remove(location.index));
            });
            removed?
        } else {
            self.roots.remove(location.index)
        };

        Some((removed, location))
    }

    /// Normalizes a sibling list so only container-capable block kinds retain
    /// children.
    ///
    /// Children attached to leaf blocks are hoisted into the same parent list
    /// immediately after the leaf that previously owned them.
    pub(crate) fn normalize_block_list(blocks: &mut Vec<Entity<Block>>, cx: &mut Context<Editor>) {
        let mut index = 0;
        while index < blocks.len() {
            let block = blocks[index].clone();
            let mut children = Self::take_children(&block, cx);
            Self::normalize_block_list(&mut children, cx);

            if block.read(cx).kind().supports_children() {
                block.update(cx, {
                    let children = children.clone();
                    move |block, _cx| {
                        block.children = children.clone();
                    }
                });
            } else if !children.is_empty() {
                blocks.splice(index + 1..index + 1, children);
            }

            index += 1;
        }
    }

    pub(crate) fn sync_block_list(
        blocks: &[Entity<Block>],
        scope: &TreeInheritanceScope,
        cx: &mut Context<Editor>,
        snapshot: &mut BlockIndex,
    ) {
        let mut numbered_list_ordinal = 0;
        let mut previous_was_list_item = false;
        for (index, block) in blocks.iter().enumerate() {
            let entity_id = block.entity_id();
            let entry_index = snapshot.entries.len();
            snapshot.entries.push(BlockEntry {
                entity: block.clone(),
            });
            snapshot.index_by_entity.insert(entity_id, entry_index);
            snapshot.location_by_entity.insert(
                entity_id,
                BlockLocation {
                    parent: scope.parent_entity.clone(),
                    index,
                },
            );

            let (block_id, kind, children, is_empty_paragraph) = {
                let block_ref = block.read(cx);
                (
                    block_ref.data.id,
                    block_ref.kind(),
                    block_ref.children.clone(),
                    block_ref.kind() == BlockKind::Paragraph
                        && block_ref.data.text.plain_text().is_empty()
                        && block_ref.children.is_empty(),
                )
            };
            let parent_is_list_item = scope
                .parent_entity
                .as_ref()
                .is_some_and(|parent| parent.read(cx).kind().is_list_item());

            let content = children
                .iter()
                .map(|child| child.read(cx).data.id)
                .collect::<Vec<_>>();
            let list_ordinal = if kind.is_numbered_list_item() {
                numbered_list_ordinal += 1;
                Some(numbered_list_ordinal)
            } else {
                numbered_list_ordinal = 0;
                None
            };
            let is_quote_container = kind.is_quote_container();
            let own_callout_variant = kind.callout_kind();
            let quote_depth = scope.quote_depth + usize::from(is_quote_container);
            let quote_group_id = if is_quote_container {
                scope.quote_group_id.or(Some(block_id))
            } else {
                scope.quote_group_id
            };
            let callout_depth = scope.callout_depth + usize::from(own_callout_variant.is_some());
            let callout_group_id = if own_callout_variant.is_some() {
                Some(block_id)
            } else {
                scope.callout_group_id
            };
            let callout_variant = own_callout_variant.or(scope.callout_variant);
            let visible_quote_depth = quote_depth.saturating_sub(callout_depth);
            let visible_quote_group_id = match kind {
                BlockKind::Blockquote => scope.visible_quote_group_id.or(Some(block_id)),
                BlockKind::Callout(_) => None,
                _ if visible_quote_depth == 0 => None,
                _ => scope.visible_quote_group_id,
            };
            let footnote_group_id = if kind.is_footnote_definition() {
                Some(block_id)
            } else {
                scope.footnote_group_id
            };
            let list_group_separator_candidate = is_empty_paragraph && previous_was_list_item;

            let parent_id = scope.parent_id;
            let list_depth = scope.list_depth;
            block.update(cx, move |block, _cx| {
                block.data.parent = parent_id;
                block.data.children = content.clone();
                block.apply_structure_context(BlockStructureContext {
                    render_depth: list_depth,
                    quote_depth,
                    quote_group_id,
                    visible_quote_depth,
                    visible_quote_group_id,
                    callout_depth,
                    callout_group_id,
                    callout_variant,
                    footnote_group_id,
                    parent_is_list_item,
                    list_ordinal,
                    list_group_separator_candidate,
                    numbered_list_restart_requested: block.numbered_list_restart_requested,
                    quote_reparse_requested: block.quote_reparse_requested,
                    tree_metadata_flags: block.kind_metadata_flags(),
                });
            });

            let last_descendant_id = if children.is_empty() {
                entity_id
            } else {
                let child_scope = scope.derive_child_scope(block.clone(), block_id, &kind);
                Self::sync_block_list(&children, &child_scope, cx, snapshot);
                snapshot
                    .last_descendant_by_entity
                    .get(&children.last().expect("children checked").entity_id())
                    .copied()
                    .unwrap_or_else(|| children.last().expect("children checked").entity_id())
            };

            snapshot
                .last_descendant_by_entity
                .insert(entity_id, last_descendant_id);
            previous_was_list_item = kind.is_list_item();
        }
    }
}
