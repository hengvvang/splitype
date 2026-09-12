//! Explorer selection: resolving the effective entries an operation applies
//! to, multi-select marks, range selection, and keyboard navigation /
//! scrolling (mirrors Zed's `select_*`/`scroll_*` methods).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use gpui::*;

use crate::state::*;

// Explorer navigation actions.
actions!(
    explorer,
    [
        SelectPrevious,
        SelectNext,
        SelectParent,
        SelectFirst,
        SelectLast,
        ExpandSelectedEntry,
        CollapseSelectedEntry,
        ExpandSelectedEntryAndChildren,
        CollapseSelectedEntryAndChildren,
        ExpandAllEntries,
        CollapseAllEntries,
        OpenSelectedEntry,
        RenameSelectedEntry,
        DeleteSelectedEntry,
        TrashSelectedEntry,
        NewFile,
        NewDirectory,
        ScrollCursorCenter,
        ScrollCursorTop,
        ScrollCursorBottom,
        ScrollUp,
        ScrollDown,
        DuplicateSelectedEntry,
        UndoFileOperation,
        RedoFileOperation,
        RevealActiveFile,
    ]
);

impl ExplorerState {
    /// Mark every visible entry row in the tree (mirrors Zed's `select_all`).
    pub(crate) fn select_all_explorer_entries(&mut self, cx: &mut App) {
        if self.edit.is_some() {
            return;
        }
        self.marked.clear();
        for row in &self.entries {
            if let ExplorerRow::Entry(entry) = row {
                self.marked.insert(SelectedEntry {
                    worktree_id: entry.worktree_id,
                    entry_id: entry.id,
                });
            }
        }
        if self.selected.is_none() {
            self.selected = self.marked.iter().next().copied();
        }
        cx.refresh_windows();
    }
    // ── Selection resolution ─────────────────────────────────────────────

    /// Locate `path` across all worktrees; returns the strongly-typed `SelectedEntry`.
    pub(crate) fn explorer_id_for_path(&self, path: &Path) -> Option<SelectedEntry> {
        for snap in &self.snapshots {
            if let Some(id) = snap.id_for_path.get(path) {
                return Some(SelectedEntry {
                    worktree_id: snap.id(),
                    entry_id: *id,
                });
            }
        }
        None
    }

    /// Look up the absolute path for an entry by its stable ID across all worktrees.
    pub(crate) fn explorer_path_for_id(&self, id: ExplorerEntryId) -> Option<PathBuf> {
        for snap in &self.snapshots {
            if let Some(path) = snap.path_for_id.get(&id) {
                return Some(path.clone());
            }
        }
        None
    }

    /// Look up the absolute path for a selected entry in its owning worktree.
    pub(crate) fn explorer_path_for_selection(&self, sel: &SelectedEntry) -> Option<PathBuf> {
        self.snapshots
            .iter()
            .find(|snap| snap.id() == sel.worktree_id)
            .and_then(|snap| snap.path_for_id.get(&sel.entry_id).cloned())
            .or_else(|| self.explorer_path_for_id(sel.entry_id))
    }

    /// Look up the visible row for a file selection.
    pub(crate) fn explorer_entry_for_selection(
        &self,
        sel: &SelectedEntry,
    ) -> Option<&VisibleExplorerEntry> {
        self.explorer_entry_by_id(sel.entry_id)
    }

    /// Filter out entries that are descendants of other selected directories (mirrors Zed's `disjoint_entries`).
    pub(crate) fn disjoint_explorer_entries(
        &self,
        entries: Vec<SelectedEntry>,
    ) -> Vec<SelectedEntry> {
        if entries.is_empty() {
            return Vec::new();
        }
        let mut dir_paths_by_wt: std::collections::HashMap<
            WorktreeId,
            std::collections::BTreeSet<PathBuf>,
        > = std::collections::HashMap::new();
        for entry in &entries {
            if let Some(snapshot) = self.snapshots.iter().find(|s| s.id() == entry.worktree_id) {
                if let Some(wentry) = snapshot.entry_for_id(entry.entry_id) {
                    if wentry.kind == crate::state::worktree::WorktreeEntryKind::Directory {
                        dir_paths_by_wt
                            .entry(entry.worktree_id)
                            .or_default()
                            .insert(wentry.path.clone());
                    }
                }
            }
        }

        entries
            .into_iter()
            .filter(|entry| {
                let Some(snapshot) = self.snapshots.iter().find(|s| s.id() == entry.worktree_id)
                else {
                    return false;
                };
                let Some(wentry) = snapshot.entry_for_id(entry.entry_id) else {
                    return false;
                };
                if let Some(dirs) = dir_paths_by_wt.get(&entry.worktree_id) {
                    let is_descendant = dirs
                        .iter()
                        .any(|dir| &wentry.path != dir && wentry.path.starts_with(dir));
                    !is_descendant
                } else {
                    true
                }
            })
            .collect()
    }

