//! Editor command actions and execution handlers.

pub mod defs;
pub mod edit;

pub use defs::*;
pub use edit::{
    strip_markdown_line_prefix, BlockStructureKind, EditCommand, InlineFormatKind, InsertionKind,
};

use std::path::Path;

use gpui::*;

use crate::editor::export::ExportFormat;
use crate::editor::Editor;
use crate::session::PendingOpenLink;
use editor_model::{AutoscrollStrategy, PaneId, PaneKindId};

impl Editor {
    /// Builds the OS window title, including the dirty marker when the document has unsaved changes.
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

        if is_dirty {
            format!("{}{} - Splitype", strings.dirty_title_marker, base_title)
        } else if base_title.is_empty() {
            "Splitype".to_string()
        } else {
            format!("{} - Splitype", base_title)
        }
    }

    pub fn toggle_maximize_pane(&mut self, cx: &mut Context<Self>) {
        let active = self.active_pane_id();
        self.session.root.toggle_maximize(active.0);
        cx.notify();
    }

    pub fn on_save_document(
        &mut self,
        _: &SaveDocument,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_tabs() {
            return;
        }
        if self.tab().file.path.is_none() {
            self.save_document_as(window, cx);
            return;
        }
        self.save_document(window, cx);
    }

    pub fn on_save_document_as(
        &mut self,
        _: &SaveDocumentAs,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_tabs() {
            return;
        }
        self.save_document_as(window, cx);
    }

    pub fn on_toggle_pane_kind(
        &mut self,
        _: &TogglePaneKind,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_tabs() {
            return;
        }
        self.toggle_pane_kind(cx);
    }

    pub fn on_toggle_maximize_pane(
        &mut self,
        _: &ToggleMaximizePane,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_maximize_pane(cx);
    }

    pub fn on_undo(&mut self, _: &Undo, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_tabs() {
            return;
        }
        cx.notify();
    }

    pub fn on_redo(&mut self, _: &Redo, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_tabs() {
            return;
        }
        cx.notify();
    }

    pub fn on_export_html(
        &mut self,
        _: &ExportHtml,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_tabs() {
            return;
        }
        self.export_document_via_prompt(
            ExportFormat::Html,
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
        if !self.has_tabs() {
            return;
        }
        self.export_document_via_prompt(
            ExportFormat::Pdf,
            window,
            cx,
        );
    }

    pub fn toggle_pane_kind(&mut self, cx: &mut Context<Self>) {
        let active_pane = self.active_pane_id();
        let current_kind = self.active_pane_kind();
        let next_kind = if current_kind == PaneKindId::WYSIWYG {
            PaneKindId::SOURCE_CODE
        } else {
            PaneKindId::WYSIWYG
        };
        self.change_pane_kind(active_pane, next_kind);

        {
            let pane_id = self.active_pane_id();
            let state = self.pane_state(pane_id);
            state.scroll.pending_autoscroll = Some(AutoscrollStrategy::Fit {
                margin: px(20.0),
            });
            state.scroll.last_viewport_size = None;
        }
        self.tab_mut().file.pending_window_title_refresh = true;
        self.tab_mut().file.close_dialog_restore_focus = None;
        self.sync_panes_with_active_tab(cx);
        cx.notify();
    }

    pub fn bump_document_revision(&mut self) {
        if let Some(tab) = self.session.active_tab_mut() {
            tab.document_revision = tab.document_revision.wrapping_add(1);
        }
    }

    pub fn mark_dirty(&mut self, cx: &mut Context<Self>) {
        if let Some(tab) = self.session.active_tab_mut() {
            tab.file.dirty = true;
            tab.file.pending_window_edited = true;
            tab.document_revision = tab.document_revision.wrapping_add(1);
            tab.cached_word_count = None;
        }
        cx.notify();
    }

    // ── Menu Request Helpers ────────────────────────────────────────────────

    pub fn request_save_document(&mut self, cx: &mut Context<Self>) {
        if !self.has_tabs() {
            return;
        }
        if !self.tab().file.pending_save {
            self.tab_mut().file.pending_save = true;
            cx.notify();
        }
    }

    pub fn request_save_document_as(&mut self, cx: &mut Context<Self>) {
        if !self.has_tabs() {
            return;
        }
        if !self.tab().file.pending_save_as {
            self.tab_mut().file.pending_save_as = true;
            cx.notify();
        }
    }

    pub fn request_open_link_prompt(
        &mut self,
        prompt_target: String,
        open_target: String,
        cx: &mut Context<Self>,
    ) {
        self.tab_mut().file.pending_open_link = Some(PendingOpenLink {
            prompt_target,
            open_target,
        });
        cx.notify();
    }

    // ── Viewport Navigation & Page Scrolling ────────────────────────────────

    pub(crate) fn on_page_up(&mut self, _: &PageUp, window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_tabs() {
            return;
        }
        let page = self.active_pane_scroll().handle.bounds().size.height;
        self.scroll_viewport_by(self.active_pane_id(), page, window, cx);
    }

    pub(crate) fn on_page_down(
        &mut self,
        _: &PageDown,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_tabs() {
            return;
        }
        let page = self.active_pane_scroll().handle.bounds().size.height;
        self.scroll_viewport_by(self.active_pane_id(), -page, window, cx);
    }

    pub(crate) fn on_jump_to_top(
        &mut self,
        _: &JumpToTop,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_tabs() {
            return;
        }
        self.set_vertical_scroll_offset(self.active_pane_id(), px(0.0), window, cx);
    }

    pub(crate) fn on_jump_to_bottom(
        &mut self,
        _: &JumpToBottom,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_tabs() {
            return;
        }
        let max_offset_y = self
            .active_pane_scroll()
            .handle
            .max_offset()
            .y
            .max(px(0.0));
        self.set_vertical_scroll_offset(self.active_pane_id(), -max_offset_y, window, cx);
    }

    pub(crate) fn scroll_viewport_by(
        &mut self,
        pane_id: PaneId,
        delta: Pixels,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self
            .pane_state_ref(pane_id)
            .map(|state| state.scroll.handle.offset().y + delta)
            .unwrap_or_default();
        self.set_vertical_scroll_offset(pane_id, target, window, cx);
    }

    pub(crate) fn set_vertical_scroll_offset(
        &mut self,
        pane_id: PaneId,
        target_y: Pixels,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let max_offset_y = self
            .pane_state_ref(pane_id)
            .map(|state| state.scroll.handle.max_offset().y.max(px(0.0)))
            .unwrap_or_default();
        let mut offset = self
            .pane_state_ref(pane_id)
            .map(|state| state.scroll.handle.offset())
            .unwrap_or_default();
        offset.y = target_y.min(px(0.0)).max(-max_offset_y);
        let pane = self.pane_state(pane_id);
        pane.scroll.handle.set_offset(offset);
        cx.notify();
    }
}
