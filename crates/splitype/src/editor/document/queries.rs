//! Document read queries: root traversal, DFS entity discovery, focus detection.

use gpui::*;

use super::Document;
use super::index::{BlockEntry, BlockIndex, BlockLocation};
use crate::editor::engine::controller::Editor;
use crate::editor::document::block::Block;

impl Document {
    pub(crate) fn new(roots: Vec<Entity<Block>>) -> Self {
        Self {
            roots,
            tree: splitype_model::tree::SumTree::new(),
            index: BlockIndex::default(),
            structure_version: 0,
            metadata_rebuild_version: 0,
        }
    }

    /// Version of the current block set; grows on every structural edit.
    pub(crate) fn structure_version(&self) -> u64 {
        self.structure_version
    }

    /// Whether the cached tree metadata was rebuilt for the current
    /// structure. Text-only edits leave it true.
    pub(crate) fn is_metadata_current(&self) -> bool {
        self.metadata_rebuild_version == self.structure_version
    }

    /// Records that runtime-only blocks (e.g. table cells) were recreated,
    /// so the next reference-context sync refreshes them even when the
    /// document registries are unchanged.
    pub(crate) fn mark_structure_changed(&mut self) {
        self.structure_version += 1;
    }

    pub(crate) fn first_root(&self) -> Option<&Entity<Block>> {
        self.roots.first()
    }

    pub(crate) fn root_blocks(&self) -> &[Entity<Block>] {
        &self.roots
    }

    pub(crate) fn root_count(&self) -> usize {
        self.roots.len()
    }

    pub(crate) fn blocks(&self) -> &[BlockEntry] {
        &self.index.entries
    }

    pub(crate) fn cloned_entries(&self) -> Vec<BlockEntry> {
        self.index.entries.clone()
    }

    pub(crate) fn focused_block_entity_id(&self, window: &Window, cx: &App) -> Option<EntityId> {
        self.index
            .entries
            .iter()
            .find(|entries| entries.entity.read(cx).focus_handle.is_focused(window))
            .map(|entries| entries.entity.entity_id())
    }

    pub(crate) fn index_for_entity_id(&self, entity_id: EntityId) -> Option<usize> {
        self.index.index_by_entity.get(&entity_id).copied()
    }

    pub(crate) fn block_entity_by_id(&self, entity_id: EntityId) -> Option<Entity<Block>> {
        self.index_for_entity_id(entity_id)
            .and_then(|index| self.index.entries.get(index))
            .map(|entries| entries.entity.clone())
    }

    pub(crate) fn find_block_location(&self, entity_id: EntityId) -> Option<BlockLocation> {
        self.index.location_by_entity.get(&entity_id).cloned()
    }

    /// Returns the sibling immediately before `entity_id` within the same
    /// parent, if any.
    pub(crate) fn previous_sibling(&self, entity_id: EntityId, cx: &App) -> Option<Entity<Block>> {
        let location = self.find_block_location(entity_id)?;
        let prev_index = location.index.checked_sub(1)?;
        match &location.parent {
            Some(parent) => parent.read(cx).children.get(prev_index).cloned(),
            None => self.roots.get(prev_index).cloned(),
        }
    }

    pub(crate) fn last_descendant(&self, entity_id: EntityId) -> Option<Entity<Block>> {
        let descendant_id = self
            .index
            .last_descendant_by_entity
            .get(&entity_id)
            .copied()?;
        self.block_entity_by_id(descendant_id)
    }

    pub(crate) fn find_entity_by_block_id(
        &self,
        block_id: splitype_model::parse::BlockId,
        cx: &App,
    ) -> Option<Entity<Block>> {
        self.index
            .entries
            .iter()
            .find(|entry| entry.entity.read(cx).data.id == block_id)
            .map(|entry| entry.entity.clone())
    }

    pub(crate) fn replace_blocks(&mut self, roots: Vec<Entity<Block>>, cx: &mut Context<Editor>) {
        self.roots = roots;
        self.structure_version += 1;
        self.rebuild_metadata_and_snapshot(cx);
    }
}
