//! Preview panel — read-only rendered snapshot of the document.

pub(crate) mod node;
pub(crate) mod render;
pub(crate) mod selection;

pub(crate) use node::{PreviewBlock, blocks_to_preview_tree};
pub(crate) use selection::{PreviewEndpoint, PreviewSelectionRange};

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use gpui::*;

use crate::editor::engine::controller::{Editor, PaneId};
use editor_wysiwyg::document::block::footnotes::{
    FootnoteDefinitionBinding, FootnoteMap, FootnoteReferenceLocation, FootnoteResolvedOccurrence,
};
use markdown::block::image::ImageReferenceDefinitions;
use markdown::block::link::LinkReferenceDefinitions;
use markdown::parse::{BlockData, BlockId, BlockKind};

/// Read-only block tree shown in the preview panel.
#[derive(Default)]
pub(crate) struct PreviewState {
    pub(crate) blocks: Vec<PreviewBlock>,
    pub(crate) selection: Option<PreviewSelectionRange>,
    pub(crate) drag_anchor: Option<PreviewEndpoint>,
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
        let Some(tab) = self.active_tab() else {
            return;
        };
        let revision = tab.document_revision;
        let synced = self
            .pane_state_ref(pane_id)
            .and_then(|state| state.as_preview())
            .and_then(|p| p.synced_revision);
        let blocks_empty = self
            .pane_state_ref(pane_id)
            .and_then(|state| state.as_preview())
            .is_none_or(|p| p.blocks.is_empty());
        if synced == Some(revision) && !blocks_empty {
            return;
        }
        let Some(doc) = self.active_doc() else {
            return;
        };
        let source = doc.serialize_markdown(cx);
        let hash = Self::hash_str(&source);
        let needs_rebuild = self
            .pane_state_ref(pane_id)
            .and_then(|state| state.as_preview())
            .is_none_or(|p| p.source_hash != hash || p.blocks.is_empty());
        if needs_rebuild {
            let data = markdown::parse::parser::parse_preview_document(&source);
            let mut roots = blocks_to_preview_tree(data);
            if roots.is_empty() {
                roots.push(PreviewBlock::new(BlockData::paragraph(String::new())));
            }
            // The preview parses its own snapshot blocks (fresh block ids), so
            // footnote bindings and first-reference locations are recomputed
            // against that tree, then pushed onto every preview block so they
            // resolve exactly like the editable blocks.
            let footnote_registry = Arc::new(Self::build_preview_footnote_registry(&roots));
            let image_registry = tab.references.image.clone();
            let link_registry = tab.references.link.clone();
            let base_dir = self.image_base_dir();
            Self::sync_preview_block_context(
                &mut roots,
                base_dir.as_deref(),
                &image_registry,
                &link_registry,
                &footnote_registry,
            );
            if let Some(state) = self.pane_state_mut(pane_id) {
                state.ensure_kind(crate::editor::engine::controller::EditorPaneKind::Preview);
                if let Some(preview) = state.as_preview_mut() {
                    preview.blocks = roots;
                    preview.source_hash = hash;
                }
            }
        }
        if let Some(preview) = self.pane_state_mut(pane_id).and_then(|p| p.as_preview_mut()) {
            preview.synced_revision = Some(revision);
        }
    }

    /// Builds a footnote registry for the preview tree, mirroring
    /// [`Self::rebuild_footnote_registry`] but walking the preview snapshot
    /// blocks (whose entity and block ids differ from the document tree).
    fn build_preview_footnote_registry(roots: &[PreviewBlock]) -> FootnoteMap {
        let mut definitions: HashMap<String, BlockId> = HashMap::new();
        let mut ordered: Vec<PreviewBlock> = Vec::new();
        Self::walk_preview_blocks(roots, None, &mut definitions, &mut ordered);

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
                        markdown::block::footnote::split_footnote_definition_text(
                            &block.data.text.plain_text(),
                        )
                        .0
                        .to_string(),
                    )
                    .or_insert(block.id());
            }
            ordered.push(block.clone());
            Self::walk_preview_blocks(
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
    fn sync_preview_block_context(
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
            Self::sync_preview_block_context(
                &mut block.children,
                base_dir,
                image_registry,
                link_registry,
                footnote_registry,
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

// ── Pane plugin contract ─────────────────────────────────────────────────

use std::ops::Range;

use crate::editor::engine::session::EditorPaneKind;
use editor_core::{outline_headings_from_markdown, EditorDocument, OutlineNode, Pane};

impl Pane for PreviewState {
    fn kind(&self) -> EditorPaneKind {
        EditorPaneKind::Preview
    }

    fn document_source(&self, doc: &dyn EditorDocument, cx: &App) -> String {
        doc.serialize_markdown(cx)
    }

    fn set_search_matches(&mut self, _matches: &[(Range<usize>, bool)]) {
        // Preview is a read-only render; there is nothing to highlight.
    }

    fn outline_items(&self, doc: &dyn EditorDocument, cx: &App) -> Vec<OutlineNode> {
        outline_headings_from_markdown(&doc.serialize_markdown(cx))
    }
}
