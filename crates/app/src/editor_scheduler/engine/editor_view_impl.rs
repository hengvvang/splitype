//! The app-side half of the `EditorView` seam: `Editor` implements the
//! editing-world view (declared by `editor_wysiwyg`) so the WYSIWYG
//! world's orchestration can live in its own crate.
//!
//! The editing world declares what it needs; this file adapts the
//! aggregate root to it. Operations that fundamentally require a
//! `Context` (selection restoration, block subscription) re-enter the
//! editor through its captured weak handle — deferred to the end of the
//! current update where the borrow is free.

use std::time::Duration;

use gpui::{App, Entity, EntityId};

use crate::editor_scheduler::engine::controller::Editor;
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
        // Model C: an unparsed tab has no tree yet — report None instead
        // of materializing one from a read-only path.
        self.session.active_tab().and_then(|t| t.document.as_ref())
    }

    fn active_doc_mut(&mut self) -> Option<&mut Document> {
        // Routes through `doc_mut` so the authoritative text is marked
        // stale on every WYSIWYG mutation.
        Some(self.doc_mut())
    }

    fn edit_target_block(&self, cx: &App) -> Option<Entity<Block>> {
        self.current_edit_target_from_state(cx)
    }

    fn capture_selection_snapshot(&self, cx: &App) -> UndoSelectionSnapshot {
        self.capture_source_selection_snapshot(cx)
    }

    fn apply_selection_snapshot(&mut self, snapshot: &UndoSelectionSnapshot, cx: &mut App) {
        // Selection restoration works on block entities and source panes
        // and needs a `Context`; run it at the end of this update through
        // the captured weak handle.
        let weak = self.self_weak.clone();
        let snapshot = snapshot.clone();
        cx.defer(move |cx| {
            if let Some(editor) = weak.upgrade() {
                let _ = editor.update(cx, |editor, cx| {
                    editor.apply_selection_snapshot_in_current_mode(&snapshot, cx);
                });
            }
        });
    }

    fn subscribe_document_blocks(&mut self, cx: &mut App) {
        let Some(doc) = self.active_doc() else {
            return;
        };
        let unsubscribed: Vec<(EntityId, Entity<Block>)> = doc
            .blocks()
            .iter()
            .filter(|entry| !self.subscribed_blocks.contains(&entry.entity.entity_id()))
            .map(|entry| (entry.entity.entity_id(), entry.entity.clone()))
            .collect();
        for (entity_id, block) in unsubscribed {
            let weak = self.self_weak.clone();
            self.subscribed_blocks.insert(entity_id);
            cx.subscribe(&block, move |block, event, cx| {
                if let Some(editor) = weak.clone().upgrade() {
                    let _ = editor.update(cx, |editor, cx| {
                        editor.on_block_event(block, event, cx);
                    });
                }
            })
            .detach();
        }
    }

    fn clear_cross_block_selection(&mut self, cx: &mut App) {
        self.clear_cross_block_selection(cx);
    }

    fn mark_dirty(&mut self, cx: &mut App) {
        self.mark_dirty(cx);
    }

    fn sync_table_axis_visuals(&mut self, cx: &mut App) {
        self.sync_table_axis_visuals(cx);
    }

    fn dismiss_contextual_overlays(&mut self, cx: &mut App) {
        self.dismiss_contextual_overlays(cx);
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
