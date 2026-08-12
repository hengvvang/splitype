//! Undo history management, selection snapshot restoration, and update-check flow.
//!
//! Types (`UndoSelectionSnapshot`, `HistoryEntry`, `PendingUndoCapture`) are
//! defined in `super::editor`; constants live on `Editor`.

use std::time::Instant;

use gpui::*;

use crate::editor::controller::{
    BlockSelectionAnchor, Editor, EditorMode, HistoryEntry, PendingUndoCapture, UndoCaptureKind,
    UndoSelectionSnapshot,
};
use crate::editor::tree::block::Block;

impl Editor {
    pub(crate) fn empty_selection_snapshot() -> UndoSelectionSnapshot {
        UndoSelectionSnapshot {
            range: 0..0,
            reversed: false,
            block_anchor: None,
        }
    }

    pub(crate) fn capture_source_selection_snapshot(&self, cx: &App) -> UndoSelectionSnapshot {
        if let Some(snapshot) = self.cross_block_source_selection_snapshot(cx) {
            return snapshot;
        }

        if self.tab().mode == EditorMode::SourceCode {
            return self
                .doc()
                .first_root()
                .map(|block| {
                    let block_ref = block.read(cx);
                    UndoSelectionSnapshot {
                        range: block_ref.selected_range.clone(),
                        reversed: block_ref.selection_reversed,
                        block_anchor: None,
                    }
                })
                .unwrap_or_else(Self::empty_selection_snapshot);
        }

        let Some(target) = self.current_edit_target_from_state(cx) else {
            return self.tab().undo.last_selection_snapshot.clone();
        };

        // Block-local caret: capture the block's structural path plus its
        // current content range. This avoids rebuilding the full-document
        // source mapping on every keystroke; restore resolves the path
        // directly on the (possibly rebuilt) block tree. Runtime-only blocks
        // such as table cells are not part of the tree, so they fall back to
        // the full-mapping path below.
        let Some(path) = self.block_tree_path(&target, cx) else {
            return self.capture_source_selection_snapshot_global(cx);
        };
        let (selected_range, selection_reversed) = target.read_with(cx, |block, _cx| {
            (block.selected_range.clone(), block.selection_reversed)
        });
        UndoSelectionSnapshot {
            range: 0..0,
            reversed: selection_reversed,
            block_anchor: Some(BlockSelectionAnchor {
                path,
                content_range: selected_range,
            }),
        }
    }

    /// Capture with a global source range, used when the selection must
    /// survive a view-mode toggle or a full document reparse (quote
    /// normalization), where the anchor's tree path may no longer fit.
    pub(crate) fn capture_source_selection_snapshot_global(
        &self,
        cx: &App,
    ) -> UndoSelectionSnapshot {
        if let Some(snapshot) = self.cross_block_source_selection_snapshot(cx) {
            return snapshot;
        }

        if self.tab().mode == EditorMode::SourceCode {
            return self
                .doc()
                .first_root()
                .map(|block| {
                    let block_ref = block.read(cx);
                    UndoSelectionSnapshot {
                        range: block_ref.selected_range.clone(),
                        reversed: block_ref.selection_reversed,
                        block_anchor: None,
                    }
                })
                .unwrap_or_else(Self::empty_selection_snapshot);
        }

        let Some(target) = self.current_edit_target_from_state(cx) else {
            return self.tab().undo.last_selection_snapshot.clone();
        };
        let Some(mapping) = self
            .build_source_target_mappings(cx)
            .into_iter()
            .find(|mapping| mapping.entity.entity_id() == target.entity_id())
        else {
            return self.tab().undo.last_selection_snapshot.clone();
        };

        let selected_range = target.read(cx).selected_range.clone();
        let content_range = target
            .read(cx)
            .display_range_to_source_range(selected_range);
        let max_offset = mapping.content_to_source.len().saturating_sub(1);
        let start = mapping.full_source_range.start
            + mapping.content_to_source[content_range.start.min(max_offset)];
        let end = mapping.full_source_range.start
            + mapping.content_to_source[content_range.end.min(max_offset)];

        UndoSelectionSnapshot {
            range: start..end,
            reversed: target.read(cx).selection_reversed,
            block_anchor: None,
        }
    }

