//! Self-contained WYSIWYG document controller — autonomous block editor engine.

pub mod blocks;
pub mod contract;
pub mod events;
pub mod format;
pub mod sync_commit;
pub mod tables;

use std::sync::Arc;

use editor_contracts::{CursorHint, PaneRenderContext, render_pane_layout};
use gpui::{
    AnyElement, App, AppContext, Context, Div, ElementId, Entity, EntityId, FontWeight,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement, Pixels, Point,
    SharedString, StatefulInteractiveElement, Styled, Task, Window, div, px, relative,
};
use ui::{render_horizontal_scrollbar, render_pane_breadcrumb, render_vertical_scrollbar};

use crate::model::Document;
use crate::model::block::Block;
use crate::model::references::ReferenceRegistries;
use crate::table::axis::TableAxisSelection;
use crate::table::grid::{TableCellBinding, TableGrids};
use markdown_parser::inline::text::BlockText;
use markdown_parser::parse::{BlockData, BlockKind};

/// State for a floating footnote definition tooltip.
#[derive(Clone, Debug)]
pub struct FootnoteTooltipState {
    pub id: String,
    pub content: SharedString,
    pub position: Point<Pixels>,
}

/// Kind of link hover tooltip.
#[derive(Clone, Debug, PartialEq)]
pub enum LinkTooltipKind {
    /// External web link: minimal single-line indicator.
    ExternalUrl {
        url: String,
    },
    /// Internal note or heading/block: rich PKM preview card.
    NotePreview {
        title: String,
        anchor: Option<String>,
        snippet: SharedString,
    },
}

