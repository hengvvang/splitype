//! DocumentHost and PanelHost implementations bridging panel callbacks back to the window Shell.

use gpui::*;
use std::path::Path;

use crate::shell::Shell;
use crate::window::record_recent_file_and_refresh;
use editor_contracts::DocumentHost;
use platform_contracts::PanelId;
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

    fn clear_outer_dropdowns(&self, cx: &mut App) {
        let _ = self
            .shell
            .update(cx, |shell, _cx| shell.panels.layout.clear_dropdowns());
    }

    fn on_document_path_changed(&self, cx: &mut App) {
        let _ = self.shell.update(cx, |shell, cx| {
            shell.notify_document_path_changed(cx);
        });
    }

    fn record_recent_file(&self, path: &Path, cx: &mut App) {
        record_recent_file_and_refresh(path, cx);
    }
}
