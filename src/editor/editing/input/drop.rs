//! Markdown file drop detection and document loading.
//!
//! Pure path detection plus the document replace flow that loads a dropped
//! file into the editor. The dirty-document dialogs that gate a replace
//! live in `crate::editor::file`.

use std::path::{Path, PathBuf};

use anyhow::{Context as AnyhowContext, Result};
use gpui::*;

use crate::editor::controller::{Editor, EditorPaneKind};

/// Returns true when `path` exists and has a `.md` or `.markdown` extension.
pub(crate) fn is_markdown_file_path(path: &Path) -> bool {
    path.is_file()
        && path.extension().is_some_and(|extension| {
            extension.to_string_lossy().eq_ignore_ascii_case("md")
                || extension.to_string_lossy().eq_ignore_ascii_case("markdown")
        })
}

/// Returns the first path in `paths` that passes [`is_markdown_file_path`].
pub(crate) fn first_dropped_markdown_path(paths: &[PathBuf]) -> Option<PathBuf> {
    paths
        .iter()
        .find(|path| is_markdown_file_path(path))
        .cloned()
}

impl Editor {
    pub(crate) fn replace_document_from_path(
        &mut self,
        path: &Path,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        let bytes =
            std::fs::read(path).with_context(|| format!("failed to read '{}'", path.display()))?;
        let markdown = String::from_utf8_lossy(&bytes).to_string();
        self.replace_document_from_markdown(markdown, Some(path.to_path_buf()), cx);
        crate::app::menus::record_recent_file_from_editor(path, cx);
        Ok(())
    }

    pub(crate) fn replace_document_from_markdown(
        &mut self,
        markdown: String,
        file_path: Option<PathBuf>,
        cx: &mut Context<Self>,
    ) {
        let normalized = markdown.replace("\r\n", "\n").replace('\r', "\n");

        self.tab_mut().file.path = file_path;
        self.tab_mut().mode = EditorPaneKind::Wysiwyg;
        self.rebuild_document_from_markdown(&normalized, cx);

        self.tab_mut().file.dirty = false;
        self.tab_mut().file.pending_window_edited = false;
        self.tab_mut().file.pending_window_title_refresh = true;
        self.tab_mut().file.pending_save = false;
        self.tab_mut().file.pending_save_as = false;
        self.tab_mut().file.pending_open_link = None;
        self.tab_mut().file.pending_close_after_save = false;
        self.tab_mut().file.close_dialog_restore_focus = None;
        self.tab_mut().file.show_unsaved_changes_dialog = false;
        self.clear_pending_drop_replace_state(cx);
        self.dismiss_contextual_overlays(cx);
        self.tab_mut().tables.axis_preview = None;
        self.tab_mut().tables.axis_selection = None;
        self.sync_table_axis_visuals(cx);
        self.clear_cross_block_selection(cx);

        // The whole document was replaced: every pane starts over at the
        // first block, scrolled to the top.
        let pending_focus = self.first_focusable_entity_id(cx);
        let pane_ids: Vec<usize> = self.tab().panes.keys().copied().collect();
        for pane_id in pane_ids {
            let pane = self.pane_state(pane_id);
            pane.focus.pending_scroll_active_block_into_view = true;
            pane.focus.pending_scroll_recheck_after_layout = true;
            pane.scroll.last_viewport_size = None;
            pane.scroll.handle.set_offset(point(px(0.0), px(0.0)));
            pane.focus.pending = pending_focus;
            pane.focus.active_entity = pending_focus;
        }

        self.tab_mut().undo.undo_entries.clear();
        self.tab_mut().undo.redo_entries.clear();
        self.tab_mut().undo.pending_capture = None;
        self.tab_mut().undo.last_selection_snapshot = Self::empty_selection_snapshot();
        self.tab_mut().undo.restore_in_progress = false;
        self.refresh_stable_document_snapshot(cx);
        if let Some(shell) = self.shell.clone() {
            let _ = shell.update(cx, |shell, cx| {
                shell.sync_explorer_after_document_path_change(cx);
            });
        }
        cx.notify();
    }
}
