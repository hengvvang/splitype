//! Explorer tree state: expansion sets, reveal-in-tree, and the derivation
//! of the flat visible row list from each worktree's cached tree.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use gpui::*;

use crate::editor::controller::Editor;
use crate::editor::explorer::state::*;

impl Editor {
    // ── Expand / collapse ────────────────────────────────────────────────

    /// Expand a directory and all of its descendants.
    pub(crate) fn expand_all_explorer_for_entry(
        &mut self,
        id: ExplorerEntryId,
        cx: &mut Context<Self>,
    ) {
        let Some(root) = self.root_for_explorer_entry(id) else {
            return;
        };
        let Some(tree) = self.panels.explorer.trees_cache.get(root).cloned() else {
            return;
        };
        let Some(node) = find_explorer_node_by_id(&tree, id) else {
            return;
        };
        let mut ids = BTreeSet::new();
        collect_descendant_dir_ids(node, &mut ids);
        self.panels
            .explorer
            .expanded
            .entry(root)
            .or_default()
            .extend(ids);
        self.rebuild_explorer_entries();
        cx.notify();
    }

    /// Collapse a directory and all of its descendants.
    pub(crate) fn collapse_all_explorer_for_entry(
        &mut self,
        id: ExplorerEntryId,
        cx: &mut Context<Self>,
    ) {
        let Some(root) = self.root_for_explorer_entry(id) else {
            return;
        };
        let Some(tree) = self.panels.explorer.trees_cache.get(root).cloned() else {
            return;
        };
        let Some(node) = find_explorer_node_by_id(&tree, id) else {
            return;
        };
        let mut ids = BTreeSet::new();
        collect_descendant_dir_ids(node, &mut ids);
        self.panels
            .explorer
            .expanded
            .entry(root)
            .or_default()
            .retain(|expanded_id| !ids.contains(expanded_id));
        self.rebuild_explorer_entries();
        cx.notify();
    }

    /// Expand every directory in every worktree.
    pub(crate) fn expand_all_explorer_nodes(&mut self, cx: &mut Context<Self>) {
        let trees = self.panels.explorer.trees_cache.clone();
        for (root, tree) in trees.iter().enumerate() {
            let mut ids = BTreeSet::new();
            collect_descendant_dir_ids(tree, &mut ids);
            self.panels
                .explorer
                .expanded
                .entry(root)
                .or_default()
                .extend(ids);
        }
        self.rebuild_explorer_entries();
        cx.notify();
    }

    pub(crate) fn toggle_explorer_node(&mut self, id: ExplorerEntryId, cx: &mut Context<Self>) {
        let Some(root) = self.root_for_explorer_entry(id) else {
            return;
        };
        let set = self.panels.explorer.expanded.entry(root).or_default();
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
        let Some(root) = self.root_for_explorer_entry(id) else {
            return;
        };
        let Some(tree) = self.panels.explorer.trees_cache.get(root).cloned() else {
            return;
        };
        let Some(node) = find_explorer_node_by_id(&tree, id) else {
            return;
        };
        if node.kind != ExplorerEntryKind::Directory {
            return;
        }
        let mut dir_ids = BTreeSet::new();
        collect_descendant_dir_ids(node, &mut dir_ids);
        let set = self.panels.explorer.expanded.entry(root).or_default();
        if set.contains(&id) {
            // Collapse: remove the entry and every descendant directory.
            for dir_id in dir_ids {
                set.remove(&dir_id);
            }
        } else {
            // Expand: insert the entry and every descendant directory.
            set.extend(dir_ids);
        }
        self.rebuild_explorer_entries();
        cx.notify();
    }

    // ── Tree cache and flat list ─────────────────────────────────────────

    /// Synchronize the explorer with the worktrees. Worktrees scan
    /// themselves in the background (see `worktree::Worktree`) and emit
    /// `UpdatedEntries`; this function only fills the gap when no worktree
    /// exists yet (e.g. deriving one from the active document).
    pub(crate) fn sync_explorer_file_tree(&mut self, cx: &mut Context<Self>) {
        if self.panels.explorer.worktrees.is_empty() {
            if let Some(path) = self.explorer_root_for_current_file() {
                self.add_explorer_worktree(path, cx);
                return;
            }
            self.panels.explorer.selected = None;
            self.panels.explorer.entries.clear();
            return;
        }
        self.select_active_file_in_tree(false);
        self.rebuild_explorer_entries();
    }

    /// Re-derive the flat visible row list: concatenate each worktree's
    /// cached tree segment, and splice in the inline edit row when an edit
    /// is active.
    pub(crate) fn rebuild_explorer_entries(&mut self) {
        let explorer = &mut self.panels.explorer;
        let trees: Vec<(usize, &ExplorerFileNode)> =
            explorer.trees_cache.iter().enumerate().collect();
        let expanded = explorer.expanded.clone();
        let edit = explorer.edit.as_ref();
        explorer.entries = build_explorer_rows(&trees, &expanded, edit);
    }

