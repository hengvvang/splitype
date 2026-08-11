//! Editor commands — save/export, view-mode switching, undo/redo, and
//! dirty tracking.
//!
//! Action handlers wired in the editor's render flow
//! (`src/editor/view/`) delegate to these methods. Window/app-level
//! commands (quit, CLI tooling, explorer) live in `crate::app`.

use std::path::Path;

use crate::editor::actions::{ExportHtml, ExportPdf, SaveDocument, SaveDocumentAs, ToggleViewMode};
use crate::editor::controller::*;
use crate::editor::editing::input::actions::{Redo, Undo};

impl Editor {
    /// Builds the OS window title, including the dirty marker when the
    /// document has unsaved changes.
    pub(crate) fn window_title(
        file_path: Option<&Path>,
        is_dirty: bool,
        strings: &crate::infra::i18n::I18nStrings,
    ) -> String {
        let base_title = if let Some(path) = file_path {
            path.file_name().map_or_else(
                || path.to_string_lossy().to_string(),
                |name| name.to_string_lossy().to_string(),
            )
        } else {
            String::new()
        };

        if base_title.is_empty() {
            String::new()
        } else if is_dirty && !strings.dirty_title_marker.is_empty() {
            format!("{} {}", strings.dirty_title_marker, base_title)
        } else {
            base_title
        }
    }

    pub(crate) fn on_toggle_view_mode_action(
        &mut self,
        _: &ToggleViewMode,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_active_tab() {
            return;
        }
        self.toggle_view_mode_from_ui(cx);
    }

    pub(crate) fn toggle_view_mode_from_ui(&mut self, cx: &mut Context<Self>) {
        self.end_block_pointer_selection_sessions(cx);
        self.tab_mut().undo.last_selection_snapshot =
            self.capture_source_selection_snapshot_global(cx);
        self.toggle_view_mode(cx);
    }

    pub(crate) fn on_undo(&mut self, _: &Undo, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_active_tab() {
            return;
        }
        self.undo_document(cx);
    }

    pub(crate) fn on_redo(&mut self, _: &Redo, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_active_tab() {
            return;
        }
        self.redo_document(cx);
    }

    pub(crate) fn on_save_document(
        &mut self,
        _: &SaveDocument,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_active_tab() {
            return;
        }
        self.request_save_document(cx);
    }

    pub(crate) fn on_save_document_as(
        &mut self,
        _: &SaveDocumentAs,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_active_tab() {
            return;
        }
        self.request_save_document_as(cx);
    }

    pub(crate) fn on_export_html(
        &mut self,
        _: &ExportHtml,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_active_tab() {
            return;
        }
        self.export_document_via_prompt(
            crate::editor::render::export::ExportFormat::Html,
            window,
            cx,
        );
    }

    pub(crate) fn on_export_pdf(
        &mut self,
        _: &ExportPdf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_active_tab() {
            return;
        }
        self.export_document_via_prompt(
            crate::editor::render::export::ExportFormat::Pdf,
            window,
            cx,
        );
    }

    pub(crate) fn toggle_view_mode(&mut self, cx: &mut Context<Self>) {
        self.end_block_pointer_selection_sessions(cx);
        let selection_snapshot = self.capture_source_selection_snapshot_global(cx);
        self.clear_cross_block_selection(cx);
        self.tab_mut().selection.select_all_cycle = None;
        match self.tab().mode {
            EditorMode::Wysiwyg => {
                let markdown = self.doc().serialize_markdown(cx);
                self.tab_mut().mode = EditorMode::SourceCode;
                self.rebuild_document_from_markdown(&markdown, cx);
            }
            EditorMode::SourceCode => {
                let source = self.doc().serialize_source_text(cx);
                self.tab_mut().mode = EditorMode::Wysiwyg;
                self.rebuild_document_from_markdown(&source, cx);
            }
        }

        self.apply_selection_snapshot_in_current_mode(&selection_snapshot, cx);
        self.tab_mut().focus.pending_scroll_active_block_into_view = true;
        self.tab_mut().focus.pending_scroll_recheck_after_layout = true;
        self.tab_mut().scroll.last_viewport_size = None;
        self.tab_mut().file.pending_window_title_refresh = true;
        self.tab_mut().file.close_dialog_restore_focus = None;
        self.tab_mut().tables.axis_preview = None;
        self.tab_mut().tables.axis_selection = None;
        self.dismiss_contextual_overlays(cx);
        self.sync_table_axis_visuals(cx);
        self.refresh_stable_document_snapshot(cx);
        cx.notify();
    }

    /// Records that the document text may have changed so derived views
    /// (preview, source panes) re-sync on their next render.
    pub(crate) fn bump_document_revision(&mut self) {
        self.tab_mut().document_revision = self.tab().document_revision.wrapping_add(1);
    }

    /// Marks the document dirty and schedules window-title and edited-state
    /// refresh for the next render frame.
    pub(crate) fn mark_dirty(&mut self, cx: &mut Context<Self>) {
        self.bump_document_revision();
        if !self.tab().file.dirty {
            self.tab_mut().file.dirty = true;
            self.tab_mut().file.pending_window_edited = true;
            self.tab_mut().file.pending_window_title_refresh = true;
            cx.notify();
        }
    }
}
