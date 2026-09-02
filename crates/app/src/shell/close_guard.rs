//! Window close guard, dirty state aggregation, and action handlers.

use gpui::*;
use std::collections::HashMap;

use super::{Shell, UnsavedDialogScope, UnsavedDialogState};
use crate::actions::{
    CloseExplorerFolder, CloseWindow, InstallCliTool, QuitApplication, ToggleExplorer,
    ToggleMaximizeArea, UninstallCliTool,
};
use crate::menus::request_quit_application;
use editor::document::{DocumentBuffer, DocumentStore};
use platform_contracts::{DocumentId, PanelId};

impl Shell {
    /// Dirty state of one panel, resolved through the generic panel contract
    /// first. Retained editor sessions are only consulted because a panel can
    /// temporarily suspend its editor view while keeping dirty tabs alive.
    pub(crate) fn dirty_info_in_panel(
        &self,
        panel_id: impl Into<PanelId>,
        cx: &App,
    ) -> (bool, String) {
        let panel_id = panel_id.into();
        if let Some(view) = self.panel_views.get(&panel_id) {
            if view.is_dirty(cx) {
                return (
                    true,
                    view.first_dirty_title(cx)
                        .unwrap_or_else(|| "Untitled".to_string()),
                );
            }
        }

        let Some(retained) = self.retained_panel_states.get(&panel_id) else {
            return (false, String::new());
        };
        let Ok(Some(descriptor)) = window::PanelRegistry::registered(retained.kind.clone()) else {
            return (false, String::new());
        };
        let (dirty, first_name) = descriptor.retained_dirty_info(retained.state.as_ref(), cx);
        (dirty, first_name.unwrap_or_else(|| "Untitled".to_string()))
    }

    /// Per-buffer view counts across this window's live panels and retained
    /// (suspended) panel states.
    fn window_buffer_counts(&self, cx: &App) -> HashMap<DocumentId, usize> {
        let mut counts = HashMap::new();
        for view in self.panel_views.values() {
            let Some(routing) = crate::routing::document_routing(&view.kind()) else {
                continue;
            };
            let Some(panel) = (routing.as_document)(view.as_ref()) else {
                continue;
            };
            for id in panel.document_buffer_ids(cx) {
                *counts.entry(id).or_insert(0) += 1;
            }
        }
        for retained in self.retained_panel_states.values() {
            let Ok(Some(descriptor)) = window::PanelRegistry::registered(retained.kind.clone())
            else {
                continue;
            };
            for id in descriptor.retained_buffer_ids(retained.state.as_ref(), cx) {
                *counts.entry(id).or_insert(0) += 1;
            }
        }
        counts
    }

    /// Whether closing this window would lose unsaved content: some dirty
    /// buffer has every one of its views inside this window.
    pub(crate) fn has_unsaved_changes(&self, cx: &App) -> bool {
        let store = cx.global::<DocumentStore>();
        let counts = self.window_buffer_counts(cx);
        store
            .dirty_buffer_ids(cx)
            .into_iter()
            .any(|id| store.view_count(id) == counts.get(&id).copied().unwrap_or(0))
    }

    pub(crate) fn prompt_close_window(&mut self, cx: &mut Context<Self>) {
        if !self.has_unsaved_changes(cx) {
            return;
        }
        let document_name = cx
            .global::<DocumentStore>()
            .first_dirty_name(cx)
            .unwrap_or_else(|| "Untitled".to_string());
        self.unsaved_dialog = Some(UnsavedDialogState {
            scope: UnsavedDialogScope::Window,
            document_name,
        });
        cx.notify();
    }

    pub(crate) fn prompt_close_panel_for(
        &mut self,
        panel_id: impl Into<PanelId>,
        cx: &mut Context<Self>,
    ) {
        let panel_id = panel_id.into();
        let (has_dirty, first_dirty_name) = self.dirty_info_in_panel(panel_id, cx);
        if !has_dirty {
            return;
        }

        self.unsaved_dialog = Some(UnsavedDialogState {
            scope: UnsavedDialogScope::Panel(panel_id),
            document_name: first_dirty_name,
        });
        cx.notify();
    }

    pub(crate) fn prompt_close_tab(
        &mut self,
        panel_id: impl Into<PanelId>,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        let panel_id = panel_id.into();
        let document_name = self
            .document_panel_for(panel_id)
            .and_then(|panel| panel.tab_display_name(index, cx))
            .unwrap_or_else(|| "Untitled".to_string());

        self.unsaved_dialog = Some(UnsavedDialogState {
            scope: UnsavedDialogScope::Tab { panel_id, index },
            document_name,
        });
        cx.notify();
    }

