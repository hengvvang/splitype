//! Lightweight read-only AST block representation for the Preview pane.

use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use gpui::*;

use crate::editor::document::block::footnotes::FootnoteMap;
use crate::editor::document::block::state::ImageHandle;
use splitype_model::block::image::{ImageReferenceDefinitions, ImageSyntax, resolve_image_source};
use splitype_model::block::link::LinkReferenceDefinitions;
use splitype_model::parse::{BlockData, BlockId, BlockKind};

/// A pure-Rust lightweight snapshot block for read-only preview rendering.
/// Holds zero `FocusHandle`, zero cursor blink tasks, and zero interactive editor state.
#[derive(Clone, Debug)]
pub(crate) struct PreviewBlock {
    pub data: BlockData,
    pub children: Vec<PreviewBlock>,
    pub search_matches: Vec<(Range<usize>, bool)>,
    pub list_ordinal: Option<usize>,
    pub base_dir: Option<PathBuf>,
    pub image_registry: Arc<ImageReferenceDefinitions>,
    pub link_registry: Arc<LinkReferenceDefinitions>,
    pub footnote_registry: Arc<FootnoteMap>,
}

impl PreviewBlock {
    pub fn new(data: BlockData) -> Self {
        Self {
            data,
            children: Vec::new(),
            search_matches: Vec::new(),
            list_ordinal: None,
            base_dir: None,
            image_registry: Arc::new(ImageReferenceDefinitions::default()),
            link_registry: Arc::new(LinkReferenceDefinitions::default()),
            footnote_registry: Arc::new(FootnoteMap::default()),
        }
    }

    #[inline]
    pub fn id(&self) -> BlockId {
        self.data.id
    }

    #[inline]
    pub fn kind(&self) -> BlockKind {
        self.data.kind.clone()
    }

    #[inline]
    pub fn display_text(&self) -> String {
        self.data.text.plain_text()
    }

    #[inline]
    pub fn display_len(&self) -> usize {
        self.data.text.plain_len()
    }

    pub fn compute_image_handle(
        &self,
        base_dir: Option<&Path>,
        syntax: ImageSyntax,
    ) -> Option<ImageHandle> {
        let resolved_target = syntax.resolve_target(&self.image_registry)?;
        Some(ImageHandle {
            alt: syntax.alt,
            src: resolved_target.src.clone(),
            title: resolved_target.title,
            resolved_source: resolve_image_source(&resolved_target.src, base_dir),
        })
    }

    pub fn image_handle_for_syntax(&self, syntax: ImageSyntax) -> Option<ImageHandle> {
        self.compute_image_handle(self.base_dir.as_deref(), syntax)
    }

    pub fn has_footnote_definition_backref(&self) -> bool {
        let plain_text = self.data.text.plain_text();
        let (id, _) = splitype_model::block::footnote::split_footnote_definition_text(&plain_text);
        self.footnote_registry
            .bindings
            .get(id)
            .and_then(|b| b.first_reference.as_ref())
            .is_some()
    }

    pub fn set_reference_context(
        &mut self,
        base_dir: Option<PathBuf>,
        image_registry: Arc<ImageReferenceDefinitions>,
        link_registry: Arc<LinkReferenceDefinitions>,
        footnote_registry: Arc<FootnoteMap>,
    ) {
        self.base_dir = base_dir;
        self.image_registry = image_registry;
        self.link_registry = link_registry;
        self.footnote_registry = footnote_registry;
    }

    pub fn is_standalone_image(&self) -> bool {
        let plain = self.data.text.plain_text();
        splitype_model::block::image::parse_standalone_image(&plain).is_some()
    }

    pub fn index_for_mouse_position(&self, _position: Point<Pixels>) -> usize {
        0
    }
}

/// Convert a flat list of `BlockData` into a tree of pure `PreviewBlock`s.
pub(crate) fn blocks_to_preview_tree(data: Vec<BlockData>) -> Vec<PreviewBlock> {
    let block_count = data.len();
    let mut blocks: HashMap<uuid::Uuid, PreviewBlock> = HashMap::with_capacity(block_count);
    for block in &data {
        blocks.insert(block.id.0, PreviewBlock::new(block.clone()));
    }

    // Connect child relationships
    for block in &data {
        if block.children.is_empty() {
            continue;
        }
        let child_blocks: Vec<PreviewBlock> = block
            .children
            .iter()
            .filter_map(|child_id| blocks.remove(&child_id.0))
            .collect();
        if let Some(parent) = blocks.get_mut(&block.id.0) {
            parent.children = child_blocks;
        }
    }

    let mut roots: Vec<PreviewBlock> = data
        .iter()
        .filter(|block| block.parent.is_none())
        .filter_map(|block| blocks.remove(&block.id.0))
        .collect();

    assign_list_ordinals(&mut roots);
    roots
}

fn assign_list_ordinals(blocks: &mut [PreviewBlock]) {
    let mut ordinal = 1;
    for block in blocks.iter_mut() {
        if matches!(block.kind(), BlockKind::NumberedListItem) {
            block.list_ordinal = Some(ordinal);
            ordinal += 1;
        } else {
            ordinal = 1;
        }
        if !block.children.is_empty() {
            assign_list_ordinals(&mut block.children);
        }
    }
}
