//! EditorView — the editing-world view of the `Editor` aggregate root.
//!
//! The WYSIWYG world owns its document/undo/selection logic; the
//! `Editor` entity (composition root) implements this view and the
//! editing world drives itself through it. Interface by the consumer:
//! the editing world declares exactly what it needs from the editor,
//! the app adapts. Default methods implement the orchestration
//! (undo capture/restore) so call sites stay `self.xxx(cx)`.

use std::time::Duration;

use gpui::{App, Entity, EntityId};

use crate::document::block::Block;
use crate::document::protocol::UndoCaptureKind;
use crate::document::Document;
use crate::history::delta::Transaction;
use crate::markdown::inline::text::BlockText;
use crate::markdown::parse::BlockId;
use crate::state::{
    AutoscrollStrategy, HistoryEntry, PendingUndoCapture, UndoHistory, UndoSelectionSnapshot,
};

/// The editing-world access surface of the editor aggregate root.
pub trait EditorView {
    /// Coalescing window for adjacent typing edits.
    const HISTORY_COALESCE_WINDOW: Duration;
    /// Maximum undo entries retained per tab.
    const HISTORY_LIMIT: usize;

    /// The entity id of this editor (for repaint notifications).
    fn editor_entity_id(&self) -> EntityId;

    /// The active tab's undo history.
    fn undo_history(&self) -> &UndoHistory;
    /// The active tab's undo history (mutable).
    fn undo_history_mut(&mut self) -> &mut UndoHistory;

    /// The active tab's document, if any.
    fn active_doc(&self) -> Option<&Document>;
    /// The active tab's document (mutable), if any.
    fn active_doc_mut(&mut self) -> Option<&mut Document>;

    /// The block currently receiving edit input, if any.
    fn edit_target_block(&self, cx: &App) -> Option<Entity<Block>>;

    /// Capture the current selection for undo/redo restoration.
    fn capture_selection_snapshot(&self, cx: &App) -> UndoSelectionSnapshot;
    /// Restore a captured selection in the active mode.
    fn apply_selection_snapshot(&mut self, snapshot: &UndoSelectionSnapshot, cx: &mut App);

    /// Subscribe this editor to every document block (post-restore).
    fn subscribe_document_blocks(&mut self, cx: &mut App);
    /// Clear any cross-block selection in the active pane.
    fn clear_cross_block_selection(&mut self, cx: &mut App);
    /// Mark the active tab dirty (unsaved changes).
    fn mark_dirty(&mut self, cx: &mut App);
    /// Resync table axis preview/selection visuals.
    fn sync_table_axis_visuals(&mut self, cx: &mut App);
    /// Dismiss floating overlays (context menu, dialogs).
    fn dismiss_contextual_overlays(&mut self, cx: &mut App);
    /// Request an autoscroll on the active pane.
    fn request_pane_autoscroll(&mut self, strategy: AutoscrollStrategy);
    /// Repaint this editor (after structural changes).
    fn notify_editor(&mut self, cx: &mut App);

    // ── Undo orchestration (default implementations) ────────────────────

    /// Begin an undo capture for the current edit target.
    fn prepare_undo_capture(&mut self, kind: UndoCaptureKind, cx: &mut App) {
        let target_block_info = self
            .edit_target_block(cx)
            .map(|target| target.read_with(cx, |b, _cx| (b.data.id, b.data.text.clone())));
        self.prepare_undo_capture_with_snapshot(
            kind,
            target_block_info.as_ref().map(|info| info.0),
            target_block_info.map(|info| info.1),
            cx,
        );
    }

