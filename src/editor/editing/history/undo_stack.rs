//! Undo and redo stack operations, keystroke coalescing, and document restore flow.

use std::time::Instant;

use gpui::*;

use crate::editor::controller::{Editor, HistoryEntry, PendingUndoCapture, UndoCaptureKind};

impl Editor {
    pub(crate) fn capture_history_entry(&self, kind: UndoCaptureKind, cx: &App) -> HistoryEntry {
        HistoryEntry {
            source_text: self.serialized_document_text(cx),
            selection: self.capture_source_selection_snapshot(cx),
            timestamp: Instant::now(),
            kind,
        }
    }

    pub(crate) fn capture_stable_history_entry(&self, kind: UndoCaptureKind) -> HistoryEntry {
        HistoryEntry {
            source_text: self.tab().undo.last_stable_source_text.clone(),
            selection: self.tab().undo.last_selection_snapshot.clone(),
            timestamp: Instant::now(),
            kind,
        }
    }

    pub(crate) fn prepare_undo_capture(&mut self, kind: UndoCaptureKind, cx: &mut Context<Self>) {
        if self.tab().undo.restore_in_progress || self.tab().undo.pending_capture.is_some() {
            return;
        }
        self.tab_mut().undo.pending_capture = Some(PendingUndoCapture {
            snapshot: self.capture_history_entry(kind, cx),
        });
    }

    pub(crate) fn prepare_undo_capture_from_stable_snapshot(&mut self, kind: UndoCaptureKind) {
        if self.tab().undo.restore_in_progress || self.tab().undo.pending_capture.is_some() {
            return;
        }
        self.tab_mut().undo.pending_capture = Some(PendingUndoCapture {
            snapshot: self.capture_stable_history_entry(kind),
        });
    }

    pub(crate) fn refresh_stable_document_snapshot(&mut self, cx: &App) {
        let source = self.serialized_document_text(cx);
        self.refresh_stable_document_snapshot_with_source(source, cx);
    }

    /// Refreshes the stable undo snapshot, reusing an already-serialized
    /// source instead of serializing the document a second time.
    fn refresh_stable_document_snapshot_with_source(&mut self, source: String, cx: &App) {
        self.tab_mut().undo.last_selection_snapshot = self.capture_source_selection_snapshot(cx);
        self.tab_mut().undo.last_stable_source_text = source;
    }

    pub(crate) fn finalize_pending_undo_capture(&mut self, cx: &mut Context<Self>) {
        if self.tab().undo.restore_in_progress {
            self.tab_mut().undo.pending_capture = None;
            return;
        }

        let Some(pending) = self.tab_mut().undo.pending_capture.take() else {
            self.refresh_stable_document_snapshot(cx);
            return;
        };

        let current_source = self.serialized_document_text(cx);
        if current_source == pending.snapshot.source_text {
            self.refresh_stable_document_snapshot_with_source(current_source, cx);
            return;
        }

        // A fresh edit invalidates any forward history available for redo.
        self.tab_mut().undo.redo_entries.clear();

        let should_merge = matches!(pending.snapshot.kind, UndoCaptureKind::CoalescibleText)
            && self.tab().undo.undo_entries.last().is_some_and(|entry| {
                matches!(entry.kind, UndoCaptureKind::CoalescibleText)
                    && pending
                        .snapshot
                        .timestamp
                        .saturating_duration_since(entry.timestamp)
                        <= Self::HISTORY_COALESCE_WINDOW
            });
        if !should_merge {
            self.tab_mut().undo.undo_entries.push(pending.snapshot);
            if self.tab().undo.undo_entries.len() > Self::HISTORY_LIMIT {
                let overflow = self.tab().undo.undo_entries.len() - Self::HISTORY_LIMIT;
                self.tab_mut().undo.undo_entries.drain(0..overflow);
            }
        }
        self.refresh_stable_document_snapshot_with_source(current_source, cx);
    }

    pub(crate) fn restore_history_entry(&mut self, entry: &HistoryEntry, cx: &mut Context<Self>) {
        self.rebuild_document_from_markdown(&entry.source_text, cx);

        self.apply_selection_snapshot_in_current_mode(&entry.selection, cx);
        {
            let pane = self.active_pane_state();
            pane.focus.pending_scroll_active_block_into_view = true;
            pane.focus.pending_scroll_recheck_after_layout = true;
            pane.scroll.last_viewport_size = None;
        }
        self.refresh_stable_document_snapshot(cx);
    }

    pub(crate) fn undo_document(&mut self, cx: &mut Context<Self>) {
        let Some(entry) = self.tab_mut().undo.undo_entries.pop() else {
            return;
        };

        // Snapshot the current document so redo can step forward to it.
        let current = self.capture_history_entry(UndoCaptureKind::NonCoalescible, cx);
        self.tab_mut().undo.pending_capture = None;
        self.tab_mut().undo.restore_in_progress = true;
        self.clear_cross_block_selection(cx);
        self.restore_history_entry(&entry, cx);
        self.tab_mut().undo.restore_in_progress = false;
        self.tab_mut().undo.redo_entries.push(current);
        self.mark_dirty(cx);
        self.sync_table_axis_visuals(cx);
        self.dismiss_contextual_overlays(cx);
        cx.notify();
    }

    pub(crate) fn redo_document(&mut self, cx: &mut Context<Self>) {
        let Some(entry) = self.tab_mut().undo.redo_entries.pop() else {
            return;
        };

        // Snapshot the current document so undo can step back to it again.
        let current = self.capture_history_entry(UndoCaptureKind::NonCoalescible, cx);
        self.tab_mut().undo.pending_capture = None;
        self.tab_mut().undo.restore_in_progress = true;
        self.clear_cross_block_selection(cx);
        self.restore_history_entry(&entry, cx);
        self.tab_mut().undo.restore_in_progress = false;
        self.tab_mut().undo.undo_entries.push(current);
        self.mark_dirty(cx);
        self.sync_table_axis_visuals(cx);
        self.dismiss_contextual_overlays(cx);
        cx.notify();
    }
}
