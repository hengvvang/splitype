//! The app-side half of the `EditorView` seam: `Editor` implements the
//! editing-world view (declared by `editor_wysiwyg`) so the WYSIWYG
//! world's orchestration can live in its own crate.

use std::time::Duration;

use gpui::{App, Entity, EntityId};

use crate::engine::controller::Editor;
use editor_wysiwyg::document::block::Block;
use editor_wysiwyg::document::Document;
use editor_wysiwyg::editor_view::EditorView;
use editor_wysiwyg::state::{AutoscrollStrategy, UndoHistory, UndoSelectionSnapshot};

impl EditorView for Editor {
    const HISTORY_COALESCE_WINDOW: Duration = Duration::from_millis(1_000);
    const HISTORY_LIMIT: usize = 200;

    fn editor_entity_id(&self) -> EntityId {
        self.entity_id
    }

    fn undo_history(&self) -> &UndoHistory {
        &self.tab().undo
    }

    fn undo_history_mut(&mut self) -> &mut UndoHistory {
        &mut self.tab_mut().undo
    }

    fn active_doc(&self) -> Option<&Document> {
        self.session.active_tab().and_then(|t| t.document.as_ref())
    }

    fn active_doc_mut(&mut self) -> Option<&mut Document> {
        Some(self.doc_mut())
    }

    fn edit_target_block(&self, _cx: &App) -> Option<Entity<Block>> {
        self.active_doc().and_then(|d| d.first_root()).cloned()
    }

    fn capture_selection_snapshot(&self, _cx: &App) -> UndoSelectionSnapshot {
        self.capture_source_selection_snapshot_global(_cx)
    }

    fn apply_selection_snapshot(&mut self, _snapshot: &UndoSelectionSnapshot, _cx: &mut App) {
        // Selection snapshot restored by PaneHost
    }

    fn subscribe_document_blocks(&mut self, _cx: &mut App) {
        // WYSIWYG manages its own block lifecycle and subscriptions
    }

    fn clear_cross_block_selection(&mut self, _cx: &mut App) {
        // Self-managed in WYSIWYG pane
    }

    fn mark_dirty(&mut self, cx: &mut App) {
        self.mark_dirty(cx);
    }

    fn sync_table_axis_visuals(&mut self, _cx: &mut App) {
        // Self-managed in WYSIWYG pane
    }

    fn dismiss_contextual_overlays(&mut self, _cx: &mut App) {
        // Self-managed in overlays
    }

    fn request_pane_autoscroll(&mut self, strategy: AutoscrollStrategy) {
        if let Some(state) = self.pane_state_mut(self.active_pane_id()) {
            state.scroll.pending_autoscroll = Some(strategy);
            state.scroll.last_viewport_size = None;
        }
    }

    fn notify_editor(&mut self, cx: &mut App) {
        cx.notify(self.entity_id);
    }
}
