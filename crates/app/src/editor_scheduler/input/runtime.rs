//! Editor runtime helpers: block creation, focus queries, and reference
//! registry rebuilds.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use gpui::*;

use crate::editor_scheduler::engine::controller::*;
use editor_wysiwyg::document::block::Block;

impl Editor {
    pub(crate) fn current_edit_target_entity_id_from_state(&self, cx: &App) -> Option<EntityId> {
        let focus = self.active_pane_focus();
        focus
            .active_entity
            .filter(|entity_id| self.focusable_entity_by_id(*entity_id).is_some())
            .or_else(|| {
                focus
                    .pending
                    .filter(|entity_id| self.focusable_entity_by_id(*entity_id).is_some())
            })
            .or_else(|| self.first_focusable_entity_id(cx))
    }

    pub(crate) fn current_edit_target_from_state(&self, cx: &App) -> Option<Entity<Block>> {
        self.current_edit_target_entity_id_from_state(cx)
            .and_then(|entity_id| self.focusable_entity_by_id(entity_id))
    }

    pub(crate) fn end_block_pointer_selection_sessions_inner(
        &mut self,
        cx: &mut Context<Self>,
        notify: bool,
    ) -> bool {
        let mut changed = false;

        if let Some(target) = self.current_edit_target_from_state(cx) {
            target.update(cx, |block, _cx| {
                changed |= block.end_pointer_selection_session();
            });
        }

        for entries in self.doc().blocks() {
            entries.entity.update(cx, |block, _cx| {
                changed |= block.end_pointer_selection_session();
            });
        }

        let cells: Vec<Entity<Block>> = self
            .tab()
            .tables
            .cells
            .values()
            .map(|binding| binding.cell.clone())
            .collect();
        for cell in cells {
            cell.update(cx, |block, _cx| {
                changed |= block.end_pointer_selection_session();
            });
        }

        if changed && notify {
            cx.notify();
        }
        changed
    }

    pub(crate) fn end_block_pointer_selection_sessions(&mut self, cx: &mut Context<Self>) -> bool {
        self.end_block_pointer_selection_sessions_inner(cx, true)
    }

    /// Creates a new block entity and subscribes this editor to its
    /// [`BlockEvent`](editor_wysiwyg::document::protocol::BlockEvent) stream.
    pub(crate) fn new_block(&mut self, cx: &mut Context<Self>, data: BlockData) -> Entity<Block> {
        let block = cx.new(|cx| Block::with_data(cx, data));
        cx.subscribe(&block, Self::on_block_event).detach();
        self.subscribed_blocks.insert(block.entity_id());
        block
    }

    /// Subscribes this editor to every document block that is not yet
    /// subscribed.
    ///
    /// Structure mutations that create blocks outside `new_block` (undo
    /// restore deltas, document rebuilds in the WYSIWYG crate) must call
    /// this afterwards; without the subscription the block's
    /// [`BlockEvent`]s (undo capture, structural requests) never reach
    /// the editor.
    pub(crate) fn subscribe_document_blocks(&mut self, cx: &mut Context<Self>) {
        let Some(doc) = self.active_doc() else {
            return;
        };
        let unsubscribed: Vec<(EntityId, Entity<Block>)> = doc
            .blocks()
            .iter()
            .filter(|entry| !self.subscribed_blocks.contains(&entry.entity.entity_id()))
            .map(|entry| (entry.entity.entity_id(), entry.entity.clone()))
            .collect();
        for (entity_id, block) in unsubscribed {
            self.subscribed_blocks.insert(entity_id);
            cx.subscribe(&block, Self::on_block_event).detach();
        }
    }


    pub(crate) fn new_table_cell_block(
        &mut self,
        cx: &mut Context<Self>,
        text: BlockText,
        position: TableCellPosition,
        alignment: TableColumnAlignment,
    ) -> Entity<Block> {
        let block = self.new_block(cx, BlockData::new(BlockKind::Paragraph, text));
        block.update(cx, |block, _cx| {
            block.set_table_cell_mode(position, alignment);
        });
        block
    }

    pub(crate) fn image_base_dir(&self) -> Option<PathBuf> {
        self.tab()
            .file
            .path
            .as_ref()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .or_else(|| std::env::current_dir().ok())
    }

    pub(crate) fn sync_reference_context_for_block(
        &self,
        block: &Entity<Block>,
        base_dir: Option<&Path>,
        cx: &mut Context<Self>,
    ) {
        editor_wysiwyg::document::references::sync_reference_context_for_block(
            block,
            base_dir,
            self.tab().references.image.clone(),
            self.tab().references.link.clone(),
            self.tab().references.footnotes.clone(),
            cx,
        );
    }

    pub(crate) fn rebuild_footnote_registry(&mut self, cx: &App) -> FootnoteMap {
        editor_wysiwyg::document::references::rebuild_footnote_registry(self.doc(), cx)
    }

    /// Whether this block could contribute reference definitions
    /// (`[label]: url` lines), footnote content, or standalone images to the
    /// document-wide scans.
    ///
    /// Reference definitions are only ever detected in raw-preserving block
    /// kinds or in text containing `]:`; footnote bindings need `[^` markers;
    /// standalone images start with `![`. Code-block text is fence-suppressed
    fn block_has_registry_candidates(block: &Block) -> bool {
        editor_wysiwyg::document::references::block_has_registry_candidates(block)
    }

    /// Entity ids of every block and table cell whose text could contribute
    /// reference definitions, footnote content, or standalone-image syntax to
    /// the document-wide registries.
    fn collect_registry_candidates(&self, cx: &App) -> HashSet<EntityId> {
        editor_wysiwyg::document::references::collect_registry_candidates(
            self.doc(),
            &self.tab().tables,
            cx,
        )
    }

