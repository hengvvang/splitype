//! Inline filename editor operations: create, rename, validation, and lifecycle.

use std::path::PathBuf;

use gpui::*;

use crate::state::ExplorerState;
use crate::state::undo::ExplorerChange;
use crate::state::{ExplorerEditState, ExplorerRow, ExplorerValidation, SelectedEntry};
use platform_contracts::actions::DismissTransientUi;

impl ExplorerState {
    /// Real-time validation of the inline filename (mirrors Zed's
    /// `populate_validation_error`): whitespace warns, illegal characters and
    /// name collisions error.
    pub(crate) fn populate_explorer_validation(&mut self, cx: &mut App) {
        let Some(edit) = self.edit.as_ref() else {
            return;
        };
        let filename = self.filename_editor().read(cx).text().to_string();
        let _is_rename = edit.target_id.is_some();

        let validation = if filename.trim() != filename {
            Some(ExplorerValidation::Warning(
                "File name has leading or trailing whitespace.".into(),
            ))
        } else if filename.contains('\0') {
            Some(ExplorerValidation::Error(
                "File name cannot contain null characters.".into(),
            ))
        } else if filename.contains([':', '*', '?', '"', '<', '>', '|']) {
            Some(ExplorerValidation::Error(
                "File name contains illegal characters (: * ? \" < > |).".into(),
            ))
        } else if filename
            .split(['/', '\\'])
            .any(|segment| segment == "." || segment == "..")
        {
            Some(ExplorerValidation::Error(
                "Path components cannot be '.' or '..'.".into(),
            ))
        } else {
            self.explorer_duplicate_name_error(&filename, cx)
        };

        if let Some(edit) = self.edit.as_mut() {
            edit.validation = validation;
        }
    }

    fn explorer_duplicate_name_error(
        &self,
        filename: &str,
        _cx: &App,
    ) -> Option<ExplorerValidation> {
        let edit = self.edit.as_ref()?;
        let snapshot = self
            .snapshots
            .iter()
            .find(|snap| snap.id() == edit.worktree_id)?;
        let trimmed = filename.trim();
        if trimmed.is_empty() {
            return None;
        }
        // New entry: check the target path. Rename: check except itself.
        let new_path = if edit.target_id.is_none() {
            let relative_parts = trimmed.split(['/', '\\']).filter(|s| !s.is_empty());
            let mut path = edit.path.clone();
            for part in relative_parts {
                path.push(part);
            }
            path
        } else {
            let parent = edit.path.parent()?;
            let relative_parts = trimmed.split(['/', '\\']).filter(|s| !s.is_empty());
            let mut path = parent.to_path_buf();
            for part in relative_parts {
                path.push(part);
            }
            path
        };
        let existing = snapshot.entry_for_path(&new_path);
        let is_self = edit
            .target_id
            .is_some_and(|id| existing.is_some_and(|entry| entry.id == id));
        if existing.is_some() && !is_self {
            Some(ExplorerValidation::Error(format!(
                "File or directory '{trimmed}' already exists at this location."
            )))
        } else {
            None
        }
    }

