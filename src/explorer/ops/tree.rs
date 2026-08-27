//! Explorer tree state: expansion sets, reveal-in-tree, and direct derivation
//! of visible entries from WorktreeSnapshot.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use gpui::*;

use crate::app::shell::Shell;
use crate::explorer::state::state::*;
use crate::explorer::state::worktree::{WorktreeEntryKind, WorktreeId, WorktreeSnapshot};

impl Shell {
    // ── Expand / collapse ────────────────────────────────────────────────

    /// Find the worktree entity and snapshot that contains the entry id.
    pub(crate) fn worktree_for_explorer_entry(
        &self,
        id: ExplorerEntryId,
    ) -> Option<(WorktreeId, Arc<WorktreeSnapshot>)> {
        for snap in &self.panels.explorer.snapshots {
            if snap.path_for_id.contains_key(&id) {
                return Some((snap.id(), snap.clone()));
            }
        }
        None
    }

    /// Expand a directory and all of its descendants.
    pub(crate) fn expand_all_explorer_for_entry(
        &mut self,
        id: ExplorerEntryId,
        cx: &mut Context<Self>,
    ) {
        let Some((worktree_id, snapshot)) = self.worktree_for_explorer_entry(id) else {
            return;
        };
        let Some(entry_path) = snapshot.path_for_id.get(&id) else {
            return;
        };
        let mut ids = BTreeSet::new();
        for (path, entry) in &snapshot.entries_by_path {
            if path.starts_with(entry_path) && entry.kind == WorktreeEntryKind::Directory {
                ids.insert(entry.id);
            }
        }
        if !ids.is_empty() {
            self.panels
                .explorer
                .expanded
                .entry(worktree_id)
                .or_default()
                .extend(ids);
            self.rebuild_explorer_entries();
            cx.notify();
        }
    }

    /// Collapse a directory and all of its descendants.
    pub(crate) fn collapse_all_explorer_for_entry(
        &mut self,
        id: ExplorerEntryId,
        cx: &mut Context<Self>,
    ) {
        let Some((worktree_id, snapshot)) = self.worktree_for_explorer_entry(id) else {
            return;
        };
        let Some(entry_path) = snapshot.path_for_id.get(&id) else {
            return;
        };
        let mut ids = BTreeSet::new();
        for (path, entry) in &snapshot.entries_by_path {
            if path.starts_with(entry_path) && entry.kind == WorktreeEntryKind::Directory {
                ids.insert(entry.id);
            }
        }
        if !ids.is_empty() {
            self.panels
                .explorer
                .expanded
                .entry(worktree_id)
                .or_default()
                .retain(|expanded_id| !ids.contains(expanded_id));
            self.rebuild_explorer_entries();
            cx.notify();
        }
    }

    /// Expand every directory in every worktree.
    pub(crate) fn expand_all_explorer_nodes(&mut self, cx: &mut Context<Self>) {
        for snap in &self.panels.explorer.snapshots {
            let mut ids = BTreeSet::new();
            for entry in snap.entries_by_path.values() {
                if entry.kind == WorktreeEntryKind::Directory {
                    ids.insert(entry.id);
                }
            }
            self.panels
                .explorer
                .expanded
                .entry(snap.id())
                .or_default()
                .extend(ids);
        }
        self.rebuild_explorer_entries();
        cx.notify();
    }

    /// Collapse all directories in all worktrees.
    pub(crate) fn collapse_all_explorer_nodes(&mut self, cx: &mut Context<Self>) {
        self.panels.explorer.expanded.clear();
        self.rebuild_explorer_entries();
        cx.notify();
    }

    pub(crate) fn toggle_explorer_node(&mut self, id: ExplorerEntryId, cx: &mut Context<Self>) {
        let Some((worktree_id, _)) = self.worktree_for_explorer_entry(id) else {
            return;
        };
        let set = self.panels.explorer.expanded.entry(worktree_id).or_default();
        let will_expand = !set.remove(&id);
        if will_expand {
            set.insert(id);
        }
        self.rebuild_explorer_entries();
        cx.notify();
    }

    /// Alt+click on a directory: recursively expand or collapse the whole
    /// subtree (mirrors Zed's `toggle_expand_all`).
    pub(crate) fn toggle_explorer_subtree(&mut self, id: ExplorerEntryId, cx: &mut Context<Self>) {
        let Some((worktree_id, snapshot)) = self.worktree_for_explorer_entry(id) else {
            return;
        };
        let Some(entry_path) = snapshot.path_for_id.get(&id) else {
            return;
        };
        let mut dir_ids = BTreeSet::new();
        for (path, entry) in &snapshot.entries_by_path {
            if path.starts_with(entry_path) && entry.kind == WorktreeEntryKind::Directory {
                dir_ids.insert(entry.id);
            }
        }
        if dir_ids.is_empty() {
            return;
        }
        let set = self.panels.explorer.expanded.entry(worktree_id).or_default();
        if set.contains(&id) {
            for dir_id in dir_ids {
                set.remove(&dir_id);
            }
        } else {
            set.extend(dir_ids);
        }
        self.rebuild_explorer_entries();
        cx.notify();
    }