    /// Resolve the entries an operation applies to (Zed's `effective_entries`):
    /// the selection when nothing is marked, otherwise the marked set. The
    /// worktree roots are excluded so destructive operations (delete / cut /
    /// move / copy) can never target a root row. Descendant entries of
    /// selected directories are pruned to prevent double operations.
    pub(crate) fn effective_explorer_entries(&self) -> Vec<SelectedEntry> {
        let root_ids: HashSet<ExplorerEntryId> = self
            .snapshots
            .iter()
            .filter_map(|snap| snap.root_entry().map(|e| e.id))
            .collect();
        let filter = |sel: &SelectedEntry| !root_ids.contains(&sel.entry_id);
        let raw: Vec<SelectedEntry> = if self.marked.is_empty() {
            match self.selected {
                Some(sel) if filter(&sel) => vec![sel],
                _ => Vec::new(),
            }
        } else {
            self.marked
                .iter()
                .filter(|sel| filter(sel))
                .copied()
                .collect()
        };
        self.disjoint_explorer_entries(raw)
    }

    /// Look up a visible entry row by its stable id.
    pub(crate) fn explorer_entry_by_id(
        &self,
        id: ExplorerEntryId,
    ) -> Option<&VisibleExplorerEntry> {
        self.entries.iter().find_map(|row| match row {
            ExplorerRow::Entry(entry) if entry.id == id => Some(entry),
            _ => None,
        })
    }

    /// Toggle an entry in the multi-select mark set (Alt+click).
    pub(crate) fn toggle_explorer_mark(&mut self, selection: SelectedEntry, cx: &mut App) {
        if !self.marked.remove(&selection) {
            self.marked.insert(selection);
        }
        self.selected = Some(selection);
        if let Some(idx) = self
            .entries
            .iter()
            .position(|r| matches!(r, ExplorerRow::Entry(e) if e.id == selection.entry_id))
        {
            self.selection_anchor = Some(idx);
        }
        cx.refresh_windows();
    }

    /// Range-select from the current selection anchor to `target_id` (Shift+click).
    pub(crate) fn select_explorer_range(&mut self, target_id: ExplorerEntryId, cx: &mut App) {
        let rows = &self.entries;
        let target_index = rows
            .iter()
            .position(|row| matches!(row, ExplorerRow::Entry(entry) if entry.id == target_id));
        let Some(target_index) = target_index else {
            return;
        };
        let anchor_index = self
            .selection_anchor
            .or_else(|| {
                self.selected.and_then(|sel| {
                    rows.iter().position(
                        |row| matches!(row, ExplorerRow::Entry(entry) if entry.id == sel.entry_id),
                    )
                })
            })
            .unwrap_or(target_index);

        self.marked.clear();
        let mut target_worktree_id = None;
        for row in &rows[anchor_index.min(target_index)..=anchor_index.max(target_index)] {
            if let ExplorerRow::Entry(entry) = row {
                let selection = SelectedEntry {
                    worktree_id: entry.worktree_id,
                    entry_id: entry.id,
                };
                if entry.id == target_id {
                    target_worktree_id = Some(entry.worktree_id);
                }
                self.marked.insert(selection);
            }
        }
        if let Some(worktree_id) = target_worktree_id {
            self.selected = Some(SelectedEntry {
                worktree_id,
                entry_id: target_id,
            });
        }
        self.selection_anchor = Some(anchor_index);
        self.autoscroll_explorer_selection();
        cx.refresh_windows();
    }

