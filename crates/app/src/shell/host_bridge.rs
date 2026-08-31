//! DocumentHost and PanelHost implementations bridging panel callbacks back to the window Shell.

use gpui::*;
use std::path::Path;
use std::sync::Arc;

use crate::shell::Shell;
use crate::window::record_recent_file_and_refresh;
use core_contracts::{DocumentHost, TabKind};
use core_contracts::{PanelHost, PanelId, PanelKind};
use splitter::tree::SplitAxis;

/// Bridges the [`DocumentHost`] contract to the window shell. Constructed
/// by the shell when it wires a document-routing panel.
pub(crate) struct ShellDocumentHost {
    shell: WeakEntity<Shell>,
}

impl ShellDocumentHost {
    pub(crate) fn new(shell: WeakEntity<Shell>) -> Self {
        Self { shell }
    }
}

/// Generic host passed to every panel created by the panel registry.
pub(crate) struct ShellPanelHost {
    shell: WeakEntity<Shell>,
}

impl ShellPanelHost {
    pub(crate) fn new(shell: WeakEntity<Shell>) -> Self {
        Self { shell }
    }

    pub(crate) fn shared(shell: WeakEntity<Shell>) -> Arc<dyn PanelHost> {
        Arc::new(Self::new(shell))
    }
}

impl PanelHost for ShellPanelHost {
    fn activate_panel(&self, panel_id: PanelId, cx: &mut App) {
        let _ = self
            .shell
            .update(cx, |shell, cx| shell.activate_panel(panel_id, cx));
    }

    fn close_panel(&self, panel_id: PanelId, cx: &mut App) {
        let _ = self
            .shell
            .update(cx, |shell, cx| shell.request_close_panel(panel_id, cx));
    }

    fn split_panel(
        &self,
        panel_id: PanelId,
        axis: SplitAxis,
        new_kind: Option<PanelKind>,
        cx: &mut App,
    ) {
        let _ = self.shell.update(cx, |shell, cx| {
            let Some(new_panel) = shell.split_panel(panel_id, axis, 0.5, false, cx) else {
                return;
            };
            if let Some(kind) = new_kind {
                shell.change_panel_kind(new_panel.0, kind, cx);
            }
        });
    }

    fn toggle_maximize(&self, panel_id: PanelId, cx: &mut App) {
        let _ = self
            .shell
            .update(cx, |shell, cx| shell.toggle_panel_maximize(panel_id, cx));
    }
}

impl DocumentHost for ShellDocumentHost {
    fn activate_panel(&self, panel_id: PanelId, cx: &mut App) {
        let _ = self
            .shell
            .update(cx, |shell, cx| shell.activate_panel(panel_id, cx));
    }

    fn toggle_panel_dropdown(&self, panel_id: PanelId, cx: &mut App) {
        let _ = self
            .shell
            .update(cx, |shell, cx| shell.toggle_panel_dropdown(panel_id, cx));
    }

    fn split_panel(
        &self,
        panel_id: PanelId,
        axis: SplitAxis,
        ratio: f32,
        copy_content: bool,
        cx: &mut App,
    ) {
        let _ = self.shell.update(cx, |shell, cx| {
            shell.split_panel(panel_id, axis, ratio, copy_content, cx);
        });
    }

    fn toggle_panel_maximize(&self, panel_id: PanelId, cx: &mut App) {
        let _ = self
            .shell
            .update(cx, |shell, cx| shell.toggle_panel_maximize(panel_id, cx));
    }

    fn request_close_panel(&self, panel_id: PanelId, cx: &mut App) {
        let _ = self
            .shell
            .update(cx, |shell, cx| shell.request_close_panel(panel_id, cx));
    }

    fn prompt_close_tab(&self, panel_id: PanelId, index: usize, cx: &mut App) {
        let _ = self
            .shell
            .update(cx, |shell, cx| shell.prompt_close_tab(panel_id, index, cx));
    }

    fn open_file_in_active_document_panel(
        &self,
        path: &Path,
        kind: TabKind,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        self.shell
            .update(cx, |shell, cx| {
                shell.open_file_in_active_document_panel(path, kind, window, cx)
            })
            .unwrap_or(false)
    }

    fn hide_info_dialog(&self, cx: &mut App) {
        let _ = self
            .shell
            .update(cx, |shell, cx| shell.hide_info_dialog(cx));
    }

    fn clear_outer_dropdowns(&self, cx: &mut App) {
        let _ = self
            .shell
            .update(cx, |shell, _cx| shell.panels.layout.clear_dropdowns());
    }

    fn on_document_path_changed(&self, cx: &mut App) {
        let _ = self.shell.update(cx, |shell, cx| {
            shell.notify_sidebar_document_path_changed(cx);
        });
    }

    fn record_recent_file(&self, path: &Path, cx: &mut App) {
        record_recent_file_and_refresh(path, cx);
    }
}
