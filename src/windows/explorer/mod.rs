//! Explorer — file-tree sidebar with create/rename/delete actions.
//!
//! Architecture (mirroring Zed's project panel):
//! - The directory tree is scanned **on a background thread** and stored as
//!   an immutable [`ExplorerFileNode`]; the UI thread only swaps in the
//!   result (see `sync_explorer_file_tree`).
//! - The renderable data source is a **flat list of visible rows**
//!   ([`VisibleExplorerEntry`]), derived from the scanned tree plus the
//!   expansion set (`flatten_file_tree`). The virtualized `uniform_list`
//!   renders only the visible range of rows.
//! - Expansion ids live in a sorted `BTreeSet` (binary-search friendly);
//!   collapsing a directory prunes its whole subtree from the flat list.
//! - Selection is keyed by stable [`ExplorerEntryId`]; opening a file
//!   reveals its ancestors (`expand_to_path`) and centers it
//!   (`autoscroll_explorer_selection`).
//! - The outline tree (headings) shares the sidebar state but keeps its own
//!   string-keyed expansion set.

pub(crate) mod filename_editor;
pub(crate) mod state;

use crate::ui::components::button::icon_chip_button;

use std::ops::Range;
use std::path::{Path, PathBuf};

use futures::StreamExt;
use gpui::*;
use notify::Watcher as _;

