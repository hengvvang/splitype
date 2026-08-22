//! Undo and redo stack operations, transaction dispatch, and document restore flow.

use std::time::Instant;

use gpui::*;

use crate::editor::controller::{Editor, HistoryEntry, UndoCaptureKind, UndoSelectionSnapshot};
use crate::editor::editing::history::delta::Transaction;

impl Editor {
    pub(crate) fn prepare_undo_capture(&mut self, kind: UndoCaptureKind, cx: &mut Context<Self>) {
        let target_block_info = self
            .current_edit_target_from_state(cx)
            .map(|target| target.read_with(cx, |b, _cx| (b.data.id, b.data.text.clone())));
        self.prepare_undo_capture_with_snapshot(
            kind,
            target_block_info.as_ref().map(|info| info.0),
            target_block_info.map(|info| info.1),
            cx,
        );
    }

    pub(crate) fn prepare_undo_capture_with_snapshot(
        &mut self,
        kind: UndoCaptureKind,
        target_block_id: Option<crate::model::parse::BlockId>,
        initial_text: Option<crate::model::inline::text::BlockText>,
        cx: &mut Context<Self>,
    ) {
        if self.tab().undo.restore_in_progress || self.tab().undo.pending_capture.is_some() {
            return;
        }
        let initial_roots = if matches!(kind, UndoCaptureKind::NonCoalescible) {
            Some(self.doc().roots.iter().map(|r| r.read(cx).data.clone()).collect())
        } else {
            None
        };
        let entry = HistoryEntry {
            transaction: Transaction::empty(),
            selection_before: self.capture_source_selection_snapshot(cx),
            selection_after: UndoSelectionSnapshot::default(),
            timestamp: Instant::now(),
            kind,
        };
        self.tab_mut().undo.pending_capture = Some(crate::editor::controller::PendingUndoCapture {
            snapshot: entry,
            target_block_id,
            initial_text,
            initial_roots,
        });
    }

    pub(crate) fn finalize_pending_undo_capture(&mut self, cx: &mut Context<Self>) {
        if self.tab().undo.restore_in_progress {
            self.tab_mut().undo.pending_capture = None;
            return;
        }
        let Some(mut pending) = self.tab_mut().undo.pending_capture.take() else {
            return;
        };

        // If roots changed structurally, generate ReplaceRoots delta
        if let Some(old_roots) = pending.initial_roots {
            let current_roots: Vec<crate::model::parse::BlockData> =
                self.doc().roots.iter().map(|r| r.read(cx).data.clone()).collect();
            if current_roots != old_roots {
                pending.snapshot.transaction.ops.push(
                    crate::editor::editing::history::delta::DocDelta::ReplaceRoots {
                        old_roots,
                        new_roots: current_roots,
                    },
                );
            }
        } else if let (Some(block_id), Some(old_text)) = (pending.target_block_id, pending.initial_text) {
            if let Some(target) = self.doc().find_entity_by_block_id(block_id, cx) {
                let current_text = target.read(cx).data.text.clone();
                if current_text != old_text {
                    pending.snapshot.transaction.ops.push(
                        crate::editor::editing::history::delta::DocDelta::UpdateBlockText {
                            block_id,
                            old_text,
                            new_text: current_text,
                        },
                    );
                }
            }
        }

        if pending.snapshot.transaction.is_empty() {
            return;
        }

        self.tab_mut().undo.redo_entries.clear();
        let selection_after = self.capture_source_selection_snapshot(cx);
        pending.snapshot.selection_after = selection_after.clone();
        self.tab_mut().undo.last_selection_snapshot = selection_after;

        let now = Instant::now();
        let should_coalesce = matches!(pending.snapshot.kind, UndoCaptureKind::CoalescibleText)
            && self.tab().undo.undo_entries.last().is_some_and(|entry| {
                matches!(entry.kind, UndoCaptureKind::CoalescibleText)
                    && now.saturating_duration_since(entry.timestamp) <= Self::HISTORY_COALESCE_WINDOW
            });

        if should_coalesce {
            if let Some(last) = self.tab_mut().undo.undo_entries.last_mut() {
                for new_op in pending.snapshot.transaction.ops {
                    match new_op {
                        crate::editor::editing::history::delta::DocDelta::UpdateBlockText {
                            block_id,
                            new_text,
                            ..
                        } => {
                            if let Some(existing) =
                                last.transaction.ops.iter_mut().find_map(|op| match op {
                                    crate::editor::editing::history::delta::DocDelta::UpdateBlockText {
                                        block_id: b,
                                        new_text: nt,
                                        ..
                                    } if *b == block_id => Some(nt),
                                    _ => None,
                                })
                            {
                                *existing = new_text;
                            } else {
                                last.transaction.ops.push(
                                    crate::editor::editing::history::delta::DocDelta::UpdateBlockText {
                                        block_id,
                                        old_text: crate::model::inline::text::BlockText::plain(""),
                                        new_text,
                                    },
                                );
                            }
                        }
                        other => last.transaction.ops.push(other),
                    }
                }
                last.selection_after = pending.snapshot.selection_after;
                last.timestamp = now;
                return;
            }
        }

        self.tab_mut().undo.undo_entries.push(pending.snapshot);
        if self.tab().undo.undo_entries.len() > Self::HISTORY_LIMIT {
            let overflow = self.tab().undo.undo_entries.len() - Self::HISTORY_LIMIT;
            self.tab_mut().undo.undo_entries.drain(0..overflow);
        }
    }

