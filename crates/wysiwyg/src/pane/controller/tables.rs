//! WysiwygDocumentController — tables handlers.

use gpui::{Context, EntityId, Pixels, Point};

use markdown_parser::block::table::{TableColumnAlignment, TableData};
use markdown_parser::inline::text::BlockText;
use markdown_parser::parse::{BlockData, BlockKind};

use super::WysiwygContextMenuState;
use super::WysiwygDocumentController;
impl WysiwygDocumentController {
    pub fn insert_callout_after(&mut self, target_id: EntityId, cx: &mut Context<Self>) {
        let Some(doc) = &mut self.document else {
            return;
        };
        let Some(location) = doc.find_block_location(target_id) else {
            return;
        };
        let new_block = Self::new_block(
            cx,
            BlockData::new(BlockKind::Blockquote, BlockText::plain("[!NOTE]\n")),
        );
        doc.insert_blocks_at(
            location.parent,
            location.index + 1,
            vec![new_block.clone()],
            cx,
        );
        self.active_entity = Some(new_block);
        self.pending_edit = true;
        self.commit_document_edit(false, cx);
        cx.notify();
    }

    pub fn insert_mermaid_after(&mut self, target_id: EntityId, cx: &mut Context<Self>) {
        let Some(doc) = &mut self.document else {
            return;
        };
        let Some(location) = doc.find_block_location(target_id) else {
            return;
        };
        let new_block = Self::new_block(
            cx,
            BlockData::new(
                BlockKind::CodeBlock {
                    language: Some("mermaid".into()),
                },
                BlockText::plain("graph TD\n    A --> B"),
            ),
        );
        doc.insert_blocks_at(
            location.parent,
            location.index + 1,
            vec![new_block.clone()],
            cx,
        );
        self.active_entity = Some(new_block);
        self.pending_edit = true;
        self.commit_document_edit(false, cx);
        cx.notify();
    }

    pub fn insert_table_column_at_index(
        &mut self,
        table_block_id: EntityId,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(doc) = &self.document else {
            return;
        };
        let Some(target) = doc.block_entity_by_id(table_block_id) else {
            return;
        };
        target.update(cx, |b, _cx| {
            if let Some(table) = b.data.table.as_mut() {
                crate::table::columns::insert_table_column_at(table, index);
            }
        });
        self.rebuild_table_grids(cx);
        self.pending_edit = true;
        self.commit_document_edit(false, cx);
        cx.notify();
    }

    pub fn duplicate_table_column_at_index(
        &mut self,
        table_block_id: EntityId,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(doc) = &self.document else {
            return;
        };
        let Some(target) = doc.block_entity_by_id(table_block_id) else {
            return;
        };
        target.update(cx, |b, _cx| {
            if let Some(table) = b.data.table.as_mut() {
                crate::table::columns::duplicate_table_column(table, index);
            }
        });
        self.rebuild_table_grids(cx);
        self.pending_edit = true;
        self.commit_document_edit(false, cx);
        cx.notify();
    }

    pub fn set_table_column_alignment_at_index(
        &mut self,
        table_block_id: EntityId,
        index: usize,
        alignment: TableColumnAlignment,
        cx: &mut Context<Self>,
    ) {
        let Some(doc) = &self.document else {
            return;
        };
        let Some(target) = doc.block_entity_by_id(table_block_id) else {
            return;
        };
        target.update(cx, |b, _cx| {
            if let Some(table) = b.data.table.as_mut() {
                crate::table::columns::set_table_column_alignment(table, index, alignment);
            }
        });
        self.rebuild_table_grids(cx);
        self.pending_edit = true;
        self.commit_document_edit(false, cx);
        cx.notify();
    }

    pub fn delete_table_column_at_index(
        &mut self,
        table_block_id: EntityId,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(doc) = &self.document else {
            return;
        };
        let Some(target) = doc.block_entity_by_id(table_block_id) else {
            return;
        };
        let mut deleted = false;
        target.update(cx, |b, _cx| {
            if let Some(table) = b.data.table.as_mut() {
                deleted = crate::table::columns::delete_table_column(table, index);
            }
        });
        if deleted {
            self.rebuild_table_grids(cx);
            self.pending_edit = true;
            self.commit_document_edit(false, cx);
            cx.notify();
        }
    }

    pub fn insert_table_row_at_index(
        &mut self,
        table_block_id: EntityId,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(doc) = &self.document else {
            return;
        };
        let Some(target) = doc.block_entity_by_id(table_block_id) else {
            return;
        };
        target.update(cx, |b, _cx| {
            if let Some(table) = b.data.table.as_mut() {
                crate::table::rows::insert_table_row_at(table, index);
            }
        });
        self.rebuild_table_grids(cx);
        self.pending_edit = true;
        self.commit_document_edit(false, cx);
        cx.notify();
    }

    pub fn duplicate_table_row_at_index(
        &mut self,
        table_block_id: EntityId,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(doc) = &self.document else {
            return;
        };
        let Some(target) = doc.block_entity_by_id(table_block_id) else {
            return;
        };
        target.update(cx, |b, _cx| {
            if let Some(table) = b.data.table.as_mut() {
                crate::table::rows::duplicate_table_row(table, index);
            }
        });
        self.rebuild_table_grids(cx);
        self.pending_edit = true;
        self.commit_document_edit(false, cx);
        cx.notify();
    }

