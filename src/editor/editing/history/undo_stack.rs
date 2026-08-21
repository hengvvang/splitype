//! Undo and redo stack operations, transaction dispatch, and document restore flow.

use std::time::Instant;

use gpui::*;

use crate::editor::controller::{Editor, HistoryEntry, UndoCaptureKind};
use crate::editor::editing::history::delta::Transaction;

impl Editor {
    /// Records a structured transaction into the undo history stack, coalescing adjacent typing edits.
    #[allow(dead_code)]
    pub(crate) fn record_transaction(
        &mut self,
        tx: Transaction,
        kind: UndoCaptureKind,
        cx: &App,
    ) {
        if self.tab().undo.restore_in_progress || tx.is_empty() {
            return;
        }

        // Fresh edits invalidate redo history
        self.tab_mut().undo.redo_entries.clear();

        let now = Instant::now();
        let should_coalesce = matches!(kind, UndoCaptureKind::CoalescibleText)
            && self.tab().undo.undo_entries.last().is_some_and(|entry| {
                matches!(entry.kind, UndoCaptureKind::CoalescibleText)
                    && now.saturating_duration_since(entry.timestamp) <= Self::HISTORY_COALESCE_WINDOW
            });

        let selection = self.capture_source_selection_snapshot(cx);
        if should_coalesce {
            if let Some(last) = self.tab_mut().undo.undo_entries.last_mut() {
                last.transaction.ops.extend(tx.ops);
                last.selection = selection;
                last.timestamp = now;
                return;
            }
        }

        let entry = HistoryEntry {
            transaction: tx,
            selection,
            timestamp: now,
            kind,
        };

        self.tab_mut().undo.undo_entries.push(entry);
        if self.tab().undo.undo_entries.len() > Self::HISTORY_LIMIT {
            let overflow = self.tab().undo.undo_entries.len() - Self::HISTORY_LIMIT;
            self.tab_mut().undo.undo_entries.drain(0..overflow);
        }
    }

    pub(crate) fn prepare_undo_capture(&mut self, kind: UndoCaptureKind, cx: &mut Context<Self>) {
        if self.tab().undo.restore_in_progress || self.tab().undo.pending_capture.is_some() {
            return;
        }
        let entry = HistoryEntry {
            transaction: Transaction::empty(),
            selection: self.capture_source_selection_snapshot(cx),
            timestamp: Instant::now(),
            kind,
        };
        self.tab_mut().undo.pending_capture = Some(crate::editor::controller::PendingUndoCapture {
            snapshot: entry,
        });
    }

    pub(crate) fn prepare_undo_capture_from_stable_snapshot(&mut self, kind: UndoCaptureKind) {
        if self.tab().undo.restore_in_progress || self.tab().undo.pending_capture.is_some() {
            return;
        }
        let entry = HistoryEntry {
            transaction: Transaction::empty(),
            selection: self.tab().undo.last_selection_snapshot.clone(),
            timestamp: Instant::now(),
            kind,
        };
        self.tab_mut().undo.pending_capture = Some(crate::editor::controller::PendingUndoCapture {
            snapshot: entry,
        });
    }

    pub(crate) fn finalize_pending_undo_capture(&mut self, cx: &mut Context<Self>) {
        if self.tab().undo.restore_in_progress {
            self.tab_mut().undo.pending_capture = None;
            return;
        }
        let Some(pending) = self.tab_mut().undo.pending_capture.take() else {
            return;
        };
        self.tab_mut().undo.redo_entries.clear();
        self.tab_mut().undo.last_selection_snapshot = self.capture_source_selection_snapshot(cx);
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
        self.apply_selection_snapshot_in_current_mode(&entry.selection, cx);
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
