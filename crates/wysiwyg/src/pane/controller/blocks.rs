//! WysiwygDocumentController — blocks handlers.

use gpui::{Context, EntityId};

use markdown_parser::block::table::{TableColumnAlignment, TableData};
use markdown_parser::inline::text::BlockText;
use markdown_parser::parse::{BlockData, BlockKind};

use super::WysiwygDocumentController;
impl WysiwygDocumentController {
    pub fn insert_table_after(&mut self, target_id: EntityId, cx: &mut Context<Self>) {
        let Some(doc) = &mut self.document else {
            return;
        };
        let Some(location) = doc.find_block_location(target_id) else {
            return;
        };
        let table_data = TableData {
            header: vec![
                BlockText::plain("Col 1"),
                BlockText::plain("Col 2"),
                BlockText::plain("Col 3"),
            ],
            rows: vec![
                vec![
                    BlockText::plain(""),
                    BlockText::plain(""),
                    BlockText::plain(""),
                ],
                vec![
                    BlockText::plain(""),
                    BlockText::plain(""),
                    BlockText::plain(""),
                ],
            ],
            alignments: vec![
                TableColumnAlignment::Default,
                TableColumnAlignment::Default,
                TableColumnAlignment::Default,
            ],
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
        self.rebuild_table_grids(std::slice::from_ref(&new_block), cx);
        self.active_entity = Some(new_block);
        self.pending_edit = true;
        self.commit_document_edit(false, cx);
        cx.notify();
    }

    pub fn insert_code_block_after(&mut self, target_id: EntityId, cx: &mut Context<Self>) {
        let Some(doc) = &mut self.document else {
            return;
        };
        let Some(location) = doc.find_block_location(target_id) else {
            return;
        };
        let new_block = Self::new_block(
            cx,
            BlockData::new(
                BlockKind::CodeBlock { language: None },
                BlockText::plain(""),
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

    pub fn insert_math_block_after(&mut self, target_id: EntityId, cx: &mut Context<Self>) {
        let Some(doc) = &mut self.document else {
            return;
        };
        let Some(location) = doc.find_block_location(target_id) else {
            return;
        };
        let new_block = Self::new_block(
            cx,
            BlockData::new(BlockKind::MathBlock, BlockText::plain("")),
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

    pub fn insert_footnote_after(&mut self, target_id: EntityId, cx: &mut Context<Self>) {
        let Some(doc) = &mut self.document else {
            return;
        };
        let Some(location) = doc.find_block_location(target_id) else {
            return;
        };
        let fn_id = (self.references.footnotes.bindings.len() + 1).to_string();
        let new_block = Self::new_block(
            cx,
            BlockData::new(
                BlockKind::FootnoteDefinition,
                BlockText::plain(format!("{fn_id}: ")),
            ),
        );
        doc.insert_blocks_at(
            location.parent,
            location.index + 1,
            vec![new_block.clone()],
            cx,
        );
        self.sync_reference_context(Some(std::slice::from_ref(&new_block)), cx);
        self.active_entity = Some(new_block);
        self.pending_edit = true;
        self.commit_document_edit(false, cx);
        cx.notify();
    }

    pub fn insert_divider_after(&mut self, target_id: EntityId, cx: &mut Context<Self>) {
        let Some(doc) = &mut self.document else {
            return;
        };
        let Some(location) = doc.find_block_location(target_id) else {
            return;
        };
        let new_block = Self::new_block(
            cx,
            BlockData::new(BlockKind::ThematicBreak, BlockText::plain("---")),
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

    pub fn delete_target_block(&mut self, target_id: EntityId, cx: &mut Context<Self>) {
        let Some(doc) = &mut self.document else {
            return;
        };
        if doc.blocks().len() > 1 {
            doc.remove_block(target_id, cx);
            self.active_entity = doc.blocks().first().map(|b| b.entity.clone());
            self.rebuild_table_grids(&[], cx);
            self.sync_reference_context(None, cx);
            self.pending_edit = true;
            self.commit_document_edit(false, cx);
            cx.notify();
        }
    }
}
