//! Explorer file operations: delete, trash, cut / copy / paste, duplicate,
//! and the undo / redo flow (mirrors Zed's `remove`/`cut`/`copy`/`paste`/
//! `duplicate`/`undo`/`redo`).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use gpui::*;

use crate::app::shell::Shell;
use crate::explorer::state::state::*;
use crate::explorer::state::undo::{
    ExplorerChange, execute_explorer_change, execute_explorer_change_inverse,
    explorer_change_destination,
};
use crate::explorer::state::utils::execute_entry_ops;

impl Shell {
    /// Delete the effective selection with a confirmation prompt; after the
    /// background deletion the selection moves to the next surviving sibling
    /// (Zed's trash/delete flow, without the OS-trash variant).
    pub(crate) fn delete_explorer_selections(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let selections = self.effective_explorer_entries();
        if selections.is_empty() {
            return;
        }
        let names: Vec<String> = selections
            .iter()
            .filter_map(|sel| {
                self.explorer_entry_for_selection(sel)
                    .map(|entry| entry.label.clone())
            })
            .collect();
        let summary = if names.len() == 1 {
            names[0].clone()
        } else {
            format!("{} items", names.len())
        };
        let prompt = window.prompt(
            PromptLevel::Warning,
            &format!("Are you sure you want to delete '{summary}'?"),
            None,
            &["Delete", "Cancel"],
            cx,
        );

        let weak_shell = cx.entity().downgrade();
        let _ = cx.spawn(async move |_this, cx| {
            if prompt.await != Ok(0) {
                return;
            }
            let paths: Vec<PathBuf> = weak_shell
                .update(cx, |shell, _cx| {
                    selections
                        .iter()
                        .filter_map(|sel| shell.explorer_path_for_id(sel.entry_id))
                        .collect()
                })
                .unwrap_or_default();
            cx.background_executor()
                .spawn(async move {
                    for path in &paths {
                        if let Err(err) =
                            crate::explorer::state::undo::remove_path_symlink_safe(path)
                        {
                            tracing::error!(path = %path.display(), error = %err, "failed to delete path");
                        }
                    }
                })
                .await;
            let _ = weak_shell.update(cx, |shell, cx| {
                shell.panels.explorer.marked.clear();
                shell.rescan_explorer_worktrees(cx);
                if let Some(next) = shell.next_explorer_selection_after_deletion(&selections) {
                    shell.panels.explorer.selected = Some(next);
                }
                shell.sync_explorer_models(cx);
                shell.autoscroll_explorer_selection();
                cx.notify();
            });
        });
    }

    /// Move the effective selection to the OS trash (recoverable, no
    /// confirmation — mirrors Zed's Trash menu item).
    pub(crate) fn trash_explorer_selections(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let selections = self.effective_explorer_entries();
        if selections.is_empty() {
            return;
        }
        let paths: Vec<PathBuf> = selections
            .iter()
            .filter_map(|sel| self.explorer_path_for_id(sel.entry_id))
            .collect();
        let weak_shell = cx.entity().downgrade();
        let _ = cx.spawn(async move |_this, cx| {
            cx.background_executor()
                .spawn(async move {
                    for path in &paths {
                        if let Err(err) = trash::delete(path) {
                            tracing::error!(path = %path.display(), error = %err, "failed to trash path");
                        }
                    }
                })
                .await;
            let _ = weak_shell.update(cx, |shell, cx| {
                shell.panels.explorer.marked.clear();
                shell.rescan_explorer_worktrees(cx);
                if let Some(next) = shell.next_explorer_selection_after_deletion(&selections) {
                    shell.panels.explorer.selected = Some(next);
                }
                shell.sync_explorer_models(cx);
                shell.autoscroll_explorer_selection();
                cx.notify();
            });
        });
    }

    /// Copy the effective selection: absolute paths go to the system
    /// clipboard, entry ids are remembered for in-panel paste.
    pub(crate) fn explorer_copy(&mut self, cx: &mut Context<Self>) {
        let selections = self.effective_explorer_entries();
        if selections.is_empty() {
            return;
        }
        let paths: Vec<String> = selections
            .iter()
            .filter_map(|sel| {
                self.explorer_path_for_id(sel.entry_id)
                    .map(|entry| entry.to_string_lossy().into_owned())
            })
            .collect();
        cx.write_to_clipboard(ClipboardItem::new_string(paths.join("\n")));
        let mut set = BTreeSet::new();
        set.extend(selections);
        self.panels.explorer.clipboard = Some(ExplorerClipboard::Copied(set));
        cx.notify();
    }

    /// Cut the effective selection (same dual-clipboard behavior as copy).
    pub(crate) fn explorer_cut(&mut self, cx: &mut Context<Self>) {
        let selections = self.effective_explorer_entries();
        if selections.is_empty() {
            return;
        }
        let paths: Vec<String> = selections
            .iter()
            .filter_map(|sel| {
                self.explorer_path_for_id(sel.entry_id)
                    .map(|entry| entry.to_string_lossy().into_owned())
            })
            .collect();
        cx.write_to_clipboard(ClipboardItem::new_string(paths.join("\n")));
        let mut set = BTreeSet::new();
        set.extend(selections);
        self.panels.explorer.clipboard = Some(ExplorerClipboard::Cut(set));
        cx.notify();
    }