    /// Begin an undo capture with explicit target block info.
    fn prepare_undo_capture_with_snapshot(
        &mut self,
        kind: UndoCaptureKind,
        target_block_id: Option<BlockId>,
        initial_text: Option<BlockText>,
        cx: &mut App,
    ) {
        if self.undo_history().restore_in_progress || self.undo_history().pending_capture.is_some() {
            return;
        }
        let initial_roots = if matches!(kind, UndoCaptureKind::NonCoalescible) {
            Some(
                self.active_doc()
                    .map(|doc| {
                        doc.root_blocks()
                            .iter()
                            .map(|r| r.read(cx).data.clone())
                            .collect()
                    })
                    .unwrap_or_default(),
            )
        } else {
            None
        };
        let entry = HistoryEntry {
            transaction: Transaction::empty(),
            selection_before: self.capture_selection_snapshot(cx),
            selection_after: UndoSelectionSnapshot::default(),
            timestamp: std::time::Instant::now(),
            kind,
        };
        self.undo_history_mut().pending_capture = Some(PendingUndoCapture {
            snapshot: entry,
            target_block_id,
            initial_text,
            initial_roots,
        });
    }

    /// Diff the captured state against the live document and push the
    /// finished entry onto the undo stack.
    fn finalize_pending_undo_capture(&mut self, cx: &mut App) {
        if self.undo_history().restore_in_progress {
            self.undo_history_mut().pending_capture = None;
            return;
        }
        let Some(mut pending) = self.undo_history_mut().pending_capture.take() else {
            return;
        };
        let Some(doc) = self.active_doc() else {
            return;
        };

        // If roots changed structurally, generate a fine-grained SpliceRoots delta.
        if let Some(old_roots) = pending.initial_roots {
            let current_roots: Vec<crate::markdown::parse::BlockData> = doc
                .root_blocks()
                .iter()
                .map(|r| r.read(cx).data.clone())
                .collect();
            if current_roots != old_roots {
                let prefix_len = old_roots
                    .iter()
                    .zip(current_roots.iter())
                    .take_while(|(a, b)| a == b)
                    .count();
                let old_rem = &old_roots[prefix_len..];
                let cur_rem = &current_roots[prefix_len..];
                let suffix_len = old_rem
                    .iter()
                    .rev()
                    .zip(cur_rem.iter().rev())
                    .take_while(|(a, b)| a == b)
                    .count();

                let del_end = old_roots.len() - suffix_len;
                let ins_end = current_roots.len() - suffix_len;

                let deleted = old_roots[prefix_len..del_end].to_vec();
                let inserted = current_roots[prefix_len..ins_end].to_vec();

                pending.snapshot.transaction.ops.push(
                    crate::history::delta::DocDelta::SpliceRoots {
                        index: prefix_len,
                        deleted,
                        inserted,
                    },
                );
            }
        } else if let (Some(block_id), Some(old_text)) = (pending.target_block_id, pending.initial_text) {
            if let Some(target) = doc.find_entity_by_block_id(block_id, cx) {
                let current_text = target.read(cx).data.text.clone();
                if current_text != old_text {
                    pending.snapshot.transaction.ops.push(
                        crate::history::delta::DocDelta::UpdateBlockText {
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

        self.undo_history_mut().redo_entries.clear();
        let selection_after = self.capture_selection_snapshot(cx);
        pending.snapshot.selection_after = selection_after.clone();
        self.undo_history_mut().last_selection_snapshot = selection_after;

        let now = std::time::Instant::now();
        let should_coalesce = matches!(pending.snapshot.kind, UndoCaptureKind::CoalescibleText)
            && self.undo_history().undo_entries.last().is_some_and(|entry| {
                matches!(entry.kind, UndoCaptureKind::CoalescibleText)
                    && now.saturating_duration_since(entry.timestamp) <= Self::HISTORY_COALESCE_WINDOW
            });

        let mut contains_boundary = false;
        if should_coalesce {
            if let Some(last) = self.undo_history_mut().undo_entries.last_mut() {
                for new_op in pending.snapshot.transaction.ops {
                    match new_op {
                        crate::history::delta::DocDelta::UpdateBlockText {
                            block_id,
                            old_text,
                            new_text,
                        } => {
                            let old_plain = old_text.plain_text();
                            let new_plain = new_text.plain_text();
                            if new_plain.len() > old_plain.len() {
                                let added = &new_plain[old_plain.len()..];
                                if added.chars().any(|ch| {
                                    ch.is_whitespace()
                                        || matches!(
                                            ch,
                                            ',' | '.' | '!' | '?' | ';' | ':' | '，' | '。'
                                                | '！' | '？' | '；' | '：'
                                        )
                                }) {
                                    contains_boundary = true;
                                }
                            }

                            if let Some(existing) =
                                last.transaction.ops.iter_mut().find_map(|op| match op {
                                    crate::history::delta::DocDelta::UpdateBlockText {
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
                                    crate::history::delta::DocDelta::UpdateBlockText {
                                        block_id,
                                        old_text,
                                        new_text,
                                    },
                                );
                            }
                        }
                        other => last.transaction.ops.push(other),
                    }
                }
                last.selection_after = pending.snapshot.selection_after;
                if contains_boundary {
                    last.timestamp = now
                        .checked_sub(Self::HISTORY_COALESCE_WINDOW * 2)
                        .unwrap_or(now);
                } else {
                    last.timestamp = now;
                }
                return;
            }
        }

        self.undo_history_mut().undo_entries.push(pending.snapshot);
        if self.undo_history().undo_entries.len() > Self::HISTORY_LIMIT {
            let overflow = self.undo_history().undo_entries.len() - Self::HISTORY_LIMIT;
            self.undo_history_mut().undo_entries.drain(0..overflow);
        }
    }

    /// Restore a transaction delta into the live document state.
    fn restore_history_entry(&mut self, entry: &HistoryEntry, invert: bool, cx: &mut App) {
        let tx = if invert {
            entry.transaction.clone().invert()
        } else {
            entry.transaction.clone()
        };

        if let Some(doc) = self.active_doc_mut() {
            doc.apply_transaction(&tx, cx);
        }
        self.subscribe_document_blocks(cx);
        let selection = if invert {
            &entry.selection_before
        } else {
            &entry.selection_after
        };
        self.apply_selection_snapshot(selection, cx);
        self.request_pane_autoscroll(AutoscrollStrategy::Fit { margin: gpui::px(20.0) });
    }

    /// Undo the most recent entry.
    fn undo_document(&mut self, cx: &mut App) {
        let Some(entry) = self.undo_history_mut().undo_entries.pop() else {
            return;
        };

        self.undo_history_mut().pending_capture = None;
        self.undo_history_mut().restore_in_progress = true;
        self.clear_cross_block_selection(cx);
        self.restore_history_entry(&entry, true, cx);
        self.undo_history_mut().restore_in_progress = false;
        self.undo_history_mut().redo_entries.push(entry);
        self.mark_dirty(cx);
        self.sync_table_axis_visuals(cx);
        self.dismiss_contextual_overlays(cx);
        self.notify_editor(cx);
    }

    /// Redo the most recently undone entry.
    fn redo_document(&mut self, cx: &mut App) {
        let Some(entry) = self.undo_history_mut().redo_entries.pop() else {
            return;
        };

        self.undo_history_mut().pending_capture = None;
        self.undo_history_mut().restore_in_progress = true;
        self.clear_cross_block_selection(cx);
        self.restore_history_entry(&entry, false, cx);
        self.undo_history_mut().restore_in_progress = false;
        self.undo_history_mut().undo_entries.push(entry);
        self.mark_dirty(cx);
        self.sync_table_axis_visuals(cx);
        self.dismiss_contextual_overlays(cx);
        self.notify_editor(cx);
    }

    /// Refresh the stable (post-edit) selection snapshot.
    fn refresh_stable_document_snapshot(&mut self, cx: &App) {
        self.undo_history_mut().last_selection_snapshot = self.capture_selection_snapshot(cx);
    }
}
