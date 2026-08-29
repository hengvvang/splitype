pub(crate) mod element;
pub(crate) mod events;
pub(crate) mod highlight;
pub(crate) mod render;
pub(crate) mod state;

pub(crate) use state::SourceCodeState;

use gpui::*;

use crate::editor::engine::controller::{Editor, PaneId};

impl Editor {
    /// Ensure the Source pane's buffer is synchronized with the document.
    pub(crate) fn sync_source_pane(&mut self, pane_id: PaneId, cx: &mut Context<Self>) {
        let Some(tab) = self.active_tab() else {
            return;
        };
        let tab_index = self.session.active_tab_index();
        let revision = tab.document_revision;
        let needs_sync = match self.pane_state_ref(pane_id) {
            Some(state) => {
                state.source_code.synced_tab_index != Some(tab_index)
                    || state.source_code.synced_revision != Some(revision)
                    || (state.source_code.text.is_empty() && revision > 0)
            }
            None => true,
        };
        if !needs_sync {
            return;
        }
        let Some(doc) = self.active_doc() else {
            return;
        };
        let doc_text = doc.serialize_markdown(cx);
        let doc_hash = Self::hash_str(&doc_text);

        if let Some(state) = self.pane_state_mut(pane_id) {
            if state.source_code.synced_tab_index != Some(tab_index)
                || doc_hash != state.source_code.synced_doc_hash
            {
                state.source_code.set_text(doc_text);
                state.source_code.synced_doc_hash = doc_hash;
            }
            state.source_code.synced_revision = Some(revision);
            state.source_code.synced_tab_index = Some(tab_index);
        }
    }
}
