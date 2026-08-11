//! The window shell — the OS window's root entity.
//!
//! Owns the mapping from layout areas to content entities (`AreaContent`),
//! the window-level chrome state (the in-window menu bar), and renders the
//! window chrome (custom titlebar + menu bar) above the primary editor's
//! content. Window-level state that currently lives on the Editor (panel
//! layout, overlays) migrates here incrementally; the Editor keeps only
//! its own editing state.

use std::collections::HashMap;

use gpui::*;

use crate::app::window_area::WindowAreaKind;
use crate::app::window_chrome::MenuBarState;
use crate::app::window_panels::WindowPanels;
use crate::editor::controller::{DocumentTab, Editor, InfoDialogKind};
use crate::editor::session::EditorSession;
use crate::infra::i18n::I18nManager;
use crate::infra::theme::ThemeManager;
use crate::splitter::NodeId;
use splitype_splitter::tree::Axis;

/// The content of one area in the outer layout tree.
pub enum AreaContent {
    /// An editor with its own tab list and inner panel layout.
    Editor(Entity<Editor>),
}

/// Explorer row right-click menu: a window-level overlay rendered by the
/// Shell (it must float over every area at window coordinates).
#[derive(Clone)]
pub(crate) struct ExplorerFileMenuState {
    pub(crate) position: Point<Pixels>,
    pub(crate) path: std::path::PathBuf,
    pub(crate) is_dir: bool,
}

/// The OS window's root entity: content areas + window lifecycle.
pub struct Shell {
    /// Content entity per outer area id. An area holds a content entity
    /// only while its kind is `Editor`; switching kinds moves the entity
    /// away (its session is retained while it still holds tabs).
    pub(crate) areas: HashMap<NodeId, AreaContent>,
    /// Sessions of Editor areas that left the Editor kind with tabs
    /// (background editing): switching back recreates the entity with the
    /// retained session.
    pub(crate) retained_editor_sessions: HashMap<NodeId, EditorSession>,
    /// Open/hover state for the in-window titlebar menu bar.
    pub(crate) menu_bar: MenuBarState,
    /// Window panel state: the outer area layout tree plus the explorer /
    /// outline / settings sidebar states.
    pub(crate) panels: WindowPanels,
    /// The last rendered viewport size; area rectangles are derived from
    /// it when the layout changes (see [`Self::sync_area_states`]).
    pub(crate) last_viewport: Option<Size<Pixels>>,
    /// Explorer row right-click menu state (rendered at window level).
    pub(crate) explorer_file_menu: Option<ExplorerFileMenuState>,
    /// Informational dialog shown from the Help menu (About / update check).
    pub(crate) info_dialog: Option<InfoDialogKind>,
    /// True while an online update check is running in the background.
    pub(crate) update_check_in_progress: bool,
    /// Whether the window-close guard callback is installed on the window.
    pub(crate) close_guard_installed: bool,
}

