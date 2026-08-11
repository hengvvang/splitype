//! Explorer panel lifecycle: drawer visibility, worktree management
//! (add / remove / reorder), scan events, settings sync and path helpers.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use gpui::*;

use crate::app::shell::Shell;

use crate::editor::actions::{CloseExplorerFolder, ToggleExplorer};
use crate::editor::explorer_state::state::*;
use crate::editor::explorer_state::worktree::Worktree;
use crate::infra::config::settings::ExplorerSettingsStore;

/// Re-key a selection after a worktree removal: selections inside the
/// removed worktree fall back to `fallback`; selections in later worktrees
/// shift down by one index.
fn remap_explorer_selection(
    sel: ExplorerSelection,
    removed: usize,
    fallback: ExplorerSelection,
) -> ExplorerSelection {
    match sel {
        ExplorerSelection::File { root, entry: _ } if root == removed => fallback,
        ExplorerSelection::File { root, entry } if root > removed => ExplorerSelection::File {
            root: root - 1,
            entry,
        },
        other => other,
    }
}

/// Re-key a worktree index after `worktrees` moved `from` to `to`
/// (`from` ends up directly before the entry that was at `to`).
fn remap_explorer_root_after_move(index: usize, from: usize, to: usize) -> usize {
    let insert_at = if from < to { to - 1 } else { to };
    if index == from {
        insert_at
    } else if from < to && index > from && index <= insert_at {
        index - 1
    } else if from > to && index >= insert_at && index < from {
        index + 1
    } else {
        index
    }
}

