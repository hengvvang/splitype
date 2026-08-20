//! Inline filename editor for the explorer: text buffer operations, keyboard
//! handling, clipboard actions, the GPUI IME bridge (`EntityInputHandler`),
//! and the custom input element.
//!
//! All buffer offsets are UTF-8 byte offsets; GPUI's IME layer speaks UTF-16
//! and is bridged via `utf8_to_utf16` / `utf16_to_utf8` helpers. The input
//! element mirrors the existing `CodeLanguageInputElement` (wysiwyg render):
//! a custom `Element` shapes the line, paints selection/cursor/marked-text,
//! and registers itself as the window input handler while focused.

use std::ops::Range;
use std::path::PathBuf;

use gpui::*;

use crate::app::shell::Shell;

use crate::editor::editing::input::actions::{Copy, Cut, DismissTransientUi, Paste};
use crate::explorer::state::state::{
    EXPLORER_NODE_HEIGHT, ExplorerEditState, ExplorerFilenameEditor, ExplorerRow,
    ExplorerValidation,
};
use crate::explorer::state::undo::ExplorerChange;
use crate::infra::theme::ThemeManager;
use crate::model::inline::offsets::ImeConverter;

// ── UTF-8 / UTF-16 offset conversion ────────────────────────────────────

/// Convert a UTF-16 code-unit offset into a UTF-8 byte offset in `text`.
#[allow(dead_code)]
pub(crate) fn utf16_to_utf8_in(text: &str, utf16_offset: usize) -> usize {
    ImeConverter::utf16_to_utf8_in(text, utf16_offset)
}

/// Convert a UTF-8 byte offset in `text` into a UTF-16 code-unit offset.
pub(crate) fn utf8_to_utf16_in(text: &str, utf8_offset: usize) -> usize {
    ImeConverter::utf8_to_utf16_in(text, utf8_offset)
}

fn utf16_range_to_utf8_in(text: &str, range: &Range<usize>) -> Range<usize> {
    ImeConverter::utf16_range_to_utf8_in(text, range)
}

fn utf8_range_to_utf16_in(text: &str, range: &Range<usize>) -> Range<usize> {
    ImeConverter::utf8_range_to_utf16_in(text, range)
}

// ── Text buffer operations ──────────────────────────────────────────────

impl ExplorerFilenameEditor {
    /// Replace the whole buffer, optionally preselecting a byte range.
    pub(crate) fn set_text(&mut self, text: String, select: Option<Range<usize>>) {
        self.text = text;
        let end = self.text.len();
        let selection = select.unwrap_or(end..end);
        self.selection = selection.start.min(end)..selection.end.min(end);
        self.reversed = false;
        self.marked_range = None;
    }

    /// The selected range in forward order.
    pub(crate) fn selection_range(&self) -> Range<usize> {
        let (start, end) = if self.reversed {
            (self.selection.end, self.selection.start)
        } else {
            (self.selection.start, self.selection.end)
        };
        start..end
    }

    pub(crate) fn selected_text(&self) -> &str {
        &self.text[self.selection_range()]
    }

    pub(crate) fn cursor(&self) -> usize {
        if self.reversed {
            self.selection.start
        } else {
            self.selection.end
        }
    }

    /// Replace `range` with `new_text`, placing the caret after it.
    pub(crate) fn replace_range(&mut self, range: Range<usize>, new_text: &str) {
        let start = range.start.min(self.text.len());
        let end = range.end.min(self.text.len());
        if start > end {
            return;
        }
        self.text.replace_range(start..end, new_text);
        let cursor = start + new_text.len();
        self.selection = cursor..cursor;
        self.reversed = false;
        self.marked_range = None;
    }

    pub(crate) fn insert_at_selection(&mut self, new_text: &str) {
        let range = self.selection_range();
        self.replace_range(range, new_text);
    }

    pub(crate) fn delete_backward(&mut self) {
        let range = self.selection_range();
        if !range.is_empty() {
            self.replace_range(range, "");
            return;
        }
        let cursor = self.cursor();
        if cursor > 0 {
            let start = self.text.floor_char_boundary(cursor - 1);
            self.replace_range(start..cursor, "");
        }
    }

    pub(crate) fn delete_forward(&mut self) {
        let range = self.selection_range();
        if !range.is_empty() {
            self.replace_range(range, "");
            return;
        }
        let cursor = self.cursor();
        if cursor < self.text.len() {
            let end = self.text.ceil_char_boundary(cursor + 1);
            self.replace_range(cursor..end, "");
        }
    }