    // ── Flat list derivation ─────────────────────────────────────────────

    /// Synchronize the explorer with the worktrees.
    pub(crate) fn sync_explorer_file_tree(&mut self, cx: &mut Context<Self>) {
        self.panels.explorer.snapshots = self
            .panels
            .explorer
            .worktrees
            .iter()
            .map(|wt| wt.read(cx).snapshot())
            .collect();
        if self.panels.explorer.worktrees.is_empty() {
            self.panels.explorer.selected = None;
            self.panels.explorer.entries.clear();
            return;
        }
        self.select_active_file_in_tree(false, cx);
        self.rebuild_explorer_entries();
    }

    /// Re-derive the flat visible row list directly from each worktree's snapshot.
    pub(crate) fn rebuild_explorer_entries(&mut self) {
        let expanded = self.panels.explorer.expanded.clone();
        let edit = self.panels.explorer.edit.as_ref();
        self.panels.explorer.entries =
            build_explorer_rows(&self.panels.explorer.snapshots, &expanded, edit);
    }

    /// Follow the active document (or a pending inline-create target) in the
    /// tree. With `reveal`, ancestor directories are expanded so the entry
    /// becomes visible.
    pub(crate) fn select_active_file_in_tree(&mut self, reveal: bool, cx: &App) {
        if self
            .active_editor()
            .is_some_and(|editor| editor.read(cx).outline.selected.is_some())
        {
            self.panels.explorer.pending_select = None;
            return;
        }
        if self.panels.explorer.worktrees.is_empty() {
            return;
        }
        let pending = self.panels.explorer.pending_select.take();
        if let Some((_worktree_id, path)) = pending {
            if let Some(sel) = self.explorer_id_for_path(&path) {
                self.panels.explorer.selected = Some(sel);
                if reveal {
                    self.expand_to_path(&path);
                }
            }
            return;
        }
        // Keep an existing file selection when the entry is still in any worktree snapshot.
        if let Some(sel) = self.panels.explorer.selected
            && self.explorer_path_for_id(sel.entry_id).is_some()
        {
            return;
        }
        let Some(path) = self
            .active_editor_tab(cx)
            .and_then(|tab| tab.file.path.clone())
        else {
            return;
        };
        if let Some(sel) = self.explorer_id_for_path(&path) {
            self.panels.explorer.selected = Some(sel);
        }
    }

    /// Expand every ancestor directory of `path` that exists in the tree.
    pub(crate) fn expand_to_path(&mut self, path: &Path) {
        let Some(sel) = self.explorer_id_for_path(path) else {
            return;
        };
        let Some((worktree_id, snapshot)) = self.worktree_for_explorer_entry(sel.entry_id) else {
            return;
        };
        let set = self.panels.explorer.expanded.entry(worktree_id).or_default();
        if let Some(root_entry) = snapshot.root_entry() {
            set.insert(root_entry.id);
        }
        for ancestor in path.ancestors() {
            if let Some(id) = snapshot.id_for_path.get(ancestor) {
                set.insert(*id);
            }
        }
    }

    /// Center the selected file row in the virtualized list.
    pub(crate) fn autoscroll_explorer_selection(&self) {
        let Some(sel) = self.panels.explorer.selected else {
            return;
        };
        let Some(index) = self.panels.explorer.entries.iter().position(
            |row| matches!(row, ExplorerRow::Entry(entry_row) if entry_row.id == sel.entry_id),
        ) else {
            return;
        };
        self.panels
            .explorer
            .scroll_handle
            .scroll_to_item(index, ScrollStrategy::Center);
    }

    // ── Path resolution against worktrees ─────────────────────────────────

    /// Path of the last worktree root.
    pub(crate) fn last_explorer_root_path(&self) -> Option<PathBuf> {
        self.panels
            .explorer
            .snapshots
            .last()
            .and_then(|snap| snap.root_entry().map(|e| e.path.clone()))
    }

    /// Last worktree root as `(worktree_id, path, root_entry_id)`.
    pub(crate) fn last_explorer_root(&self) -> Option<(WorktreeId, PathBuf, ExplorerEntryId)> {
        let snap = self.panels.explorer.snapshots.last()?;
        let root_entry = snap.root_entry()?;
        Some((snap.id(), root_entry.path.clone(), root_entry.id))
    }
}
