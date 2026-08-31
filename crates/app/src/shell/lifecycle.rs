//! Panel and session lifecycle operations on the window Shell.

use gpui::*;
use std::collections::HashMap;
use std::path::Path;

use super::RetainedPanel;
use super::Shell;
use super::host_bridge::{ShellEditorHost, ShellPanelHost};
use crate::window::open_cloned_window;
use core_contracts::TabKind;
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
        let Some(kind) = self.panels.layout.tree.find_leaf_kind(new_id) else {
            return Some(PanelId(new_id));
        };
        let mut inserted = false;
        if copy_content && let Some(state) = self.clone_panel_state_for_kind(kind, cx) {
            inserted = self.restore_retained_view(PanelId(new_id), kind, state, cx);
        }
        if !inserted {
            self.ensure_registered_panel_view(PanelId(new_id), kind, cx);
        }
        self.sync_panel_states(cx);
        Some(PanelId(new_id))
    }

    /// Clones the first live view of `kind` that offers durable state.
    fn clone_panel_state_for_kind(
        &self,
        kind: PanelKind,
        cx: &mut Context<Self>,
    ) -> Option<Box<dyn std::any::Any>> {
        self.panel_views
            .values()
            .find(|view| view.kind() == kind)
            .and_then(|view| view.clone_state(cx))
    }

    /// Inserts a panel view and wires the editor host when the view belongs
    /// to the editor plugin.
    fn insert_panel_view(
        &mut self,
        panel_id: PanelId,
        view: Box<dyn PanelView>,
        cx: &mut Context<Self>,
    ) {
        self.wire_editor_host(view.as_ref(), cx);
        self.panel_views.insert(panel_id, view);
    }

    /// Wires the editor's document host after a registry-created editor view
    /// enters the shell. This is the single remaining editor-specific hook,
    /// isolated behind the generic panel insertion path.
    fn wire_editor_host(&self, view: &dyn PanelView, cx: &mut Context<Self>) {
        let Some(panel) = view.as_any().downcast_ref::<editor::EditorPanelView>() else {
            return;
        };
        let shell = cx.entity().downgrade();
        let editor = panel.editor.clone();
        editor.update(cx, |editor, cx| {
            editor.host = Some(std::sync::Arc::new(ShellEditorHost::new(shell)));
            if editor.session.has_tabs() {
                editor.sync_panes_with_active_tab(cx);
            }
        });
    }

    /// Restores a suspended state blob into a live view through its descriptor.
    fn restore_retained_view(
        &mut self,
        panel_id: PanelId,
        kind: PanelKind,
        state: Box<dyn std::any::Any>,
        cx: &mut Context<Self>,
    ) -> bool {
        let host = ShellPanelHost::shared(cx.entity().downgrade());
        match window::PanelRegistry::restore_registered_panel(kind, panel_id, host, state, cx) {
            Ok(Some(view)) => {
                self.insert_panel_view(panel_id, view, cx);
                true
            }
            Ok(None) => {
                tracing::error!(%kind, "panel descriptor could not restore its state");
                false
            }
            Err(error) => {
                tracing::error!(%kind, %error, "failed to restore registered panel");
                false
            }
        }
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
                self.insert_panel_view(panel_id, view, cx);
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
    pub(crate) fn remove_panel_view(
        &mut self,
        panel_id: impl Into<PanelId>,
    ) -> Option<Box<dyn PanelView>> {
        self.panel_views.remove(&panel_id.into())
    }

    /// Materialize the fresh sibling leaf of a plain-drag split.
    pub(crate) fn seed_split_panel(&mut self, new_id: impl Into<PanelId>, cx: &mut Context<Self>) {
        let new_id = new_id.into();
        let Some(kind) = self.panels.layout.tree.find_leaf_kind(new_id.0) else {
            return;
        };
        let mut inserted = false;
        if let Some(state) = self.clone_panel_state_for_kind(kind, cx) {
            inserted = self.restore_retained_view(new_id, kind, state, cx);
        }
        if !inserted {
            self.ensure_registered_panel_view(new_id, kind, cx);
        }
        self.sync_panel_states(cx);
    }

    /// Close an area and drop its view and retained state.
    pub(crate) fn close_panel(&mut self, panel_id: impl Into<PanelId>, cx: &mut Context<Self>) {
        let panel_id = panel_id.into();
        if let Some(target_leaf_id) = self.panels.layout.resolve_leaf(panel_id.0) {
            self.panels.layout.close_leaf(target_leaf_id);
            self.remove_panel_view(target_leaf_id);
            self.retained_panel_states.remove(&PanelId(target_leaf_id));
            self.sync_panel_states(cx);
        }
    }

    /// Clean up a joined panel's view and retained state.
    pub(crate) fn handle_joined_panel(&mut self, removed_id: NodeId, cx: &mut Context<Self>) {
        self.remove_panel_view(removed_id);
        self.retained_panel_states.remove(&PanelId(removed_id));
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
        self.sync_panel_kind(panel_id, kind, cx);
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
        let retained_a = self.retained_panel_states.remove(&a);
        let retained_b = self.retained_panel_states.remove(&b);
        if let Some(retained) = retained_a {
            self.retained_panel_states.insert(b, retained);
        }
        if let Some(retained) = retained_b {
            self.retained_panel_states.insert(a, retained);
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
        let source_retained = self.retained_panel_states.remove(&source_id);
        let target_view = self.panel_views.remove(&target_id);
        let target_retained = self.retained_panel_states.remove(&target_id);

        let source_first = matches!(dock_target, AreaDockTarget::Left | AreaDockTarget::Top);

        if source_first {
            if let Some(mut view) = source_view {
                view.set_panel_id(target_id, cx);
                self.panel_views.insert(target_id, view);
            }
            if let Some(retained) = source_retained {
                self.retained_panel_states.insert(target_id, retained);
            }
            if let Some(mut view) = target_view {
                view.set_panel_id(new_leaf_id, cx);
                self.panel_views.insert(new_leaf_id, view);
            }
            if let Some(retained) = target_retained {
                self.retained_panel_states.insert(new_leaf_id, retained);
            }
        } else {
            if let Some(mut view) = target_view {
                view.set_panel_id(target_id, cx);
                self.panel_views.insert(target_id, view);
            }
            if let Some(retained) = target_retained {
                self.retained_panel_states.insert(target_id, retained);
            }
            if let Some(mut view) = source_view {
                view.set_panel_id(new_leaf_id, cx);
                self.panel_views.insert(new_leaf_id, view);
            }
            if let Some(retained) = source_retained {
                self.retained_panel_states.insert(new_leaf_id, retained);
            }
        }
        self.sync_panel_states(cx);
    }

    /// Synchronizes the live panel view with the layout tree's current kind.
    /// The current view is suspended into [`Shell::retained_panel_states`] and
    /// a view for the new kind is created, restoring parked state when the
    /// parked kind comes back.
    pub(crate) fn sync_panel_kind(
        &mut self,
        panel_id: impl Into<PanelId>,
        kind: PanelKind,
        cx: &mut Context<Self>,
    ) {
        let panel_id = panel_id.into();
        if let Some(mut view) = self.remove_panel_view(panel_id) {
            let parked_kind = view.kind();
            if let Some(state) = view.suspend_state(cx) {
                self.retained_panel_states.insert(
                    panel_id,
                    RetainedPanel {
                        kind: parked_kind,
                        state,
                    },
                );
            }
        }

        if self
            .retained_panel_states
            .get(&panel_id)
            .is_some_and(|retained| retained.kind == kind)
        {
            let retained = self
                .retained_panel_states
                .remove(&panel_id)
                .expect("just checked");
            if self.restore_retained_view(panel_id, kind, retained.state, cx) {
                return;
            }
        }

        self.ensure_registered_panel_view(panel_id, kind, cx);
    }

    pub(crate) fn clone_container_into_new_window(
        &mut self,
        cloned: ClonedContainer<PanelKind>,
        cx: &mut Context<Self>,
    ) {
        let mut retained = HashMap::new();
        let mut cloned_explorer = None;
        for (old_id, new_id) in &cloned.id_map {
            let Some(kind) = cloned.tree.find_leaf_kind(*new_id) else {
                continue;
            };
            if kind == window::PanelKind::new("explorer") {
                cloned_explorer = Some(explorer::ExplorerState::global(cx).clone_for_new_window());
                continue;
            }
            let Some(state) = self
                .panel_views
                .get(&PanelId(*old_id))
                .and_then(|view| view.clone_state(cx))
            else {
                continue;
            };
            retained.insert(PanelId(*new_id), RetainedPanel { kind, state });
        }
        open_cloned_window(
            cloned.tree,
            cloned.next_node_id,
            retained,
            cloned_explorer,
            cx,
        );
    }
}
