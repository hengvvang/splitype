//! Dialog action handlers for unsaved changes and close behaviour.
// Migrated from engine/render.rs

use gpui::*;

use crate::engine::editor::Editor;

impl Editor {
    /// Dismiss the unsaved-changes dialog without closing the window.
    pub(crate) fn on_cancel_close_dialog(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.show_unsaved_changes_dialog = false;
        self.pending_close_after_save = false;
        if let Some(restore) = self.close_dialog_restore_focus.take() {
            self.active_entity_id = Some(restore);
        }
        cx.notify();
    }

    /// Save the current document and then close the window.
    pub(crate) fn on_save_and_close(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.show_unsaved_changes_dialog = false;
        self.pending_close_after_save = true;
        self.save_document(window, cx);
    }

    /// Discard unsaved changes and close the window immediately.
    pub(crate) fn on_discard_and_close(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        self.show_unsaved_changes_dialog = false;
        self.pending_close_after_save = false;
        self.close_dialog_restore_focus = None;
        window.remove_window();
    }

    /// Initiate window-close flow, showing the unsaved-changes prompt when
    /// the document is dirty.
    pub(crate) fn request_close_current_window(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.document_dirty {
            self.show_unsaved_changes_dialog = true;
            self.close_dialog_restore_focus = self.active_entity_id;
            cx.notify();
        } else {
            window.remove_window();
        }
    }

    /// Called by the GPUI `Window::on_window_should_close` guard.
    /// Returns `true` when the window is safe to close (clean document).
    /// Returns `false` and shows the unsaved-changes prompt when dirty.
    pub(crate) fn on_window_should_close(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.document_dirty {
            self.show_unsaved_changes_dialog = true;
            self.close_dialog_restore_focus = self.active_entity_id;
            cx.notify();
            false
        } else {
            self.close_guard_installed = false;
            true
        }
    }

    /// Cancel the pending-close-after-save flag (called when save fails or is
    /// cancelled, or when the save completes but close is no longer desired).
    pub(crate) fn abort_pending_close_after_save(&mut self, cx: &mut Context<Self>) {
        self.pending_close_after_save = false;
        self.close_dialog_restore_focus = None;
        cx.notify();
    }
}
