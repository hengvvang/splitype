//! Preview panel — read-only rendered snapshot of the document.
//!
//! The coordinating crate refreshes the preview tree from the session
//! text (model C) and routes focus/scroll; the presentation and input
//! live in `editor_preview`.

pub mod render;
pub mod selection;

pub use editor_preview::{PreviewBlock, PreviewState, blocks_to_preview_tree};

use std::sync::Arc;

use gpui::*;

use crate::engine::controller::{Editor, PaneId};
use editor_wysiwyg::markdown::parse::BlockData;

impl Editor {
    /// Rebuild the preview block tree of ONE pane whenever the document
    /// source changes.
    ///
    /// The document revision guards the expensive whole-document
    /// serialization: it only runs after an edit bumped the revision, never
    /// on unchanged frames.
    pub fn refresh_preview_blocks(&mut self, pane_id: PaneId, cx: &mut Context<Self>) {
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
            let data = editor_wysiwyg::markdown::parse::parser::parse_preview_document(&source);
            let mut roots = blocks_to_preview_tree(data);
            if roots.is_empty() {
                roots.push(PreviewBlock::new(BlockData::paragraph(String::new())));
            }
            // The preview parses its own snapshot blocks (fresh block ids), so
            // footnote bindings and first-reference locations are recomputed
            // against that tree, then pushed onto every preview block so they
            // resolve exactly like the editable blocks.
            let footnote_registry =
                Arc::new(editor_preview::build_preview_footnote_registry(&roots));
            let image_registry = tab.references.image.clone();
            let link_registry = tab.references.link.clone();
            let base_dir = self.image_base_dir();
            editor_preview::sync_preview_block_context(
                &mut roots,
                base_dir.as_deref(),
                &image_registry,
                &link_registry,
                &footnote_registry,
            );
            if let Some(state) = self.pane_state_mut(pane_id) {
                state.ensure_kind(crate::engine::controller::EditorPaneKind::Preview);
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

    pub fn hash_str(s: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        s.hash(&mut h);
        h.finish()
    }
}
