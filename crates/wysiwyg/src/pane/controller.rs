//! Self-contained WYSIWYG document controller — autonomous block editor engine.

use std::sync::Arc;

use editor_contracts::OutlineNode;
use editor_contracts::{CursorHint, EditTransaction, PaneOutlineHost, PaneRenderContext};
use editor_contracts::{SearchMatch, SearchQuery};
use gpui::{
    AnyElement, App, AppContext, ClipboardItem, Context, Div, ElementId, Entity, EntityId,
    FocusHandle, InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement,
    Pixels, Point, SharedString, StatefulInteractiveElement, Styled, Window, div, px, relative,
};
use theme::Theme;
use theme::ThemeManager;

use crate::model::Document;
use crate::model::block::state::InlineFormat;
use crate::model::block::{Block, CollapsedCaretAffinity};
use crate::model::protocol::BlockEvent;
use crate::pane::state::{ReferenceRegistries, TableAxisSelection, TableCellBinding, TableGrids};
use markdown_parser::block::table::{
    TableAxis, TableAxisMarker, TableCellPosition, TableColumnAlignment, TableData,
};
use markdown_parser::inline::text::BlockText;
use markdown_parser::parse::{BlockData, BlockKind};

/// State for a floating footnote definition tooltip.
#[derive(Clone, Debug)]
pub struct FootnoteTooltipState {
    pub id: String,
    pub content: SharedString,
    pub position: Point<Pixels>,
}

/// Active secondary submenu in the context menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextSubmenu {
    TextFormat,
    ParagraphSettings,
    Insert,
}

/// State for the context menu on a WYSIWYG block or table axis.
#[derive(Clone, Debug)]
pub enum WysiwygContextMenuState {
    Edit {
        position: Point<Pixels>,
        target_entity_id: Option<EntityId>,
        active_submenu: Option<ContextSubmenu>,
    },
    TableAxis {
        position: Point<Pixels>,
        selection: TableAxisSelection,
    },
}

/// Autonomous controller for a WYSIWYG editor pane.
pub struct WysiwygDocumentController {
    pub host: Option<Arc<dyn editor_contracts::PaneHost>>,
    pub document: Option<Document>,
    pub synced_revision: Option<u64>,
    /// Local edits exist that the host's next snapshot has not acknowledged
    /// yet; `sync_document` consumes this instead of rebuilding.
    pub pending_edit: bool,
    pub active_entity: Option<Entity<Block>>,
    pub tables: TableGrids,
    pub references: ReferenceRegistries,
    pub footnote_tooltip: Option<FootnoteTooltipState>,
    pub context_menu: Option<WysiwygContextMenuState>,
    /// Text of the document after the previous commit, used to detect
    /// typing-run continuations (single-character insertions at the same
    /// position) for undo grouping.
    last_committed_text: Option<String>,
    /// Insert position of the previous typing commit, when it was a
    /// single-character insertion.
    last_typing_insert_at: Option<usize>,
    /// Caret hint captured at the start of the current typing run.
    typing_run_start_hint: Option<CursorHint>,
    /// Caret hint after the previous commit.
    last_cursor_hint: Option<CursorHint>,
}

impl WysiwygDocumentController {
    pub fn new(document: &editor_contracts::DocumentSnapshot, cx: &mut Context<Self>) -> Self {
        let mut controller = Self {
            host: None,
            document: None,
            synced_revision: None,
            pending_edit: false,
            active_entity: None,
            tables: TableGrids::default(),
            references: ReferenceRegistries {
                base_dir: document
                    .base_dir
                    .as_deref()
                    .map(std::path::Path::to_path_buf),
                ..ReferenceRegistries::default()
            },
            footnote_tooltip: None,
            context_menu: None,
            last_committed_text: None,
            last_typing_insert_at: None,
            typing_run_start_hint: None,
            last_cursor_hint: None,
        };
        controller.rebuild_from_markdown(&document.text, document.revision, cx);
        controller
    }

    /// Creates a new block entity and subscribes this controller to its `BlockEvent` stream.
    pub fn new_block(cx: &mut Context<Self>, data: BlockData) -> Entity<Block> {
        let block = cx.new(|cx| Block::with_data(cx, data));
        cx.subscribe(&block, Self::on_block_event).detach();
        block
    }

    /// Rebuilds document tree and event subscriptions from raw Markdown text.
    pub fn rebuild_from_markdown(&mut self, text: &str, revision: u64, cx: &mut Context<Self>) {
        let parsed = markdown_parser::parse::parse_wysiwyg_document(text);
        let block_count = parsed.len();
        let mut entities: std::collections::HashMap<uuid::Uuid, Entity<Block>> =
            std::collections::HashMap::with_capacity(block_count);

        for block_data in &parsed {
            let entity = Self::new_block(cx, block_data.clone());
            entities.insert(block_data.id.0, entity);
        }

        for block_data in &parsed {
            if block_data.children.is_empty() {
                continue;
            }
            if let Some(parent_entity) = entities.get(&block_data.id.0) {
                let mut child_entities: Vec<Entity<Block>> =
                    Vec::with_capacity(block_data.children.len());
                for child_id in &block_data.children {
                    if let Some(child_entity) = entities.get(&child_id.0) {
                        child_entities.push(child_entity.clone());
                    }
                }
                if !child_entities.is_empty() {
                    parent_entity.update(cx, |parent, _cx| {
                        parent.children.extend(child_entities);
                    });
                }
            }
        }

        let mut roots: Vec<Entity<Block>> = parsed
            .iter()
            .filter(|block| block.parent.is_none())
            .filter_map(|block| entities.get(&block.id.0).cloned())
            .collect();

        if roots.is_empty() {
            let empty_block = Self::new_block(
                cx,
                BlockData::new(BlockKind::Paragraph, BlockText::plain(String::new())),
            );
            roots.push(empty_block);
        }

        let mut doc = Document::new(roots);
        doc.rebuild_metadata_and_snapshot(cx);
        self.active_entity = doc.blocks().first().map(|b| b.entity.clone());
        self.document = Some(doc);
        self.synced_revision = Some(revision);
        self.pending_edit = false;
        self.last_committed_text = None;
        self.last_typing_insert_at = None;
        self.typing_run_start_hint = None;
        self.last_cursor_hint = None;
        self.rebuild_table_grids(cx);
        self.sync_reference_context(cx);
    }