    pub fn delete_table_row_at_index(
        &mut self,
        table_block_id: EntityId,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(doc) = &self.document else {
            return;
        };
        let Some(target) = doc.block_entity_by_id(table_block_id) else {
            return;
        };
        let mut deleted = false;
        target.update(cx, |b, _cx| {
            if let Some(table) = b.data.table.as_mut() {
                deleted = if index == 0 {
                    crate::table::rows::delete_table_header_row(table)
                } else {
                    crate::table::rows::delete_table_row(table, index - 1)
                };
            }
        });
        if deleted {
            self.rebuild_table_grids(cx);
            self.pending_edit = true;
            self.commit_document_edit(false, cx);
            cx.notify();
        }
    }

    pub fn open_table_resize_picker(
        &mut self,
        table_block_id: EntityId,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let (current_rows, current_cols) = if let Some(doc) = &self.document {
            if let Some(target) = doc.block_entity_by_id(table_block_id) {
                target
                    .read(cx)
                    .data
                    .table
                    .as_ref()
                    .map(|t| (1 + t.rows.len(), t.column_count()))
                    .unwrap_or((2, 2))
            } else {
                (2, 2)
            }
        } else {
            (2, 2)
        };

        self.context_menu = Some(WysiwygContextMenuState::TableResize {
            position,
            table_block_id,
            current_rows,
            current_cols,
            hovered_rows: None,
            hovered_cols: None,
        });
        cx.notify();
    }

    pub fn set_table_resize_hover(
        &mut self,
        hovered_rows: Option<usize>,
        hovered_cols: Option<usize>,
        cx: &mut Context<Self>,
    ) {
        if let Some(WysiwygContextMenuState::TableResize {
            hovered_rows: hr,
            hovered_cols: hc,
            ..
        }) = &mut self.context_menu
        {
            *hr = hovered_rows;
            *hc = hovered_cols;
            cx.notify();
        }
    }

    pub fn resize_table(
        &mut self,
        table_block_id: EntityId,
        target_rows: usize,
        target_cols: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(doc) = &self.document else {
            return;
        };
        let Some(target) = doc.block_entity_by_id(table_block_id) else {
            return;
        };
        target.update(cx, |b, _cx| {
            if let Some(table) = b.data.table.as_mut() {
                table.resize_shape(target_rows, target_cols);
            }
        });
        self.rebuild_table_grids(cx);
        self.pending_edit = true;
        self.commit_document_edit(false, cx);
        cx.notify();
    }

    pub fn clear_all_table_axis_selections(&mut self, cx: &mut Context<Self>) {
        if let Some(doc) = &self.document {
            let mut changed = false;
            for entry in doc.blocks() {
                entry.entity.update(cx, |blk, cx| {
                    if blk.table_axis_selection.is_some() {
                        blk.table_axis_selection = None;
                        changed = true;
                        cx.notify();
                    }
                });
            }
            if changed {
                cx.notify();
            }
        }
    }

    pub fn open_table_insert_picker(
        &mut self,
        target_entity_id: Option<EntityId>,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        self.context_menu = Some(WysiwygContextMenuState::TableInsert {
            position,
            target_entity_id,
            hovered_rows: None,
            hovered_cols: None,
        });
        cx.notify();
    }

    pub fn set_table_insert_hover(
        &mut self,
        hovered_rows: Option<usize>,
        hovered_cols: Option<usize>,
        cx: &mut Context<Self>,
    ) {
        if let Some(WysiwygContextMenuState::TableInsert {
            hovered_rows: hr,
            hovered_cols: hc,
            ..
        }) = &mut self.context_menu
        {
            *hr = hovered_rows;
            *hc = hovered_cols;
            cx.notify();
        }
    }

    pub fn insert_table_with_size_after(
        &mut self,
        target_id: Option<EntityId>,
        rows: usize,
        cols: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(doc) = &mut self.document else {
            return;
        };
        let location = target_id
            .and_then(|id| doc.find_block_location(id))
            .or_else(|| {
                self.active_entity
                    .as_ref()
                    .and_then(|e| doc.find_block_location(e.entity_id()))
            })
            .or_else(|| {
                doc.blocks()
                    .last()
                    .and_then(|b| doc.find_block_location(b.entity.entity_id()))
            });
        let Some(location) = location else {
            return;
        };
        let cols = cols.max(1);
        let rows = rows.max(1);
        let body_rows_count = rows.saturating_sub(1);
        let table_data = TableData {
            header: (1..=cols)
                .map(|i| BlockText::plain(format!("Col {}", i)))
                .collect(),
            rows: (0..body_rows_count)
                .map(|_| (0..cols).map(|_| BlockText::plain("")).collect())
                .collect(),
            alignments: vec![TableColumnAlignment::Default; cols],
        };
        let mut data = BlockData::new(BlockKind::Table, BlockText::plain(""));
        data.table = Some(table_data);
        let new_block = Self::new_block(cx, data);
        doc.insert_blocks_at(
            location.parent,
            location.index + 1,
            vec![new_block.clone()],
            cx,
        );
        self.rebuild_table_grids(cx);
        self.active_entity = Some(new_block);
        self.pending_edit = true;
        self.commit_document_edit(false, cx);
        cx.notify();
    }
}