    /// Choose the next selection after deleting `deleted_selections` (mirrors Zed's
    /// `find_next_selection_after_deletion`): the next sibling in the folder, else
    /// the previous sibling, else the parent directory.
    pub(crate) fn next_explorer_selection_after_deletion(
        &self,
        deleted_selections: &[SelectedEntry],
    ) -> Option<SelectedEntry> {
        if deleted_selections.is_empty() {
            return None;
        }
        let deleted: HashSet<ExplorerEntryId> =
            deleted_selections.iter().map(|sel| sel.entry_id).collect();

        // 1. Prefer selecting a surviving sibling in the same folder (Zed)
        if let Some(last_sel) = deleted_selections.last() {
            if let Some(snapshot) = self
                .snapshots
                .iter()
                .find(|s| s.id() == last_sel.worktree_id)
            {
                if let Some(deleted_entry) = snapshot.entry_for_id(last_sel.entry_id) {
                    if let Some(parent_path) = deleted_entry.path.parent() {
                        if let Some(parent_entry) = snapshot.entry_for_path(parent_path) {
                            let child_ids = snapshot.child_ids(parent_entry.id);
                            let all_children: Vec<&crate::state::worktree::WorktreeEntry> =
                                child_ids
                                    .iter()
                                    .filter_map(|id| snapshot.entry_for_id(*id))
                                    .collect();
                            let mut sorted_all = all_children;
                            sorted_all.sort_by(|a, b| {
                                crate::state::compare_worktree_entries(
                                    a,
                                    b,
                                    self.sort_mode,
                                    self.sort_order,
                                )
                            });

                            if let Some(pos) =
                                sorted_all.iter().position(|e| e.id == deleted_entry.id)
                            {
                                if let Some(next) = sorted_all[pos + 1..]
                                    .iter()
                                    .find(|e| !deleted.contains(&e.id))
                                {
                                    return Some(SelectedEntry {
                                        worktree_id: last_sel.worktree_id,
                                        entry_id: next.id,
                                    });
                                }
                                if let Some(prev) = sorted_all[..pos]
                                    .iter()
                                    .rev()
                                    .find(|e| !deleted.contains(&e.id))
                                {
                                    return Some(SelectedEntry {
                                        worktree_id: last_sel.worktree_id,
                                        entry_id: prev.id,
                                    });
                                }
                            }

                            // No sibling left in this folder — select parent directory!
                            return Some(SelectedEntry {
                                worktree_id: last_sel.worktree_id,
                                entry_id: parent_entry.id,
                            });
                        }
                    }
                }
            }
        }

        // 2. Fallback to flat list traversal
        let rows = &self.entries;
        let last_deleted = rows.iter().rposition(
            |row| matches!(row, ExplorerRow::Entry(entry) if deleted.contains(&entry.id)),
        )?;
        for row in &rows[last_deleted + 1..] {
            if let ExplorerRow::Entry(entry) = row
                && !deleted.contains(&entry.id)
            {
                return Some(SelectedEntry {
                    worktree_id: entry.worktree_id,
                    entry_id: entry.id,
                });
            }
        }
        for row in rows[..last_deleted].iter().rev() {
            if let ExplorerRow::Entry(entry) = row
                && !deleted.contains(&entry.id)
            {
                return Some(SelectedEntry {
                    worktree_id: entry.worktree_id,
                    entry_id: entry.id,
                });
            }
        }
        None
    }

    /// Whether `id` is a worktree root.
    pub(crate) fn is_explorer_root_entry(&self, id: ExplorerEntryId) -> bool {
        self.snapshots
            .iter()
            .any(|snap| snap.root_entry().map(|e| e.id) == Some(id))
    }

    // ── Selection navigation and scrolling (mirrors Zed) ────────────────

    /// Row index of the currently selected file entry, if visible.
    fn explorer_selected_row_index(&self) -> Option<usize> {
        match self.selected {
            Some(sel) => self.entries.iter().position(
                |row| matches!(row, ExplorerRow::Entry(row_entry) if row_entry.id == sel.entry_id),
            ),
            _ => None,
        }
    }