    /// Start creating a new entry inside `parent` (a directory path).
    /// If the parent directory was previously collapsed, records it as
    /// `temporarily_unfolded` so it can be automatically re-folded on cancel (mirrors Zed).
    pub(crate) fn begin_explorer_create(
        &mut self,
        parent: PathBuf,
        is_dir: bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        let (worktree_id, parent_id) = self
            .explorer_id_for_path(&parent)
            .map(|sel| (sel.worktree_id, Some(sel.entry_id)))
            .unwrap_or_else(|| {
                let last_id = self
                    .snapshots
                    .last()
                    .map(|snap| snap.id())
                    .unwrap_or(crate::state::worktree::WorktreeId(0));
                (last_id, None)
            });

        let was_expanded = parent_id.is_some_and(|pid| {
            self.expanded
                .get(&worktree_id)
                .is_some_and(|set| set.contains(&pid))
        });
        let temporarily_unfolded = if !was_expanded {
            parent_id.map(|pid| (worktree_id, pid))
        } else {
            None
        };

        // Reveal the parent directory and rebuild flat list
        self.expand_to_path(&parent);
        self.rebuild_explorer_entries();

        let depth = match parent_id {
            Some(parent_id) => self
                .entries
                .iter()
                .find(|row| matches!(row, ExplorerRow::Entry(entry) if entry.id == parent_id))
                .map(|row| match row {
                    ExplorerRow::Entry(entry) => entry.depth + 1,
                    ExplorerRow::Edit { .. } => 0,
                })
                .unwrap_or(1),
            None => 1,
        };

        let editor = self.filename_editor().clone();
        editor.update(cx, |ed, _cx| {
            ed.clear();
        });

        self.begin_explorer_edit_inner(
            ExplorerEditState {
                worktree_id,
                parent_id,
                target_id: None,
                is_dir,
                depth,
                path: parent,
                validation: None,
                previously_selected: self.selected,
                processing: false,
                processing_filename: None,
                temporarily_unfolded,
            },
            window,
            cx,
        );
    }

    /// Start renaming the entry at `target_path`.
    /// On Windows, renaming worktree root is protected and forbidden (mirrors Zed).
    pub(crate) fn begin_explorer_rename(
        &mut self,
        target_path: PathBuf,
        window: &mut Window,
        cx: &mut App,
    ) {
        #[cfg(target_os = "windows")]
        if self
            .snapshots
            .iter()
            .any(|snap| snap.root_entry().is_some_and(|e| e.path == target_path))
        {
            return;
        }

        self.expand_to_path(&target_path);
        self.rebuild_explorer_entries();

        let Some(sel) = self.explorer_id_for_path(&target_path) else {
            return;
        };
        let Some(snapshot) = self
            .snapshots
            .iter()
            .find(|snap| snap.id() == sel.worktree_id)
        else {
            return;
        };
        let Some(entry) = snapshot.entry_for_id(sel.entry_id) else {
            return;
        };
        let depth = self
            .entries
            .iter()
            .find(|row| matches!(row, ExplorerRow::Entry(e) if e.id == sel.entry_id))
            .map(|row| match row {
                ExplorerRow::Entry(e) => e.depth,
                ExplorerRow::Edit { .. } => 0,
            })
            .unwrap_or(0);

        let file_name = entry
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        // Directories select whole name; files select only the stem (mirrors Zed)
        let selection_end = if entry.kind == crate::state::worktree::WorktreeEntryKind::Directory {
            file_name.len()
        } else {
            entry
                .path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.len())
                .unwrap_or(file_name.len())
        };

        let editor = self.filename_editor().clone();
        editor.update(cx, |ed, _cx| {
            ed.set_text(file_name, Some(0..selection_end));
        });

