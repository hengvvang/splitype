//! Explorer selection: resolving the effective entries an operation applies
//! to, multi-select marks, range selection, and keyboard navigation /
//! scrolling (mirrors Zed's `select_*`/`scroll_*` methods).

use std::path::Path;

use gpui::*;

use crate::editor::controller::Editor;
use crate::editor::panels::explorer::state::*;

// Explorer navigation actions. They intentionally carry no default key
// bindings (keybinding design is out of scope); handlers are wired on the
// explorer root in `render_explorer_files_tree`.
actions!(
    explorer,
    [
        SelectPrevious,
        SelectNext,
        SelectParent,
        SelectFirst,
        SelectLast,
        ScrollCursorCenter,
        ScrollCursorTop,
        ScrollCursorBottom,
        ScrollUp,
        ScrollDown,
    ]
);

impl Editor {
    // ── Selection resolution ─────────────────────────────────────────────

    /// Worktree index whose cached tree contains `id`. Ids are globally
    /// unique across worktrees, so this is unambiguous.
    pub(crate) fn root_for_explorer_entry(&self, id: ExplorerEntryId) -> Option<usize> {
        self.panels
            .explorer
            .trees_cache
            .iter()
            .position(|tree| explorer_tree_contains_id(tree, id))
    }

    /// Locate `path` in the cached trees; returns the worktree index and the
    /// entry's stable id.
    pub(crate) fn explorer_id_for_path(&self, path: &Path) -> Option<(usize, ExplorerEntryId)> {
        self.panels
            .explorer
            .trees_cache
            .iter()
            .enumerate()
            .find_map(|(root, tree)| find_explorer_node(tree, path).map(|node| (root, node.id)))
    }

    /// Look up the visible row for a file selection (root + entry id).
    pub(crate) fn explorer_entry_for_selection(
        &self,
        sel: &ExplorerSelection,
    ) -> Option<&VisibleExplorerEntry> {
        match sel {
            ExplorerSelection::File { entry, .. } => self.explorer_entry_by_id(*entry),
        }
    }

    /// Resolve the entries an operation applies to (Zed's `effective_entries`):
    /// the selection when nothing is marked, otherwise the marked set. The
    /// worktree roots are excluded so destructive operations (delete / cut /
    /// move / copy) can never target a root row.
    pub(crate) fn effective_explorer_entries(&self) -> Vec<ExplorerSelection> {
        let root_ids: std::collections::HashSet<ExplorerEntryId> = self
            .panels
            .explorer
            .trees_cache
            .iter()
            .map(|tree| tree.id)
            .collect();
        let filter = |sel: &ExplorerSelection| match sel {
            ExplorerSelection::File { entry, .. } => !root_ids.contains(entry),
        };
        if self.panels.explorer.marked.is_empty() {
            return match &self.panels.explorer.selected {
                Some(sel) if filter(sel) => vec![sel.clone()],
                _ => Vec::new(),
            };
        }
        self.panels
            .explorer
            .marked
            .iter()
            .filter(|sel| filter(sel))
            .cloned()
            .collect()
    }

    /// Look up a visible entry row by its stable id.
    pub(crate) fn explorer_entry_by_id(
        &self,
        id: ExplorerEntryId,
    ) -> Option<&VisibleExplorerEntry> {
        self.panels
            .explorer
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
        selection: ExplorerSelection,
        cx: &mut Context<Self>,
    ) {
        let marked = &mut self.panels.explorer.marked;
        if let Some(index) = marked.iter().position(|item| item == &selection) {
            marked.remove(index);
        } else {
            marked.push(selection.clone());
        }
        self.panels.explorer.selected = Some(selection);
        cx.notify();
    }

    /// Range-select from the current selection to `target_id` (Shift+click).
    pub(crate) fn select_explorer_range(
        &mut self,
        target_id: ExplorerEntryId,
        cx: &mut Context<Self>,
    ) {
        let anchor = match &self.panels.explorer.selected {
            Some(ExplorerSelection::File { entry, .. }) => *entry,
            _ => target_id,
        };
        let rows = &self.panels.explorer.entries;
        let anchor_index = rows
            .iter()
            .position(|row| matches!(row, ExplorerRow::Entry(entry) if entry.id == anchor));
        let target_index = rows
            .iter()
            .position(|row| matches!(row, ExplorerRow::Entry(entry) if entry.id == target_id));
        let (Some(anchor_index), Some(target_index)) = (anchor_index, target_index) else {
            return;
        };
        self.panels.explorer.marked.clear();
        let mut target_root = None;
        for row in &rows[anchor_index.min(target_index)..=anchor_index.max(target_index)] {
            if let ExplorerRow::Entry(entry) = row {
                let selection = ExplorerSelection::File {
                    root: entry.root,
                    entry: entry.id,
                };
                if entry.id == target_id {
                    target_root = Some(entry.root);
                }
                self.panels.explorer.marked.push(selection);
            }
        }
        self.panels.explorer.selected = Some(ExplorerSelection::File {
            root: target_root.unwrap_or(0),
            entry: target_id,
        });
        self.autoscroll_explorer_selection();
        cx.notify();
    }

