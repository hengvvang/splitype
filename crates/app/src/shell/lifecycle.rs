//! Panel and session lifecycle operations on the window Shell.

use gpui::*;
use std::collections::HashMap;
use std::path::Path;

use super::Shell;
use super::host_bridge::{ShellEditorHost, ShellPanelHost};
use crate::window::open_cloned_window;
use core_contracts::TabKind;
use editor::{Editor, EditorSession};
use splitter::NodeId;
use splitter::policy::ClonedContainer;
use splitter::sessions::AreaDockTarget;
use splitter::tree::SplitAxis;
use window::actions::{OpenPath, OpenPathInSplit};
use window::{PanelId, PanelKind, PanelView};

impl Shell {
    /// Marks `panel_id` as the active editor area and re-pushes the
    /// active-flag to every editor entity.
    pub(crate) fn activate_panel(&mut self, panel_id: impl Into<PanelId>, cx: &mut Context<Self>) {
        self.panels.layout.activate_leaf(panel_id.into().0);
        self.sync_panel_states(cx);
    }

    /// Split `panel_id` at `ratio` with a sibling of the SAME kind.
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
        if self.panels.layout.tree.find_leaf_kind(target_leaf_id)
            == Some(window::PanelKind::new("editor"))
        {
            let session = if copy_content {
                self.primary_editor()
                    .map(|editor| editor.update(cx, |editor, cx| editor.clone_session(cx)))
                    .unwrap_or_else(EditorSession::empty)
            } else {
                EditorSession::empty()
            };
            self.add_editor_panel(new_id, session, cx);
        } else if let Some(kind) = self.panels.layout.tree.find_leaf_kind(new_id) {
            self.ensure_registered_panel_view(new_id, kind, cx);
        }
        self.sync_panel_states(cx);
        Some(PanelId(new_id))
    }

    /// Materializes a panel view for `panel_id` through the registry with a
    /// mandatory host handle.
    pub(crate) fn ensure_registered_panel_view(
        &mut self,
        panel_id: impl Into<PanelId>,
        kind: PanelKind,
        cx: &mut Context<Self>,
    ) -> bool {
        let panel_id = panel_id.into();
        if self.panel_views.contains_key(&panel_id) {
            return true;
        }
        let host = ShellPanelHost::shared(cx.entity().downgrade());
        match window::PanelRegistry::create_registered_panel(kind, panel_id, host, cx) {
            Ok(Some(view)) => {
                self.panel_views.insert(panel_id, view);
                true
            }
            Ok(None) => {
                tracing::error!(%kind, "no panel descriptor is registered");
                false
            }
            Err(error) => {
                tracing::error!(%kind, %error, "failed to create registered panel");
                false
            }
        }
    }

    /// Removes a panel view of any kind from the layout-to-view mapping.
    pub(crate) fn remove_panel_view(&mut self, panel_id: impl Into<PanelId>) -> Option<Box<dyn PanelView>> {
        self.panel_views.remove(&panel_id.into())
    }

    /// Materialize the fresh sibling leaf of a plain-drag split.
    pub(crate) fn seed_split_panel(&mut self, new_id: impl Into<PanelId>, cx: &mut Context<Self>) {
        let new_id = new_id.into();
        if self.panels.layout.tree.find_leaf_kind(new_id.0)
            == Some(window::PanelKind::new("editor"))
        {
            let session = self
                .primary_editor()
                .map(|editor| editor.update(cx, |editor, cx| editor.clone_session(cx)))
                .unwrap_or_else(EditorSession::empty);
            self.add_editor_panel(new_id, session, cx);
        }
        self.sync_panel_states(cx);
    }

    /// Close an area, clean up its editor session, and drop the content entity.
    pub(crate) fn close_panel(&mut self, panel_id: impl Into<PanelId>, cx: &mut Context<Self>) {
        let panel_id = panel_id.into();
        if let Some(target_leaf_id) = self.panels.layout.resolve_leaf(panel_id.0) {
            self.panels.layout.close_leaf(target_leaf_id);
            self.remove_editor_panel(target_leaf_id, cx);
            self.remove_panel_view(target_leaf_id);
            self.retained_editor_sessions.remove(&PanelId(target_leaf_id));
            self.sync_panel_states(cx);
        }
    }

    /// Clean up a joined panel's editor session and sync panel states.
    pub(crate) fn handle_joined_panel(&mut self, removed_id: NodeId, cx: &mut Context<Self>) {
        self.remove_editor_panel(removed_id, cx);
        self.remove_panel_view(removed_id);
        self.retained_editor_sessions.remove(&PanelId(removed_id));
        self.sync_panel_states(cx);
    }

    /// Update panel contents when a swap operation has already swapped tree kinds.
    pub(crate) fn handle_swapped_panels(&mut self, a: NodeId, b: NodeId, cx: &mut Context<Self>) {
        self.swap_panel_contents(a, b, cx);
        self.sync_panel_states(cx);
    }

    /// Change an area's kind.
    pub(crate) fn change_panel_kind(
        &mut self,
        panel_id: NodeId,
        kind: PanelKind,
        cx: &mut Context<Self>,
    ) {
        let previous = self.panels.layout.tree.find_leaf_kind(panel_id);
        self.panels.layout.set_kind(panel_id, kind);
        self.sync_panel_kind(panel_id, kind == window::PanelKind::new("editor"), cx);
        if kind == window::PanelKind::new("editor")
            && previous != Some(window::PanelKind::new("editor"))
        {
            self.panels.layout.activate_leaf(panel_id);
        }
        self.sync_panel_states(cx);
    }

    /// The editor area that a file open should target.
    #[inline]
    pub(crate) fn active_editor_panel(&self) -> Option<NodeId> {
        self.panels
            .layout
            .active_leaf_of_kind(window::PanelKind::new("editor"))
    }

    /// Opens `path` in the active editor's tab list, if an active editor exists.
    pub(crate) fn open_file_in_active_editor(
        &mut self,
        path: &Path,
        kind: TabKind,
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
        editor.update(cx, |editor, cx| {
            editor.open_file_in_panel(path, kind, window, cx)
        });
        cx.notify();
        true
    }

    /// Creates a fresh Editor entity serving `panel_id` and registers it in the panel_views map.
    pub(crate) fn add_editor_panel(
        &mut self,
        panel_id: impl Into<PanelId>,
        session: EditorSession,
        cx: &mut Context<Self>,
    ) -> Entity<Editor> {
        let panel_id = panel_id.into();
        let shell = cx.entity().downgrade();
        let editor = cx.new(|cx| Editor::with_session(panel_id, session, cx));

        editor.update(cx, |editor, cx| {
            editor.host = Some(std::sync::Arc::new(ShellEditorHost::new(shell)));
            if editor.session.has_tabs() {
                editor.sync_panes_with_active_tab(cx);
            }
        });
        self.panel_views.insert(
            panel_id,
            Box::new(editor::EditorPanelView::new(editor.clone())),
        );
        editor
    }

    /// Sync open document tabs across all panels when a file/directory is moved or renamed.
    pub(crate) fn sync_open_tabs_after_fs_change(
        &mut self,
        change: &explorer::state::undo::ExplorerChange,
        cx: &mut App,
    ) {
        use explorer::state::undo::ExplorerChange;

        match change {
            ExplorerChange::Moved { from, to } | ExplorerChange::Renamed { from, to } => {
                for view in self.panel_views.values_mut() {
                    view.on_fs_path_renamed(from, to, cx);
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

    pub(crate) fn on_update_open_tab_paths(
        &mut self,
        action: &explorer::ops::selection::UpdateOpenTabPaths,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sync_open_tabs_after_fs_change(
            &explorer::state::undo::ExplorerChange::Renamed {
                from: std::path::PathBuf::from(&action.from),
                to: std::path::PathBuf::from(&action.to),
            },
            cx,
        );
    }

    pub(crate) fn on_open_in_editor(
        &mut self,
        action: &OpenPath,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let path = std::path::PathBuf::from(&action.path);
        let kind = if action.persistent {
            TabKind::Persistent
        } else {
            TabKind::Transient
        };
        self.open_file_in_active_editor(&path, kind, window, cx);
    }

    pub(crate) fn on_open_in_split(
        &mut self,
        action: &OpenPathInSplit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let path = std::path::PathBuf::from(&action.path);
        let open_in_active = |shell: &mut Self, window: &mut Window, cx: &mut Context<Self>| {
            shell.open_file_in_active_editor(&path, TabKind::Persistent, window, cx);
        };
        let Some(active) = self.active_editor_panel() else {
            open_in_active(self, window, cx);
            return;
        };
        let Some(new_panel) =
            self.split_panel(PanelId(active), SplitAxis::Horizontal, 0.5, false, cx)
        else {
            open_in_active(self, window, cx);
            return;
        };
        let Some(editor) = self.editor_for(new_panel).cloned() else {
            open_in_active(self, window, cx);
            return;
        };
        self.panels.layout.activate_leaf(new_panel.0);
        editor.update(cx, |editor, cx| {
            editor.open_file_in_panel(&path, TabKind::Persistent, window, cx)
        });
    }

    pub(crate) fn remove_editor_panel(
        &mut self,
        panel_id: impl Into<PanelId>,
        cx: &mut Context<Self>,
    ) -> Option<EditorSession> {
        let panel_id = panel_id.into();
        let view = self.panel_views.remove(&panel_id)?;
        let panel = view.as_any().downcast_ref::<editor::EditorPanelView>()?;
        let entity = panel.editor.clone();
        Some(entity.update(cx, |editor, cx| {
            editor.clear_search_highlights_from_document(cx);
            editor.search.visible = false;
            editor.search.matches.clear();
            std::mem::replace(&mut editor.session, EditorSession::empty())
        }))
    }

    pub(crate) fn swap_panel_contents(
        &mut self,
        a: impl Into<PanelId>,
        b: impl Into<PanelId>,
        cx: &mut Context<Self>,
    ) {
        let a = a.into();
        let b = b.into();
        let view_a = self.panel_views.remove(&a);
        let view_b = self.panel_views.remove(&b);
        if let Some(mut view) = view_a {
            view.set_panel_id(b, cx);
            self.panel_views.insert(b, view);
        }
        if let Some(mut view) = view_b {
            view.set_panel_id(a, cx);
            self.panel_views.insert(a, view);
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

    pub(crate) fn handle_moved_and_docked_panel(
        &mut self,
        source_id: impl Into<PanelId>,
        target_id: impl Into<PanelId>,
        new_leaf_id: impl Into<PanelId>,
        dock_target: AreaDockTarget,
        cx: &mut Context<Self>,
    ) {
        let source_id = source_id.into();
        let target_id = target_id.into();
        let new_leaf_id = new_leaf_id.into();
        let source_view = self.panel_views.remove(&source_id);
        let source_retained = self.retained_editor_sessions.remove(&source_id);
        let target_view = self.panel_views.remove(&target_id);
        let target_retained = self.retained_editor_sessions.remove(&target_id);

        let source_first = matches!(dock_target, AreaDockTarget::Left | AreaDockTarget::Top);

        if source_first {
            if let Some(mut view) = source_view {
                view.set_panel_id(target_id, cx);
                self.panel_views.insert(target_id, view);
            }
            if let Some(session) = source_retained {
                self.retained_editor_sessions.insert(target_id, session);
            }
            if let Some(mut view) = target_view {
                view.set_panel_id(new_leaf_id, cx);
                self.panel_views.insert(new_leaf_id, view);
            }
            if let Some(session) = target_retained {
                self.retained_editor_sessions.insert(new_leaf_id, session);
            }
        } else {
            if let Some(mut view) = target_view {
                view.set_panel_id(target_id, cx);
                self.panel_views.insert(target_id, view);
            }
            if let Some(session) = target_retained {
                self.retained_editor_sessions.insert(target_id, session);
            }
            if let Some(mut view) = source_view {
                view.set_panel_id(new_leaf_id, cx);
                self.panel_views.insert(new_leaf_id, view);
            }
            if let Some(session) = source_retained {
                self.retained_editor_sessions.insert(new_leaf_id, session);
            }
        }
        self.sync_panel_states(cx);
    }

    pub(crate) fn sync_panel_kind(
        &mut self,
        panel_id: impl Into<PanelId>,
        is_editor: bool,
        cx: &mut Context<Self>,
    ) {
        let panel_id = panel_id.into();
        if is_editor {
            if self.editor_for(panel_id).is_some() {
                return;
            }
            let session = self
                .retained_editor_sessions
                .remove(&panel_id)
                .unwrap_or_else(EditorSession::empty);
            self.add_editor_panel(panel_id, session, cx);
        } else {
            if let Some(session) = self.remove_editor_panel(panel_id, cx) {
                if session.has_tabs() {
                    self.retained_editor_sessions.insert(panel_id, session);
                }
            }
            if let Some(kind) = self.panels.layout.tree.find_leaf_kind(panel_id.0) {
                self.ensure_registered_panel_view(panel_id, kind, cx);
            }
        }
    }

    pub(crate) fn clone_container_into_new_window(
        &mut self,
        cloned: ClonedContainer<PanelKind>,
        cx: &mut Context<Self>,
    ) {
        let mut sessions = HashMap::new();
        let mut cloned_explorer = None;
        for (old_id, new_id) in &cloned.id_map {
            match cloned.tree.find_leaf_kind(*new_id) {
                Some(k) if k.as_str() == "editor" => {
                    if let Some(editor) = self.editor_for(*old_id) {
                        let session = editor.update(cx, |editor, cx| editor.clone_session(cx));
                        sessions.insert(PanelId(*new_id), session);
                    }
                }
                Some(k) if k.as_str() == "explorer" => {
                    cloned_explorer =
                        Some(explorer::ExplorerState::global(cx).clone_for_new_window());
                }
                _ => {}
            }
        }
        open_cloned_window(
            cloned.tree,
            cloned.next_node_id,
            sessions,
            cloned_explorer,
            cx,
        );
    }
}