    /// Rebuilds table grid structures and bindings for all table blocks.
    pub fn rebuild_table_grids(&mut self, cx: &mut Context<Self>) {
        self.tables.cells.clear();
        self.tables.axis_preview = None;
        self.tables.axis_selection = None;
        let Some(doc) = &self.document else {
            return;
        };
        let mut tables_to_install = Vec::new();
        for entry in doc.blocks() {
            entry.entity.update(cx, |block, _cx| block.clear_table_grid());
            let block = entry.entity.read(cx);
            if block.kind() == BlockKind::Table
                && let Some(table) = block.data.table.clone()
            {
                tables_to_install.push((entry.entity.clone(), table));
            }
        }
        for (table_block, table_data) in tables_to_install {
            let mut bindings = Vec::new();
            let header = table_data
                .header
                .iter()
                .cloned()
                .enumerate()
                .map(|(column, text)| {
                    let alignment = table_data
                        .alignments
                        .get(column)
                        .copied()
                        .unwrap_or(TableColumnAlignment::Default);
                    let position = TableCellPosition { row: 0, column };
                    let cell = Self::new_block(cx, BlockData::new(BlockKind::Paragraph, text));
                    cell.update(cx, |b, _cx| b.set_table_cell_mode(position, alignment));
                    bindings.push(TableCellBinding {
                        table_block: table_block.clone(),
                        cell: cell.clone(),
                        position,
                    });
                    cell
                })
                .collect::<Vec<_>>();

            let rows = table_data
                .rows
                .iter()
                .cloned()
                .enumerate()
                .map(|(body_row_index, row)| {
                    row.into_iter()
                        .enumerate()
                        .map(|(column, text)| {
                            let alignment = table_data
                                .alignments
                                .get(column)
                                .copied()
                                .unwrap_or(TableColumnAlignment::Default);
                            let position = TableCellPosition {
                                row: body_row_index + 1,
                                column,
                            };
                            let cell = Self::new_block(cx, BlockData::new(BlockKind::Paragraph, text));
                            cell.update(cx, |b, _cx| b.set_table_cell_mode(position, alignment));
                            bindings.push(TableCellBinding {
                                table_block: table_block.clone(),
                                cell: cell.clone(),
                                position,
                            });
                            cell
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();

            table_block.update(cx, {
                let grid = crate::table::TableGrid { header, rows };
                move |block, _cx| block.set_table_grid(grid.clone())
            });

            for binding in bindings {
                self.tables.cells.insert(binding.cell.entity_id(), binding);
            }
        }
    }

    fn sync_reference_context(&mut self, cx: &mut App) {
        let Some(document) = &self.document else {
            return;
        };
        self.references.footnotes =
            Arc::new(crate::model::references::rebuild_footnote_registry(document, cx));
        for entry in document.blocks() {
            crate::model::references::sync_reference_context_for_block(
                &entry.entity,
                self.references.base_dir.as_deref(),
                self.references.image.clone(),
                self.references.link.clone(),
                self.references.footnotes.clone(),
                cx,
            );
        }
        for binding in self.tables.cells.values() {
            crate::model::references::sync_reference_context_for_block(
                &binding.cell,
                self.references.base_dir.as_deref(),
                self.references.image.clone(),
                self.references.link.clone(),
                self.references.footnotes.clone(),
                cx,
            );
        }
    }

    /// Handles events emitted by child blocks.
    pub fn on_block_event(
        &mut self,
        block: Entity<Block>,
        event: &BlockEvent,
        cx: &mut Context<Self>,
    ) {
        if let Some(binding) = self.tables.cells.get(&block.entity_id()).cloned() {
            match event {
                BlockEvent::Changed => {
                    crate::table::grid::sync_table_data_from_grid(&binding.table_block, cx);
                    self.pending_edit = true;
                    self.commit_document_edit(false, cx);
                    cx.notify();
                    return;
                }
                BlockEvent::RequestTableCellMoveHorizontal { delta } => {
                    let current = binding.position;
                    let table_block = binding.table_block.clone();
                    if let Some(grid) = table_block.read(cx).table_grid.clone() {
                        let col_count = grid.header.len();
                        let total_rows = grid.rows.len() + 1;
                        if col_count > 0 {
                            let (next_row, next_col) = if *delta > 0 {
                                if current.column + 1 < col_count {
                                    (current.row, current.column + 1)
                                } else if current.row + 1 < total_rows {
                                    (current.row + 1, 0)
                                } else {
                                    table_block.update(cx, |b, _cx| {
                                        if let Some(table) = b.data.table.as_mut() {
                                            crate::table::rows::append_table_row(table);
                                        }
                                    });
                                    self.rebuild_table_grids(cx);
                                    self.pending_edit = true;
                                    self.commit_document_edit(false, cx);
                                    (current.row + 1, 0)
                                }
                            } else if current.column > 0 {
                                (current.row, current.column - 1)
                            } else if current.row > 0 {
                                (current.row - 1, col_count.saturating_sub(1))
                            } else {
                                (0, 0)
                            };
                            let next_pos = TableCellPosition {
                                row: next_row,
                                column: next_col,
                            };
                            if let Some(cell) = table_block
                                .read(cx)
                                .table_grid
                                .as_ref()
                                .and_then(|g| g.cell(next_pos))
                            {
                                self.active_entity = Some(cell.clone());
                                cell.update(cx, |c, cx| {
                                    c.start_cursor_blink(cx);
                                    cx.notify();
                                });
                                cx.notify();
                            }
                        }
                    }
                    return;
                }
                BlockEvent::RequestTableCellMoveVertical { delta } => {
                    let current = binding.position;
                    let table_block = binding.table_block.clone();
                    if let Some(grid) = table_block.read(cx).table_grid.clone() {
                        let total_rows = grid.rows.len() + 1;
                        let next_row = if *delta > 0 {
                            (current.row + 1).min(total_rows.saturating_sub(1))
                        } else {
                            current.row.saturating_sub(1)
                        };
                        let next_pos = TableCellPosition {
                            row: next_row,
                            column: current.column,
                        };
                        if let Some(cell) = grid.cell(next_pos) {
                            self.active_entity = Some(cell.clone());
                            cell.update(cx, |c, cx| {
                                c.start_cursor_blink(cx);
                                cx.notify();
                            });
                            cx.notify();
                        }
                    }
                    return;
                }
                _ => {}
            }
        }

        match event {
            BlockEvent::Changed => {
                self.pending_edit = true;
                let merge = self
                    .document_text(cx)
                    .map(|text| self.is_typing_continuation(&text, cx))
                    .unwrap_or(false);
                self.commit_document_edit(merge, cx);
                cx.notify();
            }
            BlockEvent::RequestFocus => {
                self.active_entity = Some(block.clone());
                cx.notify();
            }
            BlockEvent::RequestNewline {
                trailing,
                source_already_mutated: _,
            } => {
                if let Some(doc) = &mut self.document {
                    let Some(location) = doc.find_block_location(block.entity_id()) else {
                        return;
                    };
                    let current_kind = block.read(cx).kind();
                    let new_block = Self::new_block(
                        cx,
                        BlockData::new(current_kind.newline_sibling_kind(), trailing.clone()),
                    );
                    doc.insert_blocks_at(
                        location.parent,
                        location.index + 1,
                        vec![new_block.clone()],
                        cx,
                    );
                    self.active_entity = Some(new_block.clone());
                    new_block.update(cx, |b, cx| {
                        b.start_cursor_blink(cx);
                        cx.notify();
                    });
                    self.pending_edit = true;
                    self.commit_document_edit(false, cx);
                    cx.notify();
                }
            }
            BlockEvent::RequestNewlineAbove => {
                if let Some(doc) = &mut self.document {
                    let Some(location) = doc.find_block_location(block.entity_id()) else {
                        return;
                    };
                    let new_block = Self::new_block(
                        cx,
                        BlockData::new(BlockKind::Paragraph, BlockText::plain(String::new())),
                    );
                    doc.insert_blocks_at(
                        location.parent,
                        location.index,
                        vec![new_block.clone()],
                        cx,
                    );
                    self.active_entity = Some(new_block.clone());
                    new_block.update(cx, |b, cx| {
                        b.start_cursor_blink(cx);
                        cx.notify();
                    });
                    self.pending_edit = true;
                    self.commit_document_edit(false, cx);
                    cx.notify();
                }
            }
            BlockEvent::RequestMergeIntoPrevious { content } => {
                if let Some(doc) = &mut self.document {
                    let blocks = doc.blocks();
                    let current_idx = doc.index_for_entity_id(block.entity_id()).unwrap_or(0);
                    if current_idx > 0 {
                        let prev = blocks[current_idx - 1].entity.clone();
                        let cursor_pos = prev.read(cx).display_text().len();
                        let current_content = content.clone();
                        prev.update(cx, move |prev, cx| {
                            let mut text = prev.data.text.clone();
                            text.append(current_content);
                            prev.data.set_text(text);
                            prev.sync_render_cache();
                            prev.assign_collapsed_selection_offset(
                                cursor_pos,
                                CollapsedCaretAffinity::Default,
                                None,
                            );
                            prev.cursor_blink_epoch = std::time::Instant::now();
                            cx.notify();
                        });
                        doc.remove_block(block.entity_id(), cx);
                        self.active_entity = Some(prev.clone());
                        self.pending_edit = true;
                        self.commit_document_edit(false, cx);
                        cx.notify();
                    }
                }
            }
            BlockEvent::RequestDelete => {
                if let Some(doc) = &mut self.document {
                    let blocks = doc.blocks();
                    let count = blocks.len();
                    let current_idx = doc.index_for_entity_id(block.entity_id()).unwrap_or(0);
                    if count > 1 {
                        let target_idx = if current_idx > 0 { current_idx - 1 } else { 0 };
                        let target = blocks[target_idx].entity.clone();
                        doc.remove_block(block.entity_id(), cx);
                        self.active_entity = Some(target.clone());
                        target.update(cx, |t, cx| {
                            t.start_cursor_blink(cx);
                            cx.notify();
                        });
                    } else {
                        block.update(cx, |b, cx| {
                            b.data.text = BlockText::plain(String::new());
                            b.sync_render_cache();
                            cx.notify();
                        });
                    }
                    self.pending_edit = true;
                    self.commit_document_edit(false, cx);
                    cx.notify();
                }
            }
            BlockEvent::RequestFocusPrevious { preferred_x } => {
                if let Some(doc) = &self.document {
                    let blocks = doc.blocks();
                    let current_idx = doc.index_for_entity_id(block.entity_id()).unwrap_or(0);
                    if current_idx > 0 {
                        let prev = blocks[current_idx - 1].entity.clone();
                        prev.update(cx, |prev, cx| {
                            let offset =
                                prev.entry_offset_for_vertical_focus(true, preferred_x.map(px));
                            prev.move_to_with_preferred_x(offset, preferred_x.map(px), cx);
                            prev.start_cursor_blink(cx);
                            cx.notify();
                        });
                        self.active_entity = Some(prev.clone());
                        cx.notify();
                    }
                }
            }
            BlockEvent::RequestFocusNext { preferred_x } => {
                if let Some(doc) = &self.document {
                    let blocks = doc.blocks();
                    let current_idx = doc.index_for_entity_id(block.entity_id()).unwrap_or(0);
                    if current_idx + 1 < blocks.len() {
                        let next = blocks[current_idx + 1].entity.clone();
                        next.update(cx, |next, cx| {
                            let offset =
                                next.entry_offset_for_vertical_focus(false, preferred_x.map(px));
                            next.move_to_with_preferred_x(offset, preferred_x.map(px), cx);
                            next.start_cursor_blink(cx);
                            cx.notify();
                        });
                        self.active_entity = Some(next.clone());
                        cx.notify();
                    }
                }
            }
            BlockEvent::RequestIndent => {
                if let Some(doc) = &mut self.document {
                    let blocks = doc.blocks();
                    let current_idx = doc.index_for_entity_id(block.entity_id()).unwrap_or(0);
                    if current_idx > 0 {
                        if let Some(location) = doc.find_block_location(block.entity_id()) {
                            let target_parent = blocks[current_idx - 1].entity.clone();
                            if location.parent.as_ref().map(|p| p.entity_id())
                                != Some(target_parent.entity_id())
                            {
                                doc.remove_block(block.entity_id(), cx);
                                let child_index = target_parent.read(cx).children.len();
                                doc.insert_blocks_at(
                                    Some(target_parent.clone()),
                                    child_index,
                                    vec![block.clone()],
                                    cx,
                                );
                                self.active_entity = Some(block.clone());
                                self.pending_edit = true;
                                self.commit_document_edit(false, cx);
                                cx.notify();
                            }
                        }
                    }
                }
            }
            BlockEvent::RequestOutdent => {
                if let Some(doc) = &mut self.document {
                    if let Some(location) = doc.find_block_location(block.entity_id()) {
                        if let Some(parent) = location.parent.clone() {
                            if let Some(parent_location) =
                                doc.find_block_location(parent.entity_id())
                            {
                                doc.remove_block(block.entity_id(), cx);
                                doc.insert_blocks_at(
                                    parent_location.parent,
                                    parent_location.index + 1,
                                    vec![block.clone()],
                                    cx,
                                );
                                self.active_entity = Some(block.clone());
                            }
                        } else {
                            block.update(cx, |b, cx| b.convert_to_paragraph(cx));
                        }
                        self.pending_edit = true;
                        self.commit_document_edit(false, cx);
                        cx.notify();
                    }
                }
            }
            BlockEvent::RequestToggleTaskChecked => {
                block.update(cx, |b, cx| {
                    let checked = match b.kind() {
                        BlockKind::TaskListItem { checked } => checked,
                        _ => return,
                    };
                    b.data.kind = BlockKind::TaskListItem { checked: !checked };
                    b.sync_edit_mode_from_kind();
                    b.sync_render_cache();
                    b.cursor_blink_epoch = std::time::Instant::now();
                    cx.notify();
                });
                self.pending_edit = true;
                self.commit_document_edit(false, cx);
                cx.notify();
            }
            BlockEvent::RequestAppendTableColumn => {
                let target = block.clone();
                let mut modified = false;
                target.update(cx, |b, _cx| {
                    if let Some(table) = b.data.table.as_mut() {
                        crate::table::columns::append_table_column(table);
                        modified = true;
                    }
                });
                if modified {
                    self.rebuild_table_grids(cx);
                    self.pending_edit = true;
                    self.commit_document_edit(false, cx);
                    cx.notify();
                }
            }
            BlockEvent::RequestAppendTableRow => {
                let target = block.clone();
                let mut modified = false;
                target.update(cx, |b, _cx| {
                    if let Some(table) = b.data.table.as_mut() {
                        crate::table::rows::append_table_row(table);
                        modified = true;
                    }
                });
                if modified {
                    self.rebuild_table_grids(cx);
                    self.pending_edit = true;
                    self.commit_document_edit(false, cx);
                    cx.notify();
                }
            }
            BlockEvent::RequestTableAxisPreview { kind, index, hovered } => {
                block.update(cx, |b, cx| {
                    b.table_axis_preview = if *hovered {
                        Some(TableAxisMarker {
                            kind: *kind,
                            index: *index,
                        })
                    } else {
                        None
                    };
                    cx.notify();
                });
                cx.notify();
            }
            BlockEvent::RequestSelectTableAxis { kind, index } => {
                block.update(cx, |b, cx| {
                    b.table_axis_selection = Some(TableAxisMarker {
                        kind: *kind,
                        index: *index,
                    });
                    cx.notify();
                });
                cx.notify();
            }
            BlockEvent::RequestReorderTableAxis { kind, from, to } => {
                crate::table::axis::reorder_table_axis(&block, *kind, *from, *to, cx);
                self.rebuild_table_grids(cx);
                self.pending_edit = true;
                self.commit_document_edit(false, cx);
                cx.notify();
            }
            BlockEvent::RequestInsertTableAxisAt { kind, index } => {
                block.update(cx, |b, _cx| {
                    if let Some(table) = b.data.table.as_mut() {
                        match kind {
                            TableAxis::Column => {
                                crate::table::columns::insert_table_column_at(table, *index);
                            }
                            TableAxis::Row => {
                                crate::table::rows::insert_table_row_at(table, *index);
                            }
                        }
                    }
                });
                self.rebuild_table_grids(cx);
                self.pending_edit = true;
                self.commit_document_edit(false, cx);
                cx.notify();
            }
            BlockEvent::RequestFootnoteTooltip {
                id,
                content,
                position,
                show,
            } => {
                if *show {
                    let resolved_content = content.clone().or_else(|| {
                        let binding = self.references.footnotes.binding(id)?;
                        let doc = self.document.as_ref()?;
                        let def_entry = doc
                            .blocks()
                            .iter()
                            .find(|e| e.entity.read(cx).data.id == binding.definition_block_id)?;
                        let plain = def_entry.entity.read(cx).data.text.plain_text();
                        let text = markdown_parser::block::footnote::split_footnote_definition_text(&plain).1;
                        Some(SharedString::from(text.trim().to_string()))
                    });
                    if let Some(content) = resolved_content {
                        self.footnote_tooltip = Some(FootnoteTooltipState {
                            id: id.clone(),
                            content,
                            position: *position,
                        });
                        cx.notify();
                    }
                } else if self.footnote_tooltip.as_ref().map(|t| &t.id) == Some(id) {
                    self.footnote_tooltip = None;
                    cx.notify();
                }
            }
            BlockEvent::RequestJumpToFootnoteDefinition { id } => {
                if let Some(binding) = self.references.footnotes.binding(id)
                    && let Some(doc) = &self.document
                    && let Some(entry) = doc
                        .blocks()
                        .iter()
                        .find(|e| e.entity.read(cx).data.id == binding.definition_block_id)
                {
                    let target = entry.entity.clone();
                    self.active_entity = Some(target.clone());
                    target.update(cx, |b, cx| {
                        b.selected_range = 0..0;
                        b.selection_reversed = false;
                        b.marked_range = None;
                        b.start_cursor_blink(cx);
                        cx.notify();
                    });
                    cx.notify();
                }
            }
            BlockEvent::RequestJumpToFootnoteBackref { id } => {
                if let Some(binding) = self.references.footnotes.binding(id)
                    && let Some(first_ref) = binding.first_reference.as_ref()
                    && let Some(doc) = &self.document
                    && let Some(entry) = doc
                        .blocks()
                        .iter()
                        .find(|e| e.entity.read(cx).data.id == first_ref.block_id)
                {
                    let target = entry.entity.clone();
                    self.active_entity = Some(target.clone());
                    target.update(cx, |b, cx| {
                        b.sync_inline_projection_for_focus(true);
                        if let Some(range) =
                            b.display_range_for_footnote_occurrence(first_ref.occurrence_index)
                        {
                            b.selected_range = range;
                            b.selection_reversed = false;
                            b.marked_range = None;
                        }
                        b.start_cursor_blink(cx);
                        cx.notify();
                    });
                    cx.notify();
                }
            }
            BlockEvent::RequestOpenTableAxisMenu {
                kind,
                index,
                position,
            } => {
                let table_block_id = block.entity_id();
                self.context_menu = Some(WysiwygContextMenuState::TableAxis {
                    position: *position,
                    selection: TableAxisSelection {
                        table_block_id,
                        kind: *kind,
                        index: *index,
                    },
                });
                cx.notify();
            }
            _ => {}
        }
    }

    pub fn open_context_menu(
        &mut self,
        entity_id: EntityId,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        if let Some(doc) = &self.document
            && let Some(target) = doc.block_entity_by_id(entity_id)
        {
            self.active_entity = Some(target);
        }
        self.context_menu = Some(WysiwygContextMenuState::Edit {
            position,
            target_entity_id: Some(entity_id),
            active_submenu: None,
        });
        cx.notify();
    }

    pub fn set_context_menu_submenu(
        &mut self,
        submenu: Option<ContextSubmenu>,
        cx: &mut Context<Self>,
    ) {
        if let Some(WysiwygContextMenuState::Edit { active_submenu, .. }) = &mut self.context_menu {
            if *active_submenu != submenu {
                *active_submenu = submenu;
                cx.notify();
            }
        }
    }

    pub fn close_context_menu(&mut self, cx: &mut Context<Self>) {
        self.context_menu = None;
        cx.notify();
    }

    pub fn paste_plain_into_active(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(active) = &self.active_entity else {
            return;
        };
        if let Some(item) = cx.read_from_clipboard() {
            if let Some(text) = item.text() {
                active.update(cx, |b, cx| {
                    let range = b.selected_range.clone();
                    b.apply_source_space_text_edit(range, &text, None, false, cx);
                });
                self.pending_edit = true;
                self.commit_document_edit(false, cx);
                cx.notify();
            }
        }
    }

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
        doc.insert_blocks_at(location.parent, location.index + 1, vec![new_block.clone()], cx);
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
        doc.insert_blocks_at(location.parent, location.index + 1, vec![new_block.clone()], cx);
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
        let Some(doc) = &self.document else { return; };
        let Some(target) = doc.block_entity_by_id(table_block_id) else { return; };
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
        let Some(doc) = &self.document else { return; };
        let Some(target) = doc.block_entity_by_id(table_block_id) else { return; };
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
        let Some(doc) = &self.document else { return; };
        let Some(target) = doc.block_entity_by_id(table_block_id) else { return; };
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
        let Some(doc) = &self.document else { return; };
        let Some(target) = doc.block_entity_by_id(table_block_id) else { return; };
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
        let Some(doc) = &self.document else { return; };
        let Some(target) = doc.block_entity_by_id(table_block_id) else { return; };
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
        let Some(doc) = &self.document else { return; };
        let Some(target) = doc.block_entity_by_id(table_block_id) else { return; };
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
        let Some(doc) = &self.document else { return; };
        let Some(target) = doc.block_entity_by_id(table_block_id) else { return; };
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

    pub fn cut_active_selection(&mut self, cx: &mut Context<Self>) {
        let Some(active) = &self.active_entity else {
            return;
        };
        active.update(cx, |b, cx| {
            if !b.selected_range.is_empty() {
                let text = b.selected_text();
                cx.write_to_clipboard(ClipboardItem::new_string(text));
                b.apply_source_space_text_edit(b.selected_range.clone(), "", None, false, cx);
            } else {
                let text = b.data.text.plain_text();
                cx.write_to_clipboard(ClipboardItem::new_string(text));
                b.data.text = BlockText::plain(String::new());
                b.mark_changed(cx);
            }
        });
        self.pending_edit = true;
        self.commit_document_edit(false, cx);
        cx.notify();
    }

    pub fn copy_active_selection(&self, cx: &mut Context<Self>) {
        let Some(active) = &self.active_entity else {
            return;
        };
        active.read_with(cx, |b, cx| {
            let text = if !b.selected_range.is_empty() {
                b.selected_text()
            } else {
                b.data.text.plain_text()
            };
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        });
    }

    pub fn paste_into_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(active) = &self.active_entity else {
            return;
        };
        active.update(cx, |b, cx| {
            b.on_paste(&platform_contracts::actions::Paste, window, cx);
        });
    }

    pub fn toggle_active_format(&mut self, format: InlineFormat, cx: &mut Context<Self>) {
        let Some(active) = &self.active_entity else {
            return;
        };
        active.update(cx, |b, cx| {
            b.toggle_inline_format(format, cx);
        });
        self.pending_edit = true;
        self.commit_document_edit(false, cx);
        cx.notify();
    }

    pub fn wrap_active_selection(
        &mut self,
        left_delim: &str,
        right_delim: &str,
        cx: &mut Context<Self>,
    ) {
        let Some(active) = &self.active_entity else {
            return;
        };
        active.update(cx, |b, cx| {
            if !b.selected_range.is_empty() {
                let text = b.selected_text();
                let wrapped = format!("{left_delim}{text}{right_delim}");
                b.apply_source_space_text_edit(b.selected_range.clone(), &wrapped, None, false, cx);
            } else {
                let wrapped = format!("{left_delim}{right_delim}");
                let cur = b.selected_range.start;
                b.apply_source_space_text_edit(cur..cur, &wrapped, None, false, cx);
            }
        });
        self.pending_edit = true;
        self.commit_document_edit(false, cx);
        cx.notify();
    }

    pub fn clear_active_selection_format(&mut self, cx: &mut Context<Self>) {
        let Some(active) = &self.active_entity else {
            return;
        };
        active.update(cx, |b, cx| {
            if !b.selected_range.is_empty() {
                let text = b.selected_text();
                let cleaned = text
                    .replace("**", "")
                    .replace("~~", "")
                    .replace("==", "")
                    .replace(['*', '`', '$'], "");
                b.apply_source_space_text_edit(b.selected_range.clone(), &cleaned, None, false, cx);
            }
        });
        self.pending_edit = true;
        self.commit_document_edit(false, cx);
        cx.notify();
    }

    pub fn convert_target_block(
        &mut self,
        target_id: EntityId,
        kind: BlockKind,
        cx: &mut Context<Self>,
    ) {
        let Some(doc) = &self.document else {
            return;
        };
        let Some(target) = doc.block_entity_by_id(target_id) else {
            return;
        };
        target.update(cx, |b, cx| {
            b.data.kind = kind;
            b.mark_changed(cx);
        });
        self.pending_edit = true;
        self.commit_document_edit(false, cx);
        cx.notify();
    }

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
                vec![BlockText::plain(""), BlockText::plain(""), BlockText::plain("")],
                vec![BlockText::plain(""), BlockText::plain(""), BlockText::plain("")],
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
        doc.insert_blocks_at(location.parent, location.index + 1, vec![new_block.clone()], cx);
        self.rebuild_table_grids(cx);
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
            BlockData::new(BlockKind::CodeBlock { language: None }, BlockText::plain("")),
        );
        doc.insert_blocks_at(location.parent, location.index + 1, vec![new_block.clone()], cx);
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
        doc.insert_blocks_at(location.parent, location.index + 1, vec![new_block.clone()], cx);
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
        doc.insert_blocks_at(location.parent, location.index + 1, vec![new_block.clone()], cx);
        self.sync_reference_context(cx);
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
        doc.insert_blocks_at(location.parent, location.index + 1, vec![new_block.clone()], cx);
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
            self.rebuild_table_grids(cx);
            self.sync_reference_context(cx);
            self.pending_edit = true;
            self.commit_document_edit(false, cx);
            cx.notify();
        }
    }

