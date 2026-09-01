//! Editor command actions and their dispatch handlers.
//!
//! This module only defines the action protocol ([`defs`], [`edit`]) and the
//! [`Editor`] handlers that route each action to its domain operation. All
//! state changes live in the domain modules (`session`, `navigation`, …).

pub mod defs;
pub mod edit;

pub use defs::*;
pub use edit::{BlockStructureKind, EditCommand, InlineFormatKind, InsertionKind};

use gpui::*;

use crate::editor::Editor;
use core_contracts::ExportFormat;

impl Editor {
    pub fn on_save_document(
        &mut self,
        _: &SaveDocument,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let is_untitled = self
            .session
            .active_tab()
            .is_some_and(|t| t.file.path.is_none());
        if is_untitled {
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

    pub fn on_export_html(&mut self, _: &ExportHtml, window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_tabs() {
            return;
        }
        self.export_document_via_prompt(ExportFormat::Html, window, cx);
    }

    pub fn on_export_pdf(&mut self, _: &ExportPdf, window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_tabs() {
            return;
        }
        self.export_document_via_prompt(ExportFormat::Pdf, window, cx);
    }

    // ── Viewport scrolling handlers ─────────────────────────────────────────

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
        let max_offset_y = self.active_pane_scroll().handle.max_offset().y.max(px(0.0));
        self.set_vertical_scroll_offset(self.active_pane_id(), -max_offset_y, window, cx);
    }
}
