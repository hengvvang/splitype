//! Block tree indexing, flattened DFS order metadata, and inheritance scope.

use std::collections::HashMap;

use gpui::*;

use crate::editor::document::block::Block;
use crate::model::block::CalloutKind;
use crate::model::parse::{BlockId, BlockKind};

/// A block together with its position in the flattened document (DFS) order.
#[derive(Clone)]
pub(crate) struct BlockEntry {
    pub entity: Entity<Block>,
}

/// A block's position inside the runtime tree.
#[derive(Clone)]
pub(crate) struct BlockLocation {
    pub parent: Option<Entity<Block>>,
    pub index: usize,
}

/// Cached flattened-order metadata for the current runtime tree.
#[derive(Default, Clone)]
pub(crate) struct BlockIndex {
    pub(crate) entries: Vec<BlockEntry>,
    pub(crate) index_by_entity: HashMap<EntityId, usize>,
    pub(crate) location_by_entity: HashMap<EntityId, BlockLocation>,
    pub(crate) last_descendant_by_entity: HashMap<EntityId, EntityId>,
}

impl BlockIndex {
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.index_by_entity.clear();
        self.location_by_entity.clear();
        self.last_descendant_by_entity.clear();
    }
}

/// Top-down inherited context scope during document tree traversal.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct TreeInheritanceScope {
    pub(crate) parent_entity: Option<Entity<Block>>,
    pub(crate) parent_id: Option<BlockId>,
    pub(crate) list_depth: usize,
    pub(crate) quote_depth: usize,
    pub(crate) quote_group_id: Option<BlockId>,
    pub(crate) visible_quote_group_id: Option<BlockId>,
    pub(crate) callout_depth: usize,
    pub(crate) callout_group_id: Option<BlockId>,
    pub(crate) callout_variant: Option<CalloutKind>,
    pub(crate) footnote_group_id: Option<BlockId>,
}

impl TreeInheritanceScope {
    /// Create the root traversal scope.
    pub(crate) const fn root() -> Self {
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
    pub(crate) fn derive_child_scope(
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
