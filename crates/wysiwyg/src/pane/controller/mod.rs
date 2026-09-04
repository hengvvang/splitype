//! Self-contained WYSIWYG document controller — autonomous block editor engine.

pub mod blocks;
pub mod contract;
pub mod events;
pub mod format;
pub mod sync_commit;
pub mod tables;

use std::sync::Arc;

use editor_contracts::{CursorHint, PaneOutlineHost, PaneRenderContext};
use gpui::{
    AnyElement, App, AppContext, Context, Div, ElementId, Entity, EntityId, InteractiveElement,
    IntoElement, MouseButton, MouseDownEvent, ParentElement, Pixels, Point, SharedString,
    StatefulInteractiveElement, Styled, Window, div, px, relative,
};

use crate::model::Document;
use crate::model::block::Block;
use crate::model::references::ReferenceRegistries;
use crate::table::axis::TableAxisSelection;
use crate::table::grid::{TableCellBinding, TableGrids};
use markdown_parser::block::table::{TableCellPosition, TableColumnAlignment};
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
    TableResize {
        position: Point<Pixels>,
        table_block_id: EntityId,
        current_rows: usize,
        current_cols: usize,
        hovered_rows: Option<usize>,
        hovered_cols: Option<usize>,
    },
    TableInsert {
        position: Point<Pixels>,
        target_entity_id: Option<EntityId>,
        hovered_rows: Option<usize>,
        hovered_cols: Option<usize>,
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
    /// Byte length of the last snapshot synced from the buffer; the
    /// full-text replacement commit uses it as the replaced range.
    last_synced_len: usize,
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
            last_synced_len: 0,
            last_typing_insert_at: None,
            typing_run_start_hint: None,
            last_cursor_hint: None,
        };
        // Prefer the buffer's shared block projection when it matches the
        // revision; otherwise parse the text once at creation.
        if let Some(blocks) = &document.blocks {
            controller.rebuild_from_blocks(blocks, document.revision, document.text.len(), cx);
        } else {
            controller.rebuild_from_markdown(&document.text, document.revision, cx);
        }
        controller
    }

    /// Creates a new block entity and subscribes this controller to its `BlockEvent` stream.
    pub fn new_block(cx: &mut Context<Self>, data: BlockData) -> Entity<Block> {
        let block = cx.new(|cx| Block::with_data(cx, data));
        cx.subscribe(&block, Self::on_block_event).detach();
        block
    }

    /// Rebuilds document tree and event subscriptions from raw Markdown
    /// text (fallback path when the buffer's shared block projection lags
    /// behind the text revision).
    pub fn rebuild_from_markdown(&mut self, text: &str, revision: u64, cx: &mut Context<Self>) {
        let parsed = markdown_parser::parse::parse_wysiwyg_document(text);
        self.rebuild_from_blocks(&parsed, revision, text.len(), cx);
    }

    /// Rebuilds the view's block entity tree from the document-level block
    /// projection: the buffer parses the Markdown once and every structured
    /// pane derives its view entities from the same data — per-pane view
    /// state (carets, focus, expansion) is the only thing each pane owns.
    pub fn rebuild_from_blocks(
        &mut self,
        parsed: &[markdown_parser::parse::BlockData],
        revision: u64,
        text_len: usize,
        cx: &mut Context<Self>,
    ) {
        let block_count = parsed.len();
        let mut entities: std::collections::HashMap<uuid::Uuid, Entity<Block>> =
            std::collections::HashMap::with_capacity(block_count);

        for block_data in parsed {
            let entity = Self::new_block(cx, block_data.clone());
            entities.insert(block_data.id.0, entity);
        }

        for block_data in parsed {
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
        self.last_synced_len = text_len;
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
            entry
                .entity
                .update(cx, |block, _cx| block.clear_table_grid());
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
                            let cell =
                                Self::new_block(cx, BlockData::new(BlockKind::Paragraph, text));
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
        self.references.footnotes = Arc::new(crate::model::references::rebuild_footnote_registry(
            document, cx,
        ));
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
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _event, _window, cx| {
                                this.clear_all_table_axis_selections(cx);
                            }),
                        )
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
