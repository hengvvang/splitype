//! Explorer drag & drop — internal entries and external files, mirroring
//! Zed's project panel:
//!
//! - Panel-level moves handle the drag cursor style (copy vs. move) and a
//!   continuous hover-scroll whose speed grows near the list edges
//!   (Zed's `handle_drag_move` + `hover_scroll_task`).
//! - Row-level moves set the drop target: the entry under the pointer plus
//!   the highlight entry (a directory highlights itself, a file highlights
//!   its parent — the highlight extends to all descendants), and schedule a
//!   hover-expand of collapsed directories after 500ms (restarted on every
//!   move). The row that set a target is also responsible for clearing it
//!   once the pointer leaves its bounds.
//! - Dropping internal entries moves them by default and copies them with
//!   the copy modifier; worktree root rows are dragged to reorder
//!   worktrees (Zed's `move_worktree`). Nested selections are reduced to
//!   their outermost directories (Zed's `disjoint_entries`), and a
//!   disambiguated copy opens the inline rename editor.
//! - Dropping external files copies them; name collisions prompt for
//!   confirmation before replacing (Zed's `drop_external_files`).

use std::path::{Path, PathBuf};
use std::time::Duration;

use gpui::*;

use crate::editor::controller::Editor;
use crate::editor::panels::explorer::state::*;
use crate::editor::panels::explorer::undo::{ExplorerChange, explorer_change_destination};
use crate::editor::panels::explorer::utils::{execute_entry_ops, explorer_is_copy_modifier};
use crate::infra::theme::ThemeManager;

impl Editor {
    // ── Panel-level drag handling (cursor style + hover scroll) ─────────

    /// Refresh the drag cursor to signal move vs. copy while a drag is
    /// active (mirrors Zed's `refresh_drag_cursor_style`). The copy modifier
    /// is Alt on macOS and Ctrl elsewhere.
    pub(crate) fn refresh_explorer_drag_cursor(
        &self,
        modifiers: &Modifiers,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(existing_cursor) = cx.active_drag_cursor_style() {
            let new_cursor = if explorer_is_copy_modifier(modifiers) {
                CursorStyle::DragCopy
            } else {
                CursorStyle::PointingHand
            };
            if existing_cursor != new_cursor {
                cx.set_active_drag_cursor_style(new_cursor, window);
            }
        }
    }