    /// Rebuild the per-worktree tree cache from each worktree's snapshot.
    /// Call whenever a worktree scan completes or a worktree is added.
    ///
    /// The cache stays indexed identically to `worktrees`: a worktree whose
    /// initial scan is still in flight yields a placeholder root row, so
    /// expansion sets and selections keyed by index never drift.
    pub(crate) fn refresh_explorer_trees(&mut self, cx: &mut Context<Self>) {
        let explorer = &mut self.panels.explorer;
        explorer.trees_cache = explorer
            .worktrees
            .iter()
            .map(|worktree| {
                let snapshot = worktree.read(cx).snapshot();
                build_tree_from_snapshot(&snapshot).unwrap_or_else(|| {
                    let root = worktree.read(cx).root();
                    ExplorerFileNode {
                        id: ExplorerEntryId(worktree.read(cx).root_id()),
                        path: root.to_path_buf(),
                        label: root
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| root.to_string_lossy().into_owned()),
                        kind: if root.is_dir() {
                            ExplorerEntryKind::Directory
                        } else {
                            ExplorerEntryKind::File
                        },
                        children: Vec::new(),
                    }
                })
            })
            .collect();
    }

    /// Follow the active document (or a pending inline-create target) in the
    /// tree. With `reveal`, ancestor directories are expanded so the entry
    /// becomes visible.
    ///
    /// An existing file selection is preserved when it is still in the tree
    /// (filesystem-driven rescans must not steal the user's selection); only
    /// a pending target or a missing selection falls back to the active
    /// document.
    pub(crate) fn select_active_file_in_tree(&mut self, reveal: bool) {
        // When the sidebar is showing the outline, the selection belongs to
        // the outline panel; do not steal it with a file-tree reveal.
        if self.panels.outline.selected.is_some() {
            self.panels.explorer.pending_select = None;
            return;
        }
        let trees = self.panels.explorer.trees_cache.clone();
        if trees.is_empty() {
            return;
        }
        let pending = self.panels.explorer.pending_select.take();
        if let Some((root, path)) = pending {
            if let Some(tree) = trees.get(root)
                && let Some(node) = find_explorer_node(tree, &path)
            {
                self.panels.explorer.selected = Some(ExplorerSelection::File {
                    root,
                    entry: node.id,
                });
                if reveal {
                    self.expand_to_path(&path);
                }
            }
            return;
        }
        // Keep an existing file selection when the entry is still visible.
        if let Some(ExplorerSelection::File { entry, .. }) = self.panels.explorer.selected {
            if trees
                .iter()
                .any(|tree| explorer_tree_contains_id(tree, entry))
            {
                return;
            }
        }
        let Some(path) = self
            .active_editor_tab()
            .and_then(|tab| tab.file.path.clone())
        else {
            return;
        };
        if let Some((root, id)) = self.explorer_id_for_path(&path) {
            self.panels.explorer.selected = Some(ExplorerSelection::File { root, entry: id });
        }
    }

    /// Expand every ancestor directory of `path` that exists in the tree
    /// (mirrors Zed's `expand_to_selection`). The root row itself is always
    /// expanded so the target can become visible.
    pub(crate) fn expand_to_path(&mut self, path: &Path) {
        let Some((root, _)) = self.explorer_id_for_path(path) else {
            return; // not in any worktree — nothing to reveal
        };
        let Some(tree) = self.panels.explorer.trees_cache.get(root).cloned() else {
            return;
        };
        // The root row must be expanded for anything below it to show.
        let set = self.panels.explorer.expanded.entry(root).or_default();
        set.insert(tree.id);
        let mut ancestors = Vec::new();
        for ancestor in path.ancestors() {
            if ancestor == tree.path.as_path() {
                break;
            }
            ancestors.push(ancestor.to_path_buf());
        }
        ancestors.reverse(); // root → leaf
        for ancestor in ancestors {
            if let Some(node) = find_explorer_node(&tree, &ancestor) {
                if node.kind == ExplorerEntryKind::Directory {
                    set.insert(node.id);
                }
            }
        }
    }

    /// Center the selected file row in the virtualized list.
    pub(crate) fn autoscroll_explorer_selection(&self) {
        let Some(ExplorerSelection::File { entry, .. }) = self.panels.explorer.selected else {
            return;
        };
        let Some(index) =
            self.panels.explorer.entries.iter().position(
                |row| matches!(row, ExplorerRow::Entry(entry_row) if entry_row.id == entry),
            )
        else {
            return;
        };
        self.panels
            .explorer
            .scroll_handle
            .scroll_to_item(index, ScrollStrategy::Center);
    }

    // ── Path resolution against the cached trees ─────────────────────────

    /// Worktree index whose root contains `path` (for pending selections and
    /// expansion targeting).
    pub(crate) fn root_for_explorer_path(&self, path: &Path) -> Option<usize> {
        self.panels
            .explorer
            .trees_cache
            .iter()
            .position(|tree| path.starts_with(&tree.path))
    }

    /// Path of the last worktree root (the default target for background
    /// drops and pastes without a selection).
    pub(crate) fn last_explorer_root_path(&self) -> Option<PathBuf> {
        self.panels
            .explorer
            .trees_cache
            .last()
            .map(|tree| tree.path.clone())
    }

    /// Last worktree root as `(index, path, root entry id)` — for background
    /// right-click / double-click targeting the root row.
    pub(crate) fn last_explorer_root(&self) -> Option<(usize, PathBuf, ExplorerEntryId)> {
        let index = self.panels.explorer.trees_cache.len().checked_sub(1)?;
        let tree = self.panels.explorer.trees_cache.get(index)?;
        Some((index, tree.path.clone(), tree.id))
    }
}