    pub fn sync_document(
        &mut self,
        document: &editor_contracts::DocumentSnapshot,
        cx: &mut Context<Self>,
    ) {
        let next_base_dir = document
            .base_dir
            .as_deref()
            .map(std::path::Path::to_path_buf);
        let base_dir_changed = self.references.base_dir != next_base_dir;
        self.references.base_dir = next_base_dir;

        if self.synced_revision == Some(document.revision) && self.document.is_some() {
            if base_dir_changed {
                self.sync_reference_context(cx);
            }
            return;
        }
        if self.pending_edit {
            self.synced_revision = Some(document.revision);
            self.pending_edit = false;
            if base_dir_changed {
                self.sync_reference_context(cx);
            }
            return;
        }
        self.rebuild_from_markdown(&document.text, document.revision, cx);
        if let Some(hint) = document.restore_cursor {
            self.restore_cursor_hint(hint, cx);
        }
    }

    pub fn document_text(&self, cx: &App) -> Option<String> {
        self.document.as_ref().map(|d| d.serialize_markdown(cx))
    }

    /// Current caret position of the active block as a document-level
    /// cursor hint (1-based line/column in the serialized Markdown).
    pub fn cursor_hint(&self, cx: &App) -> CursorHint {
        let Some(doc) = &self.document else {
            return CursorHint::new(1, 1);
        };
        let Some(active) = &self.active_entity else {
            return CursorHint::new(1, 1);
        };
        let (lines, mappings) = doc.serialize_markdown_lines_with_mapping(cx);
        if lines.is_empty() || mappings.is_empty() {
            return CursorHint::new(1, 1);
        }
        let Some(mapping) = mappings.iter().find(|m| m.entity_id == active.entity_id()) else {
            return CursorHint::new(1, 1);
        };
        if mapping.own_start_line >= mapping.own_end_line || mapping.own_start_line >= lines.len() {
            return CursorHint::new((mapping.own_start_line + 1).min(lines.len()) as u32, 1);
        }

        let block = active.read(cx);
        let caret = block.cursor_offset();
        let intra = block.display_range_to_source_range(caret..caret);
        let markdown_text = block.data.text.serialize_markdown();
        let intra_offset = markdown_parser::inline::serialize::clamp_to_char_boundary(
            &markdown_text,
            intra.start.min(markdown_text.len()),
        );
        let before = &markdown_text[..intra_offset];
        let line_in_block = before.matches('\n').count();
        let col_in_block = before.rsplit('\n').next().unwrap_or("").chars().count();

        let num_own_lines = mapping.own_end_line - mapping.own_start_line;
        let line_offset_in_own = if block.kind().is_code_block() {
            (line_in_block + 1).min(num_own_lines.saturating_sub(1))
        } else {
            line_in_block.min(num_own_lines.saturating_sub(1))
        };
        let doc_line = mapping.own_start_line + line_offset_in_own;
        if doc_line >= lines.len() {
            return CursorHint::new(lines.len() as u32, 1);
        }

        let line_str = &lines[doc_line];
        let text_line = markdown_text.split('\n').nth(line_in_block).unwrap_or("");
        let prefix_bytes = if line_str.ends_with(text_line) {
            line_str.len() - text_line.len()
        } else {
            markdown_parser::inline::serialize::clamp_to_char_boundary(
                line_str,
                line_str.len().saturating_sub(text_line.len()),
            )
        };
        let prefix_chars = line_str[..prefix_bytes].chars().count();
        let col_chars = prefix_chars + col_in_block;

        CursorHint::new((doc_line + 1) as u32, (col_chars + 1) as u32)
    }