/// State for a floating link target/preview tooltip.
#[derive(Clone, Debug)]
pub struct LinkTooltipState {
    pub target: String,
    pub kind: LinkTooltipKind,
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
    /// Pane id this controller renders into, captured on the first render.
    pub pane_id: Option<editor_contracts::PaneId>,
    pub document: Option<Document>,
    pub synced_revision: Option<u64>,
    /// Local edits exist that the host's next snapshot has not acknowledged
    /// yet; `sync_document` consumes this instead of rebuilding.
    pub pending_edit: bool,
    pub deferred_commit: bool,
    pub active_entity: Option<Entity<Block>>,
    pub tables: TableGrids,
    pub references: ReferenceRegistries,
    pub footnote_tooltip: Option<FootnoteTooltipState>,
    pub link_tooltip: Option<LinkTooltipState>,
    pub link_hover_task: Option<Task<()>>,
    pub hovered_link_target: Option<String>,
    pub context_menu: Option<WysiwygContextMenuState>,
    /// Serialization of the document after the previous commit, used as
    /// the diff baseline for the next commit (which therefore carries
    /// only the changed bytes) and to detect typing-run continuations
    /// (single-character insertions at the same position) for undo
    /// grouping.
    last_committed: Option<crate::pane::controller::sync_commit::Serialization>,
    /// Byte length of the last snapshot synced from the buffer; the
    /// first commit after a sync replaces that whole range.
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
            pane_id: None,
            document: None,
            synced_revision: None,
            pending_edit: false,
            deferred_commit: false,
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
            link_tooltip: None,
            link_hover_task: None,
            hovered_link_target: None,
            context_menu: None,
            last_committed: None,
            last_synced_len: 0,
            last_typing_insert_at: None,
            typing_run_start_hint: None,
            last_cursor_hint: None,
        };
        // The buffer's block projection always matches the revision; the
        // pane derives its view entities from it without parsing.
        controller.rebuild_from_blocks(
            document.blocks.as_slice(),
            document.revision,
            document.text.len(),
            cx,
        );
        controller
    }

    /// Creates a new block entity and subscribes this controller to its `BlockEvent` stream.
    pub fn new_block(cx: &mut Context<Self>, data: BlockData) -> Entity<Block> {
        let block = cx.new(|cx| Block::with_data(cx, data));
        cx.subscribe(&block, Self::on_block_event).detach();
        block
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
        self.apply_block_projection(parsed, revision, text_len, false, cx);
    }

    /// Applies a new block projection to the existing entity tree by
    /// content-addressed diffing: entities whose content is unchanged at
    /// the same DFS position are reused (keeping their view state); only
    /// the changed middle segment is rebuilt. Falls back to a full rebuild
    /// when there is no previous tree.
    fn patch_from_blocks(
        &mut self,
        parsed: &[markdown_parser::parse::BlockData],
        revision: u64,
        text_len: usize,
        cx: &mut Context<Self>,
    ) {
        self.apply_block_projection(parsed, revision, text_len, true, cx);
    }

    /// Shared implementation of the projection application. `patch`
    /// enables the content-addressed reuse of unchanged entities.
    fn apply_block_projection(
        &mut self,
        parsed: &[markdown_parser::parse::BlockData],
        revision: u64,
        text_len: usize,
        patch: bool,
        cx: &mut Context<Self>,
    ) {
        let old_entries: Vec<Entity<Block>> = match (&self.document, patch) {
            (Some(doc), true) => doc.index.entries.iter().map(|e| e.entity.clone()).collect(),
            _ => Vec::new(),
        };
        let old_datas: Vec<markdown_parser::parse::BlockData> = old_entries
            .iter()
            .map(|entity| entity.read(cx).data.clone())
            .collect();

        // Content-addressed diff: match unchanged blocks at both ends of the
        // DFS sequence; the middle segment is rebuilt from scratch.
        let (prefix, suffix) = diff_block_sequences(&old_datas, parsed);

        let mut entities: Vec<Entity<Block>> = Vec::with_capacity(parsed.len());
        // Unchanged prefix: reuse entities, adopt the new ids.
        for (entity, data) in old_entries[..prefix].iter().zip(&parsed[..prefix]) {
            entity.update(cx, |block, _cx| {
                block.data = data.clone();
                block.children.clear();
            });
            entities.push(entity.clone());
        }
        // Changed middle: build fresh entities.
        for data in &parsed[prefix..parsed.len() - suffix] {
            entities.push(Self::new_block(cx, data.clone()));
        }
        // Unchanged suffix: reuse, adopt ids.
        for (entity, data) in old_entries[old_entries.len() - suffix..]
            .iter()
            .zip(&parsed[parsed.len() - suffix..])
        {
            entity.update(cx, |block, _cx| {
                block.data = data.clone();
                block.children.clear();
            });
            entities.push(entity.clone());
        }

        // Reassemble the tree from the new projection's ids.
        let entities_by_id: std::collections::HashMap<uuid::Uuid, Entity<Block>> = entities
            .iter()
            .map(|entity| (entity.read(cx).data.id.0, entity.clone()))
            .collect();
        for data in parsed {
            if data.children.is_empty() {
                continue;
            }
            let Some(parent) = entities_by_id.get(&data.id.0) else {
                continue;
            };
            let children: Vec<Entity<Block>> = data
                .children
                .iter()
                .filter_map(|child_id| entities_by_id.get(&child_id.0).cloned())
                .collect();
            if !children.is_empty() {
                parent.update(cx, |parent, _cx| parent.children.extend(children));
            }
        }
        let mut roots: Vec<Entity<Block>> = parsed
            .iter()
            .filter(|block| block.parent.is_none())
            .filter_map(|block| entities_by_id.get(&block.id.0).cloned())
            .collect();
        if roots.is_empty() {
            let empty_block = Self::new_block(
                cx,
                BlockData::new(BlockKind::Paragraph, BlockText::plain(String::new())),
            );
            roots.push(empty_block);
        }

        // Entities built or re-adopted in this patch are the only ones whose
        // derived state (table grids, reference contexts) can have changed.
        let changed: Vec<Entity<Block>> = entities[prefix..entities.len() - suffix].to_vec();
        let changed_tables: Vec<Entity<Block>> = changed
            .iter()
            .filter(|entity| entity.read(cx).kind() == BlockKind::Table)
            .cloned()
            .collect();

        let mut doc = Document::new(roots);
        // The freshly assembled tree is structurally clean by construction
        // (children only hang off container kinds), so the index DFS needs no
        // normalization pass.
        doc.rebuild_metadata_from_clean_tree(cx);
        // Keep the active entity when its block survived the patch.
        let active_survived = self
            .active_entity
            .as_ref()
            .is_some_and(|active| entities_by_id.contains_key(&active.read(cx).data.id.0));
        if !active_survived {
            self.active_entity = doc.blocks().first().map(|b| b.entity.clone());
        }
        self.document = Some(doc);
        self.synced_revision = Some(revision);
        self.pending_edit = false;
        self.last_committed = None;
        self.last_synced_len = text_len;
        self.last_typing_insert_at = None;
        self.typing_run_start_hint = None;
        self.last_cursor_hint = None;
        self.rebuild_table_grids(&changed_tables, cx);
        self.sync_reference_context(Some(&changed), cx);
    }

    /// Rebuilds table grid structures and bindings for the given table
    /// blocks (all tables on the first build, only the changed ones on
    /// patches). Bindings whose table no longer exists are dropped.
    pub fn rebuild_table_grids(
        &mut self,
        changed_tables: &[Entity<Block>],
        cx: &mut Context<Self>,
    ) {
        let Some(doc) = &self.document else {
            return;
        };
        if !changed_tables.is_empty() {
            self.tables.axis_preview = None;
            self.tables.axis_selection = None;
        }

        // Drop bindings of tables that no longer exist in the document.
        let live_table_ids: std::collections::HashSet<EntityId> = doc
            .index
            .table_entities
            .iter()
            .map(|entity| entity.entity_id())
            .collect();
        self.tables
            .cells
            .retain(|_, binding| live_table_ids.contains(&binding.table_block.entity_id()));

        for table_block in changed_tables {
            let Some(table) = table_block.read(cx).data.table.clone() else {
                continue;
            };
            self.tables
                .cells
                .retain(|_, binding| binding.table_block.entity_id() != table_block.entity_id());
            let bindings = crate::table::grid::install_table_grid_for_block(
                table_block,
                &table,
                |text, position, alignment, cx| {
                    let cell = Self::new_block(cx, BlockData::new(BlockKind::Paragraph, text));
                    cell.update(cx, |b, _cx| b.set_table_cell_mode(position, alignment));
                    let binding = TableCellBinding {
                        table_block: table_block.clone(),
                        cell: cell.clone(),
                        position,
                    };
                    (cell, binding)
                },
                cx,
            );
            for binding in bindings {
                self.tables.cells.insert(binding.cell.entity_id(), binding);
            }
        }
    }

    /// Syncs block reference contexts against the document-wide registries.
    ///
    /// `changed` is the entity set a projection patch rebuilt (`None` means
    /// everything — the initial build or a base-directory change). A patch
    /// whose changed blocks cannot contribute to or depend on the registries
    /// (no links, images, footnotes, raw/definition text, tables) needs no
    /// registry rebuild and no per-block resync at all.
    fn sync_reference_context(&mut self, changed: Option<&[Entity<Block>]>, cx: &mut App) {
        let Some(document) = &self.document else {
            return;
        };
        let registry_affected = match changed {
            None => true,
            Some(changed) => changed.iter().any(|entity| {
                let block = entity.read(cx);
                block.kind() == BlockKind::Table
                    || crate::model::references::block_has_registry_candidates(block)
            }),
        };
        if changed.is_some_and(|changed| changed.is_empty()) && !registry_affected {
            return;
        }
        if registry_affected {
            self.references.footnotes = Arc::new(
                crate::model::references::rebuild_footnote_registry(document, cx),
            );
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
        } else {
            for entity in changed.into_iter().flatten() {
                crate::model::references::sync_reference_context_for_block(
                    entity,
                    self.references.base_dir.as_deref(),
                    self.references.image.clone(),
                    self.references.link.clone(),
                    self.references.footnotes.clone(),
                    cx,
                );
            }
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
        self.pane_id = Some(ctx.pane_id);

        let theme = cx.global::<theme::ThemeManager>().current_arc();
        let d = &theme.dimensions;
        let c = &theme.colors;
        let pane_id = ctx.pane_id;

        if let Some(doc) = &self.document {
            let blocks = doc.blocks();
            let plans = crate::render::viewport::plan_document_rows(blocks, d);
            let scroll_bounds = ctx.scroll.bounds();
            let origin = scroll_bounds.origin;
            let pane_size = scroll_bounds.size;
            let pane_width = f32::from(pane_size.width);
            let pane_height = f32::from(pane_size.height);

            let scroll_width = pane_width;
            let viewport_width = if scroll_width > 0.0 {
                scroll_width
            } else if let Some(estimated) = ctx.estimated_viewport_width.filter(|&w| w > 0.0) {
                estimated
            } else {
                let win_w = f32::from(window.viewport_size().width);
                if win_w > 0.0 { win_w } else { 800.0 }
            };
            let centered_width = crate::render::layout::centered_column_width(viewport_width, d);

            let scroll_y = (-f32::from(ctx.scroll.offset().y)).max(0.0);
            let viewport_height = if pane_height > 0.0 {
                pane_height
            } else {
                let win_h = f32::from(window.viewport_size().height);
                if win_h > 0.0 { win_h } else { 800.0 }
            };

            let offsets = crate::render::blocks::viewport::cumulative_row_offsets(&plans);
            let (mut start_row, mut end_row) = crate::render::blocks::viewport::visible_row_window(
                &offsets,
                scroll_y,
                scroll_y + viewport_height,
                6,
            );

            if let Some(active) = &self.active_entity {
                let active_id = active.entity_id();
                if let Some(active_row) = plans.iter().position(|p| {
                    blocks[p.start..p.end].iter().any(|b| b.entity.entity_id() == active_id)
                }) {
                    start_row = start_row.min(active_row);
                    end_row = end_row.max(active_row + 1);
                }
            }

            start_row = start_row.min(plans.len());
            end_row = end_row.min(plans.len()).max(start_row);

            let top_spacer = if start_row < offsets.len() {
                offsets[start_row]
            } else {
                0.0
            };
            let bottom_spacer = if end_row < offsets.len() {
                offsets.last().copied().unwrap_or(0.0) - offsets[end_row]
            } else {
                0.0
            };

            let mut row_elements: Vec<AnyElement> = Vec::with_capacity(end_row - start_row + 2);
            if top_spacer > 0.0 {
                row_elements.push(
                    div()
                        .id(ElementId::Name(format!("top-scroll-spacer-{pane_id}").into()))
                        .w(px(centered_width))
                        .h(px(top_spacer))
                        .flex_shrink_0()
                        .into_any_element(),
                );
            }
            for plan in &plans[start_row..end_row] {
                row_elements.push(crate::render::viewport::build_planned_row_element(
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
                ));
            }
            if bottom_spacer > 0.0 {
                row_elements.push(
                    div()
                        .id(ElementId::Name(format!("bottom-scroll-spacer-{pane_id}").into()))
                        .w(px(centered_width))
                        .h(px(bottom_spacer))
                        .flex_shrink_0()
                        .into_any_element(),
                );
            }

            let footnote_tooltip_element = self.footnote_tooltip.as_ref().map(|tooltip| {
                let top = (tooltip.position.y - origin.y + px(4.0)).max(px(0.0));
                let max_width = 420.0_f32;
                let mut left_f32 = f32::from(tooltip.position.x - origin.x);
                if left_f32 + 200.0 > pane_width {
                    left_f32 = (pane_width - max_width.min(pane_width) - 16.0).max(8.0);
                }
                let left = px(left_f32.max(8.0));
                ui::tooltip_container(c, d)
                    .absolute()
                    .left(left)
                    .top(top)
                    .max_w(px(max_width))
                    .child(tooltip.content.clone())
                    .into_any_element()
            });

            let link_tooltip_element = self.link_tooltip.as_ref().map(|tooltip| {
                let top = (tooltip.position.y - origin.y + px(18.0)).max(px(0.0));
                let mut left_f32 = f32::from(tooltip.position.x - origin.x);
                match &tooltip.kind {
                    LinkTooltipKind::ExternalUrl { url } => {
                        let max_width = 380.0_f32;
                        if left_f32 + 200.0 > pane_width {
                            left_f32 = (pane_width - max_width.min(pane_width) - 16.0).max(8.0);
                        }
                        let left = px(left_f32.max(8.0));
                        div()
                            .occlude()
                            .absolute()
                            .left(left)
                            .top(top)
                            .max_w(px(max_width))
                            .rounded(px(d.button_radius))
                            .bg(c.dialog_surface)
                            .border(px(1.0))
                            .border_color(c.dialog_border)
                            .shadow_md()
                            .px(px(8.0))
                            .py(px(4.0))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(c.dialog_title)
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .child(url.clone()),
                            )
                            .into_any_element()
                    }
                    LinkTooltipKind::NotePreview {
                        title,
                        anchor,
                        snippet,
                    } => {
                        let max_width = 460.0_f32;
                        if left_f32 + 250.0 > pane_width {
                            left_f32 = (pane_width - max_width.min(pane_width) - 16.0).max(8.0);
                        }
                        let left = px(left_f32.max(8.0));
                        div()
                            .occlude()
                            .absolute()
                            .left(left)
                            .top(top)
                            .max_w(px(max_width))
                            .rounded(px(d.button_radius))
                            .bg(c.dialog_surface)
                            .border(px(1.0))
                            .border_color(c.dialog_border)
                            .shadow_md()
                            .p(px(10.0))
                            .flex()
                            .flex_col()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap(px(6.0))
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(c.dialog_title)
                                            .child(title.clone()),
                                    )
                                    .children(anchor.as_ref().map(|a| {
                                        div()
                                            .text_size(px(11.0))
                                            .px(px(4.0))
                                            .py(px(1.0))
                                            .rounded(px(3.0))
                                            .bg(c.focus_accent.opacity(0.12))
                                            .text_color(c.focus_accent)
                                            .child(if a.starts_with('^') {
                                                format!("^{}", &a[1..])
                                            } else {
                                                format!("#{}", a)
                                            })
                                    })),
                            )
                            .child(div().h(px(1.0)).bg(c.dialog_border))
                            .child(
                                div()
                                    .max_h(px(200.0))
                                    .overflow_hidden()
                                    .text_size(px(12.5))
                                    .text_color(c.dialog_body)
                                    .line_height(relative(1.45))
                                    .child(snippet.clone()),
                            )
                            .into_any_element()
                    }
                }
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

            let host_toggle = ctx.host.clone();
            let host_links = ctx.host.clone();
            let host_search = ctx.host.clone();
            let breadcrumb = render_pane_breadcrumb(
                ("wysiwyg-breadcrumb", pane_id.as_usize()),
                ctx.file_path,
                ctx.is_outline_docked,
                ctx.is_links_open,
                ctx.is_search_open,
                &theme,
                move |_event, _window, cx| {
                    host_toggle.toggle_outline_docked(pane_id, cx);
                },
                move |_event, _window, cx| {
                    host_links.toggle_links_widget(pane_id, cx);
                },
                move |_event, window, cx| {
                    host_search.toggle_search(pane_id, window, cx);
                },
            );

            let outline_indicator: Option<AnyElement> = None;

            let v_scrollbar = render_vertical_scrollbar(
                ("wysiwyg-v-scrollbar", pane_id.as_usize()),
                ctx.scroll,
                c,
                d,
                window,
                cx,
            );

            let h_scrollbar = render_horizontal_scrollbar(
                ("wysiwyg-h-scrollbar", pane_id.as_usize()),
                ctx.scroll,
                c,
                d,
                window,
                cx,
            );

            let content = div()
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
                .children(footnote_tooltip_element)
                .children(link_tooltip_element)
                .children(context_menu_element)
                .into_any_element();

            render_pane_layout(
                pane_id,
                Some(breadcrumb),
                content,
                outline_indicator,
                v_scrollbar,
                h_scrollbar,
            )
        } else {
            div().into_any_element()
        }
    }
}