impl Shell {
    /// The active editor area's active tab, if any. Reads through the
    /// editor content entity of the active area.
    pub(crate) fn active_editor_tab<'a>(&self, cx: &'a App) -> Option<&'a DocumentTab> {
        let area = self.active_editor_area()?;
        let editor = self.editor_for(area)?;
        editor.read(cx).active_editor_tab()
    }

    /// The window's primary (first) editor area content, if any.
    pub(crate) fn primary_editor(&self) -> Option<&Entity<Editor>> {
        self.areas.values().find_map(|area| match area {
            AreaContent::Editor(editor) => Some(editor),
        })
    }

    /// Recomputes every editor area's pushed state — the area rectangle,
    /// active-editor flag, and maximized flag — from the outer layout.
    /// Called after every layout change and on render (with the current
    /// viewport).
    pub(crate) fn sync_area_states(&mut self, cx: &mut Context<Self>) {
        let Some(viewport) = self.last_viewport else {
            return;
        };
        let outer_rects = self.panels.layout.leaf_rects(viewport);
        let active = self.panels.layout.active_area;
        let leaf_count = self.panels.layout.tree.count_leaves();
        let editors: Vec<(NodeId, Entity<Editor>)> = self
            .areas
            .iter()
            .filter_map(|(area_id, content)| match content {
                AreaContent::Editor(entity) => Some((*area_id, entity.clone())),
            })
            .collect();
        for (area_id, entity) in editors {
            let rect = outer_rects
                .iter()
                .find(|rect| rect.id == area_id)
                .map(|rect| Bounds {
                    origin: point(px(rect.x), px(rect.y)),
                    size: size(px(rect.width), px(rect.height)),
                });
            let is_maximized = self
                .panels
                .layout
                .tree
                .find_leaf(area_id)
                .is_some_and(|panel| panel.maximized);
            let _ = entity.update(cx, |editor, _cx| {
                editor.area_rect = rect;
                editor.is_active_area = active == Some(area_id);
                editor.is_maximized = is_maximized;
                editor.leaf_count = leaf_count;
            });
        }
    }

    /// Marks `area_id` as the active editor area and re-pushes the
    /// active-flag to every editor entity.
    pub(crate) fn activate_area(&mut self, area_id: NodeId, cx: &mut Context<Self>) {
        self.panels.layout.activate_area(area_id);
        self.sync_area_states(cx);
    }

    /// Dismisses the primary editor's floating overlays (context menu,
    /// table-insert dialog). Explorer actions run on the Shell but the
    /// overlays still live on the editor entity.
    pub(crate) fn dismiss_contextual_overlays(&mut self, cx: &mut Context<Self>) {
        if let Some(editor) = self.primary_editor() {
            let _ = editor.update(cx, |editor, cx| editor.dismiss_contextual_overlays(cx));
        }
    }

    /// Opens the explorer row context menu at `position` (window
    /// coordinates). The Shell renders the menu itself, so it floats over
    /// every area regardless of which tiles are present.
    pub(crate) fn open_explorer_file_context_menu(
        &mut self,
        position: Point<Pixels>,
        path: std::path::PathBuf,
        is_dir: bool,
        cx: &mut Context<Self>,
    ) {
        self.dismiss_contextual_overlays(cx);
        self.explorer_file_menu = Some(ExplorerFileMenuState {
            position,
            path,
            is_dir,
        });
        cx.notify();
    }

    /// Closes the explorer row context menu, if open.
    pub(crate) fn close_explorer_file_menu(&mut self, cx: &mut Context<Self>) {
        if self.explorer_file_menu.take().is_some() {
            cx.notify();
        }
    }

    /// Opens or closes the area-type dropdown of `area_id` (topbar click).
    pub(crate) fn toggle_area_dropdown(&mut self, area_id: NodeId, cx: &mut Context<Self>) {
        self.panels.layout.toggle_dropdown(area_id);
        cx.notify();
    }

    /// Shows the Help-menu informational dialog (About / update check),
    /// unless the unsaved-changes dialog is open.
    pub(crate) fn show_info_dialog(&mut self, kind: InfoDialogKind, cx: &mut Context<Self>) {
        if self.unsaved_dialog_open(cx) {
            return;
        }
        self.info_dialog = Some(kind);
        cx.notify();
    }

    /// Closes the Help-menu informational dialog, if open.
    pub(crate) fn hide_info_dialog(&mut self, cx: &mut Context<Self>) {
        if self.info_dialog.take().is_some() {
            cx.notify();
        }
    }

    /// True when any editor area's active tab shows the unsaved-changes
    /// dialog (which must not overlap the info dialog).
    pub(crate) fn unsaved_dialog_open(&self, cx: &App) -> bool {
        self.areas.values().any(|area| match area {
            AreaContent::Editor(entity) => entity
                .read(cx)
                .active_editor_tab()
                .is_some_and(|tab| tab.file.show_unsaved_changes_dialog),
        })
    }

    /// Toggles the maximized state of `area_id`'s tile (topbar click).
    pub(crate) fn toggle_area_maximize(&mut self, area_id: NodeId, cx: &mut Context<Self>) {
        self.panels.layout.toggle_maximize(area_id);
        self.sync_area_states(cx);
        cx.notify();
    }

    /// Split `area_id` at `ratio` with a sibling of the SAME kind. With
    /// `copy_content = false` the new Editor area starts with a fresh
    /// blank session; with `true` the sibling is a deep copy of the source
    /// editor's session (inner panel layout + tab list). Returns the new
    /// area's id.
    pub(crate) fn split_window_area(
        &mut self,
        area_id: NodeId,
        direction: Axis,
        ratio: f32,
        copy_content: bool,
        cx: &mut Context<Self>,
    ) -> Option<NodeId> {
        let new_id = self.panels.layout.split_leaf(area_id, direction, ratio)?;
        if self.panels.layout.tree.find_leaf_kind(area_id) == Some(WindowAreaKind::Editor) {
            let session = if copy_content {
                self.primary_editor()
                    .map(|editor| editor.update(cx, |editor, cx| editor.clone_session(cx)))
                    .unwrap_or_else(EditorSession::welcome)
            } else {
                EditorSession::welcome()
            };
            self.add_editor_area(new_id, session, cx);
        }
        self.sync_area_states(cx);
        Some(new_id)
    }

    /// Split `area_id` with a same-kind sibling, seeding the new Editor
    /// area per `copy_content` (see [`Self::split_window_area`]).
    pub(crate) fn split_area(
        &mut self,
        area_id: NodeId,
        direction: Axis,
        ratio: f32,
        copy_content: bool,
        cx: &mut Context<Self>,
    ) -> Option<NodeId> {
        self.split_window_area(area_id, direction, ratio, copy_content, cx)
    }

    /// Materialize the fresh sibling leaf of a plain-drag split: for an
    /// Editor-kind leaf, deep-copy the primary editor's session into a new
    /// entity. Non-Editor areas have no content to copy.
    pub(crate) fn seed_split_area(&mut self, new_id: NodeId, cx: &mut Context<Self>) {
        if self.panels.layout.tree.find_leaf_kind(new_id) == Some(WindowAreaKind::Editor) {
            let session = self
                .primary_editor()
                .map(|editor| editor.update(cx, |editor, cx| editor.clone_session(cx)))
                .unwrap_or_else(EditorSession::welcome);
            self.add_editor_area(new_id, session, cx);
        }
        self.sync_area_states(cx);
    }

    /// Close an area, clean up its editor session, and drop the content
    /// entity.
    pub(crate) fn close_window_area(&mut self, area_id: NodeId, cx: &mut Context<Self>) {
        self.panels.layout.close_leaf(area_id);
        self.remove_editor_area(area_id, cx);
        self.retained_editor_sessions.remove(&area_id);
        self.sync_area_states(cx);
    }

    /// Swap the area kind of area `a` and area `b`. Editor entities and
    /// retained sessions move along with the Editor kind so the new Editor
    /// area inherits the swapped-in tabs and panel layout.
    pub(crate) fn swap_window_area_kinds(&mut self, a: NodeId, b: NodeId, cx: &mut Context<Self>) {
        self.panels.layout.swap_kinds(a, b);
        self.swap_area_contents(a, b, cx);
        self.sync_area_states(cx);
    }

    /// Change an area's kind. Leaving Editor keeps the session while it
    /// still holds tabs (background editing — switching back restores it)
    /// and drops it once empty; the content entity is materialized or
    /// discarded accordingly.
    pub(crate) fn change_window_area_kind(
        &mut self,
        area_id: NodeId,
        kind: WindowAreaKind,
        cx: &mut Context<Self>,
    ) {
        let previous = self.panels.layout.tree.find_leaf_kind(area_id);
        self.panels.layout.set_kind(area_id, kind);
        self.sync_area_kind(area_id, kind == WindowAreaKind::Editor, cx);
        if kind == WindowAreaKind::Editor && previous != Some(WindowAreaKind::Editor) {
            // Entering Editor is an explicit interaction, so the area
            // becomes the active editor.
            self.panels.layout.activate_area(area_id);
        }
        self.sync_area_states(cx);
    }

    /// The editor area that a file open should target: the active area
    /// when it is an Editor area.
    pub(crate) fn active_editor_area(&self) -> Option<NodeId> {
        let area = self.panels.layout.active_area?;
        (self.panels.layout.tree.find_leaf_kind(area) == Some(WindowAreaKind::Editor))
            .then_some(area)
    }

    /// Opens `path` in the active editor's tab list, if an active editor
    /// exists. Returns whether the file was opened.
    pub(crate) fn open_file_in_active_editor(
        &mut self,
        path: &std::path::Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(area_id) = self.active_editor_area() else {
            return false;
        };
        let Some(editor) = self.editor_for(area_id) else {
            return false;
        };
        let _ = editor.update(cx, |editor, cx| {
            editor.open_file_in_area(area_id, path, window, cx)
        });
        true
    }
    /// Creates a fresh Editor entity serving `area_id` and registers it in
    /// the areas map. The new entity is wired to this Shell and its runtime
    /// registries are rebuilt for the (possibly cloned) document tabs.
    pub(crate) fn add_editor_area(
        &mut self,
        area_id: NodeId,
        session: EditorSession,
        cx: &mut Context<Self>,
    ) -> Entity<Editor> {
        let shell = cx.entity().downgrade();
        let editor = cx.new(|cx| Editor::with_session(area_id, session, cx));
        editor.update(cx, |editor, cx| {
            editor.shell = Some(shell);
            if !editor.session.tab_list.tabs.is_empty() {
                editor.rebuild_table_runtimes(cx);
                editor.rebuild_image_runtimes(cx);
                editor.refresh_preview_blocks(cx);
                editor.refresh_stable_document_snapshot(cx);
            }
        });
        self.areas
            .insert(area_id, AreaContent::Editor(editor.clone()));
        editor
    }

    /// Removes the content entity of `area_id` (if any) and returns its
    /// session so the caller can retain or discard it.
    pub(crate) fn remove_editor_area(
        &mut self,
        area_id: NodeId,
        cx: &mut Context<Self>,
    ) -> Option<EditorSession> {
        let AreaContent::Editor(entity) = self.areas.remove(&area_id)?;
        Some(entity.update(cx, |editor, _cx| {
            std::mem::replace(&mut editor.session, EditorSession::welcome())
        }))
    }

    /// Swaps the content entities of two areas so they follow the swapped
    /// area kinds; retained sessions swap along with them.
    pub(crate) fn swap_area_contents(&mut self, a: NodeId, b: NodeId, cx: &mut Context<Self>) {
        let content_a = self.areas.remove(&a);
        let content_b = self.areas.remove(&b);
        if let Some(AreaContent::Editor(entity)) = content_a {
            entity.update(cx, |editor, _cx| editor.area_id = b);
            self.areas.insert(b, AreaContent::Editor(entity));
        }
        if let Some(AreaContent::Editor(entity)) = content_b {
            entity.update(cx, |editor, _cx| editor.area_id = a);
            self.areas.insert(a, AreaContent::Editor(entity));
        }
        let retained_a = self.retained_editor_sessions.remove(&a);
        let retained_b = self.retained_editor_sessions.remove(&b);
        if let Some(session) = retained_a {
            self.retained_editor_sessions.insert(b, session);
        }
        if let Some(session) = retained_b {
            self.retained_editor_sessions.insert(a, session);
        }
    }

    /// Aligns the areas map with an area-kind change: entering Editor
    /// recreates the entity (restoring a retained session when one exists);
    /// leaving Editor removes the entity, retaining its session while it
    /// still holds tabs (background editing).
    pub(crate) fn sync_area_kind(
        &mut self,
        area_id: NodeId,
        is_editor: bool,
        cx: &mut Context<Self>,
    ) {
        if is_editor {
            if self.areas.contains_key(&area_id) {
                return;
            }
            let session = self
                .retained_editor_sessions
                .remove(&area_id)
                .unwrap_or_else(EditorSession::welcome);
            self.add_editor_area(area_id, session, cx);
        } else if let Some(session) = self.remove_editor_area(area_id, cx) {
            if !session.tab_list.tabs.is_empty() {
                self.retained_editor_sessions.insert(area_id, session);
            }
        }
    }

    /// First dirty tab across every editor area (foreground entities and
    /// retained background sessions), if any. Consumed by the window-close
    /// flow on the Shell.
    pub(crate) fn first_dirty_tab(&mut self, cx: &mut Context<Self>) -> Option<(NodeId, usize)> {
        for (area_id, session) in &self.retained_editor_sessions {
            for (index, tab) in session.tab_list.tabs.iter().enumerate() {
                if tab.file.dirty {
                    return Some((*area_id, index));
                }
            }
        }
        let ids: Vec<NodeId> = self
            .areas
            .iter()
            .filter_map(|(id, area)| match area {
                AreaContent::Editor(_) => Some(*id),
            })
            .collect();
        for area_id in ids {
            let dirty = match &self.areas[&area_id] {
                AreaContent::Editor(entity) => entity.read(cx).first_dirty_tab(),
            };
            if let Some(dirty) = dirty {
                return Some(dirty);
            }
        }
        None
    }

    /// Shift-drag default behavior: open the DRAGGED panel in a new
    /// independent window. The engine hands over a fresh single-leaf tree
    /// of the panel's kind; every Editor area's session is deep-copied
    /// (inner panel layout + tab list) and the explorer state is cloned.
    pub(crate) fn clone_container_into_new_window(
        &mut self,
        cloned: crate::splitter::policy::ClonedContainer<crate::app::window_area::WindowAreaKind>,
        cx: &mut Context<Self>,
    ) {
        let mut sessions = HashMap::new();
        let mut cloned_explorer = None;
        for (old_id, new_id) in &cloned.id_map {
            match cloned.tree.find_leaf_kind(*new_id) {
                Some(crate::app::window_area::WindowAreaKind::Editor) => {
                    if let Some(editor) = self.editor_for(*old_id) {
                        let session = editor.update(cx, |editor, cx| editor.clone_session(cx));
                        sessions.insert(*new_id, session);
                    }
                }
                Some(crate::app::window_area::WindowAreaKind::Explorer) => {
                    // The explorer model is window-global: deep-copy it so
                    // the new window shows the same file tree.
                    cloned_explorer = Some(self.panels.explorer.clone_for_new_window());
                }
                Some(crate::app::window_area::WindowAreaKind::Settings) | None => {}
            }
        }
        crate::app::window::open_cloned_window(
            cloned.tree,
            cloned.next_node_id,
            sessions,
            cloned_explorer,
            cx,
        );
    }

    /// The editor content entity of `area_id`, if the area holds one.
    pub(crate) fn editor_for(&self, area_id: NodeId) -> Option<&Entity<Editor>> {
        match self.areas.get(&area_id) {
            Some(AreaContent::Editor(editor)) => Some(editor),
            _ => None,
        }
    }

    /// Marks the dirty tab of `area_id` (from [`Self::first_dirty_tab`])
    /// as showing the unsaved-changes dialog, restoring its focus on
    /// cancel.
    fn prompt_unsaved_changes_for(
        &mut self,
        area_id: NodeId,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(editor) = self.editor_for(area_id).cloned() else {
            return;
        };
        let _ = editor.update(cx, |editor, cx| {
            let tab = &mut editor.session.tab_list.tabs[index];
            tab.file.show_unsaved_changes_dialog = true;
            tab.file.close_dialog_restore_focus = tab.focus.active_entity;
            cx.notify();
        });
    }

    /// Installs the window-close guard once: the callback aggregates dirty
    /// tabs across every editor area. Called on every render; idempotent.
    pub(crate) fn install_close_guard(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.close_guard_installed {
            return;
        }
        self.force_install_close_guard(window, cx);
    }

    /// Registers the window-close guard callback unconditionally (window
    /// construction path).
    pub(crate) fn force_install_close_guard(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let shell = cx.entity().downgrade();
        window.on_window_should_close(cx, move |window, cx| {
            shell
                .update(cx, |shell, cx| shell.on_window_should_close(window, cx))
                .unwrap_or(true)
        });
        self.close_guard_installed = true;
    }

    /// Called by the GPUI `Window::on_window_should_close` guard. Returns
    /// `true` when the window is safe to close (no dirty tab anywhere);
    /// otherwise marks the first dirty tab to show the unsaved-changes
    /// dialog and returns `false`.
    pub(crate) fn on_window_should_close(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some((area_id, index)) = self.first_dirty_tab(cx) else {
            return true;
        };
        self.prompt_unsaved_changes_for(area_id, index, cx);
        false
    }

    /// Initiate window-close flow, showing the unsaved-changes prompt when
    /// any document is dirty.
    pub(crate) fn request_close_current_window(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((area_id, index)) = self.first_dirty_tab(cx) else {
            window.remove_window();
            return;
        };
        self.prompt_unsaved_changes_for(area_id, index, cx);
    }

    /// CloseWindow action handler on the window root: fires before the
    /// global menu route and runs entirely on the Shell (no editor entity
    /// is locked when the aggregated dirty check reads them).
    pub(crate) fn on_close_window(
        &mut self,
        _: &crate::editor::actions::CloseWindow,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.request_close_current_window(window, cx);
    }

    /// Close-button routing for the custom titlebar: run the window-wide
    /// unsaved-changes-aware close flow.
    pub(crate) fn on_titlebar_close(
        &mut self,
        event: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !event.standard_click() {
            return;
        }
        self.request_close_current_window(window, cx);
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Track the viewport so area rectangles can be pushed to the editor
        // entities before their tiles render this frame.
        self.last_viewport = Some(window.viewport_size());
        self.sync_area_states(cx);
        self.install_close_guard(window, cx);

        let theme = cx.global::<ThemeManager>().current_arc();
        let strings = cx.global::<I18nManager>().strings_arc();
        let (titlebar, menu_panel, titlebar_height) = self.render_window_chrome(&theme, window, cx);

        let mut base = div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .relative()
            .bg(theme.colors.editor_background)
            .on_action(cx.listener(Self::on_close_window))
            .on_action(cx.listener(Self::on_toggle_explorer_action))
            .on_action(cx.listener(Self::on_close_explorer_folder_action))
            // A mouse-down anywhere in the window body closes an open
            // menu; titlebar and menu panels are siblings of the body
            // container, so their clicks never reach this listener.
            .on_any_mouse_down(cx.listener(Self::on_body_mouse_down));

        if let Some(titlebar) = titlebar {
            base = base.child(titlebar);
        }

        // The custom titlebar is absolutely positioned over the window top;
        // offset the body by its height so content starts below it. The
        // outer tiled layout renders every area; each Editor leaf embeds
        // its own content entity.
        let body = div()
            .w_full()
            .flex_1()
            .min_h(px(0.0))
            .pt(px(titlebar_height))
            .flex()
            .min_w(px(0.0))
            .child(self.render_tiled_layout(&theme, &strings, window, cx));
        base = base.child(body);

        if let Some(menu_panel) = menu_panel {
            base = base.child(menu_panel);
        }

        // The explorer row context menu floats at window level, above every
        // area, so its window-coordinate position stays accurate.
        if let Some(menu) = self.render_explorer_file_context_menu(&theme, cx) {
            base = base.child(menu);
        }

        // Window-level dialogs (unsaved changes, drop-replace, Help-menu
        // info) float above everything.
        if let Some(dialog) = self.render_window_dialogs(&theme, cx) {
            base = base.child(dialog);
        }

        base.into_any_element()
    }
}
