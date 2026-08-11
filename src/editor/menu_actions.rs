//! Editor-side entry points for menu-bar actions.
//!
//! The in-window menu-bar state machine and its rendering moved to the
//! Shell (`crate::app::window_chrome`); this module keeps only the actions
//! a menu click performs on the active editor: info dialogs, save
//! requests, and the open-link prompt.

use super::controller::*;

impl Editor {
    pub(crate) fn show_info_dialog(&mut self, kind: InfoDialogKind, cx: &mut Context<Self>) {
        if self
            .active_editor_tab()
            .is_some_and(|tab| tab.file.show_unsaved_changes_dialog)
        {
            return;
        }

        self.info_dialog = Some(kind);
        cx.notify();
    }

    pub(crate) fn hide_info_dialog(&mut self, cx: &mut Context<Self>) {
        if self.info_dialog.take().is_some() {
            cx.notify();
        }
    }

    pub(crate) fn request_save_document(&mut self, cx: &mut Context<Self>) {
        if !self.has_active_tab() {
            return;
        }
        if !self.tab().file.pending_save {
            self.tab_mut().file.pending_save = true;
            cx.notify();
        }
    }

    pub(crate) fn request_save_document_as(&mut self, cx: &mut Context<Self>) {
        if !self.has_active_tab() {
            return;
        }
        if !self.tab().file.pending_save_as {
            self.tab_mut().file.pending_save_as = true;
            cx.notify();
        }
    }

    pub(crate) fn request_open_link_prompt(
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
}