    /// Duplicate = copy + paste (Zed's `duplicate`).
    pub(crate) fn explorer_duplicate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.explorer_copy(cx);
        self.explorer_paste(window, cx);
    }

    /// Target directory for a paste: the selected directory, the parent of a
    /// selected file, or the last worktree root.
    fn explorer_paste_target_dir(&self) -> Option<PathBuf> {
        match self.panels.explorer.selected {
            Some(sel) => {
                let path = self.explorer_path_for_id(sel.entry_id)?;
                if path.is_dir() {
                    Some(path)
                } else {
                    path.parent().map(Path::to_path_buf)
                }
            }
            _ => self.last_explorer_root_path(),
        }
    }

    /// Paste the in-panel clipboard into the target directory. Cut entries
    /// are moved (`fs::rename`), copied entries are duplicated; after the
    /// background operation the last successful result is selected, a
    /// disambiguated copy opens the inline rename editor, and a rescan is
    /// scheduled.
    pub(crate) fn explorer_paste(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(clipboard) = self.panels.explorer.clipboard.clone() else {
            return;
        };
        let Some(target_dir) = self.explorer_paste_target_dir() else {
            return;
        };
        let items: Vec<PathBuf> = clipboard
            .items()
            .iter()
            .filter_map(|selection| self.explorer_path_for_id(selection.entry_id))
            .collect();
        if items.is_empty() {
            return;
        }
        let is_cut = clipboard.is_cut();
        let weak_shell = cx.entity().downgrade();
        let window_handle = window.window_handle();
        let _ = cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let result = cx
                .background_executor()
                .spawn(async move { execute_entry_ops(&items, &target_dir, is_cut, true) })
                .await;
            let _ = weak_shell.update(cx, |shell, cx| {
                if is_cut {
                    shell.panels.explorer.clipboard = shell
                        .panels
                        .explorer
                        .clipboard
                        .take()
                        .map(ExplorerClipboard::into_copied);
                }
                shell.panels.explorer.marked.clear();
                shell.rescan_explorer_worktrees(cx);
                if result.len() > 1 {
                    shell.record_explorer_change(ExplorerChange::Batch(result.clone()));
                } else if let Some(change) = result.first() {
                    shell.record_explorer_change(change.clone());
                }
                if let Some(path) = result
                    .last()
                    .and_then(explorer_change_destination)
                    .map(Path::to_path_buf)
                {
                    if let Some(sel) = shell.explorer_id_for_path(&path) {
                        shell.panels.explorer.pending_select = Some((sel.worktree_id, path.clone()));
                    }
                    let weak_shell_for_open = weak_shell.clone();
                    let _ = cx.update_window(window_handle, move |_, _window, cx| {
                        let _ = weak_shell_for_open.update(cx, |shell, _cx| {
                            shell.expand_to_path(&path);
                            shell.rebuild_explorer_entries();
                            shell.autoscroll_explorer_selection();
                        });
                    });
                    if !is_cut
                        && result.len() == 1
                        && let Some(change) = result.first()
                        && let ExplorerChange::Copied { source, dest } = change
                        && source.file_name() != dest.file_name()
                    {
                        shell.panels.explorer.pending_rename = Some((window_handle, dest.clone()));
                    }
                }
                shell.sync_explorer_models(cx);
                cx.notify();
            });
        });
    }

    // ── Undo / redo (mirrors Zed's panel undo manager) ──────────────────

    /// Record a reversible file operation (create / rename / move / copy).
    pub(crate) fn record_explorer_change(&mut self, change: ExplorerChange) {
        self.panels.explorer.undo_history.record(change);
    }

    /// Undo the most recent file operation, then rescan.
    pub(crate) fn explorer_undo(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(change) = self.panels.explorer.undo_history.undo_stack.pop() else {
            return;
        };
        let weak_shell = cx.entity().downgrade();
        let change_for_execution = change.clone();
        let _ = cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let result = cx.background_executor()
                .spawn(async move { execute_explorer_change_inverse(&change_for_execution) })
                .await;
            if let Err(err) = result {
                tracing::error!(error = %err, "failed to execute explorer undo");
            }
            let _ = weak_shell.update(cx, |shell, cx| {
                shell.panels.explorer.undo_history.redo_stack.push(change);
                shell.rescan_explorer_worktrees(cx);
                shell.sync_explorer_models(cx);
                cx.notify();
            });
        });
    }

    /// Redo the most recently undone operation, then rescan.
    pub(crate) fn explorer_redo(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(change) = self.panels.explorer.undo_history.redo_stack.pop() else {
            return;
        };
        let weak_shell = cx.entity().downgrade();
        let change_for_execution = change.clone();
        let _ = cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let result = cx.background_executor()
                .spawn(async move { execute_explorer_change(&change_for_execution) })
                .await;
            if let Err(err) = result {
                tracing::error!(error = %err, "failed to execute explorer redo");
            }
            let _ = weak_shell.update(cx, |shell, cx| {
                shell.panels.explorer.undo_history.undo_stack.push(change);
                shell.rescan_explorer_worktrees(cx);
                shell.sync_explorer_models(cx);
                cx.notify();
            });
        });
    }
}