    /// Moves the active caret to the document position described by a
    /// cursor hint (used to apply `restore_cursor` after undo/redo).
    pub fn restore_cursor_hint(&mut self, hint: CursorHint, cx: &mut Context<Self>) {
        let Some(doc) = &self.document else {
            return;
        };
        let (lines, mappings) = doc.serialize_markdown_lines_with_mapping(cx);
        if mappings.is_empty() || lines.is_empty() {
            return;
        }

        let target_line = hint.line.saturating_sub(1) as usize;
        let target_col = hint.column.saturating_sub(1) as usize;

        let mut best = &mappings[0];
        for m in &mappings {
            if target_line >= m.own_start_line {
                best = m;
            }
            if target_line >= m.own_start_line && target_line < m.own_end_line {
                best = m;
                break;
            }
        }

        let Some(target_entity) = doc.block_entity_by_id(best.entity_id) else {
            return;
        };

        let block_ref = target_entity.read(cx);
        let markdown_text = block_ref.data.text.serialize_markdown();
        let num_own_lines = best.own_end_line.saturating_sub(best.own_start_line);
        let own_line_offset = target_line
            .saturating_sub(best.own_start_line)
            .min(num_own_lines.saturating_sub(1));
        let doc_line = (best.own_start_line + own_line_offset).min(lines.len().saturating_sub(1));
        let line_str = &lines[doc_line];

        let text_line_idx = if block_ref.kind().is_code_block() {
            own_line_offset.saturating_sub(1)
        } else {
            own_line_offset
        };

        let text_lines: Vec<&str> = markdown_text.split('\n').collect();
        let text_line = text_lines.get(text_line_idx).copied().unwrap_or("");
        let prefix_bytes = if line_str.ends_with(text_line) {
            line_str.len() - text_line.len()
        } else {
            markdown_parser::inline::serialize::clamp_to_char_boundary(
                line_str,
                line_str.len().saturating_sub(text_line.len()),
            )
        };
        let prefix_chars = line_str[..prefix_bytes].chars().count();
        let col_in_text_line = target_col
            .saturating_sub(prefix_chars)
            .min(text_line.chars().count());

        let mut source_offset = 0usize;
        for &prev in &text_lines[..text_line_idx.min(text_lines.len())] {
            source_offset += prev.len() + 1;
        }
        for ch in text_line.chars().take(col_in_text_line) {
            source_offset += ch.len_utf8();
        }
        let source_offset = markdown_parser::inline::serialize::clamp_to_char_boundary(
            &markdown_text,
            source_offset.min(markdown_text.len()),
        );

        self.active_entity = Some(target_entity.clone());
        target_entity.update(cx, |b, cx| {
            let display = b.source_range_to_display_range(source_offset..source_offset);
            let caret = display.start.min(b.display_len());
            b.selected_range = caret..caret;
            b.selection_reversed = false;
            b.marked_range = None;
            b.start_cursor_blink(cx);
            cx.notify();
        });
        cx.notify();
    }

