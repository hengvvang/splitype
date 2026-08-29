//! Selection snapshot capture, tree-path resolution, and multi-mode caret restoration.

use std::time::Instant;

use gpui::*;

use crate::editor::engine::controller::{
    BlockSelectionAnchor, Editor, EditorPaneKind, UndoSelectionSnapshot,
};
use crate::editor::document::block::Block;

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

        if self.is_source_code() {
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

        if self.is_source_code() {
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
        let start =
            mapping.full_source_range.start + mapping.content_to_source_offset(content_range.start);
        let end =
            mapping.full_source_range.start + mapping.content_to_source_offset(content_range.end);

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

    pub(crate) fn apply_selection_snapshot_in_current_mode(
        &mut self,
        snapshot: &UndoSelectionSnapshot,
        cx: &mut Context<Self>,
    ) {
        match self.active_pane_kind() {
            EditorPaneKind::SourceCode => {
                let pane_id = self.active_pane_id();
                self.sync_source_pane(pane_id, cx);
                if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
                    let len = source.text.len();
                    let cursor = snapshot
                        .block_anchor
                        .as_ref()
                        .map(|anchor| anchor.content_range.end)
                        .unwrap_or(snapshot.range.end);
                    let pos = cursor.min(len);
                    source.move_to(pos, false);
                }
            }
            EditorPaneKind::Wysiwyg | EditorPaneKind::Preview | EditorPaneKind::Outline => {
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
                    if let Some(wysiwyg) = self.active_pane_state().as_wysiwyg_mut() {
                        wysiwyg.focus.pending = Some(entity_id);
                        wysiwyg.focus.active_entity = Some(entity_id);
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
                    let content_start = mapping.source_to_content_offset(local_start);
                    let content_end = mapping.source_to_content_offset(local_end);
                    mapping.content_to_source_offset(content_start) == local_start
                        && mapping.content_to_source_offset(content_end) == local_end
                });

                if let Some(mapping) = exact_mapping {
                    let local_start = snapshot.range.start - mapping.full_source_range.start;
                    let local_end = snapshot.range.end - mapping.full_source_range.start;
                    let content_start = mapping.source_to_content_offset(local_start);
                    let content_end = mapping.source_to_content_offset(local_end);
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
                    if let Some(wysiwyg) = self.active_pane_state().as_wysiwyg_mut() {
                        wysiwyg.focus.pending = Some(mapping.entity.entity_id());
                        wysiwyg.focus.active_entity = Some(mapping.entity.entity_id());
                    }
                    return;
                }

                let caret_offset = snapshot.range.end;
                let best = mappings.iter().min_by_key(|mapping| {
                    Self::source_offset_distance(&mapping.full_source_range, caret_offset)
                });
                let Some(mapping) = best else {
                    let pending = self.first_focusable_entity_id(cx);
                    if let Some(wysiwyg) = self.active_pane_state().as_wysiwyg_mut() {
                        wysiwyg.focus.pending = pending;
                        wysiwyg.focus.active_entity = pending;
                    }
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
                        crate::editor::document::block::CollapsedCaretAffinity::Default,
                        None,
                    );
                    block.marked_range = None;
                    block.cursor_blink_epoch = Instant::now();
                    cx.notify();
                });
                if let Some(wysiwyg) = self.active_pane_state().as_wysiwyg_mut() {
                    wysiwyg.focus.pending = Some(mapping.entity.entity_id());
                    wysiwyg.focus.active_entity = Some(mapping.entity.entity_id());
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
}