    pub(crate) fn refresh_stable_document_snapshot(&mut self, cx: &App) {
        self.tab_mut().undo.last_selection_snapshot = self.capture_source_selection_snapshot(cx);
    }

    /// Restores a transaction delta into the live document state.
    pub(crate) fn restore_history_entry(
        &mut self,
        entry: &HistoryEntry,
        invert: bool,
        cx: &mut Context<Self>,
    ) {
        let tx = if invert {
            entry.transaction.clone().invert()
        } else {
            entry.transaction.clone()
        };

        self.doc_mut().apply_transaction(&tx, cx);
        let selection = if invert {
            &entry.selection_before
        } else {
            &entry.selection_after
        };
        self.apply_selection_snapshot_in_current_mode(selection, cx);
        {
            let pane = self.active_pane_state();
            pane.focus.pending_scroll_active_block_into_view = true;
            pane.focus.pending_scroll_recheck_after_layout = true;
            pane.scroll.last_viewport_size = None;
        }
    }

    pub(crate) fn undo_document(&mut self, cx: &mut Context<Self>) {
        let Some(entry) = self.tab_mut().undo.undo_entries.pop() else {
            return;
        };

        self.tab_mut().undo.pending_capture = None;
        self.tab_mut().undo.restore_in_progress = true;
        self.clear_cross_block_selection(cx);
        self.restore_history_entry(&entry, true, cx);
        self.tab_mut().undo.restore_in_progress = false;
        self.tab_mut().undo.redo_entries.push(entry);
        self.mark_dirty(cx);
        self.sync_table_axis_visuals(cx);
        self.dismiss_contextual_overlays(cx);
        cx.notify();
    }

    pub(crate) fn redo_document(&mut self, cx: &mut Context<Self>) {
        let Some(entry) = self.tab_mut().undo.redo_entries.pop() else {
            return;
        };

        self.tab_mut().undo.pending_capture = None;
        self.tab_mut().undo.restore_in_progress = true;
        self.clear_cross_block_selection(cx);
        self.restore_history_entry(&entry, false, cx);
        self.tab_mut().undo.restore_in_progress = false;
        self.tab_mut().undo.undo_entries.push(entry);
        self.mark_dirty(cx);
        self.sync_table_axis_visuals(cx);
        self.dismiss_contextual_overlays(cx);
        cx.notify();
    }
}
