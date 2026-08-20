//! Source code panel — the raw Markdown buffer as a standalone block.

pub(crate) mod render;

use gpui::*;

use crate::editor::block_protocol::BlockEvent;
use crate::editor::controller::Editor;
use crate::editor::tree::block::Block;
use crate::model::parse::BlockData;

impl Editor {
    /// Ensure the Source pane's interactive editor block exists. Only
    /// rebuilds when the document was changed by an external source
    /// (e.g. the Block panel), never when the user is actively editing
    /// the source block itself.
    ///
    /// The block is created as a standalone entity with a minimal
    /// subscription that only syncs Changed events back to the document.
    /// Each Source pane keeps its own block in its own [`PaneState`], so
    /// multiple source panels edit independently; the document content
    /// itself stays shared.
    pub(crate) fn sync_source_pane(&mut self, pane_id: usize, cx: &mut Context<Self>) {
        let tab_index = self.session.tab_list.active_tab;
        let revision = self.tab().document_revision;
        let needs_sync = match self.pane_state_ref(pane_id) {
            Some(state) => {
                state.synced_tab_index != Some(tab_index)
                    || state.synced_revision != Some(revision)
                    || state.source_block.is_none()
            }
            None => true,
        };
        if !needs_sync {
            return;
        }
        let doc_text = self.doc().serialize_markdown(cx);
        let doc_hash = Self::hash_str(&doc_text);

        let state = self.pane_state(pane_id);
        if state.source_block.is_none() || doc_hash != state.synced_doc_hash {
            state.source_block = None;
            let block = Self::new_standalone_block(cx, BlockData::paragraph(doc_text));
            block.update(cx, |block, _cx| block.set_source_document_mode());
            let panel = pane_id;
            cx.subscribe(&block, move |this, block, event, cx| {
                this.on_source_pane_changed(panel, block, event, cx);
            })
            .detach();
            state.source_block = Some(block);
            state.synced_doc_hash = doc_hash;
        }
        state.synced_revision = Some(revision);
        state.synced_tab_index = Some(tab_index);
    }

    /// Minimal event handler for a Source pane block. Only syncs text
    /// changes back to the shared document — no structural event
    /// processing.
    pub(crate) fn on_source_pane_changed(
        &mut self,
        pane_id: usize,
        block: Entity<Block>,
        event: &BlockEvent,
        cx: &mut Context<Self>,
    ) {
        if !matches!(event, BlockEvent::Changed) {
            return;
        }
        // Only the pane's CURRENT block may write into the document. When a
        // tab switch rebuilds the block, the replaced block can stay alive
        // (mounted, even focused) for a frame or two, and a late event from
        // it must not replace the active tab's document with stale text.
        let current = self
            .pane_state_ref(pane_id)
            .and_then(|state| state.source_block.as_ref())
            .map(|block| block.entity_id());
        if current != Some(block.entity_id()) {
            return;
        }
        let text = block.read(cx).display_text().to_string();
        let doc = self.doc().serialize_markdown(cx);
        if text == doc {
            return;
        }
        self.rebuild_document_from_markdown(&text, cx);
        self.mark_dirty(cx);
        // Record the fingerprint of the synced document, not of the user's
        // raw bytes: markdown parsing normalizes the text (a trailing
        // newline, for instance, does not survive a parse round-trip), so
        // hashing the raw bytes here would make the next render rebuild the
        // block and drop the user's trailing newline. The block keeps the
        // user's bytes; the document is the parsed form.
        let synced_hash = Self::hash_str(&self.doc().serialize_markdown(cx));
        let revision = self.tab().document_revision;
        let tab_index = self.session.tab_list.active_tab;
        let state = self.pane_state(pane_id);
        state.synced_doc_hash = synced_hash;
        state.synced_revision = Some(revision);
        state.synced_tab_index = Some(tab_index);
    }
}
