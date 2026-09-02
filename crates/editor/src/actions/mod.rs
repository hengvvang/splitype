//! Editor command actions and their dispatch handlers.
//!
//! This module only defines the action protocol ([`defs`], [`edit`]) and the
//! [`Editor`] handlers that route each action to its domain operation. All
//! state changes live in the domain modules (`session`, `navigation`, …).

pub mod defs;

pub use defs::*;

use gpui::*;

use crate::editor::Editor;
use editor_contracts::ExportFormat;
use platform_contracts::actions::{Copy, Cut, Paste, SelectAll};

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
            .is_some_and(|t| t.buffer.read(cx).path.is_none());
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
        if let Some(buffer) = self.session.active_tab().map(|tab| tab.buffer.clone()) {
            buffer.update(cx, |buffer, cx| buffer.undo(cx));
        }
    }

    pub fn on_redo(&mut self, _: &Redo, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_tabs() {
            return;
        }
        if let Some(buffer) = self.session.active_tab().map(|tab| tab.buffer.clone()) {
            buffer.update(cx, |buffer, cx| buffer.redo(cx));
        }
    }

    pub fn on_copy(&mut self, _: &Copy, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_tabs() {
            return;
        }
        let pane_id = self.active_pane_id();
        if let Some(text) = self
            .pane_state_ref(pane_id)
            .and_then(|state| state.pane.selected_text(cx))
        {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    pub fn on_cut(&mut self, _: &Cut, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_tabs() {
            return;
        }
        let pane_id = self.active_pane_id();
        if let Some(text) = self
            .pane_state_ref(pane_id)
            .and_then(|state| state.pane.selected_text(cx))
        {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
        let edit = self
            .pane_state_mut(pane_id)
            .and_then(|state| state.pane.delete_selection(cx));
        if let Some(edit) = edit {
            self.commit_document_edit(edit, cx);
        }
        cx.notify();
    }

    pub fn on_paste(&mut self, _: &Paste, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_tabs() {
            return;
        }
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            return;
        };
        let pane_id = self.active_pane_id();
        let edit = self
            .pane_state_mut(pane_id)
            .and_then(|state| state.pane.insert_text(&text, cx));
        if let Some(edit) = edit {
            self.commit_document_edit(edit, cx);
        }
        cx.notify();
    }

    pub fn on_select_all(&mut self, _: &SelectAll, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_tabs() {
            return;
        }
        let pane_id = self.active_pane_id();
        if let Some(state) = self.pane_state_mut(pane_id) {
            state.pane.select_all(cx);
            cx.notify();
        }
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
