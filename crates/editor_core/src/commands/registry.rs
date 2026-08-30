//! Editor commands — save/export, view-mode switching, undo/redo, and
//! dirty tracking.
//!
//! Action handlers wired in the editor's render flow delegate to these
//! methods. Window/app-level commands (quit, CLI tooling, explorer) live
//! in `crate::app`.

use std::path::Path;

use crate::commands::actions::{
    ExportHtml, ExportPdf, SaveDocument, SaveDocumentAs, ToggleMaximizePane, TogglePaneKind,
};

use crate::engine::controller::*;
use editor_wysiwyg::actions::{Redo, Undo};

impl Editor {
    /// Builds the OS window title, including the dirty marker when the
    /// document has unsaved changes.
    pub fn window_title(
        file_path: Option<&Path>,
        is_dirty: bool,
        strings: &config::language::I18nStrings,
    ) -> String {
        let base_title = file_path
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        if base_title.is_empty() {
            String::new()
        } else if is_dirty && !strings.dirty_title_marker.is_empty() {
            format!("{} {}", strings.dirty_title_marker, base_title)
        } else {
            base_title
        }
    }

    pub fn on_toggle_maximize_pane_action(
        &mut self,
        _: &ToggleMaximizePane,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pane_id = self.active_pane_id();
        self.toggle_pane_maximize(pane_id);
        cx.notify();
    }

    pub fn on_toggle_pane_kind_action(
        &mut self,
        _: &TogglePaneKind,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_active_tab() {
            return;
        }
        self.toggle_pane_kind_from_ui(cx);
    }

    pub fn toggle_pane_kind_from_ui(&mut self, cx: &mut Context<Self>) {
        self.end_block_pointer_selection_sessions(cx);
        self.tab_mut().undo.last_selection_snapshot =
            self.capture_source_selection_snapshot_global(cx);
        self.toggle_pane_kind(cx);
    }

    pub fn on_undo(&mut self, _: &Undo, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_active_tab() {
            return;
        }
        // Undo operates on the block tree; parse it lazily if needed
        // (e.g. the tab was opened parse-free and edited in Source mode).
        self.ensure_document(cx);
        self.undo_document(cx);
    }

    pub fn on_redo(&mut self, _: &Redo, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_active_tab() {
            return;
        }
        self.ensure_document(cx);
        self.redo_document(cx);
    }

    pub fn on_save_document(
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

    pub fn on_save_document_as(
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

    pub fn on_export_html(
        &mut self,
        _: &ExportHtml,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_active_tab() {
            return;
        }
        self.export_document_via_prompt(
            editor_wysiwyg::export::ExportFormat::Html,
            window,
            cx,
        );
    }

    pub fn on_export_pdf(
        &mut self,
        _: &ExportPdf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_active_tab() {
            return;
        }
        self.export_document_via_prompt(
            editor_wysiwyg::export::ExportFormat::Pdf,
            window,
            cx,
        );
    }

    pub fn toggle_pane_kind(&mut self, cx: &mut Context<Self>) {
        self.end_block_pointer_selection_sessions(cx);
        let selection_snapshot = self.capture_source_selection_snapshot_global(cx);
        self.clear_cross_block_selection(cx);
        if let Some(selection) = self.active_pane_state().selection_mut() {
            selection.clear_all();
        }
        let active_pane = self.active_pane_id();
        let current_kind = self.active_pane_kind();
        let next_kind = match current_kind {
            EditorPaneKind::Wysiwyg => EditorPaneKind::SourceCode,
            EditorPaneKind::SourceCode | EditorPaneKind::Preview => {
                EditorPaneKind::Wysiwyg
            }
        };
        self.change_pane_kind(active_pane, next_kind);

        // Model C: switching into WYSIWYG materializes the block tree if
        // the tab was still parse-free (opened in Source mode).
        if next_kind.is_wysiwyg() {
            self.ensure_document(cx);
        }

        self.apply_selection_snapshot_in_current_mode(&selection_snapshot, cx);
        {
            let pane_id = self.active_pane_id();
            let state = self.pane_state(pane_id);
            state.scroll.pending_autoscroll = Some(crate::engine::controller::AutoscrollStrategy::Fit {
                margin: px(20.0),
            });
            state.scroll.last_viewport_size = None;
        }
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
    pub fn bump_document_revision(&mut self) {
        self.tab_mut().document_revision = self.tab().document_revision.wrapping_add(1);
    }

    /// Marks the document dirty and schedules window-title and edited-state
    /// refresh for the next render frame. If the active tab was in transient mode,
    /// it is automatically persisted to a persistent resident tab.
    pub fn mark_dirty(&mut self, cx: &mut App) {
        self.bump_document_revision();
        // Model C: every mutation marks the authoritative text stale so
        // text readers serialize from the parsed tree. Harmless when the
        // tree is unparsed (`serialized_text` reads `text` then) — this is
        // the safety net for edits that bypass `doc_mut`.
        self.tab_mut().text_stale = true;
        if self.tab().is_transient() {
            self.tab_mut().persist();
        }
        if !self.tab().file.dirty {
            self.tab_mut().file.dirty = true;
            self.tab_mut().file.pending_window_edited = true;
            self.tab_mut().file.pending_window_title_refresh = true;
            cx.notify(self.entity_id);
        }
    }
}
