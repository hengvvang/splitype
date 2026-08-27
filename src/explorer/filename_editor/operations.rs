//! Inline filename editor operations: create, rename, validation, and key handlers.

use std::path::PathBuf;

use gpui::*;

use crate::app::shell::Shell;
use crate::editor::input::actions::{Copy, Cut, DismissTransientUi, Paste};
use crate::explorer::state::state::{
    ExplorerEditState, ExplorerFilenameEditor, ExplorerRow, ExplorerValidation,
};
use crate::explorer::state::undo::ExplorerChange;

impl Shell {
    /// Real-time validation of the inline filename (mirrors Zed's
    /// `populate_validation_error`): whitespace warns, illegal characters and
    /// name collisions error.
    pub(crate) fn populate_explorer_validation(&mut self, cx: &mut Context<Self>) {
        let Some(edit) = self.panels.explorer.edit.as_ref() else {
            return;
        };
        let filename = edit.filename.text.clone();
        let is_rename = edit.target_id.is_some();

        let validation = if filename.trim() != filename {
            Some(ExplorerValidation::Warning(
                "File name has leading or trailing whitespace.".into(),
            ))
        } else if filename.contains('\0') {
            Some(ExplorerValidation::Error(
                "File name cannot contain null characters.".into(),
            ))
        } else if is_rename && filename.contains(['/', '\\']) {
            Some(ExplorerValidation::Error(
                "Rename target cannot contain '/' or '\\'.".into(),
            ))
        } else if filename.contains([':', '*', '?', '"', '<', '>', '|']) {
            Some(ExplorerValidation::Error(
                "File name contains illegal characters (: * ? \" < > |).".into(),
            ))
        } else if !is_rename
            && filename
                .split(['/', '\\'])
                .any(|segment| segment == "." || segment == "..")
        {
            Some(ExplorerValidation::Error(
                "Path components cannot be '.' or '..'.".into(),
            ))
        } else {
            self.explorer_duplicate_name_error(&filename, cx)
        };

        if let Some(edit) = self.panels.explorer.edit.as_mut() {
            edit.validation = validation;
        }
    }

