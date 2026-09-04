//! Document tree structure mutations: insertion, deletion, normalization, and
//! metadata sync.
//!
//! [`Document::rebuild_metadata_and_snapshot`] is the single place that
//! derives the flattened index from the runtime tree: one DFS that applies
//! inherited structure context, captures per-block render metrics (line
//! count, spacing groups, estimated height, headings, tables), and fills the
//! [`BlockIndex`]. Tree changes from runtime edits run one normalization pass
//! first; trees freshly assembled from a parse projection skip it because
//! they are structurally clean by construction.

use gpui::*;

use super::Document;
use super::index::{
    BlockEntry, BlockIndex, BlockLocation, IndexHeading, RowSpacingInfo, TreeInheritanceScope,
};

use crate::model::block::{Block, BlockStructureContext};
use markdown_parser::parse::BlockKind;

/// Fixed line height used for estimated block heights. Estimates only drive
/// scroll offsets and viewport windowing; actual layout uses theme metrics.
const ESTIMATED_LINE_HEIGHT: f32 = 22.0;

/// The estimated rendered height of a block from its kind and cached text
/// metrics. Single source of truth for all scroll/window estimates.
fn estimated_block_height(kind: &BlockKind, line_count: usize, byte_len: usize) -> f32 {
    let line_height = ESTIMATED_LINE_HEIGHT;
    match kind {
        BlockKind::Heading { level } => match level {
            1 => line_height * 2.2 + 16.0,
            2 => line_height * 1.8 + 14.0,
            3 => line_height * 1.5 + 12.0,
            _ => line_height * 1.3 + 10.0,
        },
        BlockKind::Paragraph => {
            let lines = (byte_len / 60).max(1);
            (lines as f32) * line_height + 10.0
        }
        BlockKind::CodeBlock { .. } => (line_count.max(1) as f32) * line_height + 24.0,
        BlockKind::Table => line_height * 4.0 + 16.0,
        BlockKind::ThematicBreak => line_height + 8.0,
        _ => line_height * 1.5 + 8.0,
    }
}

impl Document {
    pub fn insert_blocks_at(
        &mut self,
        parent: Option<Entity<Block>>,
        index: usize,
        blocks: Vec<Entity<Block>>,
        cx: &mut App,
    ) {
        self.with_structure_mutation(cx, move |tree, cx| {
            tree.insert_blocks_unindexed(parent, index, blocks, cx);
        });
    }

    pub fn remove_block(
        &mut self,
        entity_id: EntityId,
        cx: &mut App,
    ) -> Option<(Entity<Block>, BlockLocation)> {
        self.with_structure_mutation(cx, |tree, cx| tree.remove_block_unindexed(entity_id, cx))
    }

    /// Runs a tree mutation and then eagerly rebuilds metadata and the entries
    /// snapshot exactly once for that mutation batch.
    pub fn with_structure_mutation<R>(
        &mut self,
        cx: &mut App,
        mutate: impl FnOnce(&mut Self, &mut App) -> R,
    ) -> R {
        let result = mutate(self, cx);
        self.structure_version += 1;
        self.rebuild_metadata_and_snapshot(cx);
        result
    }

    /// Rebuilds tree metadata and cached flattened-order data from the current
    /// roots, normalizing impossible runtime-only shapes first (children
    /// hoisted out of leaf blocks).
    pub fn rebuild_metadata_and_snapshot(&mut self, cx: &mut App) {
        Self::normalize_block_list(&mut self.roots, cx);
        self.rebuild_metadata_from_clean_tree(cx);
    }

    /// Rebuilds tree metadata from roots that are already structurally clean
    /// (every child belongs to a container-capable block, as parse
    /// projections guarantee). One DFS, no normalization pass.
    pub fn rebuild_metadata_from_clean_tree(&mut self, cx: &mut App) {
        self.index.clear();
        Self::sync_block_list(
            &self.roots,
            &TreeInheritanceScope::root(),
            cx,
            &mut self.index,
        );
        let heights = &self.index.entries;
        let mut cumulative = Vec::with_capacity(heights.len() + 1);
        let mut running = 0.0f32;
        cumulative.push(running);
        for entry in heights {
            running += entry.height;
            cumulative.push(running);
        }
        self.index.cumulative_heights = cumulative;
        self.metadata_rebuild_version = self.structure_version;
    }

