//! Opening files from the explorer: single click, double click (focus the
//! editor), and Ctrl/Cmd+double-click (open in a freshly split area).

use std::path::PathBuf;

use gpui::*;

use crate::app::shell::Shell;

use crate::editor::explorer_state::state::*;

impl Shell {
    pub(crate) fn open_explorer_file(
        &mut self,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Key the selection by the entry's stable id when the tree knows it;
        // fall back to the path-derived id (harmless: no row will highlight).
        let (root, id) = self
            .explorer_id_for_path(&path)
            .unwrap_or_else(|| (0, ExplorerEntryId::for_path(&path)));
        self.panels.explorer.selected = Some(ExplorerSelection::File { root, entry: id });
        // Reveal: expand ancestor directories and center the row.
        self.expand_to_path(&path);
        self.rebuild_explorer_entries();
        self.autoscroll_explorer_selection();
        // Explorer interacts with the ACTIVE editor: the file opens in its
        // tab bar. With no Editor area present the click is ignored.
        if self.panels.layout.active_area.is_none() {
            return;
        }
        self.open_file_in_active_editor(&path, window, cx);
    }

    /// Open a file from a row click: single click keeps panel focus, double
    /// click also moves keyboard focus into the editor (mirrors Zed).
    pub(crate) fn open_explorer_file_click(
        &mut self,
        path: PathBuf,
        focus_editor: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_explorer_file(path, window, cx);
        if focus_editor {
            let area = self.panels.layout.active_area;
            let focused_panel = self.primary_editor().and_then(|editor| {
                editor
                    .read(cx)
                    .focused_editor_inner_panel
                    .filter(|_| area.is_some())
            });
            if let (Some(area), Some(panel_id)) = (area, focused_panel) {
                if let Some(editor) = self.primary_editor() {
                    let _ = editor.update(cx, |editor, cx| {
                        editor.focus_editor_inner_panel(area, panel_id, window, cx);
                    });
                }
            }
        }
    }

    /// Ctrl/Cmd+double-click: open the file in a freshly split editor area
    /// (mirrors Zed's split-on-open).
    pub(crate) fn split_explorer_file(
        &mut self,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(area_id) = self.panels.layout.active_area else {
            return;
        };
        let Some(new_id) =
            self.split_window_area(area_id, crate::splitter::Axis::Horizontal, 0.5, false, cx)
        else {
            return;
        };
        self.panels.layout.activate_area(new_id);
        if let Some(editor) = self.editor_for(new_id) {
            let _ = editor.update(cx, |editor, cx| {
                editor.open_file_in_area(new_id, &path, window, cx);
            });
        }
        cx.notify();
    }
}