    /// Choose the next selection after deleting `deleted_ids` (mirrors Zed's
    /// `find_next_selection_after_deletion`): the next visible sibling, else
    /// the previous one, else the parent directory.
    pub(crate) fn next_explorer_selection_after_deletion(
        &self,
        deleted_selections: &[ExplorerSelection],
    ) -> Option<ExplorerSelection> {
        let deleted: std::collections::HashSet<ExplorerEntryId> = deleted_selections
            .iter()
            .filter_map(|sel| match sel {
                ExplorerSelection::File { entry, .. } => Some(*entry),
            })
            .collect();
        let rows = &self.panels.explorer.entries;
        let last_deleted = rows.iter().rposition(
            |row| matches!(row, ExplorerRow::Entry(entry) if deleted.contains(&entry.id)),
        )?;
        for row in &rows[last_deleted + 1..] {
            if let ExplorerRow::Entry(entry) = row {
                if !deleted.contains(&entry.id) {
                    return Some(ExplorerSelection::File {
                        root: entry.root,
                        entry: entry.id,
                    });
                }
            }
        }
        for row in rows[..last_deleted].iter().rev() {
            if let ExplorerRow::Entry(entry) = row {
                if !deleted.contains(&entry.id) {
                    return Some(ExplorerSelection::File {
                        root: entry.root,
                        entry: entry.id,
                    });
                }
            }
        }
        if let ExplorerRow::Entry(last) = &rows[last_deleted]
            && let Some(tree) = self.panels.explorer.trees_cache.get(last.root)
            && let Some(parent) = last.path.parent()
            && let Some(parent_node) = find_explorer_node(tree, parent)
        {
            return Some(ExplorerSelection::File {
                root: last.root,
                entry: parent_node.id,
            });
        }
        None
    }

    /// Whether `selections` contains a worktree root (root rows are dragged
    /// to reorder worktrees, not to move files — mirrors Zed's
    /// `entry_is_worktree_root` check in `drag_onto`).
    pub(crate) fn explorer_is_root_entry(&self, id: ExplorerEntryId) -> bool {
        self.panels
            .explorer
            .trees_cache
            .iter()
            .any(|tree| tree.id == id)
    }

    // ── Selection navigation and scrolling (mirrors Zed) ────────────────

    /// Row index of the currently selected file entry, if visible.
    fn explorer_selected_row_index(&self) -> Option<usize> {
        match &self.panels.explorer.selected {
            Some(ExplorerSelection::File { entry, .. }) => {
                self.panels.explorer.entries.iter().position(
                    |row| matches!(row, ExplorerRow::Entry(row_entry) if row_entry.id == *entry),
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
        cx: &mut Context<Self>,
    ) {
        let Some(ExplorerRow::Entry(entry)) = self.panels.explorer.entries.get(index) else {
            return;
        };
        let selection = ExplorerSelection::File {
            root: entry.root,
            entry: entry.id,
        };
        if extend && !self.panels.explorer.marked.contains(&selection) {
            self.panels.explorer.marked.push(selection.clone());
        }
        self.panels.explorer.selected = Some(selection);
        self.panels
            .explorer
            .scroll_handle
            .scroll_to_item(index, ScrollStrategy::Center);
        cx.notify();
    }

    /// Move the selection by `delta` rows (signed), clamping to the list.
    fn explorer_move_selection(&mut self, delta: i32, extend: bool, cx: &mut Context<Self>) {
        if self.panels.explorer.edit.is_some() {
            return; // the inline editor owns the keyboard while editing
        }
        let len = self.panels.explorer.entries.len();
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
        cx: &mut Context<Self>,
    ) {
        if self.panels.explorer.edit.is_some() {
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
        cx: &mut Context<Self>,
    ) {
        if self.panels.explorer.edit.is_some() {
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
        cx: &mut Context<Self>,
    ) {
        if self.panels.explorer.edit.is_some() {
            return;
        }
        let Some(index) = self.explorer_selected_row_index() else {
            return;
        };
        let Some(ExplorerRow::Entry(entry)) = self.panels.explorer.entries.get(index) else {
            return;
        };
        let Some(parent_id) = entry.parent_id else {
            self.explorer_move_selection(-1, false, cx);
            return;
        };
        let Some(parent_index) = self
            .panels
            .explorer
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
        cx: &mut Context<Self>,
    ) {
        self.explorer_move_selection(i32::MIN, false, cx);
    }

    pub(crate) fn on_explorer_select_last(
        &mut self,
        _: &SelectLast,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.explorer_move_selection(i32::MAX, false, cx);
    }

    /// Page up/down: move the selection by half the rendered rows (mirrors
    /// Zed's `scroll_up`/`scroll_down`).
    pub(crate) fn on_explorer_scroll_up(
        &mut self,
        _: &ScrollUp,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let half = self.panels.explorer.rendered_rows.saturating_div(2).max(1) as i32;
        self.explorer_move_selection(-half, false, cx);
    }

    pub(crate) fn on_explorer_scroll_down(
        &mut self,
        _: &ScrollDown,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let half = self.panels.explorer.rendered_rows.saturating_div(2).max(1) as i32;
        self.explorer_move_selection(half, false, cx);
    }

    /// Strict-scroll the list to the selection without moving it (Zed's
    /// `scroll_cursor_center/top/bottom`).
    pub(crate) fn on_explorer_scroll_cursor_center(
        &mut self,
        _: &ScrollCursorCenter,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(index) = self.explorer_selected_row_index() else {
            return;
        };
        self.panels
            .explorer
            .scroll_handle
            .scroll_to_item_strict(index, ScrollStrategy::Center);
        cx.notify();
    }

    pub(crate) fn on_explorer_scroll_cursor_top(
        &mut self,
        _: &ScrollCursorTop,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(index) = self.explorer_selected_row_index() else {
            return;
        };
        self.panels
            .explorer
            .scroll_handle
            .scroll_to_item_strict(index, ScrollStrategy::Top);
        cx.notify();
    }

    pub(crate) fn on_explorer_scroll_cursor_bottom(
        &mut self,
        _: &ScrollCursorBottom,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(index) = self.explorer_selected_row_index() else {
            return;
        };
        self.panels
            .explorer
            .scroll_handle
            .scroll_to_item_strict(index, ScrollStrategy::Bottom);
        cx.notify();
    }
}
