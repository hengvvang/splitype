//! Preview panel — read-only rendered snapshot of the document.

pub(crate) mod render;

use gpui::*;

use crate::editor::controller::Editor;
use crate::editor::tree::block::Block;
use crate::model::parse::BlockData;

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
    pub(crate) fn refresh_preview_blocks(&mut self, pane_id: usize, cx: &mut Context<Self>) {
        let revision = self.tab().document_revision;
        let synced = self
            .pane_state_ref(pane_id)
            .map(|state| state.preview.synced_revision)
            .flatten();
        let blocks_empty = self
            .pane_state_ref(pane_id)
            .map_or(true, |state| state.preview.blocks.is_empty());
        if synced == Some(revision) && !blocks_empty {
            return;
        }
        let source = self.doc().serialize_markdown(cx);
        let hash = Self::hash_str(&source);
        let needs_rebuild = self.pane_state_ref(pane_id).map_or(true, |state| {
            state.preview.source_hash != hash || state.preview.blocks.is_empty()
        });
        let state = self.pane_state(pane_id);
        if needs_rebuild {
            let mut roots = Self::parse_document(cx, &source);
            if roots.is_empty() {
                roots.push(Self::new_block(cx, BlockData::paragraph(String::new())));
            }
            state.preview.blocks = roots;
            state.preview.source_hash = hash;
        }
        state.preview.synced_revision = Some(revision);
    }

    pub(crate) fn hash_str(s: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        s.hash(&mut h);
        h.finish()
    }
}