    /// Common panel-level handling for every drag move (mirrors Zed's
    /// `handle_drag_move`): refresh the cursor when the pointer actually
    /// moved, clear all drag state once the pointer leaves the panel, and
    /// start (or restart) the edge hover-scroll task.
    ///
    /// Returns `false` when the pointer is outside the panel bounds and all
    /// drag state has been cleared.
    fn explorer_handle_drag_move<T: 'static>(
        &mut self,
        event: &DragMoveEvent<T>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        eprintln!(
            "[explorer-drag] panel drag_move @ {:?}",
            event.event.position
        );
        if let Some(previous_position) = self.panels.explorer.previous_drag_position {
            // Modifiers are not refreshed when the cursor does not move, so
            // only re-check the style on actual movement.
            if event.event.position != previous_position {
                self.refresh_explorer_drag_cursor(&event.event.modifiers, window, cx);
            }
        }
        self.panels.explorer.previous_drag_position = Some(event.event.position);
        if !event.bounds.contains(&event.event.position) {
            self.clear_explorer_drag(cx);
            return false;
        }
        self.start_explorer_hover_scroll(event.event.position, event.bounds, cx);
        true
    }

    /// Start a continuous scroll task while the drag hovers the list edges;
    /// the scroll speed grows the closer the pointer gets to an edge
    /// (mirrors Zed's hover-scroll). Each move replaces the previous task —
    /// stale tasks detect the newer generation and stop themselves.
    fn start_explorer_hover_scroll(
        &mut self,
        position: Point<Pixels>,
        bounds: Bounds<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let panel_height = bounds.size.height;
        if panel_height <= px(0.0) {
            return;
        }
        let event_offset = position.y - bounds.origin.y;
        // How far along the panel is the cursor? (0. = top, 1. = bottom)
        let hovered_region_offset = event_offset / panel_height;
        let vertical_scroll_offset = if hovered_region_offset <= 0.05 {
            8.
        } else if hovered_region_offset <= 0.15 {
            5.
        } else if hovered_region_offset >= 0.95 {
            -8.
        } else if hovered_region_offset >= 0.85 {
            -5.
        } else {
            return;
        };
        let adjustment = point(px(0.0), px(vertical_scroll_offset));
        self.panels.explorer.hover_scroll_task.take();
        let generation = {
            let explorer = &mut self.panels.explorer;
            explorer.hover_scroll_generation += 1;
            explorer.hover_scroll_generation
        };
        let weak_editor = cx.entity().downgrade();
        let task = cx.spawn(async move |_this, cx: &mut AsyncApp| {
            loop {
                let keep_scrolling = weak_editor
                    .update(cx, |editor, cx| {
                        // The drag ended (mouse released outside the panel,
                        // cancelled, dropped elsewhere): stop scrolling and
                        // clear the leftover drag state so no stale
                        // highlight or task survives.
                        if !cx.has_active_drag() {
                            editor.clear_explorer_drag(cx);
                            return false;
                        }
                        if editor.panels.explorer.hover_scroll_generation != generation {
                            return false; // replaced by a newer move
                        }
                        let handle = editor.panels.explorer.scroll_handle.0.borrow_mut();
                        let offset = handle.base_handle.offset();
                        handle.base_handle.set_offset(offset + adjustment);
                        cx.notify();
                        true
                    })
                    .unwrap_or(false);
                if !keep_scrolling {
                    return;
                }
                cx.background_executor()
                    .timer(Duration::from_millis(16))
                    .await;
            }
        });
        self.panels.explorer.hover_scroll_task = Some(task);
    }

    /// Whether dropping internal selections on the background is meaningful
    /// (mirrors Zed's `should_highlight_background_for_selection_drag`):
    /// multiple entries always qualify; a single entry qualifies unless it
    /// is the last worktree's root (it would be dropped onto itself).
    fn explorer_should_highlight_background(&self, payload: &DraggedExplorerSelection) -> bool {
        if payload.selections.len() > 1 {
            return true;
        }
        match payload.active() {
            Some(ExplorerSelection::File { entry, .. }) => !self
                .panels
                .explorer
                .trees_cache
                .last()
                .is_some_and(|tree| tree.id == *entry),
            _ => true,
        }
    }

    /// Set the drag target for the empty area (targets the explorer root)
    /// — the external-file payload always qualifies. Runs the common
    /// panel-level handling (cursor style, edge scroll, out-of-bounds
    /// cleanup) on every drag move, whether it bubbled up from a row or was
    /// dispatched to the background itself; an entry target set by a row is
    /// never overridden.
    pub(crate) fn explorer_drag_hover_background_external(
        &mut self,
        event: &DragMoveEvent<ExternalPaths>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.explorer_handle_drag_move(event, window, cx) {
            return;
        }
        if self
            .panels
            .explorer
            .drag_target
            .and_then(|t| t.entry_id())
            .is_none()
        {
            self.panels.explorer.drag_target = Some(DragExplorerTarget::Background);
            cx.notify();
        }
    }

    /// Set the drag target for the empty area — the internal payload only
    /// qualifies when the drop would move something (see
    /// `explorer_should_highlight_background`). Runs the common panel-level
    /// handling on every drag move; an entry target set by a row is never
    /// overridden.
    pub(crate) fn explorer_drag_hover_background_internal(
        &mut self,
        event: &DragMoveEvent<DraggedExplorerSelection>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.explorer_handle_drag_move(event, window, cx) {
            return;
        }
        if self
            .panels
            .explorer
            .drag_target
            .and_then(|t| t.entry_id())
            .is_none()
            && self.explorer_should_highlight_background(event.drag(cx))
        {
            self.panels.explorer.drag_target = Some(DragExplorerTarget::Background);
            cx.notify();
        }
    }

    /// Clear all drag state (drop / drag leaving the panel).
    pub(crate) fn clear_explorer_drag(&mut self, cx: &mut Context<Self>) {
        let had_drag_state = self.panels.explorer.drag_target.take().is_some()
            | self.panels.explorer.hover_expand_task.take().is_some()
            | self.panels.explorer.hover_scroll_task.take().is_some()
            | self.panels.explorer.previous_drag_position.take().is_some();
        if had_drag_state {
            // Stop any in-flight hover-scroll task.
            self.panels.explorer.hover_scroll_generation += 1;
            cx.notify();
        }
    }

    // ── Row-level drag handling (target + highlight + hover-expand) ─────

    /// Set the drag target while hovering a row with external files; the
    /// highlight follows `explorer_highlight_for_drag`. Rows only set the
    /// entry target — the common panel-level handling (cursor style, edge
    /// scroll, out-of-bounds cleanup) runs on the panel background via event
    /// bubbling, which never overrides an entry target.
    pub(crate) fn explorer_drag_hover_entry_external(
        &mut self,
        event: &DragMoveEvent<ExternalPaths>,
        entry_id: ExplorerEntryId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.explorer_drag_hover_entry_impl(event, entry_id, None, window, cx);
    }

    /// Set the drag target while hovering a row with internal entries.
    /// A single dragged entry replaces the marks (mirrors Zed), and the
    /// highlight skips the dragged entry's own parent / sibling files.
    pub(crate) fn explorer_drag_hover_entry_internal(
        &mut self,
        event: &DragMoveEvent<DraggedExplorerSelection>,
        entry_id: ExplorerEntryId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let payload = event.drag(cx).clone();
        self.explorer_drag_hover_entry_impl(event, entry_id, Some(&payload), window, cx);
    }

    /// Shared row-hover logic (mirrors Zed's `render_entry` drag handlers):
    ///
    /// - The row that set the current target is responsible for clearing it
    ///   once the pointer leaves its bounds (a stale highlight must not
    ///   survive a layout shift or an edge crossing).
    /// - Re-entering a new row sets the target and, for a collapsed
    ///   directory, restarts the 500ms hover-expand timer — it only fires
    ///   while the pointer still hovers the same row.
    /// - A single-item drag keeps the marks in sync with the dragged entry.
    fn explorer_drag_hover_entry_impl<T: 'static>(
        &mut self,
        event: &DragMoveEvent<T>,
        entry_id: ExplorerEntryId,
        drag: Option<&DraggedExplorerSelection>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        eprintln!(
            "[explorer-drag] row drag_move @ {:?} entry={}",
            event.event.position, entry_id.0
        );
        let is_current_target = self
            .panels
            .explorer
            .drag_target
            .and_then(|target| target.entry_id())
            == Some(entry_id);
        if !event.bounds.contains(&event.event.position) {
            if is_current_target {
                self.panels.explorer.drag_target = None;
                cx.notify();
            }
            return;
        }
        if is_current_target {
            return; // same target: keep the highlight and the pending expand
        }
        eprintln!(
            "[explorer-drag] setting drag target to entry {}",
            entry_id.0
        );
        // Keep the multi-select state in sync with what is being dragged
        // (mirrors Zed: single item drags collapse the marks to themselves).
        if let Some(drag) = drag {
            if drag.selections.len() == 1 {
                self.panels.explorer.marked.clear();
                if let Some(active) = drag.active() {
                    self.panels.explorer.marked.push(active.clone());
                }
            }
        } else {
            self.panels.explorer.marked.clear();
        }
        let Some((highlight_entry_id, is_dir, is_expanded)) =
            self.explorer_highlight_for_drag(entry_id, drag)
        else {
            // No highlight for this target (e.g. a single item dragged onto
            // its own parent): clear any stale highlight.
            self.panels.explorer.drag_target = None;
            cx.notify();
            return;
        };
        self.panels.explorer.drag_target = Some(DragExplorerTarget::Entry {
            entry_id,
            highlight_entry_id,
        });
        // Hover-expand: restart the timer on every move onto a collapsed
        // directory (mirrors Zed).
        self.panels.explorer.hover_expand_task.take();
        if is_dir && !is_expanded {
            let bounds = event.bounds;
            let window_handle = window.window_handle();
            let weak_editor = cx.entity().downgrade();
            let task = cx.spawn(async move |_this, cx: &mut AsyncApp| {
                cx.background_executor()
                    .timer(Duration::from_millis(500))
                    .await;
                let _ = cx.update_window(window_handle, |_, window, cx| {
                    let _ = weak_editor.update(cx, |editor, cx| {
                        editor.panels.explorer.hover_expand_task = None;
                        if cx.has_active_drag()
                            && editor
                                .panels
                                .explorer
                                .drag_target
                                .and_then(|target| target.entry_id())
                                == Some(entry_id)
                            && bounds.contains(&window.mouse_position())
                            // Only expand a directory that is still folded
                            // (the user may have toggled it meanwhile).
                            && editor.explorer_entry_by_id(entry_id).is_some_and(|entry| {
                                entry.kind == ExplorerEntryKind::Directory
                                    && !entry.is_expanded
                            })
                        {
                            editor.toggle_explorer_node(entry_id, cx);
                        }
                        cx.notify();
                    });
                });
            });
            self.panels.explorer.hover_expand_task = Some(task);
        }
        cx.notify();
    }

    /// Compute the highlight entry for a drag target (mirrors Zed's
    /// `highlight_entry_for_external_drag` / `highlight_entry_for_selection_drag`):
    /// directories highlight themselves, files highlight their parent
    /// directory. For a single dragged entry, the dragged entry's own
    /// parent and its sibling files are not highlighted.
    fn explorer_highlight_for_drag(
        &self,
        entry_id: ExplorerEntryId,
        drag: Option<&DraggedExplorerSelection>,
    ) -> Option<(ExplorerEntryId, bool, bool)> {
        let entry = self.explorer_entry_by_id(entry_id)?;
        let is_dir = entry.kind == ExplorerEntryKind::Directory;
        // Single-item drag: do not highlight the dragged entry's parent
        // directory or its sibling files.
        if let Some(drag) = drag
            && drag.selections.len() == 1
            && let Some(ExplorerSelection::File {
                entry: active_id, ..
            }) = drag.active()
            && let Some(active) = self.explorer_entry_by_id(*active_id)
            && let Some(active_parent) = active.path.parent()
        {
            if active_parent == entry.path.as_path() {
                return None; // dragged onto its own parent — no highlight
            }
            if Some(active_parent) == entry.path.parent() && !is_dir {
                return None; // dragged onto a sibling file — no highlight
            }
        }
        let highlight_entry_id = if is_dir {
            entry_id
        } else {
            self.explorer_id_for_path(entry.path.parent()?)?.1
        };
        Some((highlight_entry_id, is_dir, entry.is_expanded))
    }

    // ── Drop handling ────────────────────────────────────────────────────

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

    /// Drop internal dragged entries onto an entry: worktree roots reorder
    /// the worktree list, everything else moves by default and copies with
    /// the copy modifier held (mirrors Zed's `drag_onto`).
    pub(crate) fn on_explorer_drop_internal(
        &mut self,
        payload: &DraggedExplorerSelection,
        entry_id: ExplorerEntryId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        eprintln!("[explorer-drag] drop internal onto entry {}", entry_id.0);
        self.clear_explorer_drag(cx);
        // Root rows reorder worktrees (Zed's `move_worktree_root`); resolve
        // each root's current index by id because earlier moves shift the
        // list (and re-resolve the destination for the same reason).
        for selection in &payload.selections {
            if let ExplorerSelection::File { entry, .. } = selection
                && self.explorer_is_root_entry(*entry)
            {
                let to = self.root_for_explorer_entry(entry_id).unwrap_or(0);
                if let Some(from) = self
                    .panels
                    .explorer
                    .trees_cache
                    .iter()
                    .position(|tree| tree.id == *entry)
                {
                    self.move_explorer_worktree(from, to, cx);
                }
            }
        }
        let target_dir = match self.explorer_drop_target_dir(entry_id) {
            Some(target_dir) => target_dir,
            None => return,
        };
        let paths: Vec<PathBuf> = payload
            .selections
            .iter()
            .filter(|selection| {
                !matches!(selection, ExplorerSelection::File { entry, .. }
                    if self.explorer_is_root_entry(*entry))
            })
            .filter_map(|selection| {
                self.explorer_entry_for_selection(selection)
                    .map(|entry| entry.path.clone())
            })
            .collect();
        if paths.is_empty() {
            return;
        }
        // Reduce nested selections to their outermost directories so a
        // dragged directory and its children are not processed twice.
        let paths = self.disjoint_explorer_paths(&paths);
        let is_copy = explorer_is_copy_modifier(&window.modifiers());
        self.perform_entry_ops(paths, target_dir, !is_copy, window, cx);
    }

    /// Drop internal dragged entries onto the panel background (targets the
    /// last worktree root; root rows move to the end).
    pub(crate) fn on_explorer_drop_internal_to_root(
        &mut self,
        payload: &DraggedExplorerSelection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.clear_explorer_drag(cx);
        let Some(root) = self.last_explorer_root_path() else {
            return;
        };
        // Root rows dragged onto the background move to the end of the list.
        let last_index = self.panels.explorer.worktrees.len().saturating_sub(1);
        for selection in &payload.selections {
            if let ExplorerSelection::File { entry, .. } = selection
                && self.explorer_is_root_entry(*entry)
            {
                if let Some(from) = self
                    .panels
                    .explorer
                    .trees_cache
                    .iter()
                    .position(|tree| tree.id == *entry)
                {
                    self.move_explorer_worktree(from, last_index, cx);
                }
            }
        }
        let paths: Vec<PathBuf> = payload
            .selections
            .iter()
            .filter(|selection| {
                !matches!(selection, ExplorerSelection::File { entry, .. }
                    if self.explorer_is_root_entry(*entry))
            })
            .filter_map(|selection| {
                self.explorer_entry_for_selection(selection)
                    .map(|entry| entry.path.clone())
            })
            .collect();
        if paths.is_empty() {
            return;
        }
        let paths = self.disjoint_explorer_paths(&paths);
        let is_copy = explorer_is_copy_modifier(&window.modifiers());
        self.perform_entry_ops(paths, root, !is_copy, window, cx);
    }

    /// Drop external files onto an entry (always a copy, mirrors Zed).
    pub(crate) fn on_explorer_drop_external(
        &mut self,
        paths: &[PathBuf],
        entry_id: ExplorerEntryId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        eprintln!(
            "[explorer-drag] drop external onto entry {}: {paths:?}",
            entry_id.0
        );
        self.clear_explorer_drag(cx);
        let Some(target_dir) = self.explorer_drop_target_dir(entry_id) else {
            return;
        };
        self.drop_external_files(paths, target_dir, window, cx);
    }

    /// Drop external files onto the panel background (targets the last
    /// worktree root).
    pub(crate) fn on_explorer_drop_external_to_root(
        &mut self,
        paths: &[PathBuf],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.clear_explorer_drag(cx);
        let Some(root) = self.last_explorer_root_path() else {
            return;
        };
        self.drop_external_files(paths, root, window, cx);
    }

    /// Copy external files into `target_dir`; same-named destinations
    /// prompt for confirmation before being replaced (mirrors Zed's
    /// `drop_external_files`).
    fn drop_external_files(
        &mut self,
        paths: &[PathBuf],
        target_dir: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let conflicts: Vec<PathBuf> = paths
            .iter()
            .filter(|path| {
                path.file_name()
                    .map(|name| target_dir.join(name).exists())
                    .unwrap_or(false)
            })
            .cloned()
            .collect();
        if conflicts.is_empty() {
            self.perform_entry_ops(paths.to_vec(), target_dir, false, window, cx);
            return;
        }
        let weak_editor = cx.entity().downgrade();
        let window_handle = window.window_handle();
        let paths = paths.to_vec();
        let _ = cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let mut remaining = paths;
            for conflict in &conflicts {
                // Resolve the conflicts one at a time: GPUI forbids
                // re-entrant prompts (`prompt_builder` is taken on the first
                // call), so the next prompt may only be created after the
                // previous one was answered.
                let name = conflict
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let prompt = match cx.update_window(window_handle, |_, window, cx| {
                    window.prompt(
                        PromptLevel::Info,
                        &format!(
                            "A file or folder named '{name}' already exists in the destination folder. Do you want to replace it?"
                        ),
                        None,
                        &["Replace", "Cancel"],
                        cx,
                    )
                }) {
                    Ok(prompt) => prompt,
                    Err(_) => return, // window closed mid-drop
                };
                if prompt.await != Ok(0) {
                    remaining.retain(|path| path != conflict);
                }
            }
            if remaining.is_empty() {
                return;
            }
            let _ = cx.update_window(window_handle, |_, window, cx| {
                let _ = weak_editor.update(cx, |editor, cx| {
                    editor.perform_entry_ops(remaining, target_dir, false, window, cx);
                });
            });
        });
    }

    /// Filter nested paths: when a directory and one of its descendants are
    /// both dragged, only the directory survives (mirrors Zed's
    /// `disjoint_entries`).
    fn disjoint_explorer_paths(&self, paths: &[PathBuf]) -> Vec<PathBuf> {
        let dir_paths: Vec<&Path> = paths
            .iter()
            .filter(|path| path.is_dir())
            .map(PathBuf::as_path)
            .collect();
        paths
            .iter()
            .filter(|path| {
                !dir_paths
                    .iter()
                    .any(|dir| *path != *dir && path.starts_with(dir))
            })
            .cloned()
            .collect()
    }

    /// Run entry operations (move or copy) on a background thread, record
    /// undo changes, select the last result and rescan. Copies use
    /// disambiguated names and open the inline rename editor when the name
    /// had to be suffixed (mirrors Zed).
    fn perform_entry_ops(
        &mut self,
        paths: Vec<PathBuf>,
        target_dir: PathBuf,
        is_cut: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if paths.is_empty() {
            return;
        }
        let disambiguate = !is_cut;
        let weak_editor = cx.entity().downgrade();
        let window_handle = window.window_handle();
        let _ = cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let changes = cx
                .background_executor()
                .spawn(async move { execute_entry_ops(&paths, &target_dir, is_cut, disambiguate) })
                .await;
            let _ = weak_editor.update(cx, |editor, cx| {
                editor.clear_explorer_drag(cx);
                for change in &changes {
                    editor.record_explorer_change(change.clone());
                }
                editor.rescan_explorer_worktrees(cx);
                if let Some(last) = changes.last().and_then(explorer_change_destination) {
                    let root = editor.root_for_explorer_path(last).unwrap_or(0);
                    editor.panels.explorer.pending_select = Some((root, last.to_path_buf()));
                }
                // A disambiguated copy ("name copy.ext") opens the inline
                // rename editor with the suffix pre-selected (mirrors Zed) —
                // once the rescan makes the new entry visible.
                if disambiguate
                    && changes.len() == 1
                    && let Some(ExplorerChange::Copied { source, dest }) = changes.first()
                    && source.file_name() != dest.file_name()
                {
                    editor.panels.explorer.pending_rename = Some((window_handle, dest.clone()));
                }
                editor.sync_explorer_models(cx);
                cx.notify();
            });
        });
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
                    .child(div().text_size(px(12.0)).text_color(c.text_default).child(
                        if self.count > 1 {
                            format!("{} entries", self.count)
                        } else {
                            self.label.clone()
                        },
                    )),
            )
    }
}
