//! Source code panel — the raw Markdown buffer as a standalone block.

pub(crate) mod render;

use gpui::*;

use crate::editor::block_protocol::BlockAction;
use crate::editor::controller::Editor;
use crate::editor::tree::block::Block;
use crate::model::block::BlockData;

/// The standalone raw-source block backing ONE source-code panel.
///
/// Every `Editing(SourceCode)` panel owns its own pane state (its own block
/// entity, cursor, and subscription) so multiple source panels edit
/// independently; the document content itself stays shared. The owning
/// area is captured by the subscription closure, so it is not stored here.
#[derive(Default)]
pub(crate) struct SourceCodePaneState {
    /// The panel's own raw-source block entity.
    pub(crate) block: Option<Entity<Block>>,
    /// Fingerprint of the document at the last sync: when the document is
    /// changed externally (e.g. by a Wysiwyg panel), the block is rebuilt
    /// from it. The block itself keeps the user's raw bytes in between.
    pub(crate) synced_doc_hash: u64,
    /// Document revision the pane block was last synced at; `None` until
    /// the first sync.
    pub(crate) synced_revision: Option<u64>,
}

impl Editor {
    /// Ensure the Source pane's interactive editor block exists. Only
    /// rebuilds when the document was changed by an external source
    /// (e.g. the Block panel), never when the user is actively editing
    /// the source block itself.
    ///
    /// The block is created as a standalone entity with a minimal
    /// subscription that only syncs Changed events back to the document.
    pub(crate) fn sync_source_pane(&mut self, pane_id: usize, cx: &mut Context<Self>) {
        let revision = self.tab().document_revision;
        let needs_sync = match self.source_pane_states.get(&pane_id) {
            Some(state) => state.synced_revision != Some(revision) || state.block.is_none(),
            None => true,
        };
        if !needs_sync {
            return;
        }
        let doc_text = self.doc().to_markdown(cx);
        let doc_hash = Self::hash_str(&doc_text);

        let state = self
            .source_pane_states
            .entry(pane_id)
            .or_insert_with(|| SourceCodePaneState {
                block: None,
                synced_doc_hash: 0,
                synced_revision: None,
            });
        if state.block.is_none() || doc_hash != state.synced_doc_hash {
            state.block = None;
            let block = Self::new_standalone_block(cx, BlockData::paragraph(doc_text));
            block.update(cx, |block, _cx| block.set_source_document_mode());
            let panel = pane_id;
            cx.subscribe(&block, move |this, block, event, cx| {
                this.on_source_pane_changed(panel, block, event, cx);
            })
            .detach();
            state.block = Some(block);
            state.synced_doc_hash = doc_hash;
        }
        state.synced_revision = Some(revision);
    }

    /// Minimal event handler for a Source pane block. Only syncs text
    /// changes back to the shared document — no structural event
    /// processing.
    pub(crate) fn on_source_pane_changed(
        &mut self,
        pane_id: usize,
        block: Entity<Block>,
        event: &BlockAction,
        cx: &mut Context<Self>,
    ) {
        if !matches!(event, BlockAction::Changed) {
            return;
        }
        let text = block.read(cx).display_text().to_string();
        let doc = self.doc().to_markdown(cx);
        if text == doc {
            return;
        }
        self.rebuild_document_from_markdown(&text, cx);
        // Record the fingerprint of the synced document, not of the user's
        // raw bytes: markdown parsing normalizes the text (a trailing
        // newline, for instance, does not survive a parse round-trip), so
        // hashing the raw bytes here would make the next render rebuild the
        // block and drop the user's trailing newline. The block keeps the
        // user's bytes; the document is the parsed form.
        let synced_hash = Self::hash_str(&self.doc().to_markdown(cx));
        let revision = self.tab().document_revision;
        self.mark_dirty(cx);
        if let Some(state) = self.source_pane_states.get_mut(&pane_id) {
            state.synced_doc_hash = synced_hash;
            state.synced_revision = Some(revision);
        }
    }
}
