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
//!   disambiguated copy opens the inline rename state.
//! - Dropping external files copies them; name collisions prompt for
//!   confirmation before replacing (Zed's `drop_external_files`).

use std::path::{Path, PathBuf};
use std::time::Duration;

use gpui::*;


use crate::state::state::*;
use crate::state::undo::{ExplorerChange, explorer_change_destination};
use crate::state::utils::{execute_entry_ops, explorer_is_copy_modifier};
use theme::ThemeManager;

impl ExplorerState {
    // ── Panel-level drag handling (cursor style + hover scroll) ─────────

    /// Refresh the drag cursor to signal move vs. copy while a drag is
    /// active (mirrors Zed's `refresh_drag_cursor_style`). The copy modifier
    /// is Alt on macOS and Ctrl elsewhere.
    pub(crate) fn refresh_explorer_drag_cursor(
        &self,
        modifiers: &Modifiers,
        window: &mut Window,
        cx: &mut App,
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
        cx: &mut App,
    ) -> bool {
        if let Some(previous_position) = self.previous_drag_position {
            // Modifiers are not refreshed when the cursor does not move, so
            // only re-check the style on actual movement.
            if event.event.position != previous_position {
                self.refresh_explorer_drag_cursor(&event.event.modifiers, window, cx);
            }
        }
        self.previous_drag_position = Some(event.event.position);
        if !event.bounds.contains(&event.event.position) {
            self.clear_explorer_drag(cx);
            return false;
        }
        self.start_explorer_hover_scroll(event.event.position, event.bounds, window, cx);
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
        window: &mut Window,
        cx: &mut App,
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
        self.hover_scroll_task.take();
        let generation = {
            let explorer = &mut *self;
            explorer.hover_scroll_generation += 1;
            explorer.hover_scroll_generation
        };
        let window_handle = window.window_handle();
        let task = cx.spawn(async move |cx: &mut AsyncApp| {
            loop {
                // `AnyWindowHandle::update` uses try_borrow_mut: a scroll
                // tick that lands mid-render is skipped (the next mouse
                // move restarts the loop).
                let keep_scrolling = window_handle
                    .update(cx, |_view, _window, cx| {
                        ExplorerState::update(cx, |state, cx| {
                            // The drag ended (mouse released outside the panel,
                            // cancelled, dropped elsewhere): stop scrolling and
                            // clear the leftover drag state so no stale
                            // highlight or task survives.
                            if !cx.has_active_drag() {
                                state.clear_explorer_drag(cx);
                                return false;
                            }
                            if state.hover_scroll_generation != generation {
                                return false; // replaced by a newer move
                            }
                            let handle = state.scroll_handle.0.borrow_mut();
                            let offset = handle.base_handle.offset();
                            handle.base_handle.set_offset(offset + adjustment);
                            cx.refresh_windows();
                            true
                        })
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
        self.hover_scroll_task = Some(task);
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
            Some(sel) => !self
                .snapshots
                .last()
                .and_then(|snap| snap.root_entry())
                .is_some_and(|e| e.id == sel.entry_id),
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
        cx: &mut App,
    ) {
        if !self.explorer_handle_drag_move(event, window, cx) {
            return;
        }
        if self
            .drag_target
            .and_then(|t| t.entry_id())
            .is_none()
        {
            self.drag_target = Some(DragExplorerTarget::Background);
            cx.refresh_windows();
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
        cx: &mut App,
    ) {
        if !self.explorer_handle_drag_move(event, window, cx) {
            return;
        }
        if self
            .drag_target
            .and_then(|t| t.entry_id())
            .is_none()
            && self.explorer_should_highlight_background(event.drag(cx))
        {
            self.drag_target = Some(DragExplorerTarget::Background);
            cx.refresh_windows();
        }
    }

    /// Clear all drag state (drop / drag leaving the panel).
    pub(crate) fn clear_explorer_drag(&mut self, cx: &mut App) {
        let had_drag_state = self.drag_target.take().is_some()
            | self.hover_expand_task.take().is_some()
            | self.hover_scroll_task.take().is_some()
            | self.previous_drag_position.take().is_some();
        if had_drag_state {
            // Stop any in-flight hover-scroll task.
            self.hover_scroll_generation += 1;
            cx.refresh_windows();
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
        cx: &mut App,
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
        cx: &mut App,
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
        cx: &mut App,
    ) {
        let is_current_target = self
            .drag_target
            .and_then(|target| target.entry_id())
            == Some(entry_id);
        if !event.bounds.contains(&event.event.position) {
            if is_current_target {
                self.drag_target = None;
                cx.refresh_windows();
            }
            return;
        }
        if is_current_target {
            return; // same target: keep the highlight and the pending expand
        }
        // Keep the multi-select state in sync with what is being dragged
        // (mirrors Zed: single item drags collapse the marks to themselves).
        if let Some(drag) = drag {
            if drag.selections.len() == 1 {
                self.marked.clear();
                if let Some(active) = drag.active() {
                    self.marked.insert(*active);
                }
            }
        } else {
            self.marked.clear();
        }
        let Some((highlight_entry_id, is_dir, is_expanded)) =
            self.explorer_highlight_for_drag(entry_id, drag)
        else {
            // No highlight for this target (e.g. a single item dragged onto
            // its own parent): clear any stale highlight.
            self.drag_target = None;
            cx.refresh_windows();
            return;
        };
        self.drag_target = Some(DragExplorerTarget::Entry {
            entry_id,
            highlight_entry_id,
        });
        // Hover-expand: restart the timer on every move onto a collapsed
        // directory (mirrors Zed).
        self.hover_expand_task.take();
        if is_dir && !is_expanded {
            let bounds = event.bounds;
            let window_handle = window.window_handle();
            let task = cx.spawn(async move |cx: &mut AsyncApp| {
                cx.background_executor()
                    .timer(Duration::from_millis(500))
                    .await;
                let _ = cx.update_window(window_handle, |_, window, cx| {
                    ExplorerState::update(cx, |state, cx| {
                        state.hover_expand_task = None;
                        if cx.has_active_drag()
                            && state
                                .drag_target
                                .and_then(|target| target.entry_id())
                                == Some(entry_id)
                            && bounds.contains(&window.mouse_position())
                            // Only expand a directory that is still folded
                            // (the user may have toggled it meanwhile).
                            && state.explorer_entry_by_id(entry_id).is_some_and(|entry| {
                                entry.kind == ExplorerEntryKind::Directory
                                    && !entry.is_expanded
                            })
                        {
                            state.toggle_explorer_node(entry_id, cx);
                        }
                        cx.refresh_windows();
                    });
                });
            });
            self.hover_expand_task = Some(task);
        }
        cx.refresh_windows();
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
        let node_path = self.explorer_path_for_id(entry_id)?;
        let is_dir = node_path.is_dir();

        // Self and child drag check: do not highlight when dragged onto itself or into its own subtree
        if let Some(drag) = drag {
            for selection in &drag.selections {
                if let Some(source_path) = self.explorer_path_for_id(selection.entry_id) {
                    if node_path.starts_with(&source_path) {
                        return None; // Cannot drag into itself or its descendant
                    }
                }
            }
            if drag.selections.len() == 1
                && let Some(active) = drag.active()
                && let Some(active_path) = self.explorer_path_for_id(active.entry_id)
                && let Some(active_parent) = active_path.parent()
            {
                if is_dir && active_parent == node_path.as_path() {
                    return None; // dragged onto its own containing directory
                }
                if !is_dir && Some(active_parent) == node_path.parent() {
                    return None; // dragged onto a sibling file
                }
            }
        }
        let highlight_entry_id = if is_dir {
            entry_id
        } else {
            let parent_path = node_path.parent()?;
            self.explorer_id_for_path(parent_path)?.entry_id
        };
        let is_expanded = self
            .expanded
            .values()
            .any(|set| set.contains(&highlight_entry_id));
        Some((highlight_entry_id, is_dir, is_expanded))
    }

    // ── Drop handling ────────────────────────────────────────────────────

    /// Resolve the drop target directory for an entry: the entry itself for
    /// directories, its parent for files.
    fn explorer_drop_target_dir(&self, entry_id: ExplorerEntryId) -> Option<PathBuf> {
        let path = self.explorer_path_for_id(entry_id)?;
        if path.is_dir() {
            Some(path)
        } else {
            path.parent().map(Path::to_path_buf)
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
        cx: &mut App,
    ) {
        self.clear_explorer_drag(cx);
        // Root rows reorder worktrees (Zed's `move_worktree_root`)
        for selection in &payload.selections {
            if self.is_explorer_root_entry(selection.entry_id) {
                let to = self
                    .snapshots
                    .iter()
                    .position(|snap| snap.path_for_id.contains_key(&entry_id))
                    .unwrap_or(0);
                if let Some(from) = self
                    .snapshots
                    .iter()
                    .position(|snap| snap.root_entry().is_some_and(|e| e.id == selection.entry_id))
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
            .filter(|selection| !self.is_explorer_root_entry(selection.entry_id))
            .filter_map(|selection| self.explorer_path_for_id(selection.entry_id))
            .collect();
        if paths.is_empty() {
            return;
        }
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
        cx: &mut App,
    ) {
        self.clear_explorer_drag(cx);
        let Some(root) = self.last_explorer_root_path() else {
            return;
        };
        let last_index = self.worktrees.len().saturating_sub(1);
        for selection in &payload.selections {
            if self.is_explorer_root_entry(selection.entry_id) {
                if let Some(from) = self
                    .snapshots
                    .iter()
                    .position(|snap| snap.root_entry().is_some_and(|e| e.id == selection.entry_id))
                {
                    self.move_explorer_worktree(from, last_index, cx);
                }
            }
        }
        let paths: Vec<PathBuf> = payload
            .selections
            .iter()
            .filter(|selection| !self.is_explorer_root_entry(selection.entry_id))
            .filter_map(|selection| self.explorer_path_for_id(selection.entry_id))
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
        cx: &mut App,
    ) {
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
        cx: &mut App,
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
        cx: &mut App,
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
        let window_handle = window.window_handle();
        let paths = paths.to_vec();
        let _ = cx.spawn(async move |cx: &mut AsyncApp| {
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
                ExplorerState::update(cx, |state, cx| {
                    state.perform_entry_ops(remaining, target_dir, false, window, cx);
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
        cx: &mut App,
    ) {
        if paths.is_empty() {
            return;
        }
        let disambiguate = !is_cut;
        let window_handle = window.window_handle();
        let _ = cx.spawn(async move |cx: &mut AsyncApp| {
            let changes = cx
                .background_executor()
                .spawn(async move { execute_entry_ops(&paths, &target_dir, is_cut, disambiguate) })
                .await;
            let _ = cx.update(|cx| {
                ExplorerState::update(cx, |state, cx| {
                    state.clear_explorer_drag(cx);
                    if changes.len() > 1 {
                        let batch = ExplorerChange::Batch(changes.clone());
                        state.record_explorer_change(batch.clone());
                    } else if let Some(change) = changes.first() {
                        state.record_explorer_change(change.clone());
                    }
                    for change in &changes {
                        if let Some(dest) = explorer_change_destination(change) {
                            state.expand_to_path(dest);
                        }
                    }
                    state.rescan_explorer_worktrees(cx);
                    if let Some(last) = changes.last().and_then(explorer_change_destination) {
                        if let Some(sel) = state.explorer_id_for_path(last) {
                            state.pending_select = Some((sel.worktree_id, last.to_path_buf()));
                        }
                    }
                    // A disambiguated copy ("name copy.ext") opens the inline
                    // rename editor with the suffix pre-selected (mirrors Zed) —
                    // once the rescan makes the new entry visible.
                    if disambiguate
                        && changes.len() == 1
                        && let Some(ExplorerChange::Copied { source, dest }) = changes.first()
                        && source.file_name() != dest.file_name()
                    {
                        state.pending_rename = Some((window_handle, dest.clone()));
                    }
                    state.sync_explorer_models(cx);
                    cx.refresh_windows();
                });
                // Window-scoped dispatch must run after the global lease
                // above ends (nested global leases panic).
                let _ = cx.update_window(window_handle, |_, window, cx| {
                    ExplorerState::update(cx, |state, cx| {
                        if changes.len() > 1 {
                            state.sync_open_tabs_after_fs_change(
                                &ExplorerChange::Batch(changes.clone()),
                                window,
                                cx,
                            );
                        } else if let Some(change) = changes.first() {
                            state.sync_open_tabs_after_fs_change(change, window, cx);
                        }
                    });
                });
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

