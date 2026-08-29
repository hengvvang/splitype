//! The window shell — the OS window's root entity.
//!
//! Owns the mapping from layout panel_contents to content entities (`PanelContent`),
//! the window-level chrome state (the in-window menu bar), and renders the
//! window chrome (custom titlebar + menu bar) above the primary editor's
//! content. Window-level state that currently lives on the Editor (panel
//! layout, overlays) migrates here incrementally; the Editor keeps only
//! its own editing state.

use std::collections::HashMap;

use gpui::*;

use crate::app::actions::{InstallCliTool, QuitApplication, UninstallCliTool};
use crate::app::window::chrome::MenuBarState;
use crate::app::window::panels::WindowPanels;
use workspace::{PanelId, WindowPanelKind};
use crate::editor::engine::controller::{DocumentTab, Editor, InfoDialogKind, OpenFileMode};
use crate::editor::engine::session::EditorSession;
use i18n::I18nManager;
use theme::ThemeManager;
use splitter::NodeId;
use splitter::tree::SplitAxis;

/// The polymorphic content of one area in the outer layout tree.
#[derive(Clone)]
pub enum PanelContent {
    /// An editor with its own tab list and pane layout.
    Editor(Entity<Editor>),
    /// A file explorer sidebar panel.
    Explorer,
    /// An in-window settings panel.
    Settings,
}

impl PanelContent {
    #[inline]
    pub fn is_editor(&self) -> bool {
        matches!(self, Self::Editor(_))
    }

    #[inline]
    pub fn as_editor(&self) -> Option<&Entity<Editor>> {
        match self {
            Self::Editor(editor) => Some(editor),
            _ => None,
        }
    }

}

/// Explorer row right-click menu: a window-level overlay rendered by the
/// Shell (it must float over every area at window coordinates).
#[derive(Clone)]
pub(crate) struct ExplorerFileMenuState {
    pub(crate) position: Point<Pixels>,
    pub(crate) path: std::path::PathBuf,
    pub(crate) is_dir: bool,
}

/// Scope of an unsaved-changes confirmation dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum UnsavedDialogScope {
    /// Triggered by titlebar window close, App Menu "Close Window" / "Quit", or Cmd/Ctrl+Shift+W.
    /// Targets ALL editor panels in the window and ALL open tabs.
    Window,
    /// Triggered by Editor panel topbar close button, or "Close Editor" command.
    /// Targets ONLY the specified Editor Panel and its tabs.
    EditorPanel(PanelId),
    /// Triggered by a specific tab's 'x' close button, or Cmd/Ctrl+W.
    /// Targets ONLY the single specified tab.
    Tab { panel_id: PanelId, index: usize },
}

impl UnsavedDialogScope {
    pub(crate) fn panel_id(&self) -> Option<PanelId> {
        match self {
            Self::Window => None,
            Self::EditorPanel(panel_id) => Some(*panel_id),
            Self::Tab { panel_id, .. } => Some(*panel_id),
        }
    }
}

/// State for the window-level unsaved-changes confirmation dialog.
#[derive(Clone, Debug)]
pub(crate) struct UnsavedDialogState {
    pub(crate) scope: UnsavedDialogScope,
    pub(crate) document_name: String,
    pub(crate) restore_focus: Option<EntityId>,
}

/// The OS window's root entity: content panel_contents + window lifecycle.
pub struct Shell {
    /// Content entity per outer area id. An area holds a content entity
    /// only while its kind is `Editor`; switching kinds moves the entity
    /// away (its session is retained while it still holds tabs).
    pub(crate) panel_contents: HashMap<PanelId, PanelContent>,
    /// Sessions of Editor panel_contents that left the Editor kind with tabs
    /// (background editing): switching back recreates the entity with the
    /// retained session.
    pub(crate) retained_editor_sessions: HashMap<PanelId, EditorSession>,
    /// Open/hover state for the in-window titlebar menu bar.
    pub(crate) menu_bar: MenuBarState,
    /// Window panel state: the outer area layout tree plus the explorer /
    /// outline / settings sidebar states.
    pub(crate) panels: WindowPanels,
    /// The last rendered viewport size; area rectangles are derived from
    /// it when the layout changes (see [`Self::sync_panel_states`]).
    pub(crate) last_viewport: Option<Size<Pixels>>,
    /// Explorer row right-click menu state (rendered at window level).
    pub(crate) explorer_file_menu: Option<ExplorerFileMenuState>,
    /// Informational dialog shown from the Help menu (About / update check).
    pub(crate) info_dialog: Option<InfoDialogKind>,
    /// Unsaved changes confirmation dialog state (Window, Editor Panel, or Single Tab).
    pub(crate) unsaved_dialog: Option<UnsavedDialogState>,
    /// True while an online update check is running in the background.
    pub(crate) update_check_in_progress: bool,
    /// Whether the window-close guard callback is installed on the window.
    pub(crate) close_guard_installed: bool,
    /// Randomized emoji indices for the About dialog background grid.
    pub(crate) about_bg_emojis: Vec<usize>,
}