    pub(crate) fn move_left(&mut self, extend: bool) {
        let cursor = self.cursor();
        let anchor = self.selection_anchor();
        if !extend && !self.selection_range().is_empty() {
            let new_cursor = if self.reversed {
                self.selection.start
            } else {
                self.selection.end
            };
            self.selection = new_cursor..new_cursor;
            self.reversed = false;
            return;
        }
        let target = self.text.floor_char_boundary(cursor.saturating_sub(1));
        self.set_cursor(target, anchor, extend);
    }

    pub(crate) fn move_right(&mut self, extend: bool) {
        let cursor = self.cursor();
        let anchor = self.selection_anchor();
        if !extend && !self.selection_range().is_empty() {
            let new_cursor = if self.reversed {
                self.selection.end
            } else {
                self.selection.start
            };
            self.selection = new_cursor..new_cursor;
            self.reversed = false;
            return;
        }
        let target = self
            .text
            .ceil_char_boundary(cursor + 1)
            .min(self.text.len());
        self.set_cursor(target, anchor, extend);
    }

    pub(crate) fn move_home(&mut self, extend: bool) {
        let anchor = self.selection_anchor();
        self.set_cursor(0, anchor, extend);
    }

    pub(crate) fn move_end(&mut self, extend: bool) {
        let anchor = self.selection_anchor();
        self.set_cursor(self.text.len(), anchor, extend);
    }

    fn selection_anchor(&self) -> usize {
        if self.reversed {
            self.selection.end
        } else {
            self.selection.start
        }
    }

    fn set_cursor(&mut self, cursor: usize, anchor: usize, extend: bool) {
        if extend {
            self.selection = anchor..cursor;
            self.reversed = cursor < anchor;
        } else {
            self.selection = cursor..cursor;
            self.reversed = false;
        }
    }
}

// ── Editor-side handlers: validation, confirm/cancel, keyboard, clipboard ─

