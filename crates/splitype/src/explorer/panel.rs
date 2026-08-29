//! Explorer panel lifecycle: drawer visibility, worktree management
//! (add / remove / reorder), scan events, settings sync and path helpers.

use std::path::{Path, PathBuf};

use gpui::*;

use crate::app::shell::Shell;

use crate::app::actions::{CloseExplorerFolder, ToggleExplorer};
use crate::explorer::state::state::*;
use crate::explorer::state::worktree::Worktree;
use config::settings::SettingsStore;

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
        explorer.snapshots.clear();
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
        explorer.refresh_recent_cache();
        self.rebuild_explorer_entries();
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
        let worktree_id = WorktreeId(explorer.worktrees.len());
        let hide_hidden = SettingsStore::settings(cx).explorer.hide_hidden;
        let worktree = Worktree::new(
            worktree_id,
            path.clone(),
            explorer.next_entry_id.clone(),
            hide_hidden,
            cx,
        );
        // The root row starts expanded (VSCode-style title row visible).
        let root_id = worktree.read(cx).root_id();
        explorer
            .expanded
            .entry(worktree_id)
            .or_default()
            .insert(root_id);
        cx.subscribe(&worktree, Self::on_explorer_worktree_event)
            .detach();
        explorer.worktrees.push(worktree);
        explorer.snapshots = explorer
            .worktrees
            .iter()
            .map(|wt| wt.read(cx).snapshot())
            .collect();
        explorer.file_error = None;
        self.rebuild_explorer_entries();
        cx.notify();
    }

    /// Remove the worktree at `index` (mirrors Zed's `remove_worktree`);
    /// wired to the root row's "Remove from Explorer" context menu item.
    pub(crate) fn remove_explorer_worktree(&mut self, index: usize, cx: &mut Context<Self>) {
        let explorer = &mut self.panels.explorer;
        if index >= explorer.worktrees.len() {
            return;
        }
        let removed_wt = explorer.worktrees.remove(index);
        let removed_id = removed_wt.read(cx).id();
        explorer.expanded.remove(&removed_id);
        if let Some(sel) = explorer.selected {
            if sel.worktree_id == removed_id {
                explorer.selected = None;
            }
        }
        explorer.marked.retain(|sel| sel.worktree_id != removed_id);
        explorer.snapshots = explorer
            .worktrees
            .iter()
            .map(|wt| wt.read(cx).snapshot())
            .collect();
        explorer.edit = None;
        explorer.pending_select = None;
        self.rebuild_explorer_entries();
        cx.notify();
    }

    /// Reorder worktrees by dragging a root row onto another root
    /// (mirrors Zed's `Project::move_worktree`): the dragged root ends up
    /// directly before the drop-target root.
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
        explorer.snapshots = explorer
            .worktrees
            .iter()
            .map(|wt| wt.read(cx).snapshot())
            .collect();
        explorer.edit = None;
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
        self.panels.explorer.snapshots = self
            .panels
            .explorer
            .worktrees
            .iter()
            .map(|wt| wt.read(cx).snapshot())
            .collect();
        self.select_active_file_in_tree(true, cx);
        self.rebuild_explorer_entries();
        self.autoscroll_explorer_selection();
        // Start the pending rename editor now that the copied entry is in
        // the scanned tree (the editor needs window access, so it runs in a
        // spawned task).
        if let Some((window_handle, path)) = self.panels.explorer.pending_rename.take()
            && self.explorer_id_for_path(&path).is_some()
        {
            let weak_editor = cx.entity().downgrade();
            cx.spawn(async move |_this, cx: &mut AsyncApp| {
                let _ = cx.update_window(window_handle, |_, window, cx| {
                    let _ = weak_editor.update(cx, |editor, cx| {
                        editor.begin_inline_rename(path.to_path_buf(), window, cx);

                    });
                });
            })
            .detach();
        }
        cx.notify();
    }

    pub(crate) fn open_explorer_folder_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.add_explorer_worktree(path, cx);
    }
    pub(crate) fn sync_explorer_after_document_path_change(&mut self, cx: &mut Context<Self>) {
        if self.panels.explorer.is_open && !self.panels.explorer.worktrees.is_empty() {
            self.sync_explorer_models(cx);
        }
    }
    pub(crate) fn sync_explorer_models(&mut self, cx: &mut Context<Self>) {
        // The file tree only needs a root directory, so it syncs even in
        // the welcome state (no tabs). Each Editor entity syncs its own
        // outline from its active document when it renders.
        self.sync_explorer_file_tree(cx);
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
        let weak_shell = cx.entity().downgrade();
        cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let paths = match prompt.await {
                Ok(Ok(Some(paths))) => paths,
                Ok(Ok(None)) | Err(_) => return,
                Ok(Err(err)) => {
                    tracing::error!(error = %err, "[explorer] open folder dialog error");
                    return;
                }
            };
            let Some(path) = paths.into_iter().next() else {
                return;
            };
            if path.is_dir()
                && let Err(err) = config::recent::record_recent_folder(&path)
            {
                tracing::warn!(path = %path.display(), error = %err, "failed to update recent folder history");
            }
            let _ = weak_shell.update(cx, |shell, cx| {
                shell.open_explorer_folder_path(path, cx);
                cx.notify();
            });
        })
        .detach();
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

    pub(crate) fn rescan_and_sync_explorer(&mut self, cx: &mut Context<Self>) {
        self.rescan_explorer_worktrees(cx);
        self.sync_explorer_models(cx);
        cx.notify();
    }

    /// Toggle dotfile visibility. Persists to settings and rescans.
    pub(crate) fn toggle_explorer_hidden(&mut self, cx: &mut Context<Self>) {
        let _ = SettingsStore::update(cx, |s| {
            s.explorer.hide_hidden = !s.explorer.hide_hidden;
        });
        let hide_hidden = SettingsStore::settings(cx).explorer.hide_hidden;
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
        let weak_shell = cx.entity().downgrade();
        cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let paths = match prompt.await {
                Ok(Ok(Some(paths))) => paths,
                Ok(Ok(None)) | Err(_) => return,
                Ok(Err(err)) => {
                    tracing::error!(error = %err, "[explorer] replace folder dialog error");
                    return;
                }
            };
            let Some(path) = paths.into_iter().next() else {
                return;
            };
            let _ = weak_shell.update(cx, |shell, cx| {
                if index < shell.panels.explorer.worktrees.len() {
                    shell.remove_explorer_worktree(index, cx);
                }
                shell.add_explorer_worktree(path, cx);
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn reveal_in_file_explorer(&self, path: &Path) {
        #[cfg(target_os = "windows")]
        {
            let path_str = path.to_string_lossy().replace('/', "\\");
            let _ = std::process::Command::new("explorer.exe")
                .arg(format!("/select,{}", path_str))
                .spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open")
                .arg("-R")
                .arg(path)
                .spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let parent = path.parent().unwrap_or(path);
            let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
        }
    }

    pub(crate) fn open_in_terminal(&self, path: &Path) {
        let dir = if path.is_dir() {
            path.to_path_buf()
        } else {
            path.parent().map(Path::to_path_buf).unwrap_or_else(|| path.to_path_buf())
        };
        #[cfg(target_os = "windows")]
        {
            let dir_str = dir.to_string_lossy().replace('/', "\\");
            let _ = std::process::Command::new("wt.exe")
                .arg("-d")
                .arg(&dir_str)
                .spawn()
                .or_else(|_| {
                    std::process::Command::new("powershell.exe")
                        .current_dir(&dir)
                        .spawn()
                })
                .or_else(|_| {
                    std::process::Command::new("cmd.exe")
                        .arg("/c")
                        .arg("start")
                        .arg("cmd.exe")
                        .current_dir(&dir)
                        .spawn()
                });
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open")
                .arg("-a")
                .arg("Terminal")
                .arg(&dir)
                .spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("x-terminal-emulator")
                .current_dir(&dir)
                .spawn();
        }
    }

    pub(crate) fn begin_inline_create_file(
        &mut self,
        parent: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.begin_explorer_create(parent, false, window, cx);
    }
    pub(crate) fn begin_inline_create_folder(
        &mut self,
        parent: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.expand_to_path(&parent);
        self.begin_explorer_create(parent, true, window, cx);
    }
    pub(crate) fn begin_inline_rename(
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
            .worktrees
            .iter()
            .find_map(|wt| path.strip_prefix(wt.read(cx).root()).ok())
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        cx.write_to_clipboard(ClipboardItem::new_string(relative));
    }

    /// Open the entry with the OS default application.
    pub(crate) fn open_explorer_with_system(&self, path: &Path) {
        #[cfg(target_os = "windows")]
        {
            let path_str = path.to_string_lossy().replace('/', "\\");
            let _ = std::process::Command::new("cmd")
                .arg("/c")
                .arg("start")
                .arg("")
                .arg(path_str)
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
