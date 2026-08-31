//! Explorer file operations: delete, trash, cut / copy / paste, duplicate,
//! and the undo / redo flow (mirrors Zed's `remove`/`cut`/`copy`/`paste`/
//! `duplicate`/`undo`/`redo`).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use gpui::*;

use crate::state::state::*;
use crate::state::undo::{
    ExplorerChange, execute_explorer_change, execute_explorer_change_inverse,
    explorer_change_destination,
};
use crate::state::utils::execute_entry_ops;

impl ExplorerState {
    /// Delete the effective selection with a confirmation prompt; after the
    /// background deletion the selection moves to the next surviving sibling
    /// (Zed's trash/delete flow, without the OS-trash variant).
    pub(crate) fn delete_explorer_selections(&mut self, window: &mut Window, cx: &mut App) {
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
        let weak = self.self_weak.clone();
        let _ = cx.spawn(async move |cx| {
            if prompt.await != Ok(0) {
                return;
            }
            let paths: Vec<PathBuf> = cx.update(|cx| {
                weak.update(cx, |state, _cx| {
                    selections
                        .iter()
                        .filter_map(|sel| state.explorer_path_for_id(sel.entry_id))
                        .collect::<Vec<PathBuf>>()
                })
                .unwrap_or_default()
            });
            cx.background_executor()
                .spawn(async move {
                    for path in &paths {
                        if let Err(err) = crate::fs::remove_symlink_safe(path) {
                            tracing::error!(path = %path.display(), error = %err, "failed to delete path");
                        }
                    }
                })
                .await;
            cx.update(|cx| {
                let _ = weak.update(cx, |state, cx| {
                    state.marked.clear();
                    state.rescan_explorer_worktrees(cx);
                    if let Some(next) = state.next_explorer_selection_after_deletion(&selections) {
                        state.selected = Some(next);
                    }
                    state.sync_explorer_models(cx);
                    state.autoscroll_explorer_selection();
                    cx.refresh_windows();
                });
            });
        });
    }

    /// Move the effective selection to the OS trash (recoverable, no
    /// confirmation — mirrors Zed's Trash menu item).
    pub(crate) fn trash_explorer_selections(&mut self, _window: &mut Window, cx: &mut App) {
        let selections = self.effective_explorer_entries();
        if selections.is_empty() {
            return;
        }
        let paths: Vec<PathBuf> = selections
            .iter()
            .filter_map(|sel| self.explorer_path_for_id(sel.entry_id))
            .collect();
        let weak = self.self_weak.clone();
        let _ = cx.spawn(async move |cx| {
            cx.background_executor()
                .spawn(async move {
                    for path in &paths {
                        if let Err(err) = crate::fs::trash(path) {
                            tracing::error!(path = %path.display(), error = %err, "failed to trash path");
                        }
                    }
                })
                .await;
            cx.update(|cx| {
                let _ = weak.update(cx, |state, cx| {
                    state.marked.clear();
                    state.rescan_explorer_worktrees(cx);
                    if let Some(next) = state.next_explorer_selection_after_deletion(&selections) {
                        state.selected = Some(next);
                    }
                    state.sync_explorer_models(cx);
                    state.autoscroll_explorer_selection();
                    cx.refresh_windows();
                });
            });
        });
    }

    /// Copy the effective selection: absolute paths go to the system
    /// clipboard, entry ids are remembered for in-panel paste.
    pub(crate) fn explorer_copy(&mut self, cx: &mut App) {
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
        self.clipboard = Some(ExplorerClipboard::Copied(set));
        cx.refresh_windows();
    }

    /// Cut the effective selection (same dual-clipboard behavior as copy).
    pub(crate) fn explorer_cut(&mut self, cx: &mut App) {
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
        self.clipboard = Some(ExplorerClipboard::Cut(set));
        cx.refresh_windows();
    }

    /// Duplicate = copy + paste (Zed's `duplicate`).
    pub(crate) fn explorer_duplicate(&mut self, window: &mut Window, cx: &mut App) {
        self.explorer_copy(cx);
        self.explorer_paste(window, cx);
    }

