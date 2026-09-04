//! Document read queries: root traversal, DFS entity discovery, focus detection.

use gpui::*;

use super::Document;
use super::index::{BlockEntry, BlockIndex, BlockLocation};

use crate::model::block::Block;

impl Document {
    pub fn new(roots: Vec<Entity<Block>>) -> Self {
        Self {
            roots,
            index: BlockIndex::default(),
            structure_version: 0,
            metadata_rebuild_version: 0,
        }
    }

    /// Version of the current block set; grows on every structural edit.
    pub fn structure_version(&self) -> u64 {
        self.structure_version
    }

    /// Whether the cached tree metadata was rebuilt for the current
    /// structure. Text-only edits leave it true.
    pub fn is_metadata_current(&self) -> bool {
        self.metadata_rebuild_version == self.structure_version
    }

    /// Records that runtime-only blocks (e.g. table cells) were recreated,
    /// so the next reference-context sync refreshes them even when the
    /// document registries are unchanged.
    pub fn mark_structure_changed(&mut self) {
        self.structure_version += 1;
    }

    pub fn first_root(&self) -> Option<&Entity<Block>> {
        self.roots.first()
    }

    pub fn root_blocks(&self) -> &[Entity<Block>] {
        &self.roots
    }

    pub fn root_count(&self) -> usize {
        self.roots.len()
    }

    pub fn blocks(&self) -> &[BlockEntry] {
        &self.index.entries
    }

    pub fn cloned_entries(&self) -> Vec<BlockEntry> {
        self.index.entries.clone()
    }

    pub fn focused_block_entity_id(&self, window: &Window, cx: &App) -> Option<EntityId> {
        self.index
            .entries
            .iter()
            .find(|entries| entries.entity.read(cx).focus_handle.is_focused(window))
            .map(|entries| entries.entity.entity_id())
    }

    pub fn index_for_entity_id(&self, entity_id: EntityId) -> Option<usize> {
        self.index.index_by_entity.get(&entity_id).copied()
    }

    pub fn block_entity_by_id(&self, entity_id: EntityId) -> Option<Entity<Block>> {
        self.index_for_entity_id(entity_id)
            .and_then(|index| self.index.entries.get(index))
            .map(|entries| entries.entity.clone())
    }

    pub fn find_block_location(&self, entity_id: EntityId) -> Option<BlockLocation> {
        self.index.location_by_entity.get(&entity_id).cloned()
    }

    /// Returns the sibling immediately before `entity_id` within the same
    /// parent, if any.
    pub fn previous_sibling(&self, entity_id: EntityId, cx: &App) -> Option<Entity<Block>> {
        let location = self.find_block_location(entity_id)?;
        let prev_index = location.index.checked_sub(1)?;
        match &location.parent {
            Some(parent) => parent.read(cx).children.get(prev_index).cloned(),
            None => self.roots.get(prev_index).cloned(),
        }
    }

    pub fn last_descendant(&self, entity_id: EntityId) -> Option<Entity<Block>> {
        let descendant_id = self
            .index
            .last_descendant_by_entity
            .get(&entity_id)
            .copied()?;
        self.block_entity_by_id(descendant_id)
    }

    pub fn find_entity_by_block_id(
        &self,
        block_id: markdown_parser::parse::BlockId,
        cx: &App,
    ) -> Option<Entity<Block>> {
        self.index
            .entries
            .iter()
            .find(|entry| entry.entity.read(cx).data.id == block_id)
            .map(|entry| entry.entity.clone())
    }
}
