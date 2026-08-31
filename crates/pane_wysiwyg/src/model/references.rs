//! Document-wide reference registries (images, links, footnotes).
//!
//! Pure computation over a [`Document`]: scanning candidate blocks,
//! rebuilding the footnote registry, and pushing reference contexts onto
//! block entities. The editor's per-edit entry point orchestrates these
//! with the cached candidate set.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use gpui::{App, Entity, EntityId};

use crate::model::block::footnotes::{
    FootnoteDefinitionBinding, FootnoteMap, FootnoteReferenceLocation, FootnoteResolvedOccurrence,
};
use crate::model::block::Block;
use crate::model::Document;
use crate::state::TableGrids;
use crate::markdown::block::footnote::split_footnote_definition_text;
use crate::markdown::block::image::ImageReferenceDefinitions;
use crate::markdown::block::link::LinkReferenceDefinitions;
use crate::markdown::parse::{BlockId, BlockKind};

/// Rebuild the document-wide footnote registry (definitions bound to
/// their first inline reference, plus per-block occurrence lists).
pub fn rebuild_footnote_registry(doc: &Document, cx: &App) -> FootnoteMap {
    let mut definitions = HashMap::new();
    for entry in doc.blocks() {
        let block = entry.entity.read(cx);
        if block.kind() != BlockKind::FootnoteDefinition {
            continue;
        }

        let allow_definition = doc
            .find_block_location(entry.entity.entity_id())
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
            .entry(
                split_footnote_definition_text(&block.data.text.plain_text())
                    .0
                    .to_string(),
            )
            .or_insert(entry.entity.entity_id());
    }

    let mut bindings = HashMap::<String, FootnoteDefinitionBinding>::new();
    for (id, entity_id) in definitions {
        bindings.insert(
            id,
            FootnoteDefinitionBinding {
                definition_entity_id: entity_id,
                first_reference: None,
            },
        );
    }

    let mut occurrence_index = 0usize;
    let mut block_occurrences = HashMap::<BlockId, Vec<FootnoteResolvedOccurrence>>::new();
    for entry in doc.blocks() {
        let block = entry.entity.read(cx);
        let block_id = block.data.id;
        for fragment in &block.data.text.fragments {
            let Some(footnote) = fragment.footnote() else {
                continue;
            };
            if let Some(binding) = bindings.get_mut(&footnote.id)
                && binding.first_reference.is_none()
            {
                binding.first_reference = Some(FootnoteReferenceLocation {
                    entity_id: entry.entity.entity_id(),
                    occurrence_index,
                });
            }
            block_occurrences
                .entry(block_id)
                .or_default()
                .push(FootnoteResolvedOccurrence {
                    id: footnote.id.clone(),
                    occurrence_index,
                });
            occurrence_index += 1;
        }
    }

    FootnoteMap {
        bindings,
        block_occurrences,
    }
}

/// Whether this block could contribute reference definitions
/// (`[label]: url` lines), footnote content, or standalone images to the
/// document-wide scans.
///
/// Reference definitions are only ever detected in raw-preserving block
/// kinds or in text containing `]:`; footnote bindings need `[^` markers;
/// standalone images start with `![`. Code-block text is fence-suppressed.
pub fn block_has_registry_candidates(block: &Block) -> bool {
    if block.data.preserves_raw_source() || block.kind() == BlockKind::FootnoteDefinition {
        return true;
    }
    if matches!(block.kind(), BlockKind::CodeBlock { .. }) {
        return false;
    }
    if block
        .data
        .text
        .fragments
        .iter()
        .any(|f| f.link().is_some() || f.footnote().is_some())
    {
        return true;
    }
    let plain_text = block.data.text.plain_text();
    plain_text.contains("]:") || plain_text.contains("[^") || plain_text.contains("![")
}

/// Entity ids of every block and table cell whose text could contribute
/// reference definitions, footnote content, or standalone-image syntax to
/// the document-wide registries.
pub fn collect_registry_candidates(
    doc: &Document,
    table_grids: &TableGrids,
    cx: &App,
) -> HashSet<EntityId> {
    let mut candidates = HashSet::new();
    for entries in doc.blocks() {
        if block_has_registry_candidates(entries.entity.read(cx)) {
            candidates.insert(entries.entity.entity_id());
        }
    }
    for binding in table_grids.cells.values() {
        if block_has_registry_candidates(binding.cell.read(cx)) {
            candidates.insert(binding.cell.entity_id());
        }
    }
    candidates
}

/// Push the current reference context onto one block (only repainting
/// when the context actually changed).
pub fn sync_reference_context_for_block(
    block: &Entity<Block>,
    base_dir: Option<&Path>,
    image_reference_definitions: Arc<ImageReferenceDefinitions>,
    link_reference_definitions: Arc<LinkReferenceDefinitions>,
    footnote_registry: Arc<FootnoteMap>,
    cx: &mut App,
) {
    let next_base_dir = base_dir.map(Path::to_path_buf);
    block.update(cx, move |block, cx| {
        // Only repaint blocks whose reference context actually changed;
        // with the registries now reused by value comparison, most blocks
        // keep their old context on any given edit.
        if block.set_reference_context(
            next_base_dir.clone(),
            image_reference_definitions.clone(),
            link_reference_definitions.clone(),
            footnote_registry.clone(),
        ) {
            cx.notify();
        }
    });
}