    fn explorer_duplicate_name_error(
        &self,
        filename: &str,
        _cx: &App,
    ) -> Option<ExplorerValidation> {
        let edit = self.panels.explorer.edit.as_ref()?;
        let Some(tree) = self.panels.explorer.trees_cache.get(edit.root) else {
            return None;
        };
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
            parent.join(trimmed)
        };
        let existing = crate::explorer::state::state::find_explorer_node(tree, &new_path);
        let is_self = edit
            .target_id
            .is_some_and(|id| existing.is_some_and(|node| node.id == id));
        if existing.is_some() && !is_self {
            Some(ExplorerValidation::Error(format!(
                "File or directory '{trimmed}' already exists at this location."
            )))
        } else {
            None
        }
    }

    /// Start creating a new entry inside `parent` (a directory path). The
    /// entry is created on disk only when the edit is confirmed.
    pub(crate) fn begin_explorer_create(
        &mut self,
        parent: PathBuf,
        is_dir: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Reveal the parent directory and rebuild the flat list BEFORE
        // computing the edit-row depth: the expanded parent row must exist
        // in `entries`, otherwise the edit row lands at depth 0 (or, worse,
        // in front of the root row).
        self.expand_to_path(&parent);
        self.rebuild_explorer_entries();

        // Locate the parent in its worktree; when the path is not part of
        // any cached tree yet, fall back to the last worktree (root-level
        // create — the edit row is inserted after that root row).
        let (root, parent_id) = self
            .explorer_id_for_path(&parent)
            .map(|(root, id)| (root, Some(id)))
            .unwrap_or_else(|| (self.panels.explorer.worktrees.len().saturating_sub(1), None));
        // Row depth of the edit row: one below its parent row (a root-level
        // create is a sibling of the root's children, depth 1).
        let depth = match parent_id {
            Some(parent_id) => self
                .panels
                .explorer
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

        self.begin_explorer_edit_inner(
            ExplorerEditState {
                root,
                parent_id,
                target_id: None,
                is_dir,
                depth,
                path: parent,
                validation: None,
                filename: ExplorerFilenameEditor::default(),
                previously_selected: self.panels.explorer.selected.clone(),
                processing: false,
            },
            window,
            cx,
        );
    }

    /// Start renaming the entry at `target_path`. The root row cannot be
    /// renamed (mirrors Zed: rename is hidden for worktree roots).
    pub(crate) fn begin_explorer_rename(
        &mut self,
        target_path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .panels
            .explorer
            .trees_cache
            .iter()
            .any(|tree| tree.path == target_path)
        {
            return; // worktree roots cannot be renamed (mirrors Zed)
        }
        let Some((root, node_id)) = self.explorer_id_for_path(&target_path) else {
            return;
        };
        let Some(tree) = self.panels.explorer.trees_cache.get(root).cloned() else {
            return;
        };
        let Some(node) = crate::explorer::state::state::find_explorer_node(&tree, &target_path)
        else {
            return;
        };
        let depth = self
            .panels
            .explorer
            .entries
            .iter()
            .find(|row| matches!(row, ExplorerRow::Entry(entry) if entry.id == node_id))
            .map(|row| match row {
                ExplorerRow::Entry(entry) => entry.depth,
                ExplorerRow::Edit { .. } => 0,
            })
            .unwrap_or(0);

        // Preselect the file stem for files (extensions stay untouched), the
        // whole name for directories — mirrors Zed's rename UX.
        let file_name = node.label.clone();
        let selection_end =
            if node.kind == crate::explorer::state::state::ExplorerEntryKind::Directory {
                file_name.len()
            } else if let Some(last_dot) = file_name.rfind('.') {
                if last_dot > 0 {
                    last_dot
                } else {
                    file_name.len()
                }
            } else {
                file_name.len()
            };

        let mut filename = ExplorerFilenameEditor::default();
        filename.set_text(file_name, Some(0..selection_end));

        self.begin_explorer_edit_inner(
            ExplorerEditState {
                root,
                parent_id: None,
                target_id: Some(node.id),
                is_dir: node.kind == crate::explorer::state::state::ExplorerEntryKind::Directory,
                depth,
                path: target_path,
                validation: None,
                filename,
                previously_selected: self.panels.explorer.selected.clone(),
                processing: false,
            },
            window,
            cx,
        );
    }

    fn begin_explorer_edit_inner(
        &mut self,
        mut edit: ExplorerEditState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        edit.filename.focus_handle = Some(cx.focus_handle());
        let focus_handle = edit.filename.focus_handle.clone().unwrap();
        self.panels.explorer.selected = None;
        self.panels.explorer.edit = Some(edit);
        self.rebuild_explorer_entries();
        self.autoscroll_explorer_edit(window, cx);

        // Blur auto-commits when possible and discards otherwise, mirroring
        // Zed: `EditorEvent::Blurred` -> confirm; an empty, duplicate, or
        // unchanged name drops the edit. Window deactivation never commits
        // nor cancels.
        cx.on_blur(&focus_handle, window, |shell, window, cx| {
            if !window.is_window_active() {
                return;
            }
            if shell.panels.explorer.edit.is_some() && !shell.confirm_explorer_edit(window, cx) {
                shell.discard_explorer_edit(cx);
            }
        })
        .detach();
        window.focus(&focus_handle, cx);
        cx.notify();
    }

    /// Scroll the edit row into view and keep it visible while typing.
    pub(crate) fn autoscroll_explorer_edit(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(index) = self.explorer_edit_row_index() else {
            return;
        };
        self.panels
            .explorer
            .scroll_handle
            .scroll_to_item(index, ScrollStrategy::Center);
        cx.notify();
    }

    /// Row index of the inline edit row in the flat list, if any.
    pub(crate) fn explorer_edit_row_index(&self) -> Option<usize> {
        self.panels.explorer.edit.as_ref().and_then(|_| {
            self.panels
                .explorer
                .entries
                .iter()
                .position(|row| matches!(row, ExplorerRow::Edit { .. }))
        })
    }

    /// Commit the inline create/rename: writes to disk on a background
    /// thread, then refreshes the tree and selects the new entry. On failure
    /// the edit stays open with the error surfaced.
    ///
    /// Returns `true` when a commit is in progress (or was already), `false`
    /// when nothing was submitted (empty name, duplicate, or missing edit) —
    /// callers decide whether to keep the edit open (Enter) or discard it
    /// (blur, mirroring Zed).
    pub(crate) fn confirm_explorer_edit(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(edit) = self.panels.explorer.edit.as_ref() else {
            return false;
        };
        if edit.processing {
            return true;
        }
        let filename = edit.filename.text.trim().to_string();
        if filename.is_empty() {
            return false;
        }
        // Re-check duplicate names at commit time.
        if self.explorer_duplicate_name_error(&filename, cx).is_some() {
            self.populate_explorer_validation(cx);
            cx.notify();
            return false;
        }

        let is_create = edit.target_id.is_none();
        let is_dir = edit.is_dir;
        let root = edit.root;
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
        let missing_dirs = if is_create {
            if let Some(worktree) = self.panels.explorer.worktrees.get(root) {
                let snapshot = worktree.read(cx).snapshot();
                crate::explorer::state::worktree::missing_parent_dirs(&snapshot, &new_path)
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };
        self.panels.explorer.edit.as_mut().unwrap().processing = true;

        let weak_shell = cx.entity().downgrade();
        let window_handle = window.window_handle();
        let new_path_for_update = new_path.clone();
        let old_path_for_record = old_path.clone();
        // Deliberately detached: the confirm task must not occupy the
        // `scan_task` slot (that slot gates background scans, and an
        // occupied-but-finished slot would starve future scans).
        cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    if is_create {
                        if is_dir {
                            if new_path.exists() {
                                Err("A folder with this name already exists".to_string())
                            } else {
                                std::fs::create_dir_all(&new_path)
                                    .map_err(|err| err.to_string())
                                    .map(|_| ())
                            }
                        } else {
                            if let Some(parent) = new_path.parent() {
                                std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
                            }
                            std::fs::OpenOptions::new()
                                .write(true)
                                .create_new(true)
                                .open(&new_path)
                                .map(|_| ())
                                .map_err(|err| {
                                    if err.kind() == std::io::ErrorKind::AlreadyExists {
                                        "A file with this name already exists".to_string()
                                    } else {
                                        err.to_string()
                                    }
                                })
                        }
                    } else {
                        std::fs::rename(&old_path, &new_path).map_err(|err| err.to_string())
                    }
                })
                .await;
            let _ = weak_shell.update(cx, |shell, cx| {
                match result {
                    Ok(()) => {
                        shell.panels.explorer.edit = None;
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
                                });
                                ExplorerChange::Batch(batch)
                            } else {
                                ExplorerChange::Created {
                                    path: new_path_for_update.clone(),
                                    is_dir,
                                }
                            }
                        } else {
                            ExplorerChange::Renamed {
                                from: old_path_for_record,
                                to: new_path_for_update.clone(),
                            }
                        };
                        shell.record_explorer_change(change);
                        shell.panels.explorer.pending_select =
                            Some((root, new_path_for_update.clone()));
                        shell.expand_to_path(&new_path_for_update);
                        shell.rescan_explorer_worktrees(cx);
                        shell.sync_explorer_models(cx);
                        if is_create && !is_dir {
                            // Opening the freshly created file mirrors the
                            // "auto open on create" behavior.
                            let weak_shell_for_open = weak_shell.clone();
                            let path_for_open = new_path_for_update.clone();
                            let _ = cx.update_window(window_handle, move |_, window, cx| {
                                let _ = weak_shell_for_open.update(cx, |shell, cx| {
                                    shell.open_explorer_file(path_for_open, window, cx);
                                });
                            });
                        }
                    }
                    Err(err) => {
                        // Keep the edit open with the error surfaced so the
                        // user can fix the name; the typed text survives.
                        if let Some(edit) = shell.panels.explorer.edit.as_mut() {
                            edit.processing = false;
                            edit.validation = Some(ExplorerValidation::Error(err));
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
        true
    }

    /// Cancel the inline edit, restoring the previous selection.
    pub(crate) fn discard_explorer_edit(&mut self, cx: &mut Context<Self>) {
        let Some(edit) = self.panels.explorer.edit.take() else {
            return;
        };
        self.panels.explorer.selected = edit.previously_selected;
        self.rebuild_explorer_entries();
        cx.notify();
    }

    /// Esc during an inline edit cancels it. The global keymap binds escape
    /// to `DismissTransientUi`, which GPUI dispatches as an action before
    /// raw key listeners; being the focused node, the input box handles it
    /// first and stops propagation so nothing else reacts.
    pub(crate) fn on_explorer_escape(
        &mut self,
        _: &DismissTransientUi,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.panels.explorer.edit.is_some() {
            self.discard_explorer_edit(cx);
            cx.stop_propagation();
        }
    }

    /// Keyboard handling for the inline filename input.
    pub(crate) fn on_explorer_filename_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(edit) = self.panels.explorer.edit.as_mut() else {
            return;
        };
        let keystroke = &event.keystroke;
        // While an IME composition is pending the platform drives edits
        // through the input handler; plain keys only apply outside of it.
        let composing = edit.filename.marked_range.is_some();
        if composing && !matches!(keystroke.key.as_str(), "enter" | "escape" | "backspace") {
            return;
        }

        if keystroke.modifiers.control || keystroke.modifiers.platform {
            return; // ctrl/cmd shortcuts are handled via actions
        }

        match keystroke.key.as_str() {
            "enter" => {
                self.confirm_explorer_edit(window, cx);
                return;
            }
            "escape" => {
                self.discard_explorer_edit(cx);
                return;
            }
            "backspace" => {
                if !composing {
                    edit.filename.delete_backward();
                }
            }
            "delete" => {
                if !composing {
                    edit.filename.delete_forward();
                }
            }
            "left" => edit.filename.move_left(keystroke.modifiers.shift),
            "right" => edit.filename.move_right(keystroke.modifiers.shift),
            "home" => edit.filename.move_home(keystroke.modifiers.shift),
            "end" => edit.filename.move_end(keystroke.modifiers.shift),
            _ => {
                // Printable characters arrive through the window input
                // handler (`WM_CHAR` / IME composition), never through
                // `key_char`: inserting here as well would duplicate every
                // character (the platform delivers both paths per key).
                return;
            }
        }
        self.populate_explorer_validation(cx);
        self.autoscroll_explorer_edit(window, cx);
    }

    pub(crate) fn on_explorer_filename_copy(
        &mut self,
        _: &Copy,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(edit) = &self.panels.explorer.edit {
            if !edit.filename.selection_range().is_empty() {
                cx.write_to_clipboard(ClipboardItem::new_string(
                    edit.filename.selected_text().to_string(),
                ));
            }
        }
    }

    pub(crate) fn on_explorer_filename_cut(
        &mut self,
        _: &Cut,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(edit) = &mut self.panels.explorer.edit {
            if !edit.filename.selection_range().is_empty() {
                let text = edit.filename.selected_text().to_string();
                edit.filename
                    .replace_range(edit.filename.selection_range(), "");
                cx.write_to_clipboard(ClipboardItem::new_string(text));
                self.populate_explorer_validation(cx);
                cx.notify();
            }
        }
    }

    pub(crate) fn on_explorer_filename_paste(
        &mut self,
        _: &Paste,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(edit) = &mut self.panels.explorer.edit else {
            return;
        };
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            return;
        };
        let sanitized = text.replace(['\r', '\n'], "");
        edit.filename.insert_at_selection(&sanitized);
        self.populate_explorer_validation(cx);
        cx.notify();
    }
}

// ── GPUI IME bridge ─────────────────────────────────────────────────────
