//! Preview-tree reference context: footnote registries, image/link
//! registries and base-directory resolution pushed onto every snapshot
//! block so ordinals, back-references, links, and image sources resolve
//! exactly like the editable blocks.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use gpui::EntityId;

use splitype_markdown::footnotes::{
    FootnoteDefinitionBinding, FootnoteMap, FootnoteReferenceLocation, FootnoteResolvedOccurrence,
};
use splitype_markdown::block::footnote::split_footnote_definition_text;
use splitype_markdown::block::image::ImageReferenceDefinitions;
use splitype_markdown::block::link::LinkReferenceDefinitions;
use splitype_markdown::parse::{BlockId, BlockKind};

use crate::node::PreviewBlock;

/// Builds a footnote registry for the preview tree, mirroring the document
/// registry but walking the preview snapshot blocks (whose entity and block
/// ids differ from the document tree).
pub fn build_preview_footnote_registry(roots: &[PreviewBlock]) -> FootnoteMap {
    let mut definitions: HashMap<String, BlockId> = HashMap::new();
    let mut ordered: Vec<PreviewBlock> = Vec::new();
    walk_preview_blocks(roots, None, &mut definitions, &mut ordered);

    let mut bindings: HashMap<String, FootnoteDefinitionBinding> = definitions
        .into_iter()
        .map(|(id, _definition_block_id)| {
            (
                id,
                FootnoteDefinitionBinding {
                    definition_entity_id: EntityId::default(),
                    first_reference: None,
                },
            )
        })
        .collect();

    let mut occurrence_index = 0usize;
    let mut block_occurrences: HashMap<BlockId, Vec<FootnoteResolvedOccurrence>> =
        HashMap::new();
    for block in &ordered {
        let block_id = block.data.id;
        let mut occurrences = Vec::new();
        for fragment in &block.data.text.fragments {
            let Some(footnote) = fragment.footnote() else {
                continue;
            };
            if let Some(binding) = bindings.get_mut(&footnote.id)
                && binding.first_reference.is_none()
            {
                binding.first_reference = Some(FootnoteReferenceLocation {
                    entity_id: EntityId::default(),
                    occurrence_index,
                });
            }
            occurrences.push(FootnoteResolvedOccurrence {
                id: footnote.id.clone(),
                occurrence_index,
            });
            occurrence_index += 1;
        }
        if !occurrences.is_empty() {
            block_occurrences.insert(block_id, occurrences);
        }
    }

    FootnoteMap {
        bindings,
        block_occurrences,
    }
}

/// Pre-order walk of the preview tree feeding the footnote registry build.
/// Definitions are accepted at the root or directly under a quote
/// container, matching the document registry's rules.
fn walk_preview_blocks(
    roots: &[PreviewBlock],
    parent_kind: Option<BlockKind>,
    definitions: &mut HashMap<String, BlockId>,
    ordered: &mut Vec<PreviewBlock>,
) {
    for block in roots {
        let allowed = parent_kind
            .as_ref()
            .is_none_or(|kind| kind.is_quote_container());
        if block.kind() == BlockKind::FootnoteDefinition && allowed {
            definitions
                .entry(
                    split_footnote_definition_text(&block.data.text.plain_text())
                        .0
                        .to_string(),
                )
                .or_insert(block.id());
        }
        ordered.push(block.clone());
        walk_preview_blocks(
            &block.children,
            Some(block.kind()),
            definitions,
            ordered,
        );
    }
}

/// Pushes the editor's reference context (images, links, footnotes) onto
/// every preview snapshot block so ordinals, back-references, links, and
/// image sources resolve exactly like the editable blocks.
pub fn sync_preview_block_context(
    roots: &mut [PreviewBlock],
    base_dir: Option<&Path>,
    image_registry: &Arc<ImageReferenceDefinitions>,
    link_registry: &Arc<LinkReferenceDefinitions>,
    footnote_registry: &Arc<FootnoteMap>,
) {
    for block in roots {
        block.set_reference_context(
            base_dir.map(Path::to_path_buf),
            image_registry.clone(),
            link_registry.clone(),
            footnote_registry.clone(),
        );
        sync_preview_block_context(
            &mut block.children,
            base_dir,
            image_registry,
            link_registry,
            footnote_registry,
        );
    }
}