impl Shell {
    /// Real-time validation of the inline filename (mirrors Zed's
    /// `populate_validation_error`): whitespace warns, illegal characters and
    /// name collisions error.
    pub(crate) fn populate_explorer_validation(&mut self, cx: &mut Context<Self>) {
        let Some(filename) = self
            .panels
            .explorer
            .edit
            .as_ref()
            .map(|edit| edit.filename.text.clone())
        else {
            return;
        };
        let validation = if filename.trim() != filename {
            Some(ExplorerValidation::Warning(
                "File name has leading or trailing whitespace.".into(),
            ))
        } else if filename.contains(['/', '\\', '\0']) {
            Some(ExplorerValidation::Error(
                "File name cannot contain '/' or '\\'.".into(),
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
        // New entry: check the target path. Rename: check except itself.
        let new_path = if edit.target_id.is_none() {
            edit.path.join(filename)
        } else {
            let parent = edit.path.parent()?;
            parent.join(filename)
        };
        let existing = crate::explorer::state::state::find_explorer_node(tree, &new_path);
        let is_self = edit
            .target_id
            .is_some_and(|id| existing.is_some_and(|node| node.id == id));
        if existing.is_some() && !is_self {
            Some(ExplorerValidation::Error(format!(
                "File or directory '{filename}' already exists at this location."
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
        let Some(node) =
            crate::explorer::state::state::find_explorer_node(&tree, &target_path)
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
            } else {
                target_path
                    .file_stem()
                    .map(|stem| stem.len())
                    .unwrap_or(file_name.len())
            };

        let mut filename = ExplorerFilenameEditor::default();
        filename.set_text(file_name, Some(0..selection_end));

        self.begin_explorer_edit_inner(
            ExplorerEditState {
                root,
                parent_id: None,
                target_id: Some(node.id),
                is_dir: node.kind
                    == crate::explorer::state::state::ExplorerEntryKind::Directory,
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
        // Zed: `EditorEvent::Blurred` → confirm; an empty, duplicate, or
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
        window.focus(&focus_handle);
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
        self.panels
            .explorer
            .edit
            .as_ref()
            .map(|_| {
                self.panels
                    .explorer
                    .entries
                    .iter()
                    .position(|row| matches!(row, ExplorerRow::Edit { .. }))
            })
            .flatten()
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
            edit.path.join(&filename)
        } else {
            edit.path
                .parent()
                .map(|parent| parent.join(&filename))
                .unwrap_or_else(|| edit.path.clone())
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
                            ExplorerChange::Created {
                                path: new_path_for_update.clone(),
                                is_dir,
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
        let sanitized = text.replace('\r', "").replace('\n', "");
        edit.filename.insert_at_selection(&sanitized);
        self.populate_explorer_validation(cx);
        cx.notify();
    }
}

// ── GPUI IME bridge ─────────────────────────────────────────────────────

impl EntityInputHandler for Shell {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let edit = self.panels.explorer.edit.as_mut()?;
        if !edit.filename.focus_handle.as_ref()?.is_focused(window) {
            return None;
        }
        let range = utf16_range_to_utf8_in(&edit.filename.text, &range_utf16);
        actual_range.replace(utf8_range_to_utf16_in(&edit.filename.text, &range));
        Some(edit.filename.text[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let edit = self.panels.explorer.edit.as_ref()?;
        if !edit.filename.focus_handle.as_ref()?.is_focused(window) {
            return None;
        }
        Some(UTF16Selection {
            range: utf8_range_to_utf16_in(&edit.filename.text, &edit.filename.selection_range()),
            reversed: edit.filename.reversed,
        })
    }

    fn marked_text_range(
        &self,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        let edit = self.panels.explorer.edit.as_ref()?;
        if !edit.filename.focus_handle.as_ref()?.is_focused(window) {
            return None;
        }
        edit.filename
            .marked_range
            .as_ref()
            .map(|range| utf8_range_to_utf16_in(&edit.filename.text, range))
    }

    fn unmark_text(&mut self, window: &mut Window, _cx: &mut Context<Self>) {
        if let Some(edit) = self.panels.explorer.edit.as_mut() {
            if edit
                .filename
                .focus_handle
                .as_ref()
                .is_some_and(|handle| handle.is_focused(window))
            {
                edit.filename.marked_range = None;
            }
        }
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(edit) = self.panels.explorer.edit.as_mut() else {
            return;
        };
        if !edit
            .filename
            .focus_handle
            .as_ref()
            .is_some_and(|handle| handle.is_focused(window))
        {
            return;
        }
        let text = edit.filename.text.clone();
        let range = range_utf16
            .as_ref()
            .map(|range| utf16_range_to_utf8_in(&text, range))
            .or_else(|| edit.filename.marked_range.clone())
            .unwrap_or_else(|| edit.filename.selection_range());
        edit.filename.replace_range(range, new_text);
        self.populate_explorer_validation(cx);
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(edit) = self.panels.explorer.edit.as_mut() else {
            return;
        };
        if !edit
            .filename
            .focus_handle
            .as_ref()
            .is_some_and(|handle| handle.is_focused(window))
        {
            return;
        }
        let text = edit.filename.text.clone();
        let range = range_utf16
            .as_ref()
            .map(|range| utf16_range_to_utf8_in(&text, range))
            .or_else(|| edit.filename.marked_range.clone())
            .unwrap_or_else(|| edit.filename.selection_range());
        let sanitized = new_text.replace('\r', "").replace('\n', "");
        edit.filename.text.replace_range(range.clone(), &sanitized);
        let marked = range.start..range.start + sanitized.len();
        let selection = new_selected_range_utf16
            .as_ref()
            .map(|range| utf16_range_to_utf8_in(&sanitized, range))
            .map(|relative| marked.start + relative.start..marked.start + relative.end)
            .unwrap_or_else(|| marked.clone());
        edit.filename.marked_range = Some(marked);
        edit.filename.selection = selection;
        edit.filename.reversed = false;
        self.populate_explorer_validation(cx);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let edit = self.panels.explorer.edit.as_ref()?;
        if !edit.filename.focus_handle.as_ref()?.is_focused(window) {
            return None;
        }
        let line = shape_filename_line(window, &edit.filename.text);
        let range = utf16_range_to_utf8_in(&edit.filename.text, &range_utf16);
        Some(Bounds::from_corners(
            point(bounds.left() + line.x_for_index(range.start), bounds.top()),
            point(bounds.left() + line.x_for_index(range.end), bounds.bottom()),
        ))
    }

    fn character_index_for_point(
        &mut self,
        pt: Point<Pixels>,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let edit = self.panels.explorer.edit.as_ref()?;
        if !edit.filename.focus_handle.as_ref()?.is_focused(window) {
            return None;
        }
        let bounds = edit.filename.last_bounds?;
        let line = shape_filename_line(window, &edit.filename.text);
        let x = pt.x - bounds.left();
        let index = line.closest_index_for_x(x);
        Some(utf8_to_utf16_in(&edit.filename.text, index))
    }
}

fn shape_filename_line(window: &mut Window, text: &str) -> ShapedLine {
    let display_text: SharedString = text.to_string().into();
    let style = window.text_style();
    let font_size = style.font_size.to_pixels(window.rem_size());
    let run = TextRun {
        len: display_text.len(),
        font: style.font(),
        color: style.color,
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    window
        .text_system()
        .shape_line(display_text, font_size, &[run], None)
}

// ── Input element ───────────────────────────────────────────────────────

pub(crate) struct ExplorerFilenamePrepaintState {
    line: Option<ShapedLine>,
    selection: Option<PaintQuad>,
    cursor: Option<PaintQuad>,
    hitbox: Option<Hitbox>,
}

/// Custom element painting the inline filename text, selection, cursor, and
/// IME composition underline; registers the window input handler while
/// focused (mirrors `CodeLanguageInputElement`).
pub(crate) struct ExplorerFilenameInputElement {
    pub(crate) editor: Entity<Shell>,
}

impl IntoElement for ExplorerFilenameInputElement {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for ExplorerFilenameInputElement {
    type RequestLayoutState = ();
    type PrepaintState = ExplorerFilenamePrepaintState;

    fn id(&self) -> Option<ElementId> {
        Some(ElementId::Name("explorer-filename-input".into()))
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = px(EXPLORER_NODE_HEIGHT).into();
        style.flex_grow = 1.0;
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let theme = cx.global::<ThemeManager>().current_arc();
        let Some(edit) = self.editor.read(cx).panels.explorer.edit.clone() else {
            return ExplorerFilenamePrepaintState {
                line: None,
                selection: None,
                cursor: None,
                hitbox: None,
            };
        };
        let filename = &edit.filename;

        // Remember the bounds for IME hit-testing.
        self.editor.update(cx, |shell, _cx| {
            if let Some(edit) = shell.panels.explorer.edit.as_mut() {
                edit.filename.last_bounds = Some(bounds);
            }
        });

        let text: SharedString = filename.text.clone().into();
        let focused = filename
            .focus_handle
            .as_ref()
            .is_some_and(|handle| handle.is_focused(window));

        let base_run = TextRun {
            len: text.len(),
            font: window.text_style().font(),
            color: theme.colors.text_default,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let runs = if let Some(marked_range) =
            filename.marked_range.as_ref().filter(|_| !text.is_empty())
        {
            vec![
                TextRun {
                    len: marked_range.start,
                    ..base_run.clone()
                },
                TextRun {
                    len: marked_range.end - marked_range.start,
                    underline: Some(UnderlineStyle {
                        color: Some(theme.colors.text_default),
                        thickness: px(theme.dimensions.underline_thickness),
                        wavy: false,
                    }),
                    ..base_run.clone()
                },
                TextRun {
                    len: text.len().saturating_sub(marked_range.end),
                    ..base_run
                },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect()
        } else {
            vec![base_run]
        };

        let font_size = window.text_style().font_size.to_pixels(window.rem_size());
        let line = window
            .text_system()
            .shape_line(text, font_size, &runs, None);
        let line_height = bounds.size.height;
        let selection_range = filename.selection_range();
        let selection = if focused && !selection_range.is_empty() {
            let start = line.x_for_index(selection_range.start);
            let end = line.x_for_index(selection_range.end);
            Some(fill(
                Bounds::from_corners(
                    point(bounds.left() + start, bounds.top()),
                    point(bounds.left() + end, bounds.bottom()),
                ),
                theme.colors.selection,
            ))
        } else {
            None
        };
        let cursor = if focused && selection_range.is_empty() {
            let cursor_x = line.x_for_index(if filename.reversed {
                filename.selection.start
            } else {
                filename.selection.end
            });
            let mut cursor_color = theme.colors.cursor;
            cursor_color.a = 1.0;
            Some(fill(
                Bounds::new(
                    point(bounds.left() + cursor_x, bounds.top()),
                    size(px(theme.dimensions.cursor_width), line_height),
                ),
                cursor_color,
            ))
        } else {
            None
        };
        let hitbox = Some(window.insert_hitbox(bounds, HitboxBehavior::Normal));

        ExplorerFilenamePrepaintState {
            line: Some(line),
            selection,
            cursor,
            hitbox,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(hitbox) = prepaint.hitbox.as_ref()
            && hitbox.is_hovered(window)
        {
            window.set_cursor_style(CursorStyle::IBeam, hitbox);
        }

        let focus_handle = self
            .editor
            .read(cx)
            .panels
            .explorer
            .edit
            .as_ref()
            .and_then(|edit| edit.filename.focus_handle.clone());
        if let Some(focus_handle) = focus_handle
            && focus_handle.is_focused(window)
        {
            window.handle_input(
                &focus_handle,
                ElementInputHandler::new(bounds, self.editor.clone()),
                cx,
            );
        }

        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection);
        }

        if let Some(line) = prepaint.line.take() {
            line.paint(bounds.origin, bounds.size.height, window, cx)
                .ok();
        }

        if let Some(cursor) = prepaint.cursor.take() {
            window.paint_quad(cursor);
        }
    }
}