    /// Per-edit entry point: run the document-wide registry rebuild only
    /// when the edited block could have contributed reference definitions,
    /// footnote content, or standalone images to it. The block's own image
    /// handle is already refreshed by `sync_render_cache` during the edit.
    pub(crate) fn sync_references_after_block_change(
        &mut self,
        block: &Entity<Block>,
        cx: &mut Context<Self>,
    ) {
        let was_candidate = self
            .tab()
            .references
            .candidate_blocks
            .contains(&block.entity_id());
        let now_candidate = Self::block_has_registry_candidates(block.read(cx));
        if !was_candidate
            && !now_candidate
            && self.tab().references.base_dir == self.image_base_dir()
        {
            return;
        }
        self.rebuild_reference_registries(cx);
    }

    pub(crate) fn rebuild_reference_registries(&mut self, cx: &mut Context<Self>) {
        use std::sync::Arc;

        let base_dir = self.image_base_dir();

        // Cache which blocks/cells could contribute to the registries so the
        // per-edit path (`sync_references_after_block_change`) can skip the
        // rebuild when an unrelated block changed.
        let candidate_blocks = self.collect_registry_candidates(cx);
        self.tab_mut().references.candidate_blocks = candidate_blocks.clone();

        // Fast path: when no block can contribute reference definitions or
        // footnote content and the registries are already empty, there is
        // nothing to rebuild. This turns the per-keystroke cost from a
        // full-document serialization plus per-block resync into a cheap
        // scan of block text.
        let registries_empty = {
            let references = &self.tab().references;
            references.image.is_empty()
                && references.link.is_empty()
                && references.footnotes.bindings.is_empty()
                && references.footnotes.block_occurrences.is_empty()
        };
        if candidate_blocks.is_empty()
            && registries_empty
            && self.tab().references.base_dir == base_dir
        {
            // No block needs a reference context, so the (empty) context is
            // already correct for every block in the document.
            self.tab_mut().references.synced_structure_version = self.doc().structure_version();
            return;
        }

        let markdown = self.doc().serialize_markdown(cx);
        let next_image = Arc::new(parse_image_reference_definitions(&markdown));
        let next_link = Arc::new(parse_link_reference_definitions(&markdown));
        let next_footnotes = Arc::new(self.rebuild_footnote_registry(cx));

        // Registries are compared by value, so when nothing actually changed
        // and the block set is the same one that was last synced, the
        // per-block context sync below is skipped entirely — blocks keep
        // their current contexts and are not re-notified.
        let registries_unchanged = {
            let references = &self.tab().references;
            references.synced_structure_version == self.doc().structure_version()
                && references.base_dir == base_dir
                && *references.image == *next_image
                && *references.link == *next_link
                && *references.footnotes == *next_footnotes
        };
        if registries_unchanged {
            return;
        }

        self.tab_mut().references.base_dir = base_dir;
        self.tab_mut().references.image = next_image;
        self.tab_mut().references.link = next_link;
        self.tab_mut().references.footnotes = next_footnotes;
        self.tab_mut().references.synced_structure_version = self.doc().structure_version();

        let base_dir = self.tab().references.base_dir.clone();
        for entry in self.doc().blocks() {
            self.sync_reference_context_for_block(&entry.entity, base_dir.as_deref(), cx);
            if entry.entity.read(cx).kind() != BlockKind::Table {
                continue;
            }
            let Some(grid) = entry.entity.read(cx).table_grid.clone() else {
                continue;
            };
            for cell in grid.header {
                self.sync_reference_context_for_block(&cell, base_dir.as_deref(), cx);
            }
            for row in grid.rows {
                for cell in row {
                    self.sync_reference_context_for_block(&cell, base_dir.as_deref(), cx);
                }
            }
        }
    }

    pub(crate) fn focusable_entity_by_id(&self, entity_id: EntityId) -> Option<Entity<Block>> {
        self.doc().block_entity_by_id(entity_id).or_else(|| {
            self.tab()
                .tables
                .cells
                .get(&entity_id)
                .map(|binding| binding.cell.clone())
        })
    }

    pub(crate) fn first_focusable_entity_id(&self, cx: &App) -> Option<EntityId> {
        let first_root = self.doc().first_root()?.clone();
        if first_root.read(cx).kind() == BlockKind::Table {
            return first_root
                .read(cx)
                .table_grid
                .as_ref()
                .and_then(|grid| grid.header.first())
                .map(|cell| cell.entity_id())
                .or_else(|| Some(first_root.entity_id()));
        }
        Some(first_root.entity_id())
    }

    pub(crate) fn focused_edit_target_entity_id(
        &self,
        window: &Window,
        cx: &App,
    ) -> Option<EntityId> {
        self.doc().focused_block_entity_id(window, cx).or_else(|| {
            self.tab()
                .tables
                .cells
                .values()
                .find(|binding| binding.cell.read(cx).focus_handle.is_focused(window))
                .map(|binding| binding.cell.entity_id())
        })
    }

    pub(crate) fn focused_edit_target(&self, window: &Window, cx: &App) -> Option<Entity<Block>> {
        self.focused_edit_target_entity_id(window, cx)
            .and_then(|entity_id| self.focusable_entity_by_id(entity_id))
    }

    pub(crate) fn table_cell_binding(&self, entity_id: EntityId) -> Option<TableCellBinding> {
        self.tab().tables.cells.get(&entity_id).cloned()
    }

    pub(crate) fn table_block_by_id(&self, entity_id: EntityId, cx: &App) -> Option<Entity<Block>> {
        self.doc()
            .block_entity_by_id(entity_id)
            .filter(|block| block.read(cx).kind() == BlockKind::Table)
    }
}
