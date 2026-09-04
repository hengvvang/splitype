//! Preview-tree reference context: footnote registries, image
//! registries and base-directory resolution pushed onto every snapshot
//! block so ordinals, back-references, and image sources resolve
//! exactly like the editable blocks.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use markdown_parser::block::footnote::split_footnote_definition_text;
use markdown_parser::block::image::ImageReferenceDefinitions;
use markdown_parser::footnotes::{
    FootnoteDefinitionBinding, FootnoteMap, FootnoteReferenceLocation, FootnoteResolvedOccurrence,
};
use markdown_parser::parse::{BlockId, BlockKind};

use crate::block::PreviewBlock;

/// Builds a footnote registry for the preview tree, mirroring the document
/// registry but walking the preview snapshot blocks (whose entity and block
/// ids differ from the document tree).
pub fn build_preview_footnote_registry(roots: &[PreviewBlock]) -> FootnoteMap {
    let mut definitions: HashMap<String, BlockId> = HashMap::new();
    let mut ordered: Vec<PreviewBlock> = Vec::new();
    walk_preview_blocks(roots, None, &mut definitions, &mut ordered);

    let mut bindings: HashMap<String, FootnoteDefinitionBinding> = definitions
        .into_iter()
        .map(|(id, block_id)| {
            (
                id,
                FootnoteDefinitionBinding {
                    definition_block_id: block_id,
                    first_reference: None,
                },
            )
        })
        .collect();

    let mut occurrence_index = 0usize;
    let mut block_occurrences: HashMap<BlockId, Vec<FootnoteResolvedOccurrence>> = HashMap::new();
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
                    block_id,
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
        walk_preview_blocks(&block.children, Some(block.kind()), definitions, ordered);
    }
}

/// Pushes the editor's reference context (images, footnotes) onto every
/// preview snapshot block so ordinals, back-references, and image sources
/// resolve exactly like the editable blocks.
pub fn sync_preview_block_context(
    roots: &mut [PreviewBlock],
    base_dir: Option<&Path>,
    image_registry: &Arc<ImageReferenceDefinitions>,
    footnote_registry: &Arc<FootnoteMap>,
) {
    for block in roots {
        block.set_reference_context(
            base_dir.map(Path::to_path_buf),
            image_registry.clone(),
            footnote_registry.clone(),
        );
        sync_preview_block_context(
            &mut block.children,
            base_dir,
            image_registry,
            footnote_registry,
        );
    }
}
