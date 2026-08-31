//! Explorer selection: resolving the effective entries an operation applies
//! to, multi-select marks, range selection, and keyboard navigation /
//! scrolling (mirrors Zed's `select_*`/`scroll_*` methods).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use gpui::*;

use schemars::JsonSchema;
use serde::Deserialize;

use crate::state::state::*;

/// Update the paths of open editor tabs after a filesystem move/rename
/// (dispatched by the explorer; handled by the shell).
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema, gpui::Action)]
#[action(namespace = explorer)]
#[serde(deny_unknown_fields)]
pub struct UpdateOpenTabPaths {
    /// Old absolute path.
    pub from: String,
    /// New absolute path.
    pub to: String,
}

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
    ]
);

impl ExplorerState {
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

    /// Look up the visible row for a file selection.
    pub(crate) fn explorer_entry_for_selection(
        &self,
        sel: &SelectedEntry,
    ) -> Option<&VisibleExplorerEntry> {
        self.explorer_entry_by_id(sel.entry_id)
    }

    /// Resolve the entries an operation applies to (Zed's `effective_entries`):
    /// the selection when nothing is marked, otherwise the marked set. The
    /// worktree roots are excluded so destructive operations (delete / cut /
    /// move / copy) can never target a root row.
    pub(crate) fn effective_explorer_entries(&self) -> Vec<SelectedEntry> {
        let root_ids: HashSet<ExplorerEntryId> = self
            .snapshots
            .iter()
            .filter_map(|snap| snap.root_entry().map(|e| e.id))
            .collect();
        let filter = |sel: &SelectedEntry| !root_ids.contains(&sel.entry_id);
        if self.marked.is_empty() {
            return match self.selected {
                Some(sel) if filter(&sel) => vec![sel],
                _ => Vec::new(),
            };
        }
        self
            .marked
            .iter()
            .filter(|sel| filter(sel))
            .copied()
            .collect()
    }

    /// Look up a visible entry row by its stable id.
    pub(crate) fn explorer_entry_by_id(
        &self,
        id: ExplorerEntryId,
    ) -> Option<&VisibleExplorerEntry> {
        self
            .entries
            .iter()
            .find_map(|row| match row {
                ExplorerRow::Entry(entry) if entry.id == id => Some(entry),
                _ => None,
            })
    }

    /// Toggle an entry in the multi-select mark set (Alt+click).
    pub(crate) fn toggle_explorer_mark(
        &mut self,
        selection: SelectedEntry,
        cx: &mut App,
    ) {
        if !self.marked.remove(&selection) {
            self.marked.insert(selection);
        }
        self.selected = Some(selection);
        cx.refresh_windows();
    }

    /// Range-select from the current selection to `target_id` (Shift+click).
    pub(crate) fn select_explorer_range(
        &mut self,
        target_id: ExplorerEntryId,
        cx: &mut App,
    ) {
        let anchor = match self.selected {
            Some(sel) => sel.entry_id,
            _ => target_id,
        };
        let rows = &self.entries;
        let anchor_index = rows
            .iter()
            .position(|row| matches!(row, ExplorerRow::Entry(entry) if entry.id == anchor));
        let target_index = rows
            .iter()
            .position(|row| matches!(row, ExplorerRow::Entry(entry) if entry.id == target_id));
        let (Some(anchor_index), Some(target_index)) = (anchor_index, target_index) else {
            return;
        };
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
        self.autoscroll_explorer_selection();
        cx.refresh_windows();
    }

    /// Choose the next selection after deleting `deleted_ids` (mirrors Zed's
    /// `find_next_selection_after_deletion`): the next visible sibling, else
    /// the previous one, else the parent directory.
    pub(crate) fn next_explorer_selection_after_deletion(
        &self,
        deleted_selections: &[SelectedEntry],
    ) -> Option<SelectedEntry> {
        let deleted: HashSet<ExplorerEntryId> = deleted_selections
            .iter()
            .map(|sel| sel.entry_id)
            .collect();
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
        if let ExplorerRow::Entry(last) = &rows[last_deleted]
            && let Some(parent_id) = last.parent_id
        {
            return Some(SelectedEntry {
                worktree_id: last.worktree_id,
                entry_id: parent_id,
            });
        }
        None
    }

    /// Whether `id` is a worktree root.
    pub(crate) fn is_explorer_root_entry(&self, id: ExplorerEntryId) -> bool {
        self
            .snapshots
            .iter()
            .any(|snap| snap.root_entry().map(|e| e.id) == Some(id))
    }

    // ── Selection navigation and scrolling (mirrors Zed) ────────────────

    /// Row index of the currently selected file entry, if visible.
    fn explorer_selected_row_index(&self) -> Option<usize> {
        match self.selected {
            Some(sel) => {
                self.entries.iter().position(
                    |row| matches!(row, ExplorerRow::Entry(row_entry) if row_entry.id == sel.entry_id),
                )
            }
            _ => None,
        }
    }

    /// Set the selection to the row at `index` and center it (Zed's
    /// `autoscroll`). With `extend`, the row is also added to the marks.
    fn set_explorer_selection_at_index(
        &mut self,
        index: usize,
        extend: bool,
        cx: &mut App,
    ) {
        let Some(ExplorerRow::Entry(entry)) = self.entries.get(index) else {
            return;
        };
        let selection = SelectedEntry {
            worktree_id: entry.worktree_id,
            entry_id: entry.id,
        };
        if extend {
            self.marked.insert(selection);
        }
        self.selected = Some(selection);
        self
            .scroll_handle
            .scroll_to_item(index, ScrollStrategy::Center);
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
        self
            .scroll_handle
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
        self
            .scroll_handle
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
        self
            .scroll_handle
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
            self.open_explorer_file(
                path,
                true,
                window,
                cx,
            );
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
        let Some(index) = self.explorer_selected_row_index() else {
            return;
        };
        let Some(ExplorerRow::Entry(entry)) = self.entries.get(index) else {
            return;
        };
        let path = entry.path.clone();
        self.begin_inline_rename(path, window, cx);
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

    pub(crate) fn on_explorer_new_file(
        &mut self,
        _: &NewFile,
        window: &mut Window,
        cx: &mut App,
    ) {
        let parent = match self.selected {
            Some(sel) => {
                if let Some(node) = self.explorer_entry_by_id(sel.entry_id) {
                    if node.kind == ExplorerEntryKind::Directory {
                        node.path.clone()
                    } else {
                        node.path.parent().map(Path::to_path_buf).unwrap_or_else(|| node.path.clone())
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
                        node.path.parent().map(Path::to_path_buf).unwrap_or_else(|| node.path.clone())
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
}