    pub(crate) fn request_close_panel(
        &mut self,
        panel_id: impl Into<PanelId>,
        cx: &mut Context<Self>,
    ) {
        let panel_id = panel_id.into();
        let (has_dirty, _) = self.dirty_info_in_panel(panel_id, cx);
        if has_dirty {
            self.prompt_close_panel_for(panel_id, cx);
        } else if self.layout_leaf_count() > 1 {
            self.close_panel(panel_id, cx);
        } else if let Some(panel) = self.document_panel_mut_for(panel_id) {
            panel.clear_tabs(cx);
        }
    }

    pub(crate) fn install_close_guard(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.close_guard_installed {
            return;
        }
        self.force_install_close_guard(window, cx);
    }

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

    pub(crate) fn on_window_should_close(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.has_unsaved_changes(cx) {
            self.release_all_documents(cx);
            self.snapshot_window_state(cx);
            return true;
        }
        self.prompt_close_window(cx);
        false
    }

    /// Releases every document view of this window (live panels and
    /// retained states) without touching content.
    fn release_all_documents(&mut self, cx: &mut Context<Self>) {
        for view in self.panel_views.values_mut() {
            view.release_documents(cx);
        }
        for retained in self.retained_panel_states.values_mut() {
            let Ok(Some(descriptor)) = window::PanelRegistry::registered(retained.kind.clone())
            else {
                continue;
            };
            descriptor.release_retained(&mut retained.state, cx);
        }
    }

    /// Destroys dirty buffers that no longer have any registered view.
    pub(crate) fn sweep_orphaned_dirty_buffers(&mut self, cx: &mut Context<Self>) {
        let orphans: Vec<Entity<DocumentBuffer>> = {
            let store = cx.global::<DocumentStore>();
            store
                .dirty_buffer_ids(cx)
                .into_iter()
                .filter(|id| store.view_count(*id) == 0)
                .filter_map(|id| store.get(id))
                .collect()
        };
        for buffer in orphans {
            let id = buffer.read(cx).id;
            buffer.update(cx, |buffer, cx| buffer.mark_discarded(cx));
            cx.global_mut::<DocumentStore>().discard(id);
        }
    }

    /// Snapshots the window state (when enabled) and removes the window.
    pub(crate) fn close_window_now(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.release_all_documents(cx);
        self.snapshot_window_state(cx);
        window.remove_window();
    }

    pub(crate) fn request_close_current_window(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_unsaved_changes(cx) {
            self.close_window_now(window, cx);
            return;
        }
        self.prompt_close_window(cx);
    }

    pub(crate) fn on_close_window(
        &mut self,
        _: &CloseWindow,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.request_close_current_window(window, cx);
    }

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
        request_quit_application(cx);
    }

    pub(crate) fn on_install_cli_tool(
        &mut self,
        _: &InstallCliTool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        splitype_installer::install_cli_tool(cx);
    }

    pub(crate) fn on_uninstall_cli_tool(
        &mut self,
        _: &UninstallCliTool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        splitype_installer::uninstall_cli_tool(cx);
    }

    pub(crate) fn on_toggle_maximize_area_action(
        &mut self,
        _: &ToggleMaximizeArea,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(active) = self.panels.layout.active_leaf {
            self.panels.layout.toggle_maximize(active);
            cx.notify();
        }
    }

    pub(crate) fn on_toggle_explorer_action(
        &mut self,
        _: &ToggleExplorer,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_explorer_tree(window, cx);
    }

    pub(crate) fn on_close_explorer_folder_action(
        &mut self,
        _: &CloseExplorerFolder,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_explorer_folder_scope(cx);
    }

    pub(crate) fn on_toggle_kind_dropdown(
        &mut self,
        action: &platform_contracts::actions::ToggleKindDropdown,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.panels.layout.toggle_dropdown(action.panel);
        cx.notify();
    }

    pub(crate) fn on_split_panel(
        &mut self,
        action: &platform_contracts::actions::SplitPanel,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.split_panel(PanelId(action.panel), action.axis, 0.5, true, cx);
    }

    pub(crate) fn on_toggle_panel_maximized(
        &mut self,
        action: &platform_contracts::actions::TogglePanelMaximized,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.panels.layout.toggle_maximize(action.panel);
        cx.notify();
    }

    pub(crate) fn on_close_panel(
        &mut self,
        action: &platform_contracts::actions::ClosePanel,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_panel(PanelId(action.panel), cx);
    }
}