    /// Update the selection to the row at `index`. With `extend`, range marks from the anchor to `index` are updated.
    pub(crate) fn update_selection_at_index(&mut self, index: usize, extend: bool) {
        let Some(ExplorerRow::Entry(entry)) = self.entries.get(index) else {
            return;
        };
        let selection = SelectedEntry {
            worktree_id: entry.worktree_id,
            entry_id: entry.id,
        };
        if extend {
            let anchor = self
                .selection_anchor
                .or_else(|| self.explorer_selected_row_index())
                .unwrap_or(index);
            self.marked.clear();
            for row in &self.entries[anchor.min(index)..=anchor.max(index)] {
                if let ExplorerRow::Entry(e) = row {
                    self.marked.insert(SelectedEntry {
                        worktree_id: e.worktree_id,
                        entry_id: e.id,
                    });
                }
            }
            self.selection_anchor = Some(anchor);
        } else {
            self.marked.clear();
            self.selection_anchor = Some(index);
        }
        self.selected = Some(selection);
        self.scroll_handle
            .scroll_to_item(index, ScrollStrategy::Center);
    }

    /// Set the selection to the row at `index` and center it (Zed's
    /// `autoscroll`). With `extend`, range marks from the anchor to `index` are updated.
    fn set_explorer_selection_at_index(&mut self, index: usize, extend: bool, cx: &mut App) {
        self.update_selection_at_index(index, extend);
        cx.refresh_windows();
    }

    /// Move the selection by `delta` rows (signed), clamping to the list.
    fn explorer_move_selection(&mut self, delta: i32, extend: bool, cx: &mut App) {
        if self.edit.is_some() {
            return;
        }
        let len = self.entries.len();
        if len == 0 {
            return;
        }
        let current = self.explorer_selected_row_index();
        let next = (current.unwrap_or(0) as i32 + delta).clamp(0, len as i32 - 1) as usize;
        self.set_explorer_selection_at_index(next, extend, cx);
    }

    pub(crate) fn on_explorer_select_previous(
        &mut self,
        action: &SelectPrevious,
        window: &mut Window,
        cx: &mut App,
    ) {
        if self.edit.is_some() {
            return;
        }
        let extend = window.modifiers().shift;
        self.explorer_move_selection(-1, extend, cx);
        let _ = action;
    }

    pub(crate) fn on_explorer_select_next(
        &mut self,
        action: &SelectNext,
        window: &mut Window,
        cx: &mut App,
    ) {
        if self.edit.is_some() {
            return;
        }
        let extend = window.modifiers().shift;
        self.explorer_move_selection(1, extend, cx);
        let _ = action;
    }

    pub(crate) fn on_explorer_select_parent(
        &mut self,
        _: &SelectParent,
        _window: &mut Window,
        cx: &mut App,
    ) {
        if self.edit.is_some() {
            return;
        }
        let Some(index) = self.explorer_selected_row_index() else {
            return;
        };
        let Some(ExplorerRow::Entry(entry)) = self.entries.get(index) else {
            return;
        };
        let Some(parent_id) = entry.parent_id else {
            self.explorer_move_selection(-1, false, cx);
            return;
        };
        let Some(parent_index) = self
            .entries
            .iter()
            .position(|row| matches!(row, ExplorerRow::Entry(e) if e.id == parent_id))
        else {
            self.explorer_move_selection(-1, false, cx);
            return;
        };
        self.set_explorer_selection_at_index(parent_index, false, cx);
    }

    pub(crate) fn on_explorer_select_first(
        &mut self,
        _: &SelectFirst,
        _window: &mut Window,
        cx: &mut App,
    ) {
        self.explorer_move_selection(i32::MIN, false, cx);
    }

    pub(crate) fn on_explorer_select_last(
        &mut self,
        _: &SelectLast,
        _window: &mut Window,
        cx: &mut App,
    ) {
        self.explorer_move_selection(i32::MAX, false, cx);
    }

    /// Page up/down: move the selection by half the rendered rows (mirrors
    /// Zed's `scroll_up`/`scroll_down`).
    pub(crate) fn on_explorer_scroll_up(
        &mut self,
        _: &ScrollUp,
        _window: &mut Window,
        cx: &mut App,
    ) {
        let half = self.rendered_rows.saturating_div(2).max(1) as i32;
        self.explorer_move_selection(-half, false, cx);
    }

    pub(crate) fn on_explorer_scroll_down(
        &mut self,
        _: &ScrollDown,
        _window: &mut Window,
        cx: &mut App,
    ) {
        let half = self.rendered_rows.saturating_div(2).max(1) as i32;
        self.explorer_move_selection(half, false, cx);
    }