    /// Commits the current document as one edit transaction.
    ///
    /// `merge` marks continuation of the previous undo transaction (typing
    /// runs). The caret hints anchor the buffer-level undo/redo restore.
    pub fn commit_document_edit(&mut self, merge: bool, cx: &mut App) {
        if let Some(edit) = self.take_edit_transaction(merge, cx) {
            if let Some(host) = self.host.clone() {
                host.commit_edit(edit, cx);
            }
        }
    }

    /// Builds the edit transaction for the current document state and
    /// updates the typing-run bookkeeping. Used both by direct commits and
    /// by contract methods that hand the transaction to the editor.
    pub fn take_edit_transaction(&mut self, merge: bool, cx: &App) -> Option<EditTransaction> {
        let text = self.document_text(cx)?;
        let cursor_after = self.cursor_hint(cx);
        let cursor_before = if merge {
            self.typing_run_start_hint.unwrap_or(cursor_after)
        } else {
            self.last_cursor_hint.unwrap_or(cursor_after)
        };

        if merge {
            if self.typing_run_start_hint.is_none() {
                self.typing_run_start_hint = self.last_cursor_hint;
            }
            self.last_typing_insert_at = self.single_char_insert_at(&text);
        } else {
            self.typing_run_start_hint = self.last_cursor_hint;
            self.last_typing_insert_at = None;
        }

        self.last_cursor_hint = Some(cursor_after);
        self.last_committed_text = Some(text.clone());
        Some(EditTransaction::new(
            text,
            merge,
            cursor_before,
            cursor_after,
        ))
    }