    pub fn take_children(block: &Entity<Block>, cx: &mut App) -> Vec<Entity<Block>> {
        let mut children = Vec::new();
        block.update(cx, |block, _cx| {
            children = std::mem::take(&mut block.children);
        });
        children
    }

    pub fn insert_blocks_unindexed(
        &mut self,
        parent: Option<Entity<Block>>,
        index: usize,
        blocks: Vec<Entity<Block>>,
        cx: &mut App,
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

    pub fn remove_block_unindexed(
        &mut self,
        entity_id: EntityId,
        cx: &mut App,
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
    pub fn normalize_block_list(blocks: &mut Vec<Entity<Block>>, cx: &mut App) {
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

    /// One DFS over `blocks` that fills `snapshot` and applies the inherited
    /// structure context to every block. Also captures each block's cached
    /// render metrics (spacing groups, line counts, estimated height) and the
    /// document's headings and table blocks — the index is the only place
    /// these are derived.
    pub fn sync_block_list(
        blocks: &[Entity<Block>],
        scope: &TreeInheritanceScope,
        cx: &mut App,
        snapshot: &mut BlockIndex,
    ) {
        let mut numbered_list_ordinal = 0;
        let mut previous_was_list_item = false;
        for (index, block) in blocks.iter().enumerate() {
            let entity_id = block.entity_id();
            let entry_index = snapshot.entries.len();
            snapshot.index_by_entity.insert(entity_id, entry_index);
            snapshot.location_by_entity.insert(
                entity_id,
                BlockLocation {
                    parent: scope.parent_entity.clone(),
                    index,
                },
            );

            let (block_id, kind, children, line_count, byte_len, is_empty_paragraph, is_table) = {
                let block_ref = block.read(cx);
                (
                    block_ref.data.id,
                    block_ref.kind(),
                    block_ref.children.clone(),
                    block_ref.data.text.plain_line_count(),
                    block_ref.data.text.plain_len(),
                    block_ref.kind() == BlockKind::Paragraph
                        && block_ref.data.text.plain_len() == 0
                        && block_ref.children.is_empty(),
                    block_ref.kind() == BlockKind::Table,
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

            let spacing = RowSpacingInfo {
                quote_group_id,
                visible_quote_group_id,
                callout_group_id,
                callout_variant,
                is_callout_header: kind.is_callout(),
                footnote_group_id,
                is_footnote_header: kind.is_footnote_definition(),
                is_empty_paragraph,
            };
            let height = estimated_block_height(&kind, line_count, byte_len);
            snapshot.entries.push(BlockEntry {
                entity: block.clone(),
                line_count: line_count as u32,
                byte_len,
                spacing,
                height,
            });
            if is_table {
                snapshot.table_entities.push(block.clone());
            }
            // Headings for the outline: heading blocks directly, plus the
            // ATX-line fallback for blocks whose kind has not caught up
            // (mirrors the old per-frame extraction exactly). The text is
            // only materialized when the block could actually be a heading.
            let is_heading_kind = matches!(kind, BlockKind::Heading { .. });
            if is_heading_kind
                || plain_starts_with_hash(block.read(cx).data.text.fragments.as_slice())
            {
                let heading_text = block.read(cx).data.text.plain_text();
                let trimmed = heading_text.trim();
                let heading_level = match kind {
                    BlockKind::Heading { level } => Some(level),
                    _ => BlockKind::parse_atx_heading_line(trimmed).map(|(level, _)| level),
                };
                if let Some(level) = heading_level {
                    let label = BlockKind::parse_atx_heading_line(trimmed)
                        .map(|(_, parsed)| parsed)
                        .unwrap_or_else(|| trimmed.to_string());
                    snapshot.headings.push(IndexHeading {
                        block_index: entry_index,
                        entity_id,
                        level,
                        label: if label.is_empty() {
                            format!("Heading {level}")
                        } else {
                            label
                        },
                    });
                }
            }

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

/// Whether the plain text's first non-whitespace character is `#` — the
/// cheap pre-filter for the outline's ATX-heading fallback that avoids
/// materializing text for blocks that cannot be headings.
fn plain_starts_with_hash(fragments: &[markdown_parser::inline::text::InlineFragment]) -> bool {
    for fragment in fragments {
        for ch in fragment.text.chars() {
            if ch.is_whitespace() {
                continue;
            }
            return ch == '#';
        }
    }
    false
}