    /// Strict-scroll the list to the selection without moving it (Zed's
    /// `scroll_cursor_center/top/bottom`).
    pub(crate) fn on_explorer_scroll_cursor_center(
        &mut self,
        _: &ScrollCursorCenter,
        _window: &mut Window,
        cx: &mut App,
    ) {
        let Some(index) = self.explorer_selected_row_index() else {
            return;
        };
        self.scroll_handle
            .scroll_to_item_strict(index, ScrollStrategy::Center);
        cx.refresh_windows();
    }

    pub(crate) fn on_explorer_scroll_cursor_top(
        &mut self,
        _: &ScrollCursorTop,
        _window: &mut Window,
        cx: &mut App,
    ) {
        let Some(index) = self.explorer_selected_row_index() else {
            return;
        };
        self.scroll_handle
            .scroll_to_item_strict(index, ScrollStrategy::Top);
        cx.refresh_windows();
    }

    pub(crate) fn on_explorer_scroll_cursor_bottom(
        &mut self,
        _: &ScrollCursorBottom,
        _window: &mut Window,
        cx: &mut App,
    ) {
        let Some(index) = self.explorer_selected_row_index() else {
            return;
        };
        self.scroll_handle
            .scroll_to_item_strict(index, ScrollStrategy::Bottom);
        cx.refresh_windows();
    }

    /// Expand the selected entry (if collapsed directory, expands it; if already expanded, selects first child).
    pub(crate) fn on_explorer_expand_selected(
        &mut self,
        _: &ExpandSelectedEntry,
        _window: &mut Window,
        cx: &mut App,
    ) {
        if self.edit.is_some() {
            return;
        }
        let Some(index) = self.explorer_selected_row_index() else {
            return;
        };
        let Some(ExplorerRow::Entry(entry)) = self.entries.get(index) else {
            return;
        };
        let entry_id = entry.id;
        let kind = entry.kind;
        let is_expanded = entry.is_expanded;
        let has_children = entry.has_children;
        if kind == ExplorerEntryKind::Directory {
            if !is_expanded && has_children {
                self.toggle_explorer_node(entry_id, cx);
            } else if is_expanded && has_children {
                self.explorer_move_selection(1, false, cx);
            }
        }
    }

    /// Collapse the selected entry (if expanded directory, collapses it; if collapsed or file, selects parent).
    pub(crate) fn on_explorer_collapse_selected(
        &mut self,
        _: &CollapseSelectedEntry,
        _window: &mut Window,
        cx: &mut App,
    ) {
        if self.edit.is_some() {
            return;
        }
        let Some(index) = self.explorer_selected_row_index() else {
            return;
        };
        let Some(ExplorerRow::Entry(entry)) = self.entries.get(index) else {
            return;
        };
        let entry_id = entry.id;
        let kind = entry.kind;
        let is_expanded = entry.is_expanded;
        let parent_id = entry.parent_id;
        if kind == ExplorerEntryKind::Directory && is_expanded {
            self.toggle_explorer_node(entry_id, cx);
        } else if let Some(parent_id) = parent_id {
            if let Some(parent_index) = self
                .entries
                .iter()
                .position(|row| matches!(row, ExplorerRow::Entry(e) if e.id == parent_id))
            {
                self.set_explorer_selection_at_index(parent_index, false, cx);
            }
        }
    }

    pub(crate) fn on_explorer_expand_selected_and_children(
        &mut self,
        _: &ExpandSelectedEntryAndChildren,
        _window: &mut Window,
        cx: &mut App,
    ) {
        if self.edit.is_some() {
            return;
        }
        if let Some(sel) = self.selected {
            self.expand_all_explorer_for_entry(sel.entry_id, cx);
        }
    }

    pub(crate) fn on_explorer_collapse_selected_and_children(
        &mut self,
        _: &CollapseSelectedEntryAndChildren,
        _window: &mut Window,
        cx: &mut App,
    ) {
        if self.edit.is_some() {
            return;
        }
        if let Some(sel) = self.selected {
            self.collapse_all_explorer_for_entry(sel.entry_id, cx);
        }
    }