    /// Whether committing `new_text` continues the previous typing run: a
    /// single-character insertion at exactly the previous insertion point,
    /// or an update while an IME composition is active.
    fn is_typing_continuation(&self, new_text: &str, cx: &App) -> bool {
        if let Some(active) = &self.active_entity {
            if active.read(cx).marked_range.is_some() {
                return true;
            }
        }
        let Some(insert_at) = self.last_typing_insert_at else {
            return false;
        };
        self.single_char_insert_at(new_text) == Some(insert_at)
    }

    /// Insert position when `new_text` is a single-character insertion into
    /// the previously committed text.
    fn single_char_insert_at(&self, new_text: &str) -> Option<usize> {
        let old_text = self.last_committed_text.as_ref()?;
        if new_text.len() != old_text.len() + 1 {
            return None;
        }
        let old = old_text.as_bytes();
        let new = new_text.as_bytes();
        let mut prefix = 0;
        while prefix < old.len() && prefix < new.len() && old[prefix] == new[prefix] {
            prefix += 1;
        }
        if new_text.is_char_boundary(prefix)
            && prefix < new.len()
            && old[prefix..] == new[prefix + 1..]
        {
            Some(prefix)
        } else {
            None
        }
    }

    pub fn notify_document_changed(&mut self, cx: &mut App) {
        self.pending_edit = true;
        self.commit_document_edit(false, cx);
    }

