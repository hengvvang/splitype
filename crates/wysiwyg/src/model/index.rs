//! Block tree indexing: flattened DFS order metadata, cached render metrics,
//! and the inheritance scope used during traversal.
//!
//! [`BlockIndex`] is rebuilt by one DFS over the runtime tree. Everything a
//! render frame or navigation query needs (spacing groups, line counts,
//! estimated heights, headings, table blocks) is cached here so hot paths
//! never re-read block entities one by one.

use std::collections::HashMap;

use gpui::*;

use crate::model::block::Block;
use markdown_parser::block::CalloutKind;
use markdown_parser::parse::{BlockId, BlockKind};

/// A block together with its position in the flattened document (DFS) order
/// and the cheap metrics cached at index build time.
#[derive(Clone)]
pub struct BlockEntry {
    pub entity: Entity<Block>,
    /// Estimated plain-text line count (1 for empty blocks).
    pub line_count: u32,
    /// Plain-text byte length (no allocation; fragment length sum).
    pub byte_len: usize,
    /// Row-level spacing metadata, captured during the index DFS.
    pub spacing: RowSpacingInfo,
    /// Estimated rendered height used for scroll offsets and viewport
    /// windowing. A fixed-formula approximation; actual layout may differ
    /// slightly, so windowed renders overscan around the estimate.
    pub height: f32,
}

/// A heading found during the index DFS, for the pane outline.
#[derive(Clone)]
pub struct IndexHeading {
    pub block_index: usize,
    pub entity_id: EntityId,
    pub level: u8,
    pub label: String,
}

/// A block's position inside the runtime tree.
#[derive(Clone)]
pub struct BlockLocation {
    pub parent: Option<Entity<Block>>,
    pub index: usize,
}

/// Row-level spacing metadata describing which visual group (quote, callout,
/// footnote) a block belongs to. Decides whether consecutive rows collapse
/// their inter-row gap. Cached per block in [`BlockEntry`]; derived from the
/// block's structure context at index build time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RowSpacingInfo {
    pub quote_group_id: Option<BlockId>,
    pub visible_quote_group_id: Option<BlockId>,
    pub callout_group_id: Option<BlockId>,
    pub callout_variant: Option<CalloutKind>,
    pub is_callout_header: bool,
    pub footnote_group_id: Option<BlockId>,
    pub is_footnote_header: bool,
    pub is_empty_paragraph: bool,
}

/// Cached flattened-order metadata for the current runtime tree.
#[derive(Default, Clone)]
pub struct BlockIndex {
    pub entries: Vec<BlockEntry>,
    pub index_by_entity: HashMap<EntityId, usize>,
    pub location_by_entity: HashMap<EntityId, BlockLocation>,
    pub last_descendant_by_entity: HashMap<EntityId, EntityId>,
    /// Prefix sums of [`BlockEntry::height`]; `cumulative_heights[i]` is the
    /// estimated pixel offset of block `i`, so scroll offsets are O(1) and
    /// viewport windowing is a binary search. Length is `entries.len() + 1`.
    pub cumulative_heights: Vec<f32>,
    /// Table blocks in DFS order, for grid rebuilds (only tables are touched).
    pub table_entities: Vec<Entity<Block>>,
    /// Headings in DFS order, for the pane outline.
    pub headings: Vec<IndexHeading>,
}

impl BlockIndex {
    pub fn clear(&mut self) {
        self.entries.clear();
        self.index_by_entity.clear();
        self.location_by_entity.clear();
        self.last_descendant_by_entity.clear();
        self.cumulative_heights.clear();
        self.table_entities.clear();
        self.headings.clear();
    }
}

/// Top-down inherited context scope during document tree traversal.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TreeInheritanceScope {
    pub parent_entity: Option<Entity<Block>>,
    pub parent_id: Option<BlockId>,
    pub list_depth: usize,
    pub quote_depth: usize,
    pub quote_group_id: Option<BlockId>,
    pub visible_quote_group_id: Option<BlockId>,
    pub callout_depth: usize,
    pub callout_group_id: Option<BlockId>,
    pub callout_variant: Option<CalloutKind>,
    pub footnote_group_id: Option<BlockId>,
}

impl TreeInheritanceScope {
    /// Create the root traversal scope.
    pub const fn root() -> Self {
        Self {
            parent_entity: None,
            parent_id: None,
            list_depth: 0,
            quote_depth: 0,
            quote_group_id: None,
            visible_quote_group_id: None,
            callout_depth: 0,
            callout_group_id: None,
            callout_variant: None,
            footnote_group_id: None,
        }
    }

    /// Derives the inherited scope for children of the given block.
    pub fn derive_child_scope(
        &self,
        block_entity: Entity<Block>,
        block_id: BlockId,
        kind: &BlockKind,
    ) -> Self {
        let is_quote_container = kind.is_quote_container();
        let own_callout_variant = kind.callout_kind();
        let quote_depth = self.quote_depth + usize::from(is_quote_container);
        let quote_group_id = if is_quote_container {
            self.quote_group_id.or(Some(block_id))
        } else {
            self.quote_group_id
        };
        let callout_depth = self.callout_depth + usize::from(own_callout_variant.is_some());
        let callout_group_id = if own_callout_variant.is_some() {
            Some(block_id)
        } else {
            self.callout_group_id
        };
        let callout_variant = own_callout_variant.or(self.callout_variant);
        let visible_quote_depth = quote_depth.saturating_sub(callout_depth);
        let visible_quote_group_id = match kind {
            BlockKind::Blockquote => self.visible_quote_group_id.or(Some(block_id)),
            BlockKind::Callout(_) => None,
            _ if visible_quote_depth == 0 => None,
            _ => self.visible_quote_group_id,
        };
        let child_visible_quote_group_id = if own_callout_variant.is_some() {
            None
        } else {
            visible_quote_group_id
        };
        let footnote_group_id = if kind.is_footnote_definition() {
            Some(block_id)
        } else {
            self.footnote_group_id
        };
        let child_list_depth = self.list_depth + usize::from(kind.is_list_item());

        Self {
            parent_entity: Some(block_entity),
            parent_id: Some(block_id),
            list_depth: child_list_depth,
            quote_depth,
            quote_group_id,
            visible_quote_group_id: child_visible_quote_group_id,
            callout_depth,
            callout_group_id,
            callout_variant,
            footnote_group_id,
        }
    }
}