    pub(crate) fn on_explorer_expand_all_entries(
        &mut self,
        _: &ExpandAllEntries,
        _window: &mut Window,
        cx: &mut App,
    ) {
        self.expand_all_explorer_nodes(cx);
    }

    pub(crate) fn on_explorer_collapse_all_entries(
        &mut self,
        _: &CollapseAllEntries,
        _window: &mut Window,
        cx: &mut App,
    ) {
        self.collapse_all_explorer_nodes(cx);
    }

    pub(crate) fn on_explorer_open_selected(
        &mut self,
        _: &OpenSelectedEntry,
        window: &mut Window,
        cx: &mut App,
    ) {
        if self.edit.is_some() {
            return;
        }
        let Some(index) = self.explorer_selected_row_index() else {
            return;
        };
        let Some(ExplorerRow::Entry(entry)) = self.entries.get(index) else {
            return;
        };
        let entry_id = entry.id;
        let path = entry.path.clone();
        let kind = entry.kind;
        if kind == ExplorerEntryKind::Directory {
            self.toggle_explorer_node(entry_id, cx);
        } else {
            self.open_explorer_file(path, true, window, cx);
        }
    }

    pub(crate) fn on_explorer_rename_selected(
        &mut self,
        _: &RenameSelectedEntry,
        window: &mut Window,
        cx: &mut App,
    ) {
        if self.edit.is_some() {
            return;
        }
        let target_path = if let Some(index) = self.explorer_selected_row_index() {
            if let Some(ExplorerRow::Entry(entry)) = self.entries.get(index) {
                Some(entry.path.clone())
            } else {
                None
            }
        } else if let Some(sel) = self.selected {
            self.explorer_path_for_selection(&sel)
        } else {
            None
        };
        if let Some(path) = target_path {
            self.begin_inline_rename(path, window, cx);
        }
    }

    pub(crate) fn on_explorer_delete_selected(
        &mut self,
        _: &DeleteSelectedEntry,
        window: &mut Window,
        cx: &mut App,
    ) {
        if self.edit.is_some() {
            return;
        }
        self.delete_explorer_selections(window, cx);
    }

    pub(crate) fn on_explorer_trash_selected(
        &mut self,
        _: &TrashSelectedEntry,
        window: &mut Window,
        cx: &mut App,
    ) {
        if self.edit.is_some() {
            return;
        }
        self.trash_explorer_selections(window, cx);
    }

    pub(crate) fn on_explorer_new_file(&mut self, _: &NewFile, window: &mut Window, cx: &mut App) {
        let parent = match self.selected {
            Some(sel) => {
                if let Some(node) = self.explorer_entry_by_id(sel.entry_id) {
                    if node.kind == ExplorerEntryKind::Directory {
                        node.path.clone()
                    } else {
                        node.path
                            .parent()
                            .map(Path::to_path_buf)
                            .unwrap_or_else(|| node.path.clone())
                    }
                } else {
                    self.last_explorer_root_path().unwrap_or_default()
                }
            }
            _ => self.last_explorer_root_path().unwrap_or_default(),
        };
        if !parent.as_os_str().is_empty() {
            self.begin_inline_create_file(parent, window, cx);
        }
    }

    pub(crate) fn on_explorer_new_directory(
        &mut self,
        _: &NewDirectory,
        window: &mut Window,
        cx: &mut App,
    ) {
        let parent = match self.selected {
            Some(sel) => {
                if let Some(node) = self.explorer_entry_by_id(sel.entry_id) {
                    if node.kind == ExplorerEntryKind::Directory {
                        node.path.clone()
                    } else {
                        node.path
                            .parent()
                            .map(Path::to_path_buf)
                            .unwrap_or_else(|| node.path.clone())
                    }
                } else {
                    self.last_explorer_root_path().unwrap_or_default()
                }
            }
            _ => self.last_explorer_root_path().unwrap_or_default(),
        };
        if !parent.as_os_str().is_empty() {
            self.begin_inline_create_folder(parent, window, cx);
        }
    }

    pub(crate) fn on_explorer_reveal_active_file(
        &mut self,
        _: &RevealActiveFile,
        _window: &mut Window,
        cx: &mut App,
    ) {
        self.reveal_active_file_in_tree(true, cx);
    }
}