    pub fn focus_handle(&self, cx: &App) -> Option<FocusHandle> {
        if let Some(active) = &self.active_entity {
            return Some(active.read(cx).focus_handle.clone());
        }
        if let Some(doc) = &self.document {
            if let Some(first) = doc.blocks().first() {
                return Some(first.entity.read(cx).focus_handle.clone());
            }
        }
        None
    }

    pub fn outline_headings(&self, cx: &App) -> Vec<OutlineNode> {
        if let Some(doc) = &self.document {
            crate::pane::outline::extract_outline_headings(doc, cx)
        } else {
            Vec::new()
        }
    }

    pub fn calculate_block_scroll_offset(
        &self,
        target_block_idx: usize,
        theme: &Theme,
        cx: &App,
    ) -> f32 {
        let Some(doc) = &self.document else {
            return 0.0;
        };
        let blocks = doc.blocks();
        let font_size = theme.typography.text_size.max(14.0);
        let line_height = (font_size * theme.typography.text_line_height)
            .round()
            .max(22.0);
        let mut y = 0.0;
        for (i, entry) in blocks.iter().enumerate() {
            if i >= target_block_idx {
                break;
            }
            let block = entry.entity.read(cx);
            let est_h = match block.kind() {
                markdown_parser::parse::BlockKind::Heading { level } => match level {
                    1 => line_height * 2.2 + 16.0,
                    2 => line_height * 1.8 + 14.0,
                    3 => line_height * 1.5 + 12.0,
                    _ => line_height * 1.3 + 10.0,
                },
                markdown_parser::parse::BlockKind::Paragraph => {
                    let len = block.data.text.plain_len();
                    let lines = (len / 60).max(1);
                    (lines as f32) * line_height + 10.0
                }
                markdown_parser::parse::BlockKind::CodeBlock { .. } => {
                    let lines = block.data.text.plain_text().lines().count().max(1);
                    (lines as f32) * line_height + 24.0
                }
                markdown_parser::parse::BlockKind::Table => line_height * 4.0 + 16.0,
                markdown_parser::parse::BlockKind::ThematicBreak => line_height * 1.0 + 8.0,
                _ => line_height * 1.5 + 8.0,
            };
            y += est_h;
        }
        y
    }

