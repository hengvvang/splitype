//! Window commands — save/export/quit/close, CLI tooling, view-mode
//! switching, and dirty tracking.
//!
//! Action handlers wired in the window chrome (`src/windows/`) delegate to
//! these methods. File prompts and menu state live in `super::menu`.

use std::path::Path;

use crate::editor::controller::*;
use crate::editor::editing::input::shortcuts::{
    CloseWindow, ExportHtml, ExportPdf, InstallCliTool, QuitApplication, Redo, SaveDocument,
    SaveDocumentAs, ToggleViewMode, Undo, UninstallCliTool,
};

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
        self.toggle_view_mode_from_ui(cx);
    }

    pub(crate) fn toggle_view_mode_from_ui(&mut self, cx: &mut Context<Self>) {
        self.end_block_pointer_selection_sessions(cx);
        self.undo.last_selection_snapshot = self.capture_source_selection_snapshot(cx);
        self.toggle_view_mode(cx);
    }

    pub(crate) fn on_undo(&mut self, _: &Undo, _window: &mut Window, cx: &mut Context<Self>) {
        self.undo_document(cx);
    }

    pub(crate) fn on_redo(&mut self, _: &Redo, _window: &mut Window, cx: &mut Context<Self>) {
        self.redo_document(cx);
    }

    pub(crate) fn on_save_document(
        &mut self,
        _: &SaveDocument,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.request_save_document(cx);
    }

    pub(crate) fn on_save_document_as(
        &mut self,
        _: &SaveDocumentAs,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.request_save_document_as(cx);
    }

    pub(crate) fn on_export_html(
        &mut self,
        _: &ExportHtml,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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
        self.export_document_via_prompt(
            crate::editor::render::export::ExportFormat::Pdf,
            window,
            cx,
        );
    }

    pub(crate) fn on_quit_application(
        &mut self,
        _: &QuitApplication,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        crate::app::menus::request_quit_application(cx);
    }

    pub(crate) fn on_close_window(
        &mut self,
        _: &CloseWindow,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.request_close_current_window(window, cx);
    }

    pub(crate) fn on_install_cli_tool(
        &mut self,
        _: &InstallCliTool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        crate::platform::cli_tool::install_cli_tool(cx);
    }

    pub(crate) fn on_uninstall_cli_tool(
        &mut self,
        _: &UninstallCliTool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        crate::platform::cli_tool::uninstall_cli_tool(cx);
    }

    #[allow(dead_code)]
    pub(crate) fn set_view_mode(&mut self, target_mode: EditorMode, cx: &mut Context<Self>) {
        if self.mode != target_mode {
            self.toggle_view_mode(cx);
        }
    }

    pub(crate) fn toggle_view_mode(&mut self, cx: &mut Context<Self>) {
        self.end_block_pointer_selection_sessions(cx);
        let selection_snapshot = self.capture_source_selection_snapshot(cx);
        self.clear_cross_block_selection(cx);
        self.selection.select_all_cycle = None;
        match self.mode {
            EditorMode::Wysiwyg => {
                let markdown = self.document.to_markdown(cx);
                let block = Self::new_block(cx, BlockData::paragraph(markdown));
                block.update(cx, |block, _cx| block.set_source_document_mode());
                self.document.replace_blocks(vec![block], cx);
                self.mode = EditorMode::SourceCode;
                self.tables.cells.clear();
            }
            EditorMode::SourceCode => {
                let source = self.document.to_raw_source(cx);
                let mut roots = Self::parse_document(cx, &source);
                if roots.is_empty() {
                    roots.push(Self::new_block(cx, BlockData::paragraph(String::new())));
                }
                self.document.replace_blocks(roots, cx);
                self.mode = EditorMode::Wysiwyg;
                self.rebuild_table_runtimes(cx);
                self.rebuild_image_runtimes(cx);
            }
        }

        self.apply_selection_snapshot_in_current_mode(&selection_snapshot, cx);
        self.focus.pending_scroll_active_block_into_view = true;
        self.focus.pending_scroll_recheck_after_layout = true;
        self.scroll.last_viewport_size = None;
        self.file.pending_window_title_refresh = true;
        self.file.close_dialog_restore_focus = None;
        self.tables.axis_preview = None;
        self.tables.axis_selection = None;
        self.dismiss_contextual_overlays(cx);
        self.sync_table_axis_visuals(cx);
        self.refresh_stable_document_snapshot(cx);
        cx.notify();
    }

    /// Marks the document dirty and schedules window-title and edited-state
    /// refresh for the next render frame.
    pub(crate) fn mark_dirty(&mut self, cx: &mut Context<Self>) {
        if !self.file.dirty {
            self.file.dirty = true;
            self.file.pending_window_edited = true;
            self.file.pending_window_title_refresh = true;
            cx.notify();
        }
    }
}