        self.begin_explorer_edit_inner(
            ExplorerEditState {
                worktree_id: sel.worktree_id,
                parent_id: None,
                target_id: Some(sel.entry_id),
                is_dir: entry.kind == crate::state::worktree::WorktreeEntryKind::Directory,
                depth,
                path: target_path,
                validation: None,
                previously_selected: self.selected,
                processing: false,
                processing_filename: None,
                temporarily_unfolded: None,
            },
            window,
            cx,
        );
    }

    fn begin_explorer_edit_inner(
        &mut self,
        edit: ExplorerEditState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let edited_selection = if let Some(target_id) = edit.target_id {
            SelectedEntry {
                worktree_id: edit.worktree_id,
                entry_id: target_id,
            }
        } else {
            SelectedEntry {
                worktree_id: edit.worktree_id,
                entry_id: crate::state::worktree::NEW_ENTRY_ID,
            }
        };
        self.selected = Some(edited_selection);
        self.marked.clear();
        self.edit = Some(edit);
        self.rebuild_explorer_entries();
        self.autoscroll_explorer_edit(Some(window), cx);

        let editor = self.filename_editor().clone();
        editor.update(cx, |ed, cx| {
            ed.focus(window, cx);
        });

        cx.refresh_windows();
    }

    /// Scroll the edit row into view and keep it visible while typing.
    pub(crate) fn autoscroll_explorer_edit(&mut self, _window: Option<&mut Window>, _cx: &mut App) {
        let Some(index) = self.explorer_edit_row_index() else {
            return;
        };
        self.scroll_handle
            .scroll_to_item(index, ScrollStrategy::Center);
    }

    /// Row index of the inline edit row in the flat list, if any.
    pub(crate) fn explorer_edit_row_index(&self) -> Option<usize> {
        self.edit.as_ref().and_then(|_| {
            self.entries
                .iter()
                .position(|row| matches!(row, ExplorerRow::Edit { .. }))
        })
    }

    /// Commit the inline create/rename: writes to disk on a background
    /// thread, then refreshes the tree and selects the new entry. On failure
    /// the edit stays open with the error surfaced.
    ///
    /// Returns `true` when a commit is in progress (or was already), `false`
    /// when nothing was submitted (empty name, duplicate, or missing edit).
    pub(crate) fn confirm_explorer_edit(
        &mut self,
        mut window: Option<&mut Window>,
        cx: &mut App,
    ) -> bool {
        let Some(edit) = self.edit.as_ref() else {
            return false;
        };
        if edit.processing {
            return true;
        }

        let mut filename = self.filename_editor().read(cx).text().trim().to_string();

        // On Windows, trailing dots are ignored by the filesystem; strip them upfront (mirrors Zed)
        #[cfg(target_os = "windows")]
        while let Some(trimmed) = filename.strip_suffix('.') {
            filename = trimmed.to_string();
        }

        if filename.is_empty() {
            return false;
        }

        // Re-check duplicate names at commit time.
        if self.explorer_duplicate_name_error(&filename, cx).is_some() {
            self.populate_explorer_validation(cx);
            cx.refresh_windows();
            return false;
        }

        let is_create = edit.target_id.is_none();
        let has_trailing_slash = filename.ends_with('/') || filename.ends_with('\\');
        let is_dir = edit.is_dir || (is_create && has_trailing_slash);
        let worktree_id = edit.worktree_id;
        let old_path = edit.path.clone();
        let new_path = if is_create {
            let relative_parts = filename.split(['/', '\\']).filter(|s| !s.is_empty());
            let mut path = edit.path.clone();
            for part in relative_parts {
                path.push(part);
            }
            path
        } else {
            edit.path
                .parent()
                .map(|parent| parent.join(&filename))
                .unwrap_or_else(|| edit.path.clone())
        };

        // If an existing entry was not renamed, cleanly dismiss the edit (mirrors Zed).
        if !is_create && old_path == new_path {
            self.discard_explorer_edit(window, cx);
            return true;
        }

        let missing_dirs =
            if let Some(snapshot) = self.snapshots.iter().find(|snap| snap.id() == worktree_id) {
                crate::state::worktree::missing_parent_dirs(snapshot, &new_path)
            } else {
                Vec::new()
            };

        {
            let edit = self.edit.as_mut().unwrap();
            edit.processing = true;
            edit.processing_filename = Some(filename.clone());
        }

        if let (Some(window), Some(focus_handle)) = (window.as_mut(), &self.focus_handle) {
            window.focus(focus_handle, cx);
        }

        let window_handle = window.map(|w| w.window_handle());
        let new_path_for_update = new_path.clone();
        let old_path_for_record = old_path.clone();
        let weak = self.self_weak.clone();

        cx.spawn(async move |cx: &mut AsyncApp| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    if is_create {
                        if is_dir {
                            if new_path.exists() {
                                Err("A folder with this name already exists".to_string())
                            } else {
                                crate::fs::create_dir_all(&new_path).map_err(|err| err.to_string())
                            }
                        } else {
                            crate::fs::create_new_file(&new_path).map_err(|err| {
                                if let crate::fs::FsError::WriteFailed { source, .. } = &err
                                    && source.kind() == std::io::ErrorKind::AlreadyExists
                                {
                                    "A file with this name already exists".to_string()
                                } else {
                                    err.to_string()
                                }
                            })
                        }
                    } else {
                        if let Some(parent) = new_path.parent() {
                            crate::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
                        }
                        crate::fs::rename(&old_path, &new_path).map_err(|err| err.to_string())
                    }
                })
                .await;

            cx.update(|cx| {
                let path_for_open = weak.update(cx, |state, cx| {
                    match result {
                        Ok(()) => {
                            // Record the operation for panel undo/redo.
                            let change = if is_create {
                                if !missing_dirs.is_empty() {
                                    let mut batch = Vec::new();
                                    for dir in missing_dirs.into_iter().rev() {
                                        batch.push(ExplorerChange::DirCreated(dir));
                                    }
                                    batch.push(ExplorerChange::Created {
                                        path: new_path_for_update.clone(),
                                        is_dir,
                                        trashed_backup: None,
                                    });
                                    ExplorerChange::Batch(batch)
                                } else {
                                    ExplorerChange::Created {
                                        path: new_path_for_update.clone(),
                                        is_dir,
                                        trashed_backup: None,
                                    }
                                }
                            } else {
                                if !missing_dirs.is_empty() {
                                    let mut batch = Vec::new();
                                    for dir in missing_dirs.into_iter().rev() {
                                        batch.push(ExplorerChange::DirCreated(dir));
                                    }
                                    batch.push(ExplorerChange::Renamed {
                                        from: old_path_for_record,
                                        to: new_path_for_update.clone(),
                                    });
                                    ExplorerChange::Batch(batch)
                                } else {
                                    ExplorerChange::Renamed {
                                        from: old_path_for_record,
                                        to: new_path_for_update.clone(),
                                    }
                                }
                            };
                            state.record_explorer_change(change);

                            // Dismiss edit immediately on successful creation/rename (mirrors Zed)
                            state.edit = None;
                            state.pending_select = Some((worktree_id, new_path_for_update.clone()));
                            state.expand_to_path(&new_path_for_update);
                            state.rescan_explorer_worktrees(cx);
                            state.sync_explorer_models(cx);

                            if is_create && !is_dir {
                                Some(new_path_for_update.clone())
                            } else {
                                None
                            }
                        }
                        Err(err) => {
                            if let Some(edit) = state.edit.as_mut() {
                                edit.processing = false;
                                edit.processing_filename = None;
                                edit.validation = Some(ExplorerValidation::Error(err));
                            }
                            None
                        }
                    }
                });

                // Auto-open newly created file in active workspace (mirrors Zed)
                if let Some(path_for_open) = path_for_open.ok().flatten()
                    && let Some(window_handle) = window_handle
                {
                    let _ = cx.update_window(window_handle, {
                        let weak = weak.clone();
                        move |_, window, cx| {
                            let _ = weak.update(cx, |state, cx| {
                                state.open_explorer_file(path_for_open, true, window, cx);
                            });
                        }
                    });
                }
                cx.refresh_windows();
            });
        })
        .detach();
        true
    }

    /// Cancel the inline edit, restoring the previous selection.
    /// If a folder was temporarily unfolded, automatically re-folds it (mirrors Zed).
    pub(crate) fn discard_explorer_edit(&mut self, window: Option<&mut Window>, cx: &mut App) {
        let Some(edit) = self.edit.take() else {
            return;
        };

        // Re-fold folder if it was temporarily unfolded
        if let Some((worktree_id, parent_id)) = edit.temporarily_unfolded {
            if let Some(expanded_set) = self.expanded.get_mut(&worktree_id) {
                expanded_set.remove(&parent_id);
            }
        }

        self.selected = edit.previously_selected;
        self.rebuild_explorer_entries();

        if let (Some(window), Some(focus_handle)) = (window, &self.focus_handle) {
            window.focus(focus_handle, cx);
        }
        cx.refresh_windows();
    }

    /// Esc during an inline edit cancels it.
    pub(crate) fn on_explorer_escape(
        &mut self,
        _: &DismissTransientUi,
        window: &mut Window,
        cx: &mut App,
    ) {
        if self.edit.is_some() {
            self.discard_explorer_edit(Some(window), cx);
            cx.stop_propagation();
        }
    }
}