    pub fn navigate_to_outline(
        &mut self,
        index: usize,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Option<f32> {
        let headings = self.outline_headings(cx);
        if let Some(node) = headings.get(index) {
            if let Some(entity_id) = node.block_id {
                if let Some(doc) = &self.document {
                    if let Some(block) = doc.block_entity_by_id(entity_id) {
                        self.active_entity = Some(block.clone());
                        block.update(cx, |b, cx| {
                            b.assign_collapsed_selection_offset(
                                0,
                                CollapsedCaretAffinity::Default,
                                None,
                            );
                            b.start_cursor_blink(cx);
                            cx.notify();
                        });
                    }
                }
            }
            let target_y = self.calculate_block_scroll_offset(node.block_index, theme, cx);
            Some(target_y)
        } else {
            None
        }
    }

    pub fn search_matches(&self, query: &SearchQuery, cx: &App) -> Vec<SearchMatch> {
        if let Some(doc) = &self.document {
            crate::pane::search::search_in_document(doc, query, cx)
        } else {
            Vec::new()
        }
    }

    pub fn replace_match(
        &mut self,
        match_item: &SearchMatch,
        replace_with: &str,
        cx: &mut Context<Self>,
    ) -> Option<EditTransaction> {
        if let Some(doc) = &self.document {
            if let Some(entity_id) = match_item.entity_id {
                crate::pane::search::replace_in_block_entity(
                    doc,
                    entity_id,
                    match_item.byte_range.clone(),
                    replace_with,
                    cx,
                );
                self.pending_edit = true;
                cx.notify();
                return self.take_edit_transaction(false, cx);
            }
        }
        None
    }

    pub fn navigate_to_search_match(
        &mut self,
        match_item: &SearchMatch,
        cx: &mut Context<Self>,
    ) -> Option<f32> {
        let mut target_y = None;
        if let Some(doc) = &self.document {
            if let Some(entity_id) = match_item.entity_id {
                if let Some(block) = doc.block_entity_by_id(entity_id) {
                    self.active_entity = Some(block.clone());
                    block.update(cx, |b, cx| {
                        b.selected_range = match_item.byte_range.clone();
                        b.selection_reversed = false;
                        b.start_cursor_blink(cx);
                        cx.notify();
                    });
                }
                if let Some(block_index) = doc.index_for_entity_id(entity_id) {
                    let theme = cx.global::<ThemeManager>().current_arc();
                    target_y = Some(self.calculate_block_scroll_offset(block_index, &theme, cx));
                }
            }
        }
        target_y
    }

    /// The selected display text of the active block, when non-empty.
    pub fn selected_text(&self, cx: &App) -> Option<String> {
        self.active_entity.as_ref().and_then(|active| {
            let block = active.read(cx);
            if block.selected_range.is_empty() {
                None
            } else {
                Some(block.selected_text())
            }
        })
    }

    /// Deletes the active block's selection and returns the resulting edit
    /// transaction. The block's change event also commits it — the buffer
    /// dedupes the identical text, so the caller's commit is the one that
    /// lands with correct caret hints.
    pub fn delete_selection(&mut self, cx: &mut Context<Self>) -> Option<EditTransaction> {
        let active = self.active_entity.clone()?;
        if active.read(cx).selected_range.is_empty() {
            return None;
        }
        active.update(cx, |block, cx| {
            let range = block.selected_range.clone();
            block.replace_text_in_display_range(range, "", Some(0..0), false, cx);
        });
        self.pending_edit = true;
        cx.notify();
        self.take_edit_transaction(false, cx)
    }

    /// Inserts text at the active block's selection/caret and returns the
    /// resulting edit transaction. The block's change event also commits
    /// it — the buffer dedupes the identical text, so the caller's commit
    /// is the one that lands with correct caret hints.
    pub fn insert_text(&mut self, text: &str, cx: &mut Context<Self>) -> Option<EditTransaction> {
        let active = self.active_entity.clone()?;
        active.update(cx, |block, cx| {
            let range = block.selected_range.clone();
            if range.is_empty() {
                let cursor = block.cursor_offset();
                block.replace_text_in_display_range(cursor..cursor, text, None, false, cx);
            } else {
                block.replace_text_in_display_range(range, text, None, false, cx);
            }
        });
        self.pending_edit = true;
        cx.notify();
        self.take_edit_transaction(false, cx)
    }

    /// Selects the whole active block.
    pub fn select_all(&mut self, cx: &mut Context<Self>) {
        if let Some(active) = self.active_entity.clone() {
            active.update(cx, |block, cx| {
                block.select_all_text(cx);
            });
        }
    }

    pub fn render(
        &mut self,
        ctx: &PaneRenderContext,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.host = Some(ctx.host.clone());

        let theme = cx.global::<theme::ThemeManager>().current_arc();
        let d = &theme.dimensions;
        let c = &theme.colors;
        let pane_id = ctx.pane_id;

        if let Some(doc) = &self.document {
            let blocks = doc.blocks();
            let plans = crate::render::viewport::plan_document_rows(blocks, d, cx);
            let centered_width = crate::render::layout::centered_column_width(
                f32::from(ctx.scroll.bounds().size.width).max(600.0),
                d,
            );

            let row_elements: Vec<AnyElement> = plans
                .iter()
                .map(|plan| {
                    crate::render::viewport::build_planned_row_element(
                        plan,
                        blocks,
                        centered_width,
                        &theme,
                        d,
                        |row: Div, entity_id: EntityId| {
                            row.on_mouse_down(
                                MouseButton::Right,
                                cx.listener(move |this, event: &MouseDownEvent, _window, cx| {
                                    this.open_context_menu(entity_id, event.position, cx);
                                }),
                            )
                        },
                    )
                })
                .collect();

            let headings = self.outline_headings(cx);
            let scroll_y = -f32::from(ctx.scroll.offset().y);
            let active_index = headings
                .iter()
                .rposition(|node| {
                    let node_y = self.calculate_block_scroll_offset(node.block_index, &theme, cx);
                    node_y <= scroll_y + 30.0
                })
                .or(if headings.is_empty() { None } else { Some(0) });

            let outline_host: std::sync::Arc<dyn editor_contracts::OutlineHost> =
                std::sync::Arc::new(PaneOutlineHost {
                    pane_id: ctx.pane_id,
                    host: ctx.host.clone(),
                });

            let outline_hud = ui::render_floating_outline_hud(
                ctx.pane_id.0,
                &headings,
                active_index,
                ctx.is_outline_hovered,
                &theme,
                &outline_host,
            );

            let scroll_bounds = ctx.scroll.bounds();
            let origin = scroll_bounds.origin;
            let pane_size = scroll_bounds.size;
            let pane_width = f32::from(pane_size.width);

            let footnote_tooltip_element = self.footnote_tooltip.as_ref().map(|tooltip| {
                let top = (tooltip.position.y - origin.y + px(4.0)).max(px(0.0));
                let max_width = 420.0_f32;
                let mut left_f32 = f32::from(tooltip.position.x - origin.x);
                if left_f32 + 200.0 > pane_width {
                    left_f32 = (pane_width - max_width.min(pane_width) - 16.0).max(8.0);
                }
                let left = px(left_f32.max(8.0));
                div()
                    .absolute()
                    .occlude()
                    .left(left)
                    .top(top)
                    .max_w(px(max_width))
                    .px(px(10.0))
                    .py(px(6.0))
                    .rounded(px(5.0))
                    .bg(c.dialog_surface)
                    .border(px(1.0))
                    .border_color(c.dialog_border)
                    .shadow_md()
                    .text_size(px(13.0))
                    .text_color(c.dialog_muted)
                    .line_height(relative(1.5))
                    .child(tooltip.content.clone())
                    .into_any_element()
            });

            let context_menu_element = self.context_menu.clone().map(|menu_state| {
                crate::render::context_menu::render_wysiwyg_context_menu(
                    self,
                    &menu_state,
                    origin,
                    pane_size,
                    &theme,
                    window,
                    cx,
                )
            });

            div()
                .id(ElementId::Name(
                    format!("tiled-wysiwyg-editor-{pane_id}").into(),
                ))
                .key_context("Wysiwyg")
                .w_full()
                .h_full()
                .relative()
                .bg(c.editor_background)
                .child(
                    div()
                        .id(ElementId::Name(
                            format!("tiled-wysiwyg-scroll-{pane_id}").into(),
                        ))
                        .w_full()
                        .h_full()
                        .flex()
                        .flex_col()
                        .items_center()
                        .overflow_y_scroll()
                        .track_scroll(ctx.scroll)
                        .p(px(d.editor_padding))
                        .pb(px(d.editor_padding + 200.0))
                        .children(row_elements),
                )
                .child(outline_hud)
                .children(footnote_tooltip_element)
                .children(context_menu_element)
                .into_any_element()
        } else {
            div().into_any_element()
        }
    }
}
