//! Window close guard, dirty state aggregation, and action handlers.

use gpui::*;

use super::{Shell, UnsavedDialogScope, UnsavedDialogState};
use crate::actions::{
    CloseExplorerFolder, CloseWindow, InstallCliTool, QuitApplication, ToggleExplorer,
    ToggleMaximizeArea, UninstallCliTool,
};
use crate::menus::request_quit_application;
use window::PanelId;

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
        let Ok(Some(descriptor)) = window::PanelRegistry::registered(retained.kind) else {
            return (false, String::new());
        };
        let (dirty, first_name) = descriptor.retained_dirty_info(retained.state.as_ref(), cx);
        (dirty, first_name.unwrap_or_else(|| "Untitled".to_string()))
    }

    pub(crate) fn first_dirty_panel(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Option<(PanelId, String)> {
        for (panel_id, retained) in &self.retained_panel_states {
            let Ok(Some(descriptor)) = window::PanelRegistry::registered(retained.kind) else {
                continue;
            };
            let (dirty, first_name) = descriptor.retained_dirty_info(retained.state.as_ref(), cx);
            if dirty {
                return Some((
                    *panel_id,
                    first_name.unwrap_or_else(|| "Untitled".to_string()),
                ));
            }
        }
        for (panel_id, view) in &self.panel_views {
            if view.is_dirty(cx) {
                return Some((
                    *panel_id,
                    view.first_dirty_title(cx)
                        .unwrap_or_else(|| "Untitled".to_string()),
                ));
            }
        }
        None
    }

    pub(crate) fn has_unsaved_changes(&self, cx: &App) -> bool {
        self.panel_views.values().any(|panel| panel.is_dirty(cx))
    }

    pub(crate) fn prompt_close_window(&mut self, cx: &mut Context<Self>) {
        let Some((_, first_dirty_name)) = self.first_dirty_panel(cx) else {
            return;
        };

        self.unsaved_dialog = Some(UnsavedDialogState {
            scope: UnsavedDialogScope::Window,
            document_name: first_dirty_name,
            restore_focus: None,
        });
        cx.notify();
    }

    pub(crate) fn prompt_close_editor_for(
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
            restore_focus: None,
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

        self.unsaved_dialog = Some(UnsavedDialogState {
            scope: UnsavedDialogScope::Tab { panel_id, index },
            document_name,
            restore_focus: None,
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
        if self.first_dirty_panel(cx).is_none() {
            return true;
        }
        self.prompt_close_window(cx);
        false
    }

    pub(crate) fn request_close_current_window(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.first_dirty_panel(cx).is_none() {
            window.remove_window();
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
        for state in self.explorer_states() {
            state.update(cx, |state, cx| {
                state.toggle_explorer_drawer(&mut *window, cx)
            });
        }
    }

    pub(crate) fn on_close_explorer_folder_action(
        &mut self,
        _: &CloseExplorerFolder,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for state in self.explorer_states() {
            state.update(cx, |state, cx| state.close_explorer_folder(cx));
        }
    }

    pub(crate) fn on_toggle_kind_dropdown(
        &mut self,
        action: &window::actions::ToggleKindDropdown,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.panels.layout.toggle_dropdown(action.panel);
        cx.notify();
    }

    pub(crate) fn on_split_panel(
        &mut self,
        action: &window::actions::SplitPanel,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.split_panel(PanelId(action.panel), action.axis, 0.5, true, cx);
    }

    pub(crate) fn on_toggle_panel_maximized(
        &mut self,
        action: &window::actions::TogglePanelMaximized,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.panels.layout.toggle_maximize(action.panel);
        cx.notify();
    }

    pub(crate) fn on_close_panel(
        &mut self,
        action: &window::actions::ClosePanel,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_panel(PanelId(action.panel), cx);
    }
}