    /// Target directory for a paste: the selected directory, the parent of a
    /// selected file, or the last worktree root.
    fn explorer_paste_target_dir(&self) -> Option<PathBuf> {
        match self.selected {
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
    pub(crate) fn explorer_paste(&mut self, window: &mut Window, cx: &mut App) {
        let Some(clipboard) = self.clipboard.clone() else {
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
        let window_handle = window.window_handle();
        let weak = self.self_weak.clone();
        let _ = cx.spawn(async move |cx: &mut AsyncApp| {
            let result = cx
                .background_executor()
                .spawn(async move { execute_entry_ops(&items, &target_dir, is_cut, true) })
                .await;
            cx.update(|cx| {
                let _ = weak.update(cx, |state, cx| {
                    if is_cut {
                        state.clipboard =
                            state.clipboard.take().map(ExplorerClipboard::into_copied);
                    }
                    state.marked.clear();
                    state.rescan_explorer_worktrees(cx);
                    if result.len() > 1 {
                        state.record_explorer_change(ExplorerChange::Batch(result.clone()));
                    } else if let Some(change) = result.first() {
                        state.record_explorer_change(change.clone());
                    }
                    if let Some(path) = result
                        .last()
                        .and_then(explorer_change_destination)
                        .map(Path::to_path_buf)
                    {
                        if let Some(sel) = state.explorer_id_for_path(&path) {
                            state.pending_select = Some((sel.worktree_id, path.clone()));
                        }
                        if !is_cut
                            && result.len() == 1
                            && let Some(change) = result.first()
                            && let ExplorerChange::Copied { source, dest } = change
                            && source.file_name() != dest.file_name()
                        {
                            state.pending_rename = Some((window_handle, dest.clone()));
                        }
                    }
                    state.sync_explorer_models(cx);
                    cx.refresh_windows();
                });
                // Window-scoped work must run after the state lease above
                // ends (nested entity leases panic).
                let _ = cx.update_window(window_handle, {
                    let weak = weak.clone();
                    move |_, _window, cx| {
                        let _ = weak.update(cx, |state, _cx| {
                            if let Some(path) = result
                                .last()
                                .and_then(explorer_change_destination)
                                .map(Path::to_path_buf)
                            {
                                state.expand_to_path(&path);
                                state.rebuild_explorer_entries();
                                state.autoscroll_explorer_selection();
                            }
                        });
                    }
                });
            });
        });
    }

    // ── Undo / redo (mirrors Zed's panel undo manager) ──────────────────

    /// Record a reversible file operation (create / rename / move / copy).
    pub(crate) fn record_explorer_change(&mut self, change: ExplorerChange) {
        self.undo_history.record(change);
    }

    /// Undo the most recent file operation, then rescan.
    pub(crate) fn explorer_undo(&mut self, _window: &mut Window, cx: &mut App) {
        let Some(change) = self.undo_history.undo_stack.pop() else {
            return;
        };
        let change_for_execution = change.clone();
        let weak = self.self_weak.clone();
        let _ = cx.spawn(async move |cx: &mut AsyncApp| {
            let result = cx
                .background_executor()
                .spawn(async move { execute_explorer_change_inverse(&change_for_execution) })
                .await;
            if let Err(err) = result {
                tracing::error!(error = %err, "failed to execute explorer undo");
            }
            cx.update(|cx| {
                let _ = weak.update(cx, |state, cx| {
                    state.undo_history.redo_stack.push(change);
                    state.rescan_explorer_worktrees(cx);
                    state.sync_explorer_models(cx);
                    cx.refresh_windows();
                });
            });
        });
    }

    /// Redo the most recently undone operation, then rescan.
    pub(crate) fn explorer_redo(&mut self, _window: &mut Window, cx: &mut App) {
        let Some(change) = self.undo_history.redo_stack.pop() else {
            return;
        };
        let change_for_execution = change.clone();
        let weak = self.self_weak.clone();
        let _ = cx.spawn(async move |cx: &mut AsyncApp| {
            let result = cx
                .background_executor()
                .spawn(async move { execute_explorer_change(&change_for_execution) })
                .await;
            if let Err(err) = result {
                tracing::error!(error = %err, "failed to execute explorer redo");
            }
            cx.update(|cx| {
                let _ = weak.update(cx, |state, cx| {
                    state.undo_history.undo_stack.push(change);
                    state.rescan_explorer_worktrees(cx);
                    state.sync_explorer_models(cx);
                    cx.refresh_windows();
                });
            });
        });
    }
}