use crate::editor::actions::{CloseExplorerFolder, ToggleExplorer};
use crate::editor::controller::Editor;
use crate::infra::config::recent::{read_recent_files, read_recent_folders};
use crate::infra::i18n::{I18nManager, I18nStrings};
use crate::theme::{Theme, ThemeManager};
use crate::ui::components::empty_state::empty_state_container;
use crate::windows::explorer::filename_editor::ExplorerFilenameInputElement;
use crate::windows::explorer::state::*;

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
    pub(crate) fn toggle_explorer_drawer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.panels.explorer.is_open {
            self.panels.explorer.is_open = false;
        } else {
            self.close_menu_bar(cx);
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
        explorer.root = None;
        explorer.scanned_root = None;
        explorer.watched_root = None;
        explorer.file_tree = None;
        explorer.file_error = None;
        explorer.outline_tree = Vec::new();
        explorer.outline_source = None;
        explorer.expanded.clear();
        explorer.expanded_outline.clear();
        explorer.entries.clear();
        explorer.selected = None;
        explorer.marked.clear();
        explorer.pending_select = None;
        explorer.edit = None;
        explorer.scan_task = None;
        explorer.fs_watch_task = None;
        explorer.fs_refresh_task = None;
        cx.notify();
    }
    pub(crate) fn open_explorer_folder_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.panels.explorer.root = Some(path.clone());
        self.panels.explorer.is_open = true;
        // Cancel any in-flight scan of the previous root.
        self.panels.explorer.scan_task = None;
        self.panels.explorer.file_tree = None;
        self.panels.explorer.file_error = None;
        self.panels.explorer.expanded.clear();
        // The root row starts expanded (VSCode-style title row visible).
        self.panels.explorer
            .expanded
            .insert(ExplorerEntryId::for_path(&path));
        self.panels.explorer.entries.clear();
        self.sync_explorer_models(cx);
        cx.notify();
    }
    pub(crate) fn sync_explorer_after_document_path_change(&mut self, cx: &mut Context<Self>) {
        if self.panels.explorer.root.is_none() {
            self.panels.explorer.root = self.explorer_root_for_current_file();
        }
        self.panels.explorer.outline_source = None;
        if self.panels.explorer.is_open {
            self.sync_explorer_models(cx);
        }
    }
    pub(crate) fn sync_explorer_models(&mut self, cx: &mut Context<Self>) {
        // The file tree only needs a root directory, so it syncs even in
        // the welcome state (no tabs). The outline reads the active
        // document and only runs once a tab exists.
        self.sync_explorer_file_tree(cx);
        if self.has_active_tab() {
            self.sync_explorer_outline(cx);
        }
    }
    pub(crate) fn explorer_root_for_current_file(&self) -> Option<PathBuf> {
        self.active_editor_tab()
            .and_then(|tab| tab.file.path.as_ref())
            .and_then(|path| path.parent().map(Path::to_path_buf))
    }
    pub(crate) fn prompt_open_explorer_folder(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: false,
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
            let Some(folder_path) = paths.into_iter().next() else {
                return;
            };
            eprintln!("[explorer] selected folder: {folder_path:?}");
            if let Err(err) = crate::infra::config::recent::record_recent_folder(&folder_path) {
                eprintln!("failed to update recent folder history: {err}");
            }
            let _ = weak_editor.update(cx, |editor, cx| {
                editor.open_explorer_folder_path(folder_path, cx);
                cx.notify();
            });
        })
        .detach();
    }
    pub(crate) fn collapse_all_explorer_nodes(&mut self, cx: &mut Context<Self>) {
        self.panels.explorer.expanded.clear();
        // Keep the root row expanded (mirrors Zed's collapse-all, which
        // retains the worktree root) so the title-row buttons stay visible.
        if let Some(tree) = &self.panels.explorer.file_tree {
            self.panels.explorer.expanded.insert(tree.id);
        }
        self.rebuild_explorer_entries();
        cx.notify();
    }
    pub(crate) fn refresh_explorer_tree(&mut self, cx: &mut Context<Self>) {
        self.panels.explorer.needs_rescan = true;
        self.sync_explorer_models(cx);
        cx.notify();
    }

    /// Toggle dotfile visibility. Persists to settings and rescans.
    #[allow(dead_code)] // settings-backed capability, no toolbar button
    pub(crate) fn toggle_explorer_hidden(&mut self, cx: &mut Context<Self>) {
        let mut settings = crate::infra::config::settings::ExplorerSettingsStore::settings(cx);
        settings.hide_hidden = !settings.hide_hidden;
        crate::infra::config::settings::ExplorerSettingsStore::set(cx, settings);
        self.panels.explorer.needs_rescan = true;
        self.sync_explorer_models(cx);
        cx.notify();
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub(crate) fn delete_explorer_item(
        &mut self,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let item_name = path
            .file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default();
        let is_dir = path.is_dir();
        let prompt_msg = format!("Are you sure you want to delete '{}'?", item_name);
        let strings = cx.global::<I18nManager>().strings().clone();
        let ok_btn = "Delete";
        let cancel_btn = strings.settings_cancel.clone();

        let prompt = window.prompt(
            PromptLevel::Warning,
            &prompt_msg,
            None,
            &[ok_btn, cancel_btn.as_str()],
            cx,
        );

        let weak_editor = cx.entity().downgrade();
        let _ = cx.spawn(async move |_this, cx| {
            if let Ok(0) = prompt.await {
                let res = if is_dir {
                    std::fs::remove_dir_all(&path)
                } else {
                    std::fs::remove_file(&path)
                };
                if let Err(err) = res {
                    eprintln!("failed to delete item: {err}");
                } else {
                    let _ = weak_editor.update(cx, |editor, cx| {
                        editor.panels.explorer.needs_rescan = true;
                        editor.sync_explorer_models(cx);
                        cx.notify();
                    });
                }
            }
        });
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
    /// the absolute path when it is outside the root).
    pub(crate) fn copy_explorer_relative_path(&self, path: &Path, cx: &mut Context<Self>) {
        let relative = self
            .panels
            .explorer
            .root
            .as_deref()
            .and_then(|root| path.strip_prefix(root).ok())
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

    /// Expand a directory and all of its descendants.
    pub(crate) fn expand_all_explorer_for_entry(
        &mut self,
        id: ExplorerEntryId,
        cx: &mut Context<Self>,
    ) {
        let Some(tree) = self.panels.explorer.file_tree.clone() else {
            return;
        };
        let Some(node) = find_explorer_node_by_id(&tree, id) else {
            return;
        };
        let mut ids = std::collections::BTreeSet::new();
        collect_descendant_dir_ids(node, &mut ids);
        self.panels.explorer.expanded.extend(ids);
        self.rebuild_explorer_entries();
        cx.notify();
    }

    /// Collapse a directory and all of its descendants.
    pub(crate) fn collapse_all_explorer_for_entry(
        &mut self,
        id: ExplorerEntryId,
        cx: &mut Context<Self>,
    ) {
        let Some(tree) = self.panels.explorer.file_tree.clone() else {
            return;
        };
        let Some(node) = find_explorer_node_by_id(&tree, id) else {
            return;
        };
        let mut ids = std::collections::BTreeSet::new();
        collect_descendant_dir_ids(node, &mut ids);
        self.panels.explorer.expanded.retain(|id| !ids.contains(id));
        self.rebuild_explorer_entries();
        cx.notify();
    }

    /// Expand every directory in the tree.
    pub(crate) fn expand_all_explorer_nodes(&mut self, cx: &mut Context<Self>) {
        let Some(tree) = self.panels.explorer.file_tree.clone() else {
            return;
        };
        let mut ids = std::collections::BTreeSet::new();
        collect_descendant_dir_ids(&tree, &mut ids);
        self.panels.explorer.expanded.extend(ids);
        self.rebuild_explorer_entries();
        cx.notify();
    }

    // ── Multi-select, clipboard operations, and deletion (mirrors Zed) ──

    /// Resolve the entries an operation applies to (Zed's `effective_entries`):
    /// the selection when nothing is marked, otherwise the marked set. The
    /// worktree root is excluded so destructive operations (delete / cut /
    /// move / copy) can never target the root row.
    pub(crate) fn effective_explorer_entries(&self) -> Vec<ExplorerEntryId> {
        let root_id = self.panels.explorer.file_tree.as_ref().map(|tree| tree.id);
        let filter = |id: &ExplorerEntryId| Some(*id) != root_id;
        if self.panels.explorer.marked.is_empty() {
            return match self.panels.explorer.selected {
                Some(ExplorerSelection::File(id)) if filter(&id) => vec![id],
                _ => Vec::new(),
            };
        }
        self.panels
            .explorer
            .marked
            .iter()
            .filter_map(|selection| match selection {
                ExplorerSelection::File(id) if filter(id) => Some(*id),
                _ => None,
            })
            .collect()
    }

    /// Look up a visible entry row by its stable id.
    pub(crate) fn explorer_entry_by_id(&self, id: ExplorerEntryId) -> Option<&VisibleExplorerEntry> {
        self.panels.explorer.entries.iter().find_map(|row| match row {
            ExplorerRow::Entry(entry) if entry.id == id => Some(entry),
            _ => None,
        })
    }

    /// Toggle an entry in the multi-select mark set (Alt+click).
    pub(crate) fn toggle_explorer_mark(&mut self, selection: ExplorerSelection, cx: &mut Context<Self>) {
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
    pub(crate) fn select_explorer_range(&mut self, target_id: ExplorerEntryId, cx: &mut Context<Self>) {
        let anchor = match &self.panels.explorer.selected {
            Some(ExplorerSelection::File(id)) => *id,
            _ => target_id,
        };
        let rows = &self.panels.explorer.entries;
        let anchor_index = rows.iter().position(|row| {
            matches!(row, ExplorerRow::Entry(entry) if entry.id == anchor)
        });
        let target_index = rows.iter().position(|row| {
            matches!(row, ExplorerRow::Entry(entry) if entry.id == target_id)
        });
        let (Some(anchor_index), Some(target_index)) = (anchor_index, target_index) else {
            return;
        };
        self.panels.explorer.marked.clear();
        for row in &rows[anchor_index.min(target_index)..=anchor_index.max(target_index)] {
            if let ExplorerRow::Entry(entry) = row {
                self.panels.explorer.marked.push(ExplorerSelection::File(entry.id));
            }
        }
        self.panels.explorer.selected = Some(ExplorerSelection::File(target_id));
        self.autoscroll_explorer_selection();
        cx.notify();
    }

    /// Choose the next selection after deleting `deleted_ids` (mirrors Zed's
    /// `find_next_selection_after_deletion`): the next visible sibling, else
    /// the previous one, else the parent directory.
    fn next_explorer_selection_after_deletion(
        &self,
        deleted_ids: &[ExplorerEntryId],
    ) -> Option<ExplorerSelection> {
        let deleted: std::collections::HashSet<ExplorerEntryId> =
            deleted_ids.iter().copied().collect();
        let rows = &self.panels.explorer.entries;
        let last_deleted = rows.iter().rposition(|row| {
            matches!(row, ExplorerRow::Entry(entry) if deleted.contains(&entry.id))
        })?;
        for row in &rows[last_deleted + 1..] {
            if let ExplorerRow::Entry(entry) = row {
                if !deleted.contains(&entry.id) {
                    return Some(ExplorerSelection::File(entry.id));
                }
            }
        }
        for row in rows[..last_deleted].iter().rev() {
            if let ExplorerRow::Entry(entry) = row {
                if !deleted.contains(&entry.id) {
                    return Some(ExplorerSelection::File(entry.id));
                }
            }
        }
        if let ExplorerRow::Entry(last) = &rows[last_deleted]
            && let Some(tree) = self.panels.explorer.file_tree.as_ref()
            && let Some(parent) = last.path.parent()
            && let Some(parent_node) = find_explorer_node(tree, parent)
        {
            return Some(ExplorerSelection::File(parent_node.id));
        }
        None
    }

    /// Delete the effective selection with a confirmation prompt; after the
    /// background deletion the selection moves to the next surviving sibling
    /// (Zed's trash/delete flow, without the OS-trash variant).
    pub(crate) fn delete_explorer_selections(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let ids = self.effective_explorer_entries();
        if ids.is_empty() {
            return;
        }
        let names: Vec<String> = ids
            .iter()
            .filter_map(|id| self.explorer_entry_by_id(*id).map(|entry| entry.label.clone()))
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

        let weak_editor = cx.entity().downgrade();
        let _ = cx.spawn(async move |_this, cx| {
            if prompt.await != Ok(0) {
                return;
            }
            let paths: Vec<PathBuf> = weak_editor
                .update(cx, |editor, _cx| {
                    ids.iter()
                        .filter_map(|id| editor.explorer_entry_by_id(*id).map(|entry| entry.path.clone()))
                        .collect()
                })
                .unwrap_or_default();
            cx.background_executor()
                .spawn(async move {
                    for path in &paths {
                        let result = if path.is_dir() {
                            std::fs::remove_dir_all(path)
                        } else {
                            std::fs::remove_file(path)
                        };
                        if let Err(err) = result {
                            eprintln!("failed to delete '{}': {err}", path.display());
                        }
                    }
                })
                .await;
            let _ = weak_editor.update(cx, |editor, cx| {
                editor.panels.explorer.marked.clear();
                editor.panels.explorer.needs_rescan = true;
                if let Some(next) = editor.next_explorer_selection_after_deletion(&ids) {
                    editor.panels.explorer.selected = Some(next);
                }
                editor.sync_explorer_models(cx);
                editor.autoscroll_explorer_selection();
                cx.notify();
            });
        });
    }

    /// Copy the effective selection: absolute paths go to the system
    /// clipboard, entry ids are remembered for in-panel paste.
    pub(crate) fn explorer_copy(&mut self, cx: &mut Context<Self>) {
        let ids = self.effective_explorer_entries();
        if ids.is_empty() {
            return;
        }
        let paths: Vec<String> = ids
            .iter()
            .filter_map(|id| {
                self.explorer_entry_by_id(*id)
                    .map(|entry| entry.path.to_string_lossy().into_owned())
            })
            .collect();
        cx.write_to_clipboard(ClipboardItem::new_string(paths.join("\n")));
        self.panels.explorer.clipboard =
            Some(ExplorerClipboard::Copied(ids.into_iter().map(ExplorerSelection::File).collect()));
        cx.notify();
    }

    /// Cut the effective selection (same dual-clipboard behavior as copy).
    pub(crate) fn explorer_cut(&mut self, cx: &mut Context<Self>) {
        let ids = self.effective_explorer_entries();
        if ids.is_empty() {
            return;
        }
        let paths: Vec<String> = ids
            .iter()
            .filter_map(|id| {
                self.explorer_entry_by_id(*id)
                    .map(|entry| entry.path.to_string_lossy().into_owned())
            })
            .collect();
        cx.write_to_clipboard(ClipboardItem::new_string(paths.join("\n")));
        self.panels.explorer.clipboard =
            Some(ExplorerClipboard::Cut(ids.into_iter().map(ExplorerSelection::File).collect()));
        cx.notify();
    }

    /// Duplicate = copy + paste (Zed's `duplicate`).
    pub(crate) fn explorer_duplicate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.explorer_copy(cx);
        self.explorer_paste(window, cx);
    }

    /// Target directory for a paste: the selected directory, the parent of a
    /// selected file, or the explorer root.
    fn explorer_paste_target_dir(&self) -> Option<PathBuf> {
        match &self.panels.explorer.selected {
            Some(ExplorerSelection::File(id)) => {
                let entry = self.explorer_entry_by_id(*id)?;
                if entry.kind == ExplorerEntryKind::Directory {
                    Some(entry.path.clone())
                } else {
                    entry.path.parent().map(Path::to_path_buf)
                }
            }
            _ => self.panels.explorer.root.clone(),
        }
    }

    /// Paste the in-panel clipboard into the target directory. Cut entries
    /// are moved (`fs::rename`), copied entries are duplicated; after the
    /// background operation the last successful result is selected and a
    /// rescan is scheduled.
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
            .filter_map(|selection| match selection {
                ExplorerSelection::File(id) => {
                    self.explorer_entry_by_id(*id).map(|entry| entry.path.clone())
                }
                _ => None,
            })
            .collect();
        if items.is_empty() {
            return;
        }
        let is_cut = clipboard.is_cut();
        let weak_editor = cx.entity().downgrade();
        let window_handle = window.window_handle();
        let _ = cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let result = cx
                .background_executor()
                .spawn(async move { execute_entry_ops(&items, &target_dir, is_cut) })
                .await;
            let _ = weak_editor.update(cx, |editor, cx| {
                if is_cut {
                    // After the first paste a cut becomes a copy (Zed).
                    editor.panels.explorer.clipboard =
                        editor.panels.explorer.clipboard.take().map(ExplorerClipboard::into_copied);
                }
                editor.panels.explorer.marked.clear();
                editor.panels.explorer.needs_rescan = true;
                for change in &result {
                    editor.record_explorer_change(change.clone());
                }
                if let Some(path) = result
                    .last()
                    .map(explorer_change_destination)
                    .flatten()
                    .map(Path::to_path_buf)
                {
                    editor.panels.explorer.pending_select = Some(path.clone());
                    let weak_editor_for_open = weak_editor.clone();
                    let _ = cx.update_window(window_handle, move |_, _window, cx| {
                        let _ = weak_editor_for_open.update(cx, |editor, _cx| {
                            editor.expand_to_path(&path);
                            editor.rebuild_explorer_entries();
                            editor.autoscroll_explorer_selection();
                        });
                    });
                }
                editor.sync_explorer_models(cx);
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
        let weak_editor = cx.entity().downgrade();
        let change_for_execution = change.clone();
        let _ = cx.spawn(async move |_this, cx: &mut AsyncApp| {
            cx.background_executor()
                .spawn(async move { execute_explorer_change_inverse(&change_for_execution) })
                .await;
            let _ = weak_editor.update(cx, |editor, cx| {
                editor.panels.explorer.undo_history.redo_stack.push(change);
                editor.panels.explorer.needs_rescan = true;
                editor.sync_explorer_models(cx);
                cx.notify();
            });
        });
    }

    /// Redo the most recently undone operation, then rescan.
    pub(crate) fn explorer_redo(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(change) = self.panels.explorer.undo_history.redo_stack.pop() else {
            return;
        };
        let weak_editor = cx.entity().downgrade();
        let change_for_execution = change.clone();
        let _ = cx.spawn(async move |_this, cx: &mut AsyncApp| {
            cx.background_executor()
                .spawn(async move { execute_explorer_change(&change_for_execution) })
                .await;
            let _ = weak_editor.update(cx, |editor, cx| {
                editor.panels.explorer.undo_history.undo_stack.push(change);
                editor.panels.explorer.needs_rescan = true;
                editor.sync_explorer_models(cx);
                cx.notify();
            });
        });
    }

    // ── Drag & drop (internal entries and external files) ──────────────

    /// Resolve the drop target directory for an entry: the entry itself for
    /// directories, its parent for files.
    fn explorer_drop_target_dir(&self, entry_id: ExplorerEntryId) -> Option<PathBuf> {
        let entry = self.explorer_entry_by_id(entry_id)?;
        if entry.kind == ExplorerEntryKind::Directory {
            Some(entry.path.clone())
        } else {
            entry.path.parent().map(Path::to_path_buf)
        }
    }

    /// Set the drag target when hovering a row and schedule expansion of a
    /// collapsed directory after 500ms (mirrors Zed's hover-expand).
    pub(crate) fn explorer_drag_hover_entry(
        &mut self,
        entry_id: ExplorerEntryId,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.panels.explorer.drag_target = Some(DragExplorerTarget::Entry(entry_id));
        let Some(entry) = self.explorer_entry_by_id(entry_id) else {
            cx.notify();
            return;
        };
        if entry.kind != ExplorerEntryKind::Directory || entry.is_expanded {
            cx.notify();
            return;
        }
        if self.panels.explorer.hover_expand_task.is_some() {
            cx.notify();
            return;
        }
        let weak_editor = cx.entity().downgrade();
        let task = cx.spawn(async move |_this, cx: &mut AsyncApp| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(500))
                .await;
            let _ = weak_editor.update(cx, |editor, cx| {
                if editor.panels.explorer.drag_target == Some(DragExplorerTarget::Entry(entry_id))
                {
                    editor.toggle_explorer_node(entry_id, cx);
                }
                editor.panels.explorer.hover_expand_task = None;
                cx.notify();
            });
        });
        self.panels.explorer.hover_expand_task = Some(task);
        cx.notify();
    }

    /// Set the drag target for the empty area (targets the explorer root).
    pub(crate) fn explorer_drag_hover_background(&mut self, cx: &mut Context<Self>) {
        self.panels.explorer.drag_target = Some(DragExplorerTarget::Background);
        cx.notify();
    }

    /// Clear all drag state (mouse leave / drop).
    pub(crate) fn clear_explorer_drag(&mut self, cx: &mut Context<Self>) {
        if self.panels.explorer.drag_target.take().is_some()
            || self.panels.explorer.hover_expand_task.take().is_some()
        {
            cx.notify();
        }
    }

    /// Run entry operations (move or copy) on a background thread, record
    /// undo changes, select the last result and rescan.
    fn perform_entry_ops(
        &mut self,
        paths: Vec<PathBuf>,
        target_dir: PathBuf,
        is_cut: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if paths.is_empty() {
            return;
        }
        let weak_editor = cx.entity().downgrade();
        let _ = cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let changes = cx
                .background_executor()
                .spawn(async move { execute_entry_ops(&paths, &target_dir, is_cut) })
                .await;
            let _ = weak_editor.update(cx, |editor, cx| {
                editor.clear_explorer_drag(cx);
                for change in &changes {
                    editor.record_explorer_change(change.clone());
                }
                editor.panels.explorer.needs_rescan = true;
                if let Some(last) = changes.last().and_then(explorer_change_destination) {
                    editor.panels.explorer.pending_select = Some(last.to_path_buf());
                }
                editor.sync_explorer_models(cx);
                cx.notify();
            });
        });
    }

    /// Drop internal dragged entries onto an entry: move by default, copy
    /// with the secondary modifier held (mirrors Zed).
    pub(crate) fn on_explorer_drop_internal(
        &mut self,
        payload: &DraggedExplorerSelection,
        entry_id: ExplorerEntryId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(target_dir) = self.explorer_drop_target_dir(entry_id) else {
            return;
        };
        let paths: Vec<PathBuf> = payload
            .selections
            .iter()
            .filter_map(|selection| match selection {
                ExplorerSelection::File(id) => {
                    self.explorer_entry_by_id(*id).map(|entry| entry.path.clone())
                }
                _ => None,
            })
            .collect();
        if paths.is_empty() {
            return;
        }
        let is_copy = window.modifiers().secondary();
        self.perform_entry_ops(paths, target_dir, !is_copy, window, cx);
    }

    /// Drop external files onto an entry (always a copy, mirrors Zed).
    pub(crate) fn on_explorer_drop_external(
        &mut self,
        paths: &[PathBuf],
        entry_id: ExplorerEntryId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(target_dir) = self.explorer_drop_target_dir(entry_id) else {
            return;
        };
        self.perform_entry_ops(paths.to_vec(), target_dir, false, window, cx);
    }

    /// Drop external files onto the panel background (targets the root).
    pub(crate) fn on_explorer_drop_external_to_root(
        &mut self,
        paths: &[PathBuf],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(root) = self.panels.explorer.root.clone() else {
            return;
        };
        self.perform_entry_ops(paths.to_vec(), root, false, window, cx);
    }

    /// Drop internal dragged entries onto the panel background (targets the
    /// root).
    pub(crate) fn on_explorer_drop_internal_to_root(
        &mut self,
        payload: &DraggedExplorerSelection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(root) = self.panels.explorer.root.clone() else {
            return;
        };
        let paths: Vec<PathBuf> = payload
            .selections
            .iter()
            .filter_map(|selection| match selection {
                ExplorerSelection::File(id) => {
                    self.explorer_entry_by_id(*id).map(|entry| entry.path.clone())
                }
                _ => None,
            })
            .collect();
        if paths.is_empty() {
            return;
        }
        let is_copy = window.modifiers().secondary();
        self.perform_entry_ops(paths, root, !is_copy, window, cx);
    }

    // ── Tree state: background scan, flat list, selection ───────────────

    /// Synchronize the file tree with disk. When the root is unchanged and
    /// the cached scan is still valid, only the flat visible list is
    /// rebuilt; otherwise a **background scan** is started and the result is
    /// swapped in when it completes.
    pub(crate) fn sync_explorer_file_tree(&mut self, cx: &mut Context<Self>) {
        if self.panels.explorer.root.is_none() {
            self.panels.explorer.root = self.explorer_root_for_current_file();
        }

        let Some(root) = self.panels.explorer.root.clone() else {
            self.panels.explorer.selected = None;
            self.panels.explorer.entries.clear();
            return;
        };

        if root.as_os_str().is_empty() {
            self.panels.explorer.file_error = Some("Invalid explorer path: empty path".to_string());
            self.panels.explorer.selected = None;
            return;
        }

        // Keep a recursive filesystem watcher on the root so external
        // changes (and our own saves) refresh the tree automatically.
        self.start_explorer_fs_watch(root.clone(), cx);

        // Same root, cached tree still valid: just re-derive the flat list
        // (also follows the active document's selection).
        if self.panels.explorer.file_tree.is_some()
            && self.panels.explorer.scanned_root.as_deref() == Some(root.as_path())
            && !self.panels.explorer.needs_rescan
        {
            self.select_active_file_in_tree(false);
            self.rebuild_explorer_entries();
            return;
        }

        self.panels.explorer.needs_rescan = false;
        // A new root (first scan or a changed directory) starts expanded so
        // the root row and its title buttons are visible; rescans of the
        // same root preserve the user's fold state.
        if self.panels.explorer.scanned_root.as_deref() != Some(root.as_path()) {
            self.panels.explorer.expanded.insert(ExplorerEntryId::for_path(&root));
        }
        self.panels.explorer.scanned_root = Some(root.clone());
        self.panels.explorer.file_error = None;

        // A scan for this exact root is already in flight: don't cancel it
        // by spawning a replacement (the render path calls this every
        // frame, which would otherwise starve the scan forever).
        if self.panels.explorer.scan_task.is_some() {
            return;
        }

        let options = explorer_scan_options_from_settings(
            &crate::infra::config::settings::ExplorerSettingsStore::settings(cx),
        );
        let weak_editor = cx.entity().downgrade();
        let task = cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let result = cx
                .background_executor()
                .spawn(async move { scan_explorer_dir(&root, &options) })
                .await;
            let _ = weak_editor.update(cx, |editor, cx| {
                // The scan finished: free the slot so the next sync can
                // start a fresh scan (e.g. for a changed root).
                editor.panels.explorer.scan_task = None;
                match result {
                    Ok(tree) => {
                        let mut dir_ids = std::collections::HashSet::new();
                        collect_directory_ids(&tree, &mut dir_ids);
                        editor.panels.explorer.expanded.retain(|id| dir_ids.contains(id));
                        editor.panels.explorer.file_tree = Some(tree);
                        editor.panels.explorer.file_error = None;
                        editor.select_active_file_in_tree(true);
                        editor.rebuild_explorer_entries();
                        editor.autoscroll_explorer_selection();
                    }
                    Err(err) => {
                        editor.panels.explorer.file_tree = None;
                        editor.panels.explorer.entries.clear();
                        editor.panels.explorer.file_error = Some(err.to_string());
                    }
                }
                cx.notify();
            });
        });
        self.panels.explorer.scan_task = Some(task);
    }

    /// Start (or restart) a recursive filesystem watcher on `root`. Watch
    /// events are debounced into a single background rescan by
    /// [`Self::on_explorer_fs_event`]. The watcher lives inside a spawned
    /// task; dropping the task stops the watcher.
    fn start_explorer_fs_watch(&mut self, root: PathBuf, cx: &mut Context<Self>) {
        if self.panels.explorer.watched_root.as_deref() == Some(root.as_path()) {
            return;
        }
        self.panels.explorer.watched_root = Some(root.clone());
        self.panels.explorer.fs_watch_task = None; // drops the old watcher
        let weak_editor = cx.entity().downgrade();
        let task = cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let (tx, mut rx) = futures::channel::mpsc::unbounded::<notify::Event>();
            let mut watcher = match notify::recommended_watcher(
                move |res: notify::Result<notify::Event>| {
                    if let Ok(event) = res {
                        let _ = tx.unbounded_send(event);
                    }
                },
            ) {
                Ok(watcher) => watcher,
                Err(err) => {
                    eprintln!("[explorer] failed to start fs watcher: {err}");
                    return;
                }
            };
            if let Err(err) = watcher.watch(&root, notify::RecursiveMode::Recursive) {
                eprintln!("[explorer] failed to watch '{}': {err}", root.display());
                return;
            }
            while rx.next().await.is_some() {
                let _ = weak_editor.update(cx, |editor, cx| {
                    editor.on_explorer_fs_event(cx);
                });
            }
        });
        self.panels.explorer.fs_watch_task = Some(task);
    }

    /// Debounce filesystem events into a single background rescan.
    fn on_explorer_fs_event(&mut self, cx: &mut Context<Self>) {
        if self.panels.explorer.fs_refresh_task.is_some() {
            return;
        }
        let weak_editor = cx.entity().downgrade();
        let task = cx.spawn(async move |_this, cx: &mut AsyncApp| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(250))
                .await;
            let _ = weak_editor.update(cx, |editor, cx| {
                if editor.panels.explorer.is_open && editor.panels.explorer.root.is_some() {
                    editor.panels.explorer.needs_rescan = true;
                    editor.sync_explorer_models(cx);
                }
                cx.notify();
            });
        });
        self.panels.explorer.fs_refresh_task = Some(task);
    }

    /// Re-derive the flat visible row list from the cached tree, splicing in
    /// the inline edit row when an edit is active.
    pub(crate) fn rebuild_explorer_entries(&mut self) {
        let explorer = &mut self.panels.explorer;
        let entries = explorer
            .file_tree
            .as_ref()
            .map(|tree| {
                let expanded = explorer.expanded.clone();
                let edit = explorer.edit.as_ref();
                build_explorer_rows(tree, &expanded, edit)
            })
            .unwrap_or_default();
        explorer.entries = entries;
    }

    /// Follow the active document (or a pending inline-create target) in the
    /// tree. With `reveal`, ancestor directories are expanded so the entry
    /// becomes visible.
    ///
    /// An existing file selection is preserved when it is still in the tree
    /// (filesystem-driven rescans must not steal the user's selection); only
    /// a pending target or a missing selection falls back to the active
    /// document.
    fn select_active_file_in_tree(&mut self, reveal: bool) {
        if matches!(
            self.panels.explorer.selected,
            Some(ExplorerSelection::Outline(_))
        ) {
            self.panels.explorer.pending_select = None;
            return;
        }
        let Some(tree) = self.panels.explorer.file_tree.clone() else {
            return;
        };
        let pending = self.panels.explorer.pending_select.take();
        if let Some(path) = pending {
            if let Some(node) = find_explorer_node(&tree, &path) {
                self.panels.explorer.selected = Some(ExplorerSelection::File(node.id));
                if reveal {
                    self.expand_to_path(&path);
                }
            }
            return;
        }
        // Keep an existing file selection when the entry is still visible.
        if let Some(ExplorerSelection::File(id)) = self.panels.explorer.selected {
            if explorer_tree_contains_id(&tree, id) {
                return;
            }
        }
        let Some(path) = self.active_editor_tab().and_then(|tab| tab.file.path.clone()) else {
            return;
        };
        if let Some(node) = find_explorer_node(&tree, &path) {
            self.panels.explorer.selected = Some(ExplorerSelection::File(node.id));
        }
    }

    /// Expand every ancestor directory of `path` that exists in the tree
    /// (mirrors Zed's `expand_to_selection`). The root row itself is always
    /// expanded so the target can become visible.
    pub(crate) fn expand_to_path(&mut self, path: &Path) {
        let Some(root) = self.panels.explorer.root.clone() else {
            return;
        };
        let Some(tree) = self.panels.explorer.file_tree.clone() else {
            return;
        };
        // The root row must be expanded for anything below it to show.
        self.panels.explorer.expanded.insert(tree.id);
        let mut ancestors = Vec::new();
        for ancestor in path.ancestors() {
            if ancestor == root.as_path() {
                break;
            }
            ancestors.push(ancestor.to_path_buf());
        }
        ancestors.reverse(); // root → leaf
        for ancestor in ancestors {
            if let Some(node) = find_explorer_node(&tree, &ancestor) {
                if node.kind == ExplorerEntryKind::Directory {
                    self.panels.explorer.expanded.insert(node.id);
                }
            }
        }
    }

    /// Center the selected file row in the virtualized list.
    pub(crate) fn autoscroll_explorer_selection(&self) {
        let Some(ExplorerSelection::File(id)) = self.panels.explorer.selected else {
            return;
        };
        let Some(index) = self
            .panels
            .explorer
            .entries
            .iter()
            .position(|row| matches!(row, ExplorerRow::Entry(entry) if entry.id == id))
        else {
            return;
        };
        self.panels
            .explorer
            .scroll_handle
            .scroll_to_item(index, ScrollStrategy::Center);
    }

    // ── Selection navigation and scrolling (mirrors Zed) ───────────────

    /// Row index of the currently selected file entry, if visible.
    fn explorer_selected_row_index(&self) -> Option<usize> {
        match &self.panels.explorer.selected {
            Some(ExplorerSelection::File(id)) => self
                .panels
                .explorer
                .entries
                .iter()
                .position(|row| matches!(row, ExplorerRow::Entry(entry) if entry.id == *id)),
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
        let selection = ExplorerSelection::File(entry.id);
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
        let Some(parent_index) = self.panels.explorer.entries.iter().position(|row| {
            matches!(row, ExplorerRow::Entry(e) if e.id == parent_id)
        }) else {
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

    pub(crate) fn toggle_explorer_node(&mut self, id: ExplorerEntryId, cx: &mut Context<Self>) {
        if !self.panels.explorer.expanded.remove(&id) {
            self.panels.explorer.expanded.insert(id);
        }
        self.rebuild_explorer_entries();
        cx.notify();
    }
    pub(crate) fn toggle_outline_node(&mut self, id: &str, cx: &mut Context<Self>) {
        if !self.panels.explorer.expanded_outline.remove(id) {
            self.panels.explorer.expanded_outline.insert(id.to_string());
        }
        cx.notify();
    }
    pub(crate) fn open_explorer_file(
        &mut self,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Key the selection by the entry's stable id when the tree knows it;
        // fall back to the path-derived id (harmless: no row will highlight).
        let id = self
            .panels
            .explorer
            .file_tree
            .as_ref()
            .and_then(|tree| find_explorer_node(tree, &path))
            .map(|node| node.id)
            .unwrap_or_else(|| ExplorerEntryId::for_path(&path));
        self.panels.explorer.selected = Some(ExplorerSelection::File(id));
        // Reveal: expand ancestor directories and center the row.
        self.expand_to_path(&path);
        self.rebuild_explorer_entries();
        self.autoscroll_explorer_selection();
        // Explorer interacts with the ACTIVE editor: the file opens in its
        // tab bar. With no Editor area present the click is ignored.
        if self.panels.layout.active_editor_area.is_none() {
            return;
        }
        self.open_file_in_active_editor(&path, window, cx);
    }

    // ── Render ──────────────────────────────────────────────────────────

    pub(crate) fn render_explorer_files_tree(
        &mut self,
        area_id: usize,
        theme: &Theme,
        strings: &I18nStrings,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        // Recent files and folders give the empty state a quick-open entry
        // point; stale history entries are filtered out so clicks never fail.
        let recent_folders = read_recent_folders()
            .unwrap_or_default()
            .into_iter()
            .filter(|path| path.is_dir())
            .take(5)
            .collect::<Vec<_>>();
        let recent_files = read_recent_files()
            .unwrap_or_default()
            .into_iter()
            .filter(|path| path.is_file())
            .take(5)
            .collect::<Vec<_>>();

        if self.panels.explorer.root.is_none() {
            return self.render_explorer_empty_state(
                "Explorer is empty now",
                "",
                area_id,
                theme,
                strings,
                &recent_folders,
                &recent_files,
                cx,
            );
        }

        if let Some(error) = self.panels.explorer.file_error.as_ref() {
            return self.render_explorer_empty_state(
                "Explorer is empty now",
                error,
                area_id,
                theme,
                strings,
                &recent_folders,
                &recent_files,
                cx,
            );
        }

        if self.panels.explorer.file_tree.is_none() {
            return self.render_explorer_empty_state(
                "Explorer is empty now",
                "",
                area_id,
                theme,
                strings,
                &recent_folders,
                &recent_files,
                cx,
            );
        }

        let c = &theme.colors;

        // Virtualized row list: only the visible range is rendered. Rows
        // must be uniform height (`EXPLORER_NODE_HEIGHT`). The root row is
        // the first row; its title buttons live on the row itself (shown
        // only while the root is expanded, VSCode-style).
        let entries_len = self.panels.explorer.entries.len();
        let scroll_handle = self.panels.explorer.scroll_handle.clone();
        let row_theme = theme.clone();
        let row_editor = cx.entity().downgrade();
        let list = uniform_list(
            ("explorer-tree", area_id),
            entries_len,
            cx.processor(
                move |this: &mut Editor, range: Range<usize>, _window, cx| {
                    this.panels.explorer.rendered_rows = range.len();
                    let mut items = Vec::with_capacity(range.len());
                    for index in range {
                        if let Some(row) = this.panels.explorer.entries.get(index) {
                            items.push(this.render_explorer_row(
                                row,
                                area_id,
                                &row_theme,
                                &row_editor,
                                cx,
                            ));
                        }
                    }
                    items
                },
            ),
        )
        .track_scroll(scroll_handle)
        .flex_1()
        .min_h(px(0.0))
        .py(px(4.0));

        div()
            .id(("explorer-root", area_id))
            .key_context("ExplorerPanel")
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .bg(if matches!(
                &self.panels.explorer.drag_target,
                Some(DragExplorerTarget::Background)
            ) {
                c.dialog_secondary_button_hover
            } else {
                hsla(0.0, 0.0, 0.0, 0.0)
            })
            .on_action(cx.listener(Self::on_explorer_select_previous))
            .on_action(cx.listener(Self::on_explorer_select_next))
            .on_action(cx.listener(Self::on_explorer_select_parent))
            .on_action(cx.listener(Self::on_explorer_select_first))
            .on_action(cx.listener(Self::on_explorer_select_last))
            .on_action(cx.listener(Self::on_explorer_scroll_up))
            .on_action(cx.listener(Self::on_explorer_scroll_down))
            .on_action(cx.listener(Self::on_explorer_scroll_cursor_center))
            .on_action(cx.listener(Self::on_explorer_scroll_cursor_top))
            .on_action(cx.listener(Self::on_explorer_scroll_cursor_bottom))
            // Dropping on the background targets the explorer root.
            .on_drag_move::<ExternalPaths>(cx.listener(|editor, _paths, _window, cx| {
                editor.explorer_drag_hover_background(cx);
            }))
            .on_drop::<ExternalPaths>(cx.listener::<ExternalPaths>(
                |editor, paths, window, cx| {
                    editor.on_explorer_drop_external_to_root(paths.paths(), window, cx);
                },
            ))
            .on_drag_move::<DraggedExplorerSelection>(
                cx.listener(|editor, _payload, _window, cx| {
                    editor.explorer_drag_hover_background(cx);
                }),
            )
            .on_drop::<DraggedExplorerSelection>(
                cx.listener(|editor, payload, window, cx| {
                    editor.on_explorer_drop_internal_to_root(payload, window, cx);
                }),
            )
            .child(list)
            .into_any_element()
    }

    /// Render one file-tree row for the virtualized list: either a visible
    /// entry or the inline edit row.
    pub(crate) fn render_explorer_row(
        &self,
        row: &ExplorerRow,
        area_id: usize,
        theme: &Theme,
        editor: &WeakEntity<Editor>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match row {
            ExplorerRow::Entry(entry) if entry.parent_id.is_none() => {
                self.render_explorer_root_row(entry, area_id, theme, editor, cx)
            }
            ExplorerRow::Entry(entry) => {
                self.render_explorer_entry_row(entry, area_id, theme, editor, cx)
            }
            ExplorerRow::Edit => self.render_explorer_edit_row(area_id, theme, editor, cx),
        }
    }

    /// Render the root row: the folder name plus the title buttons (new
    /// file / new folder / refresh / collapse all / toggle hidden). The
    /// buttons are shown only while the root is expanded (VSCode-style
    /// title row); collapsing the root hides them.
    pub(crate) fn render_explorer_root_row(
        &self,
        entry: &VisibleExplorerEntry,
        area_id: usize,
        theme: &Theme,
        editor: &WeakEntity<Editor>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let t = &theme.typography;
        let selected = matches!(
            &self.panels.explorer.selected,
            Some(ExplorerSelection::File(id)) if *id == entry.id
        );
        let is_drag_target = matches!(
            &self.panels.explorer.drag_target,
            Some(DragExplorerTarget::Entry(id)) if *id == entry.id
        );
        let is_expanded = entry.is_expanded;
        let node_id = entry.id;
        let click_editor = editor.clone();
        let right_click_editor = editor.clone();
        let right_click_path = entry.path.clone();
        let arrow_node_id = entry.id;
        let arrow_editor = editor.clone();
        let mark_selection = ExplorerSelection::File(entry.id);
        let drag_entry_id = entry.id;

        let mut arrow_el = div()
            .w(px(14.0))
            .h(px(18.0))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center();

        if entry.has_children {
            arrow_el = arrow_el
                .cursor_pointer()
                .child(
                    svg()
                        .path(if is_expanded {
                            "icon/panel/chevron-down.svg"
                        } else {
                            "icon/panel/chevron-right.svg"
                        })
                        .size(px(12.0))
                        .text_color(c.dialog_muted),
                )
                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                    let _ = arrow_editor.update(cx, |editor, cx| {
                        editor.toggle_explorer_node(arrow_node_id, cx);
                    });
                    cx.stop_propagation();
                });
        }

        let ed_open = editor.clone();
        let ed_refresh = editor.clone();
        let ed_collapse = editor.clone();

        // Title buttons: visible only while the root row is expanded. The
        // set matches the panel toolbar minus new file / new folder, which
        // live on the row context menu (Zed layout).
        let buttons = if is_expanded {
            div()
                .flex()
                .items_center()
                .gap(px(2.0))
                .child(
                    icon_chip_button(c, &theme.dimensions)
                        .id(("ws-tb-open", area_id))
                        .child(
                            svg()
                                .path("icon/explorer/folder-open.svg")
                                .size(px(13.0))
                                .text_color(c.text_default),
                        )
                        .on_click(move |_ev, window, cx| {
                            let _ = ed_open.update(cx, |ed, cx| {
                                ed.prompt_open_explorer_folder(window, cx);
                            });
                            cx.stop_propagation();
                        }),
                )
                .child(
                    icon_chip_button(c, &theme.dimensions)
                        .id(("ws-tb-refresh", area_id))
                        .child(
                            svg()
                                .path("icon/explorer/refresh.svg")
                                .size(px(13.0))
                                .text_color(c.text_default),
                        )
                        .on_click(move |_ev, _window, cx| {
                            let _ = ed_refresh.update(cx, |ed, cx| {
                                ed.refresh_explorer_tree(cx);
                            });
                            cx.stop_propagation();
                        }),
                )
                .child(
                    icon_chip_button(c, &theme.dimensions)
                        .id(("ws-tb-collapse", area_id))
                        .child(
                            svg()
                                .path("icon/explorer/collapse-all.svg")
                                .size(px(13.0))
                                .text_color(c.text_default),
                        )
                        .on_click(move |_ev, _window, cx| {
                            let _ = ed_collapse.update(cx, |ed, cx| {
                                ed.collapse_all_explorer_nodes(cx);
                            });
                            cx.stop_propagation();
                        }),
                )
                .into_any_element()
        } else {
            div().into_any_element()
        };

        div()
            .id(ElementId::Name(format!("explorer-root-row-{area_id}").into()))
            .h(px(EXPLORER_NODE_HEIGHT))
            .w_full()
            .overflow_hidden()
            .flex()
            .items_center()
            .gap(px(6.0))
            .pl(px(6.0))
            .pr(px(4.0))
            .bg(if is_drag_target {
                c.callout_tip_bg
            } else if selected {
                c.panel_row_selected
            } else {
                hsla(0.0, 0.0, 0.0, 0.0)
            })
            .hover(|this| this.bg(c.panel_row_hover))
            .cursor_pointer()
            .child(arrow_el)
            .child(
                svg()
                    .path(FOLDER_ICON)
                    .size(px(17.0))
                    .flex_shrink_0()
                    .text_color(c.text_default),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .truncate()
                    .text_size(px(t.text_size * 0.9))
                    .font_weight(FontWeight::BOLD)
                    .line_height(px(t.text_size * t.text_line_height))
                    .text_color(if selected {
                        c.text_default
                    } else {
                        c.dialog_muted
                    })
                    .child(entry.label.clone()),
            )
            .child(buttons)
            .on_mouse_down(MouseButton::Right, {
                let right_click_selection = mark_selection.clone();
                move |event, _window, cx| {
                    let path = right_click_path.clone();
                    let selection = right_click_selection.clone();
                    let _ = right_click_editor.update(cx, |editor, cx| {
                        // Right-click selects the row (indicator feedback,
                        // mirroring Zed's `deploy_context_menu`).
                        editor.panels.explorer.selected = Some(selection);
                        editor.open_explorer_file_context_menu(event.position, path, true, cx);
                        cx.notify();
                    });
                    cx.stop_propagation();
                }
            })
            .on_click(move |event, _window, cx| {
                let id = node_id;
                let selection = mark_selection.clone();
                let shift = event.modifiers().shift;
                let alt = event.modifiers().alt;
                let _ = click_editor.update(cx, |editor, cx| {
                    if shift {
                        editor.select_explorer_range(id, cx);
                        return;
                    }
                    if alt {
                        editor.toggle_explorer_mark(selection, cx);
                        return;
                    }
                    editor.panels.explorer.marked.clear();
                    editor.toggle_explorer_node(id, cx);
                });
            })
            // Drag & drop support (identical to directory rows).
            .on_drag_move::<ExternalPaths>(cx.listener(move |editor, _paths, window, cx| {
                editor.explorer_drag_hover_entry(drag_entry_id, window, cx);
            }))
            .on_drop::<ExternalPaths>(cx.listener::<ExternalPaths>(
                move |editor, paths, window, cx| {
                    editor.on_explorer_drop_external(paths.paths(), drag_entry_id, window, cx);
                },
            ))
            .on_drag_move::<DraggedExplorerSelection>(
                cx.listener(move |editor, _payload, window, cx| {
                    editor.explorer_drag_hover_entry(drag_entry_id, window, cx);
                }),
            )
            .on_drop::<DraggedExplorerSelection>(cx.listener(move |editor, payload, window, cx| {
                editor.on_explorer_drop_internal(payload, drag_entry_id, window, cx);
            }))
            .into_any_element()
    }

    /// Render one flat file-tree entry row.
    pub(crate) fn render_explorer_entry_row(
        &self,
        entry: &VisibleExplorerEntry,
        area_id: usize,
        theme: &Theme,
        editor: &WeakEntity<Editor>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let t = &theme.typography;
        let selected = matches!(
            &self.panels.explorer.selected,
            Some(ExplorerSelection::File(id)) if *id == entry.id
        );
        let is_marked = self
            .panels
            .explorer
            .marked
            .contains(&ExplorerSelection::File(entry.id));
        let is_drag_target = matches!(
            &self.panels.explorer.drag_target,
            Some(DragExplorerTarget::Entry(id)) if *id == entry.id
        );
        let node_id = entry.id;
        let click_editor = editor.clone();
        let click_kind = entry.kind;
        let click_path = entry.path.clone();
        let right_click_editor = editor.clone();
        let right_click_path = entry.path.clone();
        let right_click_is_dir = entry.kind == ExplorerEntryKind::Directory;
        let arrow_node_id = entry.id;
        let arrow_editor = editor.clone();
        let mark_selection = ExplorerSelection::File(entry.id);
        // Drag payload: the effective selection (marks + this row).
        let drag_ids = {
            let mut ids = self.effective_explorer_entries();
            if !ids.contains(&entry.id) {
                ids.push(entry.id);
            }
            ids
        };
        let drag_payload = DraggedExplorerSelection {
            selections: drag_ids
                .iter()
                .copied()
                .map(ExplorerSelection::File)
                .collect(),
        };
        let drag_label = entry.label.clone();
        let drag_entry_id = entry.id;

        let icon = match entry.kind {
            ExplorerEntryKind::Directory => Some((FOLDER_ICON, c.text_default)),
            ExplorerEntryKind::MarkdownFile => Some((MARKDOWN_ICON, c.text_default)),
            ExplorerEntryKind::File => {
                let ext = entry
                    .path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                match ext.as_str() {
                    "md" => Some((MARKDOWN_ICON, c.text_default)),
                    _ => Some((FILE_ICON, c.text_default)),
                }
            }
        };

        let label_color = if selected {
            c.text_default
        } else {
            c.dialog_muted
        };

        let mut arrow_el = div()
            .w(px(14.0))
            .h(px(18.0))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center();

        if entry.has_children {
            arrow_el = arrow_el
                .cursor_pointer()
                .child(
                    svg()
                        .path(if entry.is_expanded {
                            "icon/panel/chevron-down.svg"
                        } else {
                            "icon/panel/chevron-right.svg"
                        })
                        .size(px(12.0))
                        .text_color(c.dialog_muted),
                )
                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                    let _ = arrow_editor.update(cx, |editor, cx| {
                        editor.toggle_explorer_node(arrow_node_id, cx);
                    });
                    cx.stop_propagation();
                });
        }

        div()
            .id(ElementId::Name(
                format!("explorer-node-{area_id}-{}", node_id.0).into(),
            ))
            .h(px(EXPLORER_NODE_HEIGHT))
            .w_full()
            .overflow_hidden()
            .flex()
            .items_center()
            .gap(px(6.0))
            .pl(px(6.0 + entry.depth as f32 * EXPLORER_NODE_INDENT))
            .pr(px(8.0))
            .bg(if is_drag_target {
                c.callout_tip_bg
            } else if is_marked {
                c.callout_note_bg
            } else if selected {
                c.panel_row_selected
            } else {
                hsla(0.0, 0.0, 0.0, 0.0)
            })
            .hover(|this| this.bg(c.panel_row_hover))
            .cursor_pointer()
            .child(arrow_el)
            .children(icon.map(|(path, color)| {
                svg()
                    .path(path)
                    .size(px(17.0))
                    .flex_shrink_0()
                    .text_color(color)
                    .into_any_element()
            }))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .truncate()
                    .text_size(px(t.text_size * 0.9))
                    .line_height(px(t.text_size * t.text_line_height))
                    .text_color(label_color)
                    .child(entry.label.clone()),
            )
            .on_mouse_down(MouseButton::Right, {
                let right_click_selection = mark_selection.clone();
                move |event, _window, cx| {
                    let path = right_click_path.clone();
                    let is_dir = right_click_is_dir;
                    let selection = right_click_selection.clone();
                    let _ = right_click_editor.update(cx, |editor, cx| {
                        // Right-click selects the row (indicator feedback,
                        // mirroring Zed's `deploy_context_menu`).
                        editor.panels.explorer.selected = Some(selection);
                        editor.open_explorer_file_context_menu(event.position, path, is_dir, cx);
                        cx.notify();
                    });
                    cx.stop_propagation();
                }
            })
            .on_click(move |event, window, cx| {
                let id = node_id;
                let kind = click_kind;
                let path = click_path.clone();
                let selection = mark_selection.clone();
                let shift = event.modifiers().shift;
                let alt = event.modifiers().alt;
                let _ = click_editor.update(cx, |editor, cx| {
                    if shift {
                        editor.select_explorer_range(id, cx);
                        return;
                    }
                    if alt {
                        editor.toggle_explorer_mark(selection, cx);
                        return;
                    }
                    editor.panels.explorer.marked.clear();
                    match kind {
                        ExplorerEntryKind::Directory => {
                            editor.toggle_explorer_node(id, cx);
                        }
                        ExplorerEntryKind::MarkdownFile | ExplorerEntryKind::File => {
                            editor.open_explorer_file(path, window, cx);
                        }
                    }
                });
            })
            // Drag & drop: external files are copied; internal entries are
            // moved by default and copied with the secondary modifier.
            .on_drag_move::<ExternalPaths>(cx.listener(move |editor, _paths, window, cx| {
                editor.explorer_drag_hover_entry(drag_entry_id, window, cx);
            }))
            .on_drop::<ExternalPaths>(cx.listener::<ExternalPaths>(
                move |editor, paths, window, cx| {
                    editor.on_explorer_drop_external(paths.paths(), drag_entry_id, window, cx);
                },
            ))
            .on_drag_move::<DraggedExplorerSelection>(
                cx.listener(move |editor, _payload, window, cx| {
                    editor.explorer_drag_hover_entry(drag_entry_id, window, cx);
                }),
            )
            .on_drop::<DraggedExplorerSelection>(cx.listener(move |editor, payload, window, cx| {
                editor.on_explorer_drop_internal(payload, drag_entry_id, window, cx);
            }))
            .on_drag(drag_payload, move |payload, click_offset, _window, cx| {
                let label = drag_label.clone();
                let count = payload.selections.len();
                cx.new(|_| DraggedExplorerEntryView {
                    label,
                    count,
                    click_offset,
                })
            })
            .into_any_element()
    }

    /// Render the inline create/rename row: a filename input with keyboard
    /// handling, IME bridge, and live validation feedback.
    pub(crate) fn render_explorer_edit_row(
        &self,
        area_id: usize,
        theme: &Theme,
        _editor: &WeakEntity<Editor>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(edit) = self.panels.explorer.edit.as_ref() else {
            return div().into_any_element();
        };
        let c = &theme.colors;
        let t = &theme.typography;
        let depth = edit.depth;
        let is_dir = edit.is_dir;
        let validation = edit.validation.clone();
        let focus_handle = edit.filename.focus_handle.clone().unwrap();

        let icon = if is_dir {
            (FOLDER_ICON, c.text_default)
        } else {
            (MARKDOWN_ICON, c.text_default)
        };

        let validation_label = match validation {
            Some(ExplorerValidation::Warning(message)) => Some((
                message,
                c.callout_warning_border,
            )),
            Some(ExplorerValidation::Error(message)) => Some((message, c.callout_caution_border)),
            None => None,
        };

        div()
            .id(ElementId::Name(format!("explorer-edit-{area_id}").into()))
            .h(px(EXPLORER_NODE_HEIGHT))
            .w_full()
            .overflow_hidden()
            .flex()
            .items_center()
            .gap(px(6.0))
            .pl(px(6.0 + depth as f32 * EXPLORER_NODE_INDENT))
            .pr(px(8.0))
            .bg(c.dialog_secondary_button_hover)
            // Arrow placeholder keeps the row aligned with siblings.
            .child(
                div()
                    .w(px(14.0))
                    .h(px(18.0))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(svg().size(px(12.0))),
            )
            .child(
                svg()
                    .path(icon.0)
                    .size(px(17.0))
                    .flex_shrink_0()
                    .text_color(icon.1),
            )
            .child(
                div()
                    .id(("explorer-filename-input-box", area_id))
                    .key_context("ExplorerFilenameInput")
                    .track_focus(&focus_handle)
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .items_center()
                    .on_key_down(cx.listener(Self::on_explorer_filename_key_down))
                    .on_action(cx.listener(Self::on_explorer_filename_copy))
                    .on_action(cx.listener(Self::on_explorer_filename_cut))
                    .on_action(cx.listener(Self::on_explorer_filename_paste))
                    .child(ExplorerFilenameInputElement {
                        editor: cx.entity(),
                    }),
            )
            .children(validation_label.map(|(message, color)| {
                div()
                    .max_w(px(160.0))
                    .truncate()
                    .text_size(px(t.text_size * 0.72))
                    .text_color(color)
                    .child(message)
                    .into_any_element()
            }))
            .into_any_element()
    }

    pub(crate) fn render_explorer_empty_state(
        &self,
        title: &str,
        message: &str,
        area_id: usize,
        theme: &Theme,
        strings: &I18nStrings,
        recent_folders: &[PathBuf],
        recent_files: &[PathBuf],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let click_editor = cx.entity().downgrade();

        let display_title = if title.is_empty() {
            "Explorer is empty now"
        } else {
            title
        };

        // An empty message means the empty state has no hint line at all;
        // non-empty messages (e.g. scan errors) are still rendered.
        let has_message = !message.is_empty();

        empty_state_container()
            .gap(px(10.0))
            .px(px(24.0))
            .child(
                svg()
                    .path("icon/explorer/folder-open.svg")
                    .size(px(36.0))
                    .text_color(c.dialog_muted),
            )
            .child(
                div()
                    .text_size(px(13.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(c.text_default)
                    .child(display_title.to_string()),
            )
            .child(if has_message {
                div()
                    .max_w(px(230.0))
                    .text_size(px(t.text_size * 0.78))
                    .line_height(px(t.text_size * t.text_line_height * 0.90))
                    .text_color(c.dialog_muted)
                    .child(message.to_string())
            } else {
                div()
            })
            .child(
                div()
                    .id(("explorer-empty-open-btn", area_id))
                    .cursor_pointer()
                    .mt(px(4.0))
                    .h(px(28.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(6.0))
                    .rounded(px(d.menu_item_radius))
                    .border_1()
                    .border_color(c.dialog_border)
                    .bg(c.dialog_secondary_button_bg)
                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                    .active(|this| this.opacity(0.92))
                    .child(
                        svg()
                            .path("icon/explorer/folder-open.svg")
                            .size(px(14.0))
                            .text_color(c.dialog_secondary_button_text),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(c.dialog_secondary_button_text)
                            .child("Open Folder"),
                    )
                    .on_click(move |_ev, window, cx| {
                        let _ = click_editor.update(cx, |ed, cx| {
                            ed.prompt_open_explorer_folder(window, cx);
                        });
                    }),
            )
            .child(
                // Recent folders and files quick-open list under the button;
                // hidden when both histories are empty or the state carries
                // an error message.
                if (recent_folders.is_empty() && recent_files.is_empty()) || has_message {
                    div()
                } else {
                    div()
                        .mt(px(16.0))
                        .w_full()
                        .flex()
                        .flex_col()
                        .items_start()
                        .gap(px(2.0))
                        .child(
                            div()
                                .ml(px(10.0))
                                .text_size(px(13.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(c.dialog_muted)
                                .child(strings.explorer_recent_title.clone()),
                        )
                        .children(recent_folders.iter().map(|path| {
                            let folder_name = path
                                .file_name()
                                .map(|name| name.to_string_lossy().to_string())
                                .unwrap_or_else(|| path.to_string_lossy().to_string());
                            let ed = cx.entity().downgrade();
                            let path = path.clone();
                            div()
                                .id(ElementId::Name(
                                    format!(
                                        "explorer-recent-folder-{}-{}",
                                        area_id,
                                        path.display()
                                    )
                                    .into(),
                                ))
                                .cursor_pointer()
                                .px(px(10.0))
                                .py(px(2.0))
                                .rounded(px(d.menu_item_radius))
                                .hover(|this| this.bg(c.panel_row_hover))
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .child(
                                    svg()
                                        .path("icon/explorer/folder.svg")
                                        .size(px(12.0))
                                        .text_color(c.dialog_muted),
                                )
                                .child(
                                    div()
                                        .max_w(px(190.0))
                                        .truncate()
                                        .text_size(px(12.0))
                                        .text_color(c.dialog_muted)
                                        .hover(|this| this.text_color(c.text_default))
                                        .child(folder_name),
                                )
                                .on_click(move |_, _window, cx| {
                                    let _ = ed.update(cx, |editor, cx| {
                                        editor.open_explorer_folder_path(path.clone(), cx);
                                    });
                                })
                        }))
                        .children(recent_files.iter().map(|path| {
                            let file_name = path
                                .file_name()
                                .map(|name| name.to_string_lossy().to_string())
                                .unwrap_or_else(|| path.to_string_lossy().to_string());
                            let ed = cx.entity().downgrade();
                            let path = path.clone();
                            div()
                                .id(ElementId::Name(
                                    format!("explorer-recent-{}-{}", area_id, path.display()).into(),
                                ))
                                .cursor_pointer()
                                .px(px(10.0))
                                .py(px(2.0))
                                .rounded(px(d.menu_item_radius))
                                .hover(|this| this.bg(c.panel_row_hover))
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .child(
                                    svg()
                                        .path("icon/explorer/markdown.svg")
                                        .size(px(12.0))
                                        .text_color(c.dialog_muted),
                                )
                                .child(
                                    div()
                                        .max_w(px(190.0))
                                        .truncate()
                                        .text_size(px(12.0))
                                        .text_color(c.dialog_muted)
                                        .hover(|this| this.text_color(c.text_default))
                                        .child(file_name),
                                )
                                .on_click(move |_, window, cx| {
                                    let _ = ed.update(cx, |editor, cx| {
                                        editor.open_explorer_file(path.clone(), window, cx);
                                    });
                                })
                        }))
                },
            )
            .into_any_element()
    }

    // ── Outline rendering (non-virtualized; heading trees are small) ────

    pub(crate) fn render_explorer_nodes(
        &self,
        nodes: &[ExplorerNode],
        depth: usize,
        area_id: usize,
        theme: &Theme,
        editor: &WeakEntity<Editor>,
    ) -> Vec<AnyElement> {
        let mut elements = Vec::new();
        for node in nodes {
            elements.push(self.render_explorer_node(node, depth, area_id, theme, editor));
            if !node.children.is_empty()
                && self.panels.explorer.expanded_outline.contains(&node.id)
            {
                elements.extend(self.render_explorer_nodes(
                    &node.children,
                    depth + 1,
                    area_id,
                    theme,
                    editor,
                ));
            }
        }
        elements
    }
    pub(crate) fn render_explorer_node(
        &self,
        node: &ExplorerNode,
        depth: usize,
        area_id: usize,
        theme: &Theme,
        editor: &WeakEntity<Editor>,
    ) -> AnyElement {
        let c = &theme.colors;
        let t = &theme.typography;
        let is_expanded = self.panels.explorer.expanded_outline.contains(&node.id);
        let has_children = !node.children.is_empty();
        let selected = matches!(
            &self.panels.explorer.selected,
            Some(ExplorerSelection::Outline(id)) if id == &node.id
        );
        let node_id = node.id.clone();
        let click_editor = editor.clone();
        let click_kind = node.kind.clone();
        let arrow_node_id = node.id.clone();
        let arrow_editor = editor.clone();

        let heading_badge = match &node.kind {
            ExplorerNodeKind::Heading { level, .. } => {
                let badge_color = match level {
                    1 => c.callout_note_border,
                    2 => c.callout_tip_border,
                    3 => c.callout_important_border,
                    4 => c.callout_warning_border,
                    5 => c.callout_caution_border,
                    _ => c.dialog_muted,
                };
                Some(
                    div()
                        .px(px(4.0))
                        .py(px(1.0))
                        .rounded(px(3.0))
                        .text_size(px(10.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(badge_color)
                        .bg(badge_color.opacity(0.12))
                        .child(format!("H{level}")),
                )
            }
        };

        let label_color = if selected {
            c.text_default
        } else {
            c.dialog_muted
        };

        let mut arrow_el = div()
            .w(px(14.0))
            .h(px(18.0))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center();

        if has_children {
            arrow_el = arrow_el
                .cursor_pointer()
                .child(
                    svg()
                        .path(if is_expanded {
                            "icon/panel/chevron-down.svg"
                        } else {
                            "icon/panel/chevron-right.svg"
                        })
                        .size(px(12.0))
                        .text_color(c.dialog_muted),
                )
                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                    let _ = arrow_editor.update(cx, |editor, cx| {
                        editor.toggle_outline_node(&arrow_node_id, cx);
                    });
                    cx.stop_propagation();
                });
        }

        div()
            .id(ElementId::Name(
                format!("explorer-node-{area_id}-{}", stable_node_hash(&node.id)).into(),
            ))
            .h(px(EXPLORER_NODE_HEIGHT))
            .w_full()
            .overflow_hidden()
            .flex()
            .items_center()
            .gap(px(6.0))
            .pl(px(6.0 + depth as f32 * EXPLORER_NODE_INDENT))
            .pr(px(8.0))
            .bg(if selected {
                c.panel_row_selected
            } else {
                hsla(0.0, 0.0, 0.0, 0.0)
            })
            .hover(|this| this.bg(c.panel_row_hover))
            .cursor_pointer()
            .child(arrow_el)
            .children(heading_badge)
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .truncate()
                    .text_size(px(t.text_size * 0.9))
                    .line_height(px(t.text_size * t.text_line_height))
                    .text_color(label_color)
                    .child(node.label.clone()),
            )
            .on_click(move |_event, _window, cx| {
                let node_id = node_id.clone();
                let click_kind = click_kind.clone();
                let _ = click_editor.update(cx, |editor, cx| match click_kind {
                    ExplorerNodeKind::Heading { .. } => {
                        editor.select_outline_node(node_id, cx);
                    }
                });
            })
            .into_any_element()
    }
    pub(crate) fn render_explorer_panel(
        &mut self,
        area_id: usize,
        theme: &Theme,
        strings: &I18nStrings,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.sync_explorer_models(cx);
        self.render_explorer_files_tree(area_id, theme, strings, cx)
    }
}

// ── Free helpers for background file operations ─────────────────────────

/// Map the persisted explorer settings onto the scanner options.
fn explorer_scan_options_from_settings(
    settings: &crate::infra::config::settings::ExplorerSettings,
) -> ExplorerScanOptions {
    use crate::infra::config::settings::{ExplorerSortMode as Mode, ExplorerSortOrder as Order};
    ExplorerScanOptions {
        hide_hidden: settings.hide_hidden,
        sort_mode: match settings.sort_mode {
            Mode::DirectoriesFirst => ExplorerSortMode::DirectoriesFirst,
            Mode::FilesFirst => ExplorerSortMode::FilesFirst,
            Mode::Mixed => ExplorerSortMode::Mixed,
        },
        sort_order: match settings.sort_order {
            Order::Ascending => ExplorerSortOrder::Ascending,
            Order::Descending => ExplorerSortOrder::Descending,
        },
    }
}

/// Recursively copy a directory tree (`fs::copy` is file-only).
fn copy_dir_all(source: &Path, destination: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let from = entry.path();
        let to = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// Disambiguate a paste destination on a background thread (no editor
/// access): "name copy.ext", "name copy 1.ext", …
fn disambiguated_paste_path(source: &Path, target_dir: &Path) -> PathBuf {
    let Some(name) = source.file_name() else {
        return target_dir.join("copy");
    };
    let mut candidate = target_dir.join(name);
    let mut ix = 0usize;
    while candidate.exists() {
        let stem = source
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let extension = source
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();
        let suffix = if ix == 0 {
            " copy".to_string()
        } else {
            format!(" copy {ix}")
        };
        candidate = target_dir.join(format!("{stem}{suffix}{extension}"));
        ix += 1;
    }
    candidate
}

/// Move (cut) or copy `items` into `target_dir` on a background thread;
/// returns the recorded changes for undo. Used by both paste and drag-drop.
fn execute_entry_ops(items: &[PathBuf], target_dir: &Path, is_cut: bool) -> Vec<ExplorerChange> {
    let mut changes = Vec::new();
    for source in items {
        if is_cut {
            // Moving into the entry's own subtree is a no-op.
            if source.starts_with(target_dir) {
                continue;
            }
            let destination = target_dir.join(source.file_name().unwrap_or_default());
            if std::fs::rename(source, &destination).is_ok() {
                changes.push(ExplorerChange::Moved {
                    from: source.clone(),
                    to: destination,
                });
            }
        } else {
            let destination = target_dir.join(source.file_name().unwrap_or_default());
            let destination = if destination.exists() {
                disambiguated_paste_path(source, target_dir)
            } else {
                destination
            };
            let result = if source.is_dir() {
                copy_dir_all(source, &destination)
            } else {
                std::fs::copy(source, &destination).map(|_| ())
            };
            if result.is_ok() {
                changes.push(ExplorerChange::Copied {
                    source: source.clone(),
                    dest: destination,
                });
            }
        }
    }
    changes
}

/// The destination path of a change (used to select the paste result).
fn explorer_change_destination(change: &ExplorerChange) -> Option<&Path> {
    match change {
        ExplorerChange::Created(path) => Some(path),
        ExplorerChange::Renamed { to, .. } | ExplorerChange::Moved { to, .. } => Some(to),
        ExplorerChange::Copied { dest, .. } => Some(dest),
    }
}

/// The floating view shown under the cursor while dragging entries
/// (mirrors Zed's `DraggedProjectEntryView`).
pub struct DraggedExplorerEntryView {
    pub label: String,
    pub count: usize,
    pub click_offset: Point<Pixels>,
}

impl Render for DraggedExplorerEntryView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<ThemeManager>().current_arc();
        let c = &theme.colors;
        let d = &theme.dimensions;
        div()
            .absolute()
            .pl(self.click_offset.x + px(12.0))
            .pt(self.click_offset.y + px(12.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .px(px(10.0))
                    .py(px(4.0))
                    .rounded(px(d.menu_item_radius))
                    .bg(c.dialog_surface)
                    .border_1()
                    .border_color(c.dialog_border)
                    .shadow_lg()
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(c.text_default)
                            .child(if self.count > 1 {
                                format!("{} entries", self.count)
                            } else {
                                self.label.clone()
                            }),
                    ),
            )
    }
}

/// Execute a recorded file operation (redo).
fn execute_explorer_change(change: &ExplorerChange) {
    match change {
        ExplorerChange::Created(path) => {
            if path.is_dir() {
                let _ = std::fs::create_dir_all(path);
            } else if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
                let _ = std::fs::write(path, "");
            }
        }
        ExplorerChange::Renamed { from, to } | ExplorerChange::Moved { from, to } => {
            let _ = std::fs::rename(from, to);
        }
        ExplorerChange::Copied { source, dest } => {
            let result = if source.is_dir() {
                copy_dir_all(source, dest)
            } else {
                std::fs::copy(source, dest).map(|_| ())
            };
            if let Err(err) = result {
                eprintln!("failed to redo copy '{}': {err}", dest.display());
            }
        }
    }
}

/// Execute the inverse of a recorded operation (undo).
fn execute_explorer_change_inverse(change: &ExplorerChange) {
    match change {
        ExplorerChange::Created(path) => {
            let result = if path.is_dir() {
                std::fs::remove_dir_all(path)
            } else {
                std::fs::remove_file(path)
            };
            if let Err(err) = result {
                eprintln!("failed to undo create '{}': {err}", path.display());
            }
        }
        ExplorerChange::Renamed { from, to } | ExplorerChange::Moved { from, to } => {
            if let Err(err) = std::fs::rename(to, from) {
                eprintln!("failed to undo rename '{}': {err}", to.display());
            }
        }
        ExplorerChange::Copied { dest, .. } => {
            let result = if dest.is_dir() {
                std::fs::remove_dir_all(dest)
            } else {
                std::fs::remove_file(dest)
            };
            if let Err(err) = result {
                eprintln!("failed to undo copy '{}': {err}", dest.display());
            }
        }
    }
}