/// Content-addressed diff of two DFS-ordered block sequences: the count of
/// equal blocks at the start and the end. Everything between the matched
/// prefix and suffix is rebuilt. Equality uses
/// [`markdown_parser::parse::BlockData::content_eq`], which ignores
/// parse-generated ids and parent links.
fn diff_block_sequences(
    old: &[markdown_parser::parse::BlockData],
    new: &[markdown_parser::parse::BlockData],
) -> (usize, usize) {
    let mut prefix = 0usize;
    while prefix < old.len() && prefix < new.len() && old[prefix].content_eq(&new[prefix]) {
        prefix += 1;
    }
    let mut suffix = 0usize;
    while suffix < old.len().saturating_sub(prefix)
        && suffix < new.len().saturating_sub(prefix)
        && old[old.len() - 1 - suffix].content_eq(&new[new.len() - 1 - suffix])
    {
        suffix += 1;
    }
    (prefix, suffix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use markdown_parser::inline::text::BlockText;
    use markdown_parser::parse::{BlockData, BlockKind};

    fn plain(kind: BlockKind, text: &str) -> BlockData {
        BlockData::with_plain_text(kind, text)
    }

    #[test]
    fn content_eq_ignores_ids_and_parents() {
        let mut a = plain(BlockKind::Paragraph, "same");
        let mut b = plain(BlockKind::Paragraph, "same");
        assert_ne!(a.id, b.id);
        a.parent = Some(b.id);
        assert!(a.content_eq(&b), "ids and parents must not matter");
        b.text = BlockText::plain("different".to_string());
        assert!(!a.content_eq(&b));
    }

    #[test]
    fn diff_matches_unchanged_ends_only() {
        let old = vec![
            plain(BlockKind::Paragraph, "a"),
            plain(BlockKind::Paragraph, "b"),
            plain(BlockKind::Paragraph, "c"),
        ];
        // b edited, c unchanged; a unchanged.
        let new = vec![
            plain(BlockKind::Paragraph, "a"),
            plain(BlockKind::Paragraph, "b2"),
            plain(BlockKind::Paragraph, "c"),
        ];
        assert_eq!(diff_block_sequences(&old, &new), (1, 1));
        // Append at the end: prefix everything.
        let new = vec![
            plain(BlockKind::Paragraph, "a"),
            plain(BlockKind::Paragraph, "b"),
            plain(BlockKind::Paragraph, "c"),
            plain(BlockKind::Paragraph, "d"),
        ];
        assert_eq!(diff_block_sequences(&old, &new), (3, 0));
        // Insert at the top: suffix everything.
        let new = vec![
            plain(BlockKind::Paragraph, "x"),
            plain(BlockKind::Paragraph, "a"),
            plain(BlockKind::Paragraph, "b"),
            plain(BlockKind::Paragraph, "c"),
        ];
        assert_eq!(diff_block_sequences(&old, &new), (0, 3));
        // Full rewrite: nothing matches.
        let new = vec![plain(BlockKind::Paragraph, "x")];
        assert_eq!(diff_block_sequences(&old, &new), (0, 0));
    }

    #[test]
    fn test_file_switch_zero_jump_with_estimated_viewport_width() {
        let theme = theme::Theme::default_theme();
        let d = &theme.dimensions;
        let real_pane_width = 1400.0_f32;

        // With the fix:
        // Frame 1: ScrollHandle is 0.0, but `estimated_viewport_width` is passed from Editor
        // (which remembered that this pane was previously rendered at 1400.0px):
        let frame1_estimated = Some(real_pane_width);
        let scroll_w_f1 = 0.0_f32;
        let viewport_w_f1 = if scroll_w_f1 > 0.0 {
            scroll_w_f1
        } else if let Some(estimated) = frame1_estimated.filter(|&w| w > 0.0) {
            estimated
        } else {
            real_pane_width
        };
        let frame1_content_width = crate::render::layout::centered_column_width(viewport_w_f1, d);
        let frame1_left_margin = (real_pane_width - frame1_content_width) / 2.0;

        // Frame 2: GPUI completes layout, ScrollHandle reports real width 1400.0:
        let scroll_w_f2 = real_pane_width;
        let viewport_w_f2 = if scroll_w_f2 > 0.0 {
            scroll_w_f2
        } else {
            real_pane_width
        };
        let frame2_content_width = crate::render::layout::centered_column_width(viewport_w_f2, d);
        let frame2_left_margin = (real_pane_width - frame2_content_width) / 2.0;

        let left_shift = (frame1_left_margin - frame2_left_margin).abs();
        assert_eq!(
            left_shift, 0.0,
            "Left shift must be exactly 0px, no offset jump!"
        );
    }

    #[test]
    fn test_virtualized_viewport_window_and_spacers_invariant() {
        use crate::render::blocks::viewport::{
            PlannedRow, cumulative_row_offsets, visible_row_window,
        };

        // Construct 100 planned rows, each with estimated height 30.0
        let row_count = 100;
        let rows: Vec<PlannedRow> = (0..row_count)
            .map(|i| PlannedRow {
                start: i,
                end: i + 1,
                callout_variant: None,
                outer_gap: 0.0,
                segments: Vec::new(),
                estimated_height: 30.0,
            })
            .collect();

        let offsets = cumulative_row_offsets(&rows);
        assert_eq!(offsets.len(), row_count + 1);
        let total_height = *offsets.last().unwrap();
        assert_eq!(total_height, 3000.0);

        // Viewport at y = 600..1200 (view height 600px, 20 rows visible)
        let min_y = 600.0_f32;
        let max_y = 1200.0_f32;
        let overscan = 5;

        let (start, end) = visible_row_window(&offsets, min_y, max_y, overscan);
        assert!(start < end);
        assert!(end <= row_count);
        // With overscan 5, rows visible is approximately (600/30) + 10 = ~30 rows, NOT 100!
        let visible_count = end - start;
        assert!(visible_count < 40, "Must virtualize and only materialize a window of rows");

        let top_spacer = offsets[start];
        let bottom_spacer = total_height - offsets[end];
        let window_height = offsets[end] - offsets[start];

        assert_eq!(
            top_spacer + window_height + bottom_spacer,
            total_height,
            "Top spacer + visible rows height + bottom spacer must equal total document height exactly"
        );
    }
}