impl Shell {
    pub(crate) fn toggle_explorer_drawer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.panels.explorer.is_open {
            self.panels.explorer.is_open = false;
        } else {
            self.dismiss_contextual_overlays(cx);
            self.panels.explorer.is_open = true;
            self.sync_explorer_models(cx);
            window.activate_window();
        }
        cx.notify();
    }
    pub(crate) fn on_toggle_explorer_action(
        &mut self,
        _: &ToggleExplorer,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_explorer_drawer(window, cx);
    }
    pub(crate) fn on_close_explorer_folder_action(
        &mut self,
        _: &CloseExplorerFolder,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_explorer_folder(cx);
    }
    pub(crate) fn close_explorer_folder(&mut self, cx: &mut Context<Self>) {
        let explorer = &mut self.panels.explorer;
        explorer.worktrees.clear();
        explorer.expanded.clear();
        explorer.file_error = None;
        explorer.entries.clear();
        explorer.selected = None;
        explorer.marked.clear();
        explorer.pending_select = None;
        explorer.pending_rename = None;
        explorer.edit = None;
        // Drop any in-flight drag state (highlight, hover-expand, edge
        // scroll) so closing the panel cannot leave stale tasks behind.
        explorer.drag_target = None;
        explorer.hover_expand_task = None;
        explorer.hover_scroll_task = None;
        explorer.hover_scroll_generation += 1;
        explorer.previous_drag_position = None;
        cx.notify();
    }

    /// Add a project root as a new worktree (mirrors Zed's
    /// `WorktreeStore::create_worktree`). The root row starts expanded.
    pub(crate) fn add_explorer_worktree(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let explorer = &mut self.panels.explorer;
        if explorer
            .worktrees
            .iter()
            .any(|worktree| worktree.read(cx).root() == path.as_path())
        {
            return; // already added
        }
        explorer.is_open = true;
        let index = explorer.worktrees.len();
        let hide_hidden = ExplorerSettingsStore::settings(cx).hide_hidden;
        let worktree = Worktree::new(
            path.clone(),
            explorer.next_entry_id.clone(),
            hide_hidden,
            cx,
        );
        // The root row starts expanded (VSCode-style title row visible).
        let root_id = worktree.read(cx).root_id();
        explorer
            .expanded
            .entry(index)
            .or_default()
            .insert(ExplorerEntryId(root_id));
        cx.subscribe(&worktree, Self::on_explorer_worktree_event)
            .detach();
        explorer.worktrees.push(worktree);
        explorer.file_error = None;
        self.refresh_explorer_trees(cx);
        self.rebuild_explorer_entries();
        cx.notify();
    }

    /// Remove the worktree at `index` (mirrors Zed's `remove_worktree`);
    /// wired to the root row's "Remove from Explorer" context menu item.
    pub(crate) fn remove_explorer_worktree(&mut self, index: usize, cx: &mut Context<Self>) {
        {
            let explorer = &mut self.panels.explorer;
            if index >= explorer.worktrees.len() {
                return;
            }
            explorer.worktrees.remove(index);
        }
        // Keep the tree cache indexed identically to `worktrees` before
        // remapping selection keys below.
        self.refresh_explorer_trees(cx);
        let explorer = &mut self.panels.explorer;
        // Shift the expansion map and selection keys after removal.
        let mut reindexed = HashMap::new();
        for (old_index, set) in explorer.expanded.drain() {
            if old_index == index {
                continue;
            }
            let new_index = if old_index > index {
                old_index - 1
            } else {
                old_index
            };
            reindexed.insert(new_index, set);
        }
        explorer.expanded = reindexed;
        // Resolve the fallback selection before touching any field so the
        // remap closure never aliases `explorer` (a worktree removal leaves
        // the last remaining worktree's root selected).
        let fallback = ExplorerSelection::File {
            root: explorer.worktrees.len().saturating_sub(1),
            entry: explorer
                .trees_cache
                .last()
                .map(|tree| tree.id)
                .unwrap_or(ExplorerEntryId(0)),
        };
        explorer.selected = explorer
            .selected
            .take()
            .map(|sel| remap_explorer_selection(sel, index, fallback.clone()));
        for sel in explorer.marked.iter_mut() {
            *sel = remap_explorer_selection(sel.clone(), index, fallback.clone());
        }
        explorer.edit = None;
        explorer.pending_select = None;
        self.rebuild_explorer_entries();
        cx.notify();
    }

    /// Reorder worktrees by dragging a root row onto another root
    /// (mirrors Zed's `Project::move_worktree`): the dragged root ends up
    /// directly before the drop-target root. All root-keyed state
    /// (expansions, selections, pending selects) is re-keyed accordingly.
    pub(crate) fn move_explorer_worktree(
        &mut self,
        from: usize,
        to: usize,
        cx: &mut Context<Self>,
    ) {
        let explorer = &mut self.panels.explorer;
        let len = explorer.worktrees.len();
        if from == to || from >= len || to >= len {
            return;
        }
        let worktree = explorer.worktrees.remove(from);
        let insert_at = if from < to { to - 1 } else { to };
        explorer.worktrees.insert(insert_at, worktree);
        // Re-key the expansion map.
        let mut reindexed = HashMap::new();
        for (old_index, set) in explorer.expanded.drain() {
            reindexed.insert(remap_explorer_root_after_move(old_index, from, to), set);
        }
        explorer.expanded = reindexed;
        // Re-key selections, marks and pending selects.
        let remap = |sel: &ExplorerSelection| match sel {
            ExplorerSelection::File { root, entry } => ExplorerSelection::File {
                root: remap_explorer_root_after_move(*root, from, to),
                entry: *entry,
            },
        };
        explorer.selected = explorer.selected.take().map(|sel| remap(&sel));
        explorer.marked = explorer.marked.iter().map(remap).collect();
        if let Some((root, path)) = explorer.pending_select.take() {
            explorer.pending_select = Some((remap_explorer_root_after_move(root, from, to), path));
        }
        explorer.edit = None;
        self.refresh_explorer_trees(cx);
        self.rebuild_explorer_entries();
        cx.notify();
    }

    /// Handle a worktree scan event: refresh the tree cache and rebuild the
    /// visible list (Zed's `WorktreeUpdatedEntries` handler). Also consumes
    /// a pending copy-collision rename once the new entry became visible.
    pub(crate) fn on_explorer_worktree_event(
        &mut self,
        _worktree: Entity<Worktree>,
        _event: &WorktreeEvent,
        cx: &mut Context<Self>,
    ) {
        self.refresh_explorer_trees(cx);
        self.select_active_file_in_tree(true, cx);
        self.rebuild_explorer_entries();
        self.autoscroll_explorer_selection();
        // Start the pending rename editor now that the copied entry is in
        // the scanned tree (the editor needs window access, so it runs in a
        // spawned task).
        if let Some((window_handle, path)) = self.panels.explorer.pending_rename.take() {
            if self.explorer_id_for_path(&path).is_some() {
                let weak_editor = cx.entity().downgrade();
                let _ = cx.spawn(async move |_this, cx: &mut AsyncApp| {
                    let _ = cx.update_window(window_handle, |_, window, cx| {
                        let _ = weak_editor.update(cx, |editor, cx| {
                            editor.start_inline_rename(path, window, cx);
                        });
                    });
                });
            }
        }
        cx.notify();
    }

    pub(crate) fn open_explorer_folder_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.add_explorer_worktree(path, cx);
    }
    pub(crate) fn sync_explorer_after_document_path_change(&mut self, cx: &mut Context<Self>) {
        if self.panels.explorer.worktrees.is_empty() {
            if let Some(path) = self.explorer_root_for_current_file(cx) {
                self.add_explorer_worktree(path, cx);
            }
        }
        if self.panels.explorer.is_open {
            self.sync_explorer_models(cx);
        }
    }
    pub(crate) fn sync_explorer_models(&mut self, cx: &mut Context<Self>) {
        // The file tree only needs a root directory, so it syncs even in
        // the welcome state (no tabs). Each Editor entity syncs its own
        // outline from its active document when it renders.
        self.sync_explorer_file_tree(cx);
    }
    pub(crate) fn explorer_root_for_current_file(&self, cx: &App) -> Option<PathBuf> {
        self.active_editor_tab(cx)
            .and_then(|tab| tab.file.path.as_ref())
            .and_then(|path| path.parent().map(Path::to_path_buf))
    }
    pub(crate) fn prompt_open_explorer_folder(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: true,
            multiple: false,
            prompt: None,
        });
        let weak_editor = cx.entity().downgrade();
        cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let paths = match prompt.await {
                Ok(Ok(Some(paths))) => paths,
                Ok(Ok(None)) | Err(_) => return,
                Ok(Err(err)) => {
                    eprintln!("[explorer] dialog error: {err}");
                    return;
                }
            };
            let Some(path) = paths.into_iter().next() else {
                return;
            };
            if path.is_dir() {
                if let Err(err) = crate::infra::config::recent::record_recent_folder(&path) {
                    eprintln!("failed to update recent folder history: {err}");
                }
            }
            let _ = weak_editor.update(cx, |editor, cx| {
                editor.open_explorer_folder_path(path, cx);
                cx.notify();
            });
        })
        .detach();
    }
    pub(crate) fn collapse_all_explorer_nodes(&mut self, cx: &mut Context<Self>) {
        let explorer = &mut self.panels.explorer;
        explorer.expanded.clear();
        // Keep every worktree root row expanded (mirrors Zed's collapse-all,
        // which retains worktree roots) so the title-row buttons stay
        // visible.
        for (index, tree) in explorer.trees_cache.iter().enumerate() {
            explorer.expanded.entry(index).or_default().insert(tree.id);
        }
        self.rebuild_explorer_entries();
        cx.notify();
    }
    /// Request a full background rescan of every worktree (panel-driven
    /// disk operations call this; the worktree entities coalesce rescans
    /// while one is in flight).
    pub(crate) fn rescan_explorer_worktrees(&mut self, cx: &mut Context<Self>) {
        let worktrees = self.panels.explorer.worktrees.clone();
        for worktree in worktrees {
            worktree.update(cx, |worktree, cx| worktree.rescan(cx));
        }
    }

    pub(crate) fn refresh_explorer_tree(&mut self, cx: &mut Context<Self>) {
        self.rescan_explorer_worktrees(cx);
        self.sync_explorer_models(cx);
        cx.notify();
    }

    /// Toggle dotfile visibility. Persists to settings and rescans.
    pub(crate) fn toggle_explorer_hidden(&mut self, cx: &mut Context<Self>) {
        let mut settings = ExplorerSettingsStore::settings(cx);
        settings.hide_hidden = !settings.hide_hidden;
        ExplorerSettingsStore::set(cx, settings);
        let hide_hidden = ExplorerSettingsStore::settings(cx).hide_hidden;
        let worktrees = self.panels.explorer.worktrees.clone();
        for worktree in worktrees {
            worktree.update(cx, |worktree, cx| {
                worktree.set_hide_hidden(hide_hidden, cx);
            });
        }
        self.sync_explorer_models(cx);
        cx.notify();
    }

    /// Replace the worktree at `index` with a folder picked by the user
    /// (the root row's folder button): the old root is removed and the new
    /// one is added in its place.
    pub(crate) fn replace_explorer_worktree(
        &mut self,
        index: usize,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: true,
            multiple: false,
            prompt: None,
        });
        let weak_editor = cx.entity().downgrade();
        cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let paths = match prompt.await {
                Ok(Ok(Some(paths))) => paths,
                Ok(Ok(None)) | Err(_) => return,
                Ok(Err(err)) => {
                    eprintln!("[explorer] dialog error: {err}");
                    return;
                }
            };
            let Some(path) = paths.into_iter().next() else {
                return;
            };
            let _ = weak_editor.update(cx, |editor, cx| {
                if index < editor.panels.explorer.worktrees.len() {
                    editor.remove_explorer_worktree(index, cx);
                }
                editor.add_explorer_worktree(path, cx);
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn reveal_in_file_explorer(&self, path: &Path) {
        let path = path.to_path_buf();
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("explorer.exe")
                .arg("/select,")
                .arg(&path)
                .spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open")
                .arg("-R")
                .arg(&path)
                .spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let parent = path.parent().unwrap_or(&path);
            let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
        }
    }

    pub(crate) fn start_inline_create_file(
        &mut self,
        parent: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // The file itself is created only on confirm (mirrors Zed).
        self.begin_explorer_create(parent, false, window, cx);
    }
    pub(crate) fn start_inline_create_folder(
        &mut self,
        parent: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.expand_to_path(&parent);
        self.begin_explorer_create(parent, true, window, cx);
    }
    pub(crate) fn start_inline_rename(
        &mut self,
        target: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.begin_explorer_rename(target, window, cx);
    }
    pub(crate) fn copy_path_to_clipboard(&self, path: &Path, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(
            path.to_string_lossy().to_string(),
        ));
    }

    /// Copy the path of `path` relative to the explorer root (falls back to
    /// the absolute path when it is outside every root).
    pub(crate) fn copy_explorer_relative_path(&self, path: &Path, cx: &mut Context<Self>) {
        let relative = self
            .panels
            .explorer
            .trees_cache
            .iter()
            .find_map(|tree| path.strip_prefix(&tree.path).ok())
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        cx.write_to_clipboard(ClipboardItem::new_string(relative));
    }

    /// Open the entry with the OS default application.
    pub(crate) fn open_explorer_with_system(&self, path: &Path) {
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("cmd")
                .arg("/c")
                .arg("start")
                .arg("")
                .arg(path)
                .spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(path).spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open").arg(path).spawn();
        }
    }
}