    /// Resolves a block entity to its structural path (root index, then the
    /// sibling index of each child level, root-first). Returns None for
    /// runtime-only blocks that are not part of the document tree.
    pub(crate) fn block_tree_path(&self, block: &Entity<Block>, _cx: &App) -> Option<Vec<usize>> {
        let mut path = Vec::new();
        let mut current = Some(block.entity_id());
        while let Some(entity_id) = current {
            let location = self.doc().find_block_location(entity_id)?;
            path.push(location.index);
            current = location.parent.as_ref().map(|parent| parent.entity_id());
        }
        path.reverse();
        Some(path)
    }

    /// Resolves a structural path back to its block entity, if it still fits
    /// the current tree (indices may have shifted after structural edits).
    pub(crate) fn block_entity_by_path(&self, path: &[usize], cx: &App) -> Option<Entity<Block>> {
        let mut blocks = self.doc().root_blocks();
        let mut entity: Option<Entity<Block>> = None;
        for (level, &index) in path.iter().enumerate() {
            let block = blocks.get(index)?.clone();
            entity = Some(block.clone());
            if level + 1 == path.len() {
                return Some(block);
            }
            blocks = block.read(cx).children.as_slice();
        }
        entity
    }

    pub(crate) fn capture_history_entry(&self, kind: UndoCaptureKind, cx: &App) -> HistoryEntry {
        HistoryEntry {
            source_text: self.serialize_document_for_mode(cx),
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
        let source = self.serialize_document_for_mode(cx);
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

        let current_source = self.serialize_document_for_mode(cx);
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

    pub(crate) fn apply_selection_snapshot_in_current_mode(
        &mut self,
        snapshot: &UndoSelectionSnapshot,
        cx: &mut Context<Self>,
    ) {
        match self.tab().mode {
            EditorMode::SourceCode => {
                let Some(block) = self.doc().first_root().cloned() else {
                    return;
                };
                let len = block.read(cx).display_len();
                // Anchored snapshots carry a block-local caret; treat it as a
                // raw offset into the single source block as a best effort.
                let cursor = snapshot
                    .block_anchor
                    .as_ref()
                    .map(|anchor| anchor.content_range.end)
                    .unwrap_or(snapshot.range.end);
                let selected_range = cursor.min(len)..cursor.min(len);
                block.update(cx, move |block, cx| {
                    block.selected_range = selected_range.clone();
                    block.selection_reversed = snapshot.reversed;
                    block.marked_range = None;
                    block.vertical_motion_x = None;
                    block.cursor_blink_epoch = Instant::now();
                    cx.notify();
                });
                {
                    let pane = self.active_pane_state();
                    pane.focus.pending = Some(block.entity_id());
                    pane.focus.active_entity = Some(block.entity_id());
                }
            }
            EditorMode::Wysiwyg => {
                if let Some(anchor) = &snapshot.block_anchor
                    && let Some(block) = self.block_entity_by_path(&anchor.path, cx)
                {
                    let entity_id = block.entity_id();
                    let selected_range = anchor.content_range.clone();
                    block.update(cx, move |block, cx| {
                        let len = block.display_len();
                        block.selected_range =
                            selected_range.start.min(len)..selected_range.end.min(len);
                        block.selection_reversed = snapshot.reversed;
                        block.marked_range = None;
                        block.vertical_motion_x = None;
                        block.cursor_blink_epoch = Instant::now();
                        cx.notify();
                    });
                    {
                        let pane = self.active_pane_state();
                        pane.focus.pending = Some(entity_id);
                        pane.focus.active_entity = Some(entity_id);
                    }
                    return;
                }

                if self.apply_cross_block_selection_snapshot_if_possible(snapshot, cx) {
                    return;
                }

                let mappings = self.build_source_target_mappings(cx);
                let exact_mapping = mappings.iter().find(|mapping| {
                    let contains_start = Self::source_range_contains(
                        &mapping.full_source_range,
                        snapshot.range.start,
                    );
                    let contains_end =
                        Self::source_range_contains(&mapping.full_source_range, snapshot.range.end);
                    if !contains_start || !contains_end {
                        return false;
                    }
                    let local_start = snapshot
                        .range
                        .start
                        .saturating_sub(mapping.full_source_range.start);
                    let local_end = snapshot
                        .range
                        .end
                        .saturating_sub(mapping.full_source_range.start);
                    let content_start = mapping.source_to_content
                        [local_start.min(mapping.source_to_content.len().saturating_sub(1))];
                    let content_end = mapping.source_to_content
                        [local_end.min(mapping.source_to_content.len().saturating_sub(1))];
                    let max_content = mapping.content_to_source.len().saturating_sub(1);
                    mapping.content_to_source[content_start.min(max_content)] == local_start
                        && mapping.content_to_source[content_end.min(max_content)] == local_end
                });

                if let Some(mapping) = exact_mapping {
                    let local_start = snapshot.range.start - mapping.full_source_range.start;
                    let local_end = snapshot.range.end - mapping.full_source_range.start;
                    let content_start = mapping.source_to_content[local_start];
                    let content_end = mapping.source_to_content[local_end];
                    let selected_range = mapping
                        .entity
                        .read(cx)
                        .source_range_to_display_range(content_start..content_end);
                    mapping.entity.update(cx, move |block, cx| {
                        block.selected_range = selected_range.clone();
                        block.selection_reversed = snapshot.reversed;
                        block.marked_range = None;
                        block.vertical_motion_x = None;
                        block.cursor_blink_epoch = Instant::now();
                        cx.notify();
                    });
                    {
                        let pane = self.active_pane_state();
                        pane.focus.pending = Some(mapping.entity.entity_id());
                        pane.focus.active_entity = Some(mapping.entity.entity_id());
                    }
                    return;
                }

                let caret_offset = snapshot.range.end;
                let best = mappings.iter().min_by_key(|mapping| {
                    Self::source_offset_distance(&mapping.full_source_range, caret_offset)
                });
                let Some(mapping) = best else {
                    let pending = self.first_focusable_entity_id(cx);
                    let pane = self.active_pane_state();
                    pane.focus.pending = pending;
                    pane.focus.active_entity = pending;
                    return;
                };
                let local_source = if caret_offset <= mapping.full_source_range.start {
                    0
                } else if caret_offset >= mapping.full_source_range.end {
                    mapping.full_source_range.len()
                } else {
                    caret_offset - mapping.full_source_range.start
                };
                let content_offset = mapping.source_to_content
                    [local_source.min(mapping.source_to_content.len().saturating_sub(1))];
                let display_offset = mapping
                    .entity
                    .read(cx)
                    .source_offset_to_display_offset(content_offset);
                mapping.entity.update(cx, move |block, cx| {
                    block.assign_collapsed_selection_offset(
                        display_offset,
                        crate::editor::tree::block::CollapsedCaretAffinity::Default,
                        None,
                    );
                    block.marked_range = None;
                    block.cursor_blink_epoch = Instant::now();
                    cx.notify();
                });
                {
                    let pane = self.active_pane_state();
                    pane.focus.pending = Some(mapping.entity.entity_id());
                    pane.focus.active_entity = Some(mapping.entity.entity_id());
                }
            }
        }
    }

    pub(crate) fn source_range_contains(range: &std::ops::Range<usize>, offset: usize) -> bool {
        if range.start == range.end {
            offset == range.start
        } else {
            offset >= range.start && offset <= range.end
        }
    }

    pub(crate) fn source_offset_distance(range: &std::ops::Range<usize>, offset: usize) -> usize {
        if Self::source_range_contains(range, offset) {
            0
        } else if offset < range.start {
            range.start - offset
        } else {
            offset.saturating_sub(range.end)
        }
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
