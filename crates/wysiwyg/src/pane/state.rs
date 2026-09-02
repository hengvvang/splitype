//! WYSIWYG pane state and domain types.

use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;

use gpui::{App, AppContext, Entity, EntityId, Pixels, Point};

use crate::model::block::Block;
use markdown_parser::block::image::ImageReferenceDefinitions;
use markdown_parser::block::link::LinkReferenceDefinitions;
use markdown_parser::block::table::{TableAxis, TableCellPosition};
use markdown_parser::footnotes::FootnoteMap;

use crate::pane::controller::WysiwygDocumentController;

/// View state specific to a WYSIWYG editor pane.
#[derive(Default)]
pub struct WysiwygPaneState {
    pub controller: Option<Entity<WysiwygDocumentController>>,
    /// The most recent document snapshot, seeding a lazily created controller.
    pub latest_snapshot: Option<editor_contracts::DocumentSnapshot>,
}

impl WysiwygPaneState {
    pub(crate) fn ensure_controller(&mut self, cx: &mut App) -> Entity<WysiwygDocumentController> {
        if let Some(controller) = &self.controller {
            return controller.clone();
        }
        let document = self
            .latest_snapshot
            .clone()
            .unwrap_or_else(editor_contracts::DocumentSnapshot::empty);
        let controller = cx.new(|cx| WysiwygDocumentController::new(&document, cx));
        self.controller = Some(controller.clone());
        controller
    }
}

/// Document-wide reference registries (images, links, footnotes).
#[derive(Default)]
pub struct ReferenceRegistries {
    pub image: Arc<ImageReferenceDefinitions>,
    pub link: Arc<LinkReferenceDefinitions>,
    pub footnotes: Arc<FootnoteMap>,
    /// Base directory the registries were last synced against; blocks
    /// re-resolve image sources whenever this changes.
    pub base_dir: Option<PathBuf>,
    /// Document structure version at the time every current block last
    /// received its reference context. A mismatch means blocks were added
    /// or replaced since, so the per-block sync cannot be skipped.
    pub synced_structure_version: u64,
    /// Blocks and table cells that could contribute reference definitions,
    /// footnote content, or standalone-image syntax, cached at the last full
    /// registry sync. A block edit outside this set cannot change the
    /// registries, so the per-keystroke rebuild is skipped.
    pub candidate_blocks: HashSet<EntityId>,
}

/// Native table cell bindings and axis selections.
#[derive(Default)]
pub struct TableGrids {
    pub cells: HashMap<EntityId, TableCellBinding>,
    pub axis_preview: Option<TableAxisSelection>,
    pub axis_selection: Option<TableAxisSelection>,
}

/// Mapping from one visible block's text range to canonical Markdown offsets.
#[derive(Clone)]
pub struct SourceTargetMapping {
    pub entity: Entity<Block>,
    pub full_source_range: Range<usize>,
    pub content_to_source: Vec<usize>,
    pub source_to_content: Vec<usize>,
}

impl SourceTargetMapping {
    pub fn content_to_source_offset(&self, content_offset: usize) -> usize {
        let max_content = self.content_to_source.len().saturating_sub(1);
        self.content_to_source[content_offset.min(max_content)]
    }

    pub fn source_to_content_offset(&self, source_offset: usize) -> usize {
        let max_source = self.source_to_content.len().saturating_sub(1);
        self.source_to_content[source_offset.min(max_source)]
    }
}

/// One bound native table cell: the owning table block plus the cell
/// entity at a grid position.
#[derive(Clone)]
pub struct TableCellBinding {
    pub table_block: Entity<Block>,
    pub cell: Entity<Block>,
    pub position: TableCellPosition,
}

/// Selected row or column in a rendered native table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TableAxisSelection {
    pub table_block_id: EntityId,
    pub kind: TableAxis,
    pub index: usize,
}

/// State for the interactive Table Size Matrix Picker popup.
#[derive(Clone, Debug)]
pub struct TableSizePickerState {
    pub table_block_id: EntityId,
    pub position: Point<Pixels>,
    pub current_rows: usize,
    pub current_cols: usize,
    pub hovered_rows: Option<usize>,
    pub hovered_cols: Option<usize>,
}
