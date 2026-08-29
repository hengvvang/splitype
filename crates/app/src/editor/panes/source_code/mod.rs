pub(crate) mod events;
pub(crate) mod render;

use gpui::*;

use crate::editor::engine::controller::{Editor, PaneId};

impl Editor {
    /// Ensure the Source pane's buffer is synchronized with the document.
    ///
    /// Model C: the authoritative text is the tab's `text` (or the parsed
    /// tree when it holds unflushed WYSIWYG edits); an unparsed tab — a
    /// parse-free open — synchronizes straight from `text` with no parsing.
    pub(crate) fn sync_source_pane(&mut self, pane_id: PaneId, cx: &mut Context<Self>) {
        let Some(tab) = self.active_tab() else {
            return;
        };
        let tab_index = self.session.active_tab_index();
        let revision = tab.document_revision;
        let needs_sync = match self.pane_state_ref(pane_id).and_then(|s| s.as_source_code()) {
            Some(source) => {
                source.synced_tab_index != Some(tab_index)
                    || source.synced_revision != Some(revision)
                    || (source.text.is_empty() && revision > 0)
            }
            None => true,
        };
        if !needs_sync {
            return;
        }
        let doc_text = self.serialized_document_text(cx);
        let doc_hash = Self::hash_str(&doc_text);

        if let Some(state) = self.pane_state_mut(pane_id) {
            state.ensure_kind(crate::editor::engine::controller::EditorPaneKind::SourceCode);
            if let Some(source) = state.as_source_code_mut() {
                if source.synced_tab_index != Some(tab_index)
                    || doc_hash != source.synced_doc_hash
                {
                    source.set_text(doc_text);
                    source.synced_doc_hash = doc_hash;
                }
                source.synced_revision = Some(revision);
                source.synced_tab_index = Some(tab_index);
            }
        }
    }
}
