//! Editor runtime helpers: block creation, focus queries, and reference
//! registry rebuilds.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use gpui::*;

use crate::editor::block::Block;
use crate::editor::controller::*;


impl Editor {
    pub(crate) fn current_edit_target_entity_id_from_state(&self, cx: &App) -> Option<EntityId> {
        self.focus.active_entity
            .filter(|entity_id| self.focusable_entity_by_id(*entity_id).is_some())
            .or_else(|| {
                self.focus.pending
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

        for visible in self.document.blocks().to_vec() {
            visible.entity.update(cx, |block, _cx| {
                changed |= block.end_pointer_selection_session();
            });
        }

        let cells: Vec<Entity<Block>> = self
            .tables.cells
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
    /// [`BlockAction`](crate::editor::actions::BlockAction) stream.
    pub(crate) fn new_block(cx: &mut Context<Self>, record: BlockData) -> Entity<Block> {
        let block = cx.new(|cx| Block::with_record(cx, record));
        cx.subscribe(&block, Self::on_block_event).detach();
        block
    }

    /// Creates a standalone block NOT subscribed to the Editor's event
    /// handler.  Used for the Source channel panel so its events don't
    /// interfere with the document tree.
    pub(crate) fn new_standalone_block(cx: &mut Context<Self>, record: BlockData) -> Entity<Block> {
        cx.new(|cx| Block::with_record(cx, record))
    }

    pub(crate) fn new_table_cell_block(
        cx: &mut Context<Self>,
        title: RichText,
        position: TableCellPosition,
        alignment: TableColumnAlignment,
    ) -> Entity<Block> {
        let block = Self::new_block(cx, BlockData::new(BlockKind::Paragraph, title));
        block.update(cx, |block, _cx| {
            block.set_table_cell_mode(position, alignment);
        });
        block
    }

    pub(crate) fn image_base_dir(&self) -> Option<PathBuf> {
        self.file.path
            .as_ref()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .or_else(|| std::env::current_dir().ok())
    }

    pub(crate) fn sync_runtime_context_for_block(
        &self,
        block: &Entity<Block>,
        base_dir: Option<&Path>,
        cx: &mut Context<Self>,
    ) {
        let next_base_dir = base_dir.map(Path::to_path_buf);
        let image_reference_definitions = self.references.image.clone();
        let link_reference_definitions = self.references.link.clone();
        let footnote_registry = self.references.footnotes.clone();
        block.update(cx, move |block, cx| {
            block.set_runtime_context(
                next_base_dir.clone(),
                image_reference_definitions.clone(),
                link_reference_definitions.clone(),
                footnote_registry.clone(),
            );
            cx.notify();
        });
    }

    pub(crate) fn rebuild_footnote_registry(&mut self, cx: &App) {
                use std::sync::Arc;

        let mut definitions = HashMap::new();
        let visible = self.document.blocks().to_vec();
        for visible_block in &visible {
            let block = visible_block.entity.read(cx);
            if block.kind() != BlockKind::FootnoteDefinition {
                continue;
            }

            let allow_definition = self
                .document
                .find_block_location(visible_block.entity.entity_id())
                .is_some_and(|location| {
                    location.parent.is_none()
                        || location
                            .parent
                            .as_ref()
                            .is_some_and(|parent| parent.read(cx).kind().is_quote_container())
                });
            if !allow_definition {
                continue;
            }

            definitions
                .entry(block.record.text.visible_text().to_string())
                .or_insert(visible_block.entity.entity_id());
        }

        let mut bindings = HashMap::<String, FootnoteDefinitionBinding>::new();
        for (id, entity_id) in definitions {
            bindings.insert(
                id,
                FootnoteDefinitionBinding {
                    ordinal: None,
                    definition_entity_id: entity_id,
                    first_reference: None,
                },
            );
        }

        let mut next_ordinal = 1usize;
        let mut occurrence_index = 0usize;
        let mut block_occurrences = HashMap::<BlockId, Vec<FootnoteResolvedOccurrence>>::new();
        for visible_block in visible {
            let block = visible_block.entity.read(cx);
            let block_id = block.record.id;
            for fragment in &block.record.text.fragments {
                let Some(footnote) = fragment.footnote.as_ref() else {
                    continue;
                };
                let ordinal = if let Some(binding) = bindings.get_mut(&footnote.id) {
                    if binding.ordinal.is_none() {
                        binding.ordinal = Some(next_ordinal);
                        next_ordinal += 1;
                    }
                    if binding.first_reference.is_none() {
                        binding.first_reference = Some(FootnoteReferenceLocation {
                            entity_id: visible_block.entity.entity_id(),
                            occurrence_index,
                        });
                    }
                    binding.ordinal
                } else {
                    None
                };
                block_occurrences
                    .entry(block_id)
                    .or_default()
                    .push(FootnoteResolvedOccurrence {
                        id: footnote.id.clone(),
                        ordinal,
                        occurrence_index,
                    });
                if ordinal.is_none() {
                    occurrence_index += 1;
                    continue;
                }
                occurrence_index += 1;
            }
        }

        self.references.footnotes = Arc::new(FootnoteMap {
            bindings,
            block_occurrences,
        });
    }

    pub(crate) fn rebuild_image_runtimes(&mut self, cx: &mut Context<Self>) {
        use std::sync::Arc;

        let base_dir = self.image_base_dir();
        let markdown = self.document.to_markdown(cx);
        self.references.image = Arc::new(parse_image_reference_definitions(&markdown));
        self.references.link = Arc::new(parse_link_reference_definitions(&markdown));
        self.rebuild_footnote_registry(cx);
        let visible = self.document.blocks().to_vec();
        for visible_block in visible {
            self.sync_runtime_context_for_block(&visible_block.entity, base_dir.as_deref(), cx);
            if visible_block.entity.read(cx).kind() != BlockKind::Table {
                continue;
            }
            let Some(runtime) = visible_block.entity.read(cx).table_runtime.clone() else {
                continue;
            };
            for cell in runtime.header {
                self.sync_runtime_context_for_block(&cell, base_dir.as_deref(), cx);
            }
            for row in runtime.rows {
                for cell in row {
                    self.sync_runtime_context_for_block(&cell, base_dir.as_deref(), cx);
                }
            }
        }
    }

    pub(crate) fn focusable_entity_by_id(&self, entity_id: EntityId) -> Option<Entity<Block>> {
        self.document.block_entity_by_id(entity_id).or_else(|| {
            self.tables.cells
                .get(&entity_id)
                .map(|binding| binding.cell.clone())
        })
    }

    pub(crate) fn first_focusable_entity_id(&self, cx: &App) -> Option<EntityId> {
        let first_root = self.document.first_root()?.clone();
        if first_root.read(cx).kind() == BlockKind::Table {
            return first_root
                .read(cx)
                .table_runtime
                .as_ref()
                .and_then(|runtime| runtime.header.first())
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
        self.document
            .focused_block_entity_id(window, cx)
            .or_else(|| {
                self.tables.cells
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
        self.tables.cells.get(&entity_id).cloned()
    }

    pub(crate) fn table_block_by_id(&self, entity_id: EntityId, cx: &App) -> Option<Entity<Block>> {
        self.document
            .block_entity_by_id(entity_id)
            .filter(|block| block.read(cx).kind() == BlockKind::Table)
    }
}
impl Editor {
}
