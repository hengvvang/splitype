//! Preview panel — read-only rendered snapshot of the document.

pub(crate) mod render;

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use gpui::*;

use crate::editor::engine::controller::{Editor, PaneId};
use crate::editor::document::block::Block;
use crate::editor::document::block::footnotes::{
    FootnoteDefinitionBinding, FootnoteMap, FootnoteReferenceLocation, FootnoteResolvedOccurrence,
};
use crate::model::block::image::ImageReferenceDefinitions;
use crate::model::block::link::LinkReferenceDefinitions;
use crate::model::parse::{BlockData, BlockId, BlockKind};

/// Read-only block tree shown in the preview panel.
#[derive(Default)]
pub(crate) struct PreviewState {
    pub(crate) blocks: Vec<Entity<Block>>,
    pub(crate) source_hash: u64,
    /// Document revision the preview tree was last synced at; `None` until
    /// the first build.
    pub(crate) synced_revision: Option<u64>,
}

impl Editor {
    /// Rebuild the preview block tree of ONE pane whenever the document
    /// source changes.
    ///
    /// The document revision guards the expensive whole-document
    /// serialization: it only runs after an edit bumped the revision, never
    /// on unchanged frames.
    pub(crate) fn refresh_preview_blocks(&mut self, pane_id: PaneId, cx: &mut Context<Self>) {
        let revision = self.tab().document_revision;
        let synced = self
            .pane_state_ref(pane_id)
            .and_then(|state| state.preview.synced_revision);
        let blocks_empty = self
            .pane_state_ref(pane_id)
            .is_none_or(|state| state.preview.blocks.is_empty());
        if synced == Some(revision) && !blocks_empty {
            return;
        }
        let source = self.doc().serialize_markdown(cx);
        let hash = Self::hash_str(&source);
        let needs_rebuild = self.pane_state_ref(pane_id).is_none_or(|state| {
            state.preview.source_hash != hash || state.preview.blocks.is_empty()
        });
        if needs_rebuild {
            let mut roots = Self::parse_document(cx, &source);
            if roots.is_empty() {
                roots.push(Self::new_block(cx, BlockData::paragraph(String::new())));
            }
            // The preview parses its own snapshot blocks (fresh block ids), so
            // footnote bindings and first-reference locations are recomputed
            // against that tree, then pushed onto every preview block so they
            // resolve exactly like the editable blocks.
            let footnote_registry = Arc::new(Self::build_preview_footnote_registry(&roots, cx));
            let image_registry = self.tab().references.image.clone();
            let link_registry = self.tab().references.link.clone();
            let base_dir = self.image_base_dir();
            Self::sync_preview_block_context(
                &roots,
                base_dir.as_deref(),
                &image_registry,
                &link_registry,
                &footnote_registry,
                cx,
            );
            let state = self.pane_state(pane_id);
            state.preview.blocks = roots;
            state.preview.source_hash = hash;
        }
        self.pane_state(pane_id).preview.synced_revision = Some(revision);
    }

    /// Builds a footnote registry for the preview tree, mirroring
    /// [`Self::rebuild_footnote_registry`] but walking the preview snapshot
    /// blocks (whose entity and block ids differ from the document tree).
    fn build_preview_footnote_registry(roots: &[Entity<Block>], cx: &App) -> FootnoteMap {
        let mut definitions: HashMap<String, EntityId> = HashMap::new();
        let mut ordered: Vec<Entity<Block>> = Vec::new();
        Self::walk_preview_blocks(roots, None, &mut definitions, &mut ordered, cx);

        let mut bindings: HashMap<String, FootnoteDefinitionBinding> = definitions
            .into_iter()
            .map(|(id, definition_entity_id)| {
                (
                    id,
                    FootnoteDefinitionBinding {
                        definition_entity_id,
                        first_reference: None,
                    },
                )
            })
            .collect();

        let mut occurrence_index = 0usize;
        let mut block_occurrences: HashMap<BlockId, Vec<FootnoteResolvedOccurrence>> =
            HashMap::new();
        for block_entity in &ordered {
            let block = block_entity.read(cx);
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
                        entity_id: block_entity.entity_id(),
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
        roots: &[Entity<Block>],
        parent_kind: Option<BlockKind>,
        definitions: &mut HashMap<String, EntityId>,
        ordered: &mut Vec<Entity<Block>>,
        cx: &App,
    ) {
        for entity in roots {
            let block = entity.read(cx);
            let allowed = parent_kind
                .as_ref()
                .is_none_or(|kind| kind.is_quote_container());
            if block.kind() == BlockKind::FootnoteDefinition && allowed {
                definitions
                    .entry(
                        crate::model::block::footnote::split_footnote_definition_text(
                            &block.data.text.plain_text(),
                        )
                        .0
                        .to_string(),
                    )
                    .or_insert(entity.entity_id());
            }
            ordered.push(entity.clone());
            Self::walk_preview_blocks(
                &block.children,
                Some(block.kind()),
                definitions,
                ordered,
                cx,
            );
        }
    }

    /// Pushes the editor's reference context (images, links, footnotes) onto
    /// every preview snapshot block so ordinals, back-references, links, and
    /// image sources resolve exactly like the editable blocks.
    fn sync_preview_block_context(
        roots: &[Entity<Block>],
        base_dir: Option<&Path>,
        image_registry: &Arc<ImageReferenceDefinitions>,
        link_registry: &Arc<LinkReferenceDefinitions>,
        footnote_registry: &Arc<FootnoteMap>,
        cx: &mut Context<Self>,
    ) {
        for entity in roots {
            let children = entity.read(cx).children.clone();
            entity.update(cx, |block, _cx| {
                block.set_reference_context(
                    base_dir.map(Path::to_path_buf),
                    image_registry.clone(),
                    link_registry.clone(),
                    footnote_registry.clone(),
                );
            });
            Self::sync_preview_block_context(
                &children,
                base_dir,
                image_registry,
                link_registry,
                footnote_registry,
                cx,
            );
        }
    }

    pub(crate) fn hash_str(s: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        s.hash(&mut h);
        h.finish()
    }
}