impl Shell {
    /// The active editor area's active tab, if any. Reads through the
    /// editor content entity of the active area.
    pub(crate) fn active_editor_tab<'a>(&self, cx: &'a App) -> Option<&'a DocumentTab> {
        let panel = self.active_editor_panel()?;
        let editor = self.editor_for(panel)?;
        editor.read(cx).active_tab()
    }

    /// The window's primary (first) editor area content, if any.
    pub(crate) fn primary_editor(&self) -> Option<&Entity<Editor>> {
        self.panel_contents
            .values()
            .find_map(|content| content.as_editor())
    }

    /// The currently active/focused editor, or falls back to primary_editor.
    pub(crate) fn active_editor(&self) -> Option<&Entity<Editor>> {
        self.panels
            .layout
            .active_leaf
            .and_then(|leaf| self.editor_for(leaf))
            .or_else(|| self.primary_editor())
    }

    /// Recomputes every editor area's pushed state — the area rectangle,
    /// active-editor flag, and maximized flag — from the outer layout.
    /// Called after every layout change and on render (with the current
    /// viewport).
    pub(crate) fn sync_panel_states(&mut self, cx: &mut Context<Self>) {
        let Some(viewport) = self.last_viewport else {
            return;
        };
        let theme = cx.global::<ThemeManager>().current_arc();
        let titlebar_height = ui::custom_titlebar::custom_titlebar_height_for_target_os(
            std::env::consts::OS,
            Decorations::Server,
            &theme.dimensions,
        );
        let body_height = (f32::from(viewport.height) - titlebar_height).max(0.0);
        let body_size = size(viewport.width, px(body_height));
        let outer_rects = self.panels.layout.leaf_rects(body_size);
        let active = self.panels.layout.active_leaf;
        let leaf_count = self.panels.layout.tree.count_leaves();
        let editors: Vec<(PanelId, Entity<Editor>)> = self
            .panel_contents
            .iter()
            .filter_map(|(panel_id, content)| {
                content.as_editor().map(|entity| (*panel_id, entity.clone()))
            })
            .collect();
        for (panel_id, entity) in editors {
            let rect = outer_rects
                .iter()
                .find(|rect| rect.id == panel_id.0)
                .map(|rect| Bounds {
                    origin: point(px(rect.x), px(rect.y + titlebar_height)),
                    size: size(px(rect.width), px(rect.height)),
                });
            let is_maximized = self
                .panels
                .layout
                .tree
                .find_leaf(panel_id.0)
                .is_some_and(|panel| panel.maximized);
            entity.update(cx, |editor, _cx| {
                editor.panel_rect = rect;
                editor.is_active_panel = active == Some(panel_id.0);
                editor.is_maximized = is_maximized;
                editor.leaf_count = leaf_count;
            });
        }
    }

    /// Marks `panel_id` as the active editor area and re-pushes the
    /// active-flag to every editor entity.
    pub(crate) fn activate_panel(&mut self, panel_id: impl Into<PanelId>, cx: &mut Context<Self>) {
        self.panels.layout.activate_leaf(panel_id.into().0);
        self.sync_panel_states(cx);
    }

    /// Dismisses the primary editor's floating overlays (context menu,
    /// table-insert dialog). Explorer actions run on the Shell but the
    /// overlays still live on the editor entity.
    pub(crate) fn dismiss_contextual_overlays(&mut self, cx: &mut Context<Self>) {
        if let Some(editor) = self.primary_editor() {
            editor.update(cx, |editor, cx| editor.dismiss_contextual_overlays(cx));
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

    /// Opens or closes the area-type dropdown of `panel_id` (topbar click).
    pub(crate) fn toggle_panel_dropdown(
        &mut self,
        panel_id: impl Into<PanelId>,
        cx: &mut Context<Self>,
    ) {
        self.panels.layout.toggle_dropdown(panel_id.into().0);
        cx.notify();
    }

    /// Shows the Help-menu informational dialog (About / update check),
    /// unless the unsaved-changes dialog is open.
    pub(crate) fn show_info_dialog(&mut self, kind: InfoDialogKind, cx: &mut Context<Self>) {
        if self.is_unsaved_dialog_open(cx) {
            return;
        }
        if kind == InfoDialogKind::About {
            use std::time::SystemTime;
            let seed = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(42);
            let mut rng_state = seed as u64;
            if rng_state == 0 {
                rng_state = 0xdeadbeef;
            }
            self.about_bg_emojis = (0..80)
                .map(|_| {
                    rng_state ^= rng_state << 13;
                    rng_state ^= rng_state >> 7;
                    rng_state ^= rng_state << 17;
                    (rng_state as usize) % 18
                })
                .collect();
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

    /// True when the unsaved-changes dialog is active.
    pub(crate) fn is_unsaved_dialog_open(&self, _cx: &App) -> bool {
        self.unsaved_dialog.is_some()
    }

    /// Toggles the maximized state of `panel_id`'s tile (topbar click).
    pub(crate) fn toggle_panel_maximize(
        &mut self,
        panel_id: impl Into<PanelId>,
        cx: &mut Context<Self>,
    ) {
        self.panels.layout.toggle_maximize(panel_id.into().0);
        self.sync_panel_states(cx);
        cx.notify();
    }

    /// Split `panel_id` at `ratio` with a sibling of the SAME kind. With
    /// `copy_content = false` the new Editor area starts with a fresh
    /// blank session; with `true` the sibling is a deep copy of the source
    /// editor's session (pane layout + tab list). Returns the new
    /// area's id.
    pub(crate) fn split_panel(
        &mut self,
        panel_id: impl Into<PanelId>,
        axis: SplitAxis,
        ratio: f32,
        copy_content: bool,
        cx: &mut Context<Self>,
    ) -> Option<PanelId> {
        let panel_id = panel_id.into();
        let target_leaf_id = self.panels.layout.resolve_leaf(panel_id.0)?;
        let new_id = self.panels.layout.split_leaf(target_leaf_id, axis, ratio)?;
        if self.panels.layout.tree.find_leaf_kind(target_leaf_id) == Some(WindowPanelKind::Editor) {
            let session = if copy_content {
                self.primary_editor()
                    .map(|editor| editor.update(cx, |editor, cx| editor.clone_session(cx)))
                    .unwrap_or_else(EditorSession::welcome)
            } else {
                EditorSession::welcome()
            };
            self.add_editor_panel(new_id, session, cx);
        }
        self.sync_panel_states(cx);
        Some(PanelId(new_id))
    }

    /// Materialize the fresh sibling leaf of a plain-drag split: for an
    /// Editor-kind leaf, deep-copy the primary editor's session into a new
    /// entity. Non-Editor panel_contents have no content to copy.
    pub(crate) fn seed_split_panel(&mut self, new_id: impl Into<PanelId>, cx: &mut Context<Self>) {
        let new_id = new_id.into();
        if self.panels.layout.tree.find_leaf_kind(new_id.0) == Some(WindowPanelKind::Editor) {
            let session = self
                .primary_editor()
                .map(|editor| editor.update(cx, |editor, cx| editor.clone_session(cx)))
                .unwrap_or_else(EditorSession::welcome);
            self.add_editor_panel(new_id, session, cx);
        }
        self.sync_panel_states(cx);
    }

    /// Close an area, clean up its editor session, and drop the content
    /// entity.
    pub(crate) fn close_panel(&mut self, panel_id: impl Into<PanelId>, cx: &mut Context<Self>) {
        let panel_id = panel_id.into();
        if let Some(target_leaf_id) = self.panels.layout.resolve_leaf(panel_id.0) {
            self.panels.layout.close_leaf(target_leaf_id);
            self.remove_editor_panel(target_leaf_id, cx);
            self.retained_editor_sessions.remove(&PanelId(target_leaf_id));
            self.sync_panel_states(cx);
        }
    }

    /// Clean up a joined panel's editor session and sync panel states.
    pub(crate) fn handle_joined_panel(&mut self, removed_id: NodeId, cx: &mut Context<Self>) {
        self.remove_editor_panel(removed_id, cx);
        self.retained_editor_sessions.remove(&PanelId(removed_id));
        self.sync_panel_states(cx);
    }

    /// Update panel contents when a swap operation has already swapped tree kinds.
    pub(crate) fn handle_swapped_panels(&mut self, a: NodeId, b: NodeId, cx: &mut Context<Self>) {
        self.swap_panel_contents(a, b, cx);
        self.sync_panel_states(cx);
    }

    /// Change an area's kind. Leaving Editor keeps the session while it
    /// still holds tabs (background editing — switching back restores it)
    /// and drops it once empty; the content entity is materialized or
    /// discarded accordingly.
    pub(crate) fn change_panel_kind(
        &mut self,
        panel_id: NodeId,
        kind: WindowPanelKind,
        cx: &mut Context<Self>,
    ) {
        let previous = self.panels.layout.tree.find_leaf_kind(panel_id);
        self.panels.layout.set_kind(panel_id, kind);
        self.sync_panel_kind(panel_id, kind == WindowPanelKind::Editor, cx);
        if kind == WindowPanelKind::Editor && previous != Some(WindowPanelKind::Editor) {
            // Entering Editor is an explicit interaction, so the area
            // becomes the active editor.
            self.panels.layout.activate_leaf(panel_id);
        }
        self.sync_panel_states(cx);
    }

    /// The editor area that a file open should target: the active area
    /// when it is an Editor area, or falls back to the sole foreground
    /// Editor (when only one exists) or the most recently activated Editor.
    #[inline]
    pub(crate) fn active_editor_panel(&self) -> Option<NodeId> {
        self.panels.layout.active_leaf_of_kind(WindowPanelKind::Editor)
    }

    /// Opens `path` in the active editor's tab list, if an active editor
    /// exists. Returns whether the file was opened.
    pub(crate) fn open_file_in_active_editor(
        &mut self,
        path: &std::path::Path,
        mode: OpenFileMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(panel_id) = self.active_editor_panel() else {
            return false;
        };
        self.panels.layout.activate_leaf(panel_id);
        let Some(editor) = self.editor_for(panel_id) else {
            return false;
        };
        editor.update(cx, |editor, cx| editor.open_file_in_panel(path, mode, window, cx));
        true
    }
    /// Creates a fresh Editor entity serving `panel_id` and registers it in
    /// the panel_contents map. The new entity is wired to this Shell and its
    /// reference registries are rebuilt for the (possibly cloned) document
    //    /// Creates a fresh Editor entity serving `panel_id` and registers it in
    /// the panel_contents map. The new entity is wired to this Shell and its
    /// reference registries are rebuilt for the (possibly cloned) document
    /// tabs.
    pub(crate) fn add_editor_panel(
        &mut self,
        panel_id: impl Into<PanelId>,
        session: EditorSession,
        cx: &mut Context<Self>,
    ) -> Entity<Editor> {
        let panel_id = panel_id.into();
        let shell = cx.entity().downgrade();
        let editor = cx.new(|cx| crate::editor::Editor::with_session(panel_id, session, cx));

        editor.update(cx, |editor, cx| {
            editor.shell = Some(shell);
            if editor.session.has_tabs() {
                editor.rebuild_table_grids(cx);
                editor.rebuild_reference_registries(cx);
                let pane_id = editor.active_pane_id();
                editor.refresh_preview_blocks(pane_id, cx);
                editor.refresh_stable_document_snapshot(cx);
            }
        });
        self.panel_contents
            .insert(panel_id, PanelContent::Editor(editor.clone()));
        editor
    }

    /// Sync open document tabs across all panels when a file/directory is moved or renamed.
    pub(crate) fn sync_open_tabs_after_fs_change(
        &self,
        change: &crate::explorer::state::undo::ExplorerChange,
        cx: &mut App,
    ) {
        use crate::explorer::state::undo::ExplorerChange;

        match change {
            ExplorerChange::Moved { from, to } | ExplorerChange::Renamed { from, to } => {
                for content in self.panel_contents.values() {
                    if let Some(editor) = content.as_editor() {
                        editor.update(cx, |ed, _cx| {
                            ed.update_tab_path(from, to);
                        });
                    }
                }
            }
            ExplorerChange::Batch(changes) => {
                for c in changes {
                    self.sync_open_tabs_after_fs_change(c, cx);
                }
            }
            _ => {}
        }
    }

    /// Removes the content entity of `panel_id` (if any) and returns its
    /// session so the caller can retain or discard it.
    pub(crate) fn remove_editor_panel(
        &mut self,
        panel_id: impl Into<PanelId>,
        cx: &mut Context<Self>,
    ) -> Option<EditorSession> {
        let panel_id = panel_id.into();
        let content = self.panel_contents.remove(&panel_id)?;
        let entity = match content {
            PanelContent::Editor(entity) => entity,
            _ => return None,
        };
        Some(entity.update(cx, |editor, cx| {
            editor.clear_search_highlights_from_document(cx);
            editor.search.visible = false;
            editor.search.matches.clear();
            std::mem::replace(&mut editor.session, EditorSession::welcome())
        }))
    }

    /// Swaps the content entities of two panel_contents so they follow the swapped
    /// area kinds; retained sessions swap along with them.
    pub(crate) fn swap_panel_contents(
        &mut self,
        a: impl Into<PanelId>,
        b: impl Into<PanelId>,
        cx: &mut Context<Self>,
    ) {
        let a = a.into();
        let b = b.into();
        let content_a = self.panel_contents.remove(&a);
        let content_b = self.panel_contents.remove(&b);
        if let Some(content) = content_a {
            if let Some(entity) = content.as_editor() {
                entity.update(cx, |editor, _cx| editor.panel_id = b);
            }
            self.panel_contents.insert(b, content);
        }
        if let Some(content) = content_b {
            if let Some(entity) = content.as_editor() {
                entity.update(cx, |editor, _cx| editor.panel_id = a);
            }
            self.panel_contents.insert(a, content);
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

    /// Move a panel and dock it to a target edge, rearranging surrounding panels and updating contents.
    /// Update panel contents when a move_and_dock operation has rearranged the layout tree.
    pub(crate) fn handle_moved_and_docked_panel(
        &mut self,
        source_id: impl Into<PanelId>,
        target_id: impl Into<PanelId>,
        new_leaf_id: impl Into<PanelId>,
        dock_target: splitter::sessions::AreaDockTarget,
        cx: &mut Context<Self>,
    ) {
        let source_id = source_id.into();
        let target_id = target_id.into();
        let new_leaf_id = new_leaf_id.into();
        let source_content = self.panel_contents.remove(&source_id);
        let source_retained = self.retained_editor_sessions.remove(&source_id);
        let target_content = self.panel_contents.remove(&target_id);
        let target_retained = self.retained_editor_sessions.remove(&target_id);

        let source_first = matches!(
            dock_target,
            splitter::sessions::AreaDockTarget::Left
                | splitter::sessions::AreaDockTarget::Top
        );

        if source_first {
            // target_id gets source content
            if let Some(content) = source_content {
                if let Some(entity) = content.as_editor() {
                    entity.update(cx, |editor, _cx| editor.panel_id = target_id);
                }
                self.panel_contents.insert(target_id, content);
            }
            if let Some(session) = source_retained {
                self.retained_editor_sessions.insert(target_id, session);
            }
            // new_leaf_id gets target content
            if let Some(content) = target_content {
                if let Some(entity) = content.as_editor() {
                    entity.update(cx, |editor, _cx| editor.panel_id = new_leaf_id);
                }
                self.panel_contents.insert(new_leaf_id, content);
            }
            if let Some(session) = target_retained {
                self.retained_editor_sessions.insert(new_leaf_id, session);
            }
        } else {
            // target_id keeps target content
            if let Some(content) = target_content {
                if let Some(entity) = content.as_editor() {
                    entity.update(cx, |editor, _cx| editor.panel_id = target_id);
                }
                self.panel_contents.insert(target_id, content);
            }
            if let Some(session) = target_retained {
                self.retained_editor_sessions.insert(target_id, session);
            }
            // new_leaf_id gets source content
            if let Some(content) = source_content {
                if let Some(entity) = content.as_editor() {
                    entity.update(cx, |editor, _cx| editor.panel_id = new_leaf_id);
                }
                self.panel_contents.insert(new_leaf_id, content);
            }
            if let Some(session) = source_retained {
                self.retained_editor_sessions.insert(new_leaf_id, session);
            }
        }
        self.sync_panel_states(cx);
    }

    /// Aligns the panel_contents map with an area-kind change: entering Editor
    /// recreates the entity (restoring a retained session when one exists);
    /// leaving Editor removes the entity, retaining its session while it
    /// still holds tabs (background editing).
    pub(crate) fn sync_panel_kind(
        &mut self,
        panel_id: impl Into<PanelId>,
        is_editor: bool,
        cx: &mut Context<Self>,
    ) {
        let panel_id = panel_id.into();
        if is_editor {
            if self.panel_contents.get(&panel_id).is_some_and(|c| c.is_editor()) {
                return;
            }
            let session = self
                .retained_editor_sessions
                .remove(&panel_id)
                .unwrap_or_else(EditorSession::welcome);
            self.add_editor_panel(panel_id, session, cx);
        } else {
            if let Some(session) = self.remove_editor_panel(panel_id, cx) {
                if session.has_tabs() {
                    self.retained_editor_sessions.insert(panel_id, session);
                }
            }
            if let Some(kind) = self.panels.layout.tree.find_leaf_kind(panel_id.0) {
                match kind {
                    WindowPanelKind::Explorer => {
                        self.panel_contents.insert(panel_id, PanelContent::Explorer);
                    }
                    WindowPanelKind::Settings => {
                        self.panel_contents.insert(panel_id, PanelContent::Settings);
                    }
                    WindowPanelKind::Editor => {}
                }
            }
        }
    }

    /// First dirty tab across every editor area (foreground entities and
    /// retained background sessions), if any. Consumed by the window-close
    /// flow on the Shell.
    pub(crate) fn first_dirty_tab(&mut self, cx: &mut Context<Self>) -> Option<(PanelId, usize)> {
        for (panel_id, session) in &self.retained_editor_sessions {
            for (index, tab) in session.tabs().enumerate() {
                if tab.file.dirty {
                    return Some((*panel_id, index));
                }
            }
        }
        for content in self.panel_contents.values() {
            if let Some(entity) = content.as_editor() {
                if let Some(dirty) = entity.read(cx).first_dirty_tab() {
                    return Some(dirty);
                }
            }
        }
        None
    }

    /// Shift-drag default behavior: open the DRAGGED panel in a new
    /// independent window. The engine hands over a fresh single-leaf tree
    /// of the panel's kind; every Editor area's session is deep-copied
    /// (pane layout + tab list) and the explorer state is cloned.
    pub(crate) fn clone_container_into_new_window(
        &mut self,
        cloned: splitter::policy::ClonedContainer<
            workspace::WindowPanelKind,
        >,
        cx: &mut Context<Self>,
    ) {
        let mut sessions = HashMap::new();
        let mut cloned_explorer = None;
        for (old_id, new_id) in &cloned.id_map {
            match cloned.tree.find_leaf_kind(*new_id) {
                Some(workspace::WindowPanelKind::Editor) => {
                    if let Some(editor) = self.editor_for(*old_id) {
                        let session = editor.update(cx, |editor, cx| editor.clone_session(cx));
                        sessions.insert(PanelId(*new_id), session);
                    }
                }
                Some(workspace::WindowPanelKind::Explorer) => {
                    // The explorer model is window-global: deep-copy it so
                    // the new window shows the same file tree.
                    cloned_explorer = Some(self.panels.explorer.clone_for_new_window());
                }
                Some(workspace::WindowPanelKind::Settings) | None => {}
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

    /// The editor content entity of `panel_id`, if the area holds one.
    pub(crate) fn editor_for(&self, panel_id: impl Into<PanelId>) -> Option<&Entity<Editor>> {
        self.panel_contents.get(&panel_id.into()).and_then(|c| c.as_editor())
    }

    /// Returns total number of leaves in the outer window layout tree.
    pub(crate) fn layout_leaf_count(&self) -> usize {
        self.panels.layout.tree.count_leaves()
    }

    /// Number of dirty tabs in the specified panel (if it's an editor).
    pub(crate) fn dirty_tab_info_in_panel(
        &self,
        panel_id: impl Into<PanelId>,
        cx: &App,
    ) -> (usize, String) {
        let panel_id = panel_id.into();
        let mut count = 0;
        let mut first_name = String::new();

        if let Some(editor) = self.editor_for(panel_id) {
            let ed = editor.read(cx);
            for tab in ed.session.tabs() {
                if tab.file.dirty {
                    count += 1;
                    if first_name.is_empty() {
                        first_name = tab
                            .file
                            .path
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| "Untitled".to_string());
                    }
                }
            }
        } else if let Some(session) = self.retained_editor_sessions.get(&panel_id) {
            for tab in session.tabs() {
                if tab.file.dirty {
                    count += 1;
                    if first_name.is_empty() {
                        first_name = tab
                            .file
                            .path
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| "Untitled".to_string());
                    }
                }
            }
        }

        (count, first_name)
    }

    /// Prompts window-level unsaved-changes dialog.
    pub(crate) fn prompt_close_window(&mut self, cx: &mut Context<Self>) {
        let Some((panel_id, index)) = self.first_dirty_tab(cx) else {
            return;
        };
        let first_dirty_name = self
            .editor_for(panel_id)
            .and_then(|e| {
                let editor = e.read(cx);
                editor.session.tab(index).map(|t| {
                    t.file
                        .path
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "Untitled".to_string())
                })
            })
            .unwrap_or_else(|| "Untitled".to_string());

        let restore_focus = self.active_editor().and_then(|e| {
            let ed = e.read(cx);
            ed.pane_state_ref(ed.active_pane_id())
                .and_then(|p| p.as_wysiwyg())
                .and_then(|p| p.focus.active_entity)
        });

        self.unsaved_dialog = Some(UnsavedDialogState {
            scope: UnsavedDialogScope::Window,
            document_name: first_dirty_name,
            restore_focus,
        });
        cx.notify();
    }

    /// Prompts panel-level unsaved-changes dialog.
    pub(crate) fn prompt_close_editor_for(
        &mut self,
        panel_id: impl Into<PanelId>,
        cx: &mut Context<Self>,
    ) {
        let panel_id = panel_id.into();
        let (dirty_count, first_dirty_name) = self.dirty_tab_info_in_panel(panel_id, cx);
        if dirty_count == 0 {
            return;
        }

        let restore_focus = self.editor_for(panel_id).and_then(|e| {
            let ed = e.read(cx);
            ed.pane_state_ref(ed.active_pane_id())
                .and_then(|p| p.as_wysiwyg())
                .and_then(|p| p.focus.active_entity)
        });

        self.unsaved_dialog = Some(UnsavedDialogState {
            scope: UnsavedDialogScope::EditorPanel(panel_id),
            document_name: first_dirty_name,
            restore_focus,
        });
        cx.notify();
    }

    /// Prompts single-tab-level unsaved-changes dialog.
    pub(crate) fn prompt_close_tab(
        &mut self,
        panel_id: impl Into<PanelId>,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        let panel_id = panel_id.into();
        let document_name = self
            .editor_for(panel_id)
            .and_then(|e| {
                let editor = e.read(cx);
                editor.session.tab(index).map(|t| {
                    t.file
                        .path
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "Untitled".to_string())
                })
            })
            .unwrap_or_else(|| "Untitled".to_string());

        let restore_focus = self.editor_for(panel_id).and_then(|e| {
            let ed = e.read(cx);
            ed.pane_state_ref(ed.active_pane_id())
                .and_then(|p| p.as_wysiwyg())
                .and_then(|p| p.focus.active_entity)
        });

        self.unsaved_dialog = Some(UnsavedDialogState {
            scope: UnsavedDialogScope::Tab { panel_id, index },
            document_name,
            restore_focus,
        });
        cx.notify();
    }

    /// Request closing an editor panel: checks for unsaved changes in this panel.
    pub(crate) fn request_close_panel(
        &mut self,
        panel_id: impl Into<PanelId>,
        cx: &mut Context<Self>,
    ) {
        let panel_id = panel_id.into();
        let (dirty_count, _) = self.dirty_tab_info_in_panel(panel_id, cx);
        if dirty_count > 0 {
            self.prompt_close_editor_for(panel_id, cx);
        } else if self.layout_leaf_count() > 1 {
            self.close_panel(panel_id, cx);
        } else if let Some(editor) = self.editor_for(panel_id) {
            editor.update(cx, |editor, cx| {
                editor.session.clear_tabs();
                cx.notify();
            });
        }
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
        if self.first_dirty_tab(cx).is_none() {
            return true;
        }
        self.prompt_close_window(cx);
        false
    }

    /// Initiate window-close flow, showing the unsaved-changes prompt when
    /// any document is dirty.
    pub(crate) fn request_close_current_window(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.first_dirty_tab(cx).is_none() {
            window.remove_window();
            return;
        }
        self.prompt_close_window(cx);
    }

    /// CloseWindow action handler on the window root: fires before the
    /// global menu route and runs entirely on the Shell (no editor entity
    /// is locked when the aggregated dirty check reads them).
    pub(crate) fn on_close_window(
        &mut self,
        _: &crate::app::actions::CloseWindow,
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

    pub(crate) fn on_quit_application(
        &mut self,
        _: &QuitApplication,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        crate::app::menus::request_quit_application(cx);
    }

    pub(crate) fn on_install_cli_tool(
        &mut self,
        _: &InstallCliTool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        crate::app::cli::install::install_cli_tool(cx);
    }

    pub(crate) fn on_uninstall_cli_tool(
        &mut self,
        _: &UninstallCliTool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        crate::app::cli::install::uninstall_cli_tool(cx);
    }

    pub(crate) fn on_toggle_maximize_area_action(
        &mut self,
        _: &crate::app::actions::ToggleMaximizeArea,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(active) = self.panels.layout.active_leaf {
            self.panels.layout.toggle_maximize(active);
            cx.notify();
        }
    }

    // ── workspace layout actions (dispatched by panel topbars) ──────────

    fn on_toggle_kind_dropdown(
        &mut self,
        action: &workspace::actions::ToggleKindDropdown,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.panels.layout.toggle_dropdown(action.panel);
        cx.notify();
    }

    fn on_split_panel(
        &mut self,
        action: &workspace::actions::SplitPanel,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.split_panel(PanelId(action.panel), action.axis, 0.5, true, cx);
    }

    fn on_toggle_panel_maximized(
        &mut self,
        action: &workspace::actions::TogglePanelMaximized,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.panels.layout.toggle_maximize(action.panel);
        cx.notify();
    }

    fn on_close_panel(
        &mut self,
        action: &workspace::actions::ClosePanel,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_panel(PanelId(action.panel), cx);
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Track the viewport so area rectangles can be pushed to the editor
        // entities before their tiles render this frame.
        self.last_viewport = Some(window.viewport_size());
        self.sync_panel_states(cx);
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
            .font(theme::TypographyStore::ui_font(cx))
            .on_action(cx.listener(Self::on_close_window))
            .on_action(cx.listener(Self::on_toggle_explorer_action))
            .on_action(cx.listener(Self::on_toggle_maximize_area_action))
            .on_action(cx.listener(Self::on_close_explorer_folder_action))
            .on_action(cx.listener(Self::on_quit_application))
            .on_action(cx.listener(Self::on_install_cli_tool))
            .on_action(cx.listener(Self::on_uninstall_cli_tool))
            .on_action(cx.listener(Self::on_toggle_kind_dropdown))
            .on_action(cx.listener(Self::on_split_panel))
            .on_action(cx.listener(Self::on_toggle_panel_maximized))
            .on_action(cx.listener(Self::on_close_panel))
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
