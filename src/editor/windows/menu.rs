//! Menu-bar and info-dialog state machine.
//!
//! Tracks which menu/submenu is open, hover deferral and the delayed-close
//! task, plus the save/open-link prompts that menu actions initiate. The
//! hover/expand input handlers from the editor chrome are co-located with
//! the state they drive.

use crate::editor::controller::*;

impl Editor {
    pub(crate) fn toggle_menu_bar_expanded(&mut self, cx: &mut Context<Self>) {
        self.chrome.menu_bar_expanded = !self.chrome.menu_bar_expanded;
        if !self.chrome.menu_bar_expanded {
            self.chrome.menu_bar_open = None;
            self.chrome.menu_submenu_open = None;
        }
        cx.notify();
    }

    #[allow(dead_code)]
    pub(crate) fn on_menu_bar_hover(
        &mut self,
        hovered: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_menu_bar_hovered(*hovered, cx);
    }

    pub(crate) fn on_menu_panel_hover(
        &mut self,
        hovered: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_menu_panel_hovered(*hovered, cx);
    }

    pub(crate) fn on_menu_submenu_panel_hover(
        &mut self,
        hovered: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_menu_submenu_panel_hovered(*hovered, cx);
    }

    pub(crate) fn on_menu_submenu_bridge_hover(
        &mut self,
        hovered: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_menu_submenu_bridge_hovered(*hovered, cx);
    }

    pub(crate) fn show_info_dialog(&mut self, kind: InfoDialogKind, cx: &mut Context<Self>) {
        if self.file.show_unsaved_changes_dialog {
            return;
        }

        self.chrome.menu_bar_open = None;
        self.chrome.menu_submenu_open = None;
        self.chrome.menu_submenu_panel_hovered = false;
        self.chrome.menu_submenu_bridge_hovered = false;
        self.chrome.info_dialog = Some(kind);
        cx.notify();
    }

    pub(crate) fn hide_info_dialog(&mut self, cx: &mut Context<Self>) {
        if self.chrome.info_dialog.take().is_some() {
            cx.notify();
        }
    }

    pub(crate) fn open_menu_bar(&mut self, index: usize, cx: &mut Context<Self>) {
        self.chrome.menu_close_task = None;
        if self.chrome.menu_bar_open != Some(index) {
            self.chrome.menu_bar_open = Some(index);
            self.chrome.menu_submenu_open = None;
            self.chrome.menu_submenu_panel_hovered = false;
            self.chrome.menu_submenu_bridge_hovered = false;
            cx.notify();
        }
    }

    pub(crate) fn open_menu_submenu(&mut self, index: usize, cx: &mut Context<Self>) {
        self.chrome.menu_close_task = None;
        if self.chrome.menu_submenu_open != Some(index) {
            self.chrome.menu_submenu_open = Some(index);
            cx.notify();
        }
    }

    pub(crate) fn close_menu_submenu(&mut self, cx: &mut Context<Self>) {
        let had_open_submenu = self.chrome.menu_submenu_open.take().is_some();
        let had_submenu_hover =
            self.chrome.menu_submenu_panel_hovered || self.chrome.menu_submenu_bridge_hovered;
        self.chrome.menu_submenu_panel_hovered = false;
        self.chrome.menu_submenu_bridge_hovered = false;
        if had_open_submenu || had_submenu_hover {
            cx.notify();
        }
    }

    pub(crate) fn schedule_menu_bar_close(&mut self, cx: &mut Context<Self>) {
        if self.chrome.menu_bar_open.is_none() {
            return;
        }

        let weak_editor = cx.entity().downgrade();
        self.chrome.menu_close_task = Some(cx.spawn(
            async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
                cx.background_executor()
                    .timer(Duration::from_millis(120))
                    .await;
                let _ = weak_editor.update(cx, |editor, cx| {
                    editor.chrome.menu_close_task = None;
                    if !editor.chrome.menu_bar_hovered
                        && !editor.chrome.menu_panel_hovered
                        && !editor.chrome.menu_submenu_panel_hovered
                        && !editor.chrome.menu_submenu_bridge_hovered
                    {
                        editor.close_menu_bar(cx);
                    }
                });
            },
        ));
    }

    pub(crate) fn set_menu_bar_hovered(&mut self, hovered: bool, cx: &mut Context<Self>) {
        self.chrome.menu_bar_hovered = hovered;
        if hovered {
            self.chrome.menu_close_task = None;
        } else if !self.chrome.menu_panel_hovered
            && !self.chrome.menu_submenu_panel_hovered
            && !self.chrome.menu_submenu_bridge_hovered
        {
            self.schedule_menu_bar_close(cx);
        }
    }

    pub(crate) fn set_menu_panel_hovered(&mut self, hovered: bool, cx: &mut Context<Self>) {
        self.chrome.menu_panel_hovered = hovered;
        if hovered {
            self.chrome.menu_close_task = None;
        } else if !self.chrome.menu_bar_hovered
            && !self.chrome.menu_submenu_panel_hovered
            && !self.chrome.menu_submenu_bridge_hovered
        {
            self.schedule_menu_bar_close(cx);
        }
    }

    pub(crate) fn set_menu_submenu_panel_hovered(&mut self, hovered: bool, cx: &mut Context<Self>) {
        self.chrome.menu_submenu_panel_hovered = hovered;
        if hovered {
            self.chrome.menu_close_task = None;
        } else if !self.chrome.menu_bar_hovered
            && !self.chrome.menu_panel_hovered
            && !self.chrome.menu_submenu_bridge_hovered
        {
            self.schedule_menu_bar_close(cx);
        }
    }

    /// Hover handler for the invisible gap bridge. The bridge and the submenu
    /// panel overlap, so the cursor crossing between them fires a `false` for
    /// one region and a `true` for the other in the same gesture. Keeping their
    /// hover state in separate flags lets either one hold the menu open
    /// regardless of the order those events arrive.
    pub(crate) fn set_menu_submenu_bridge_hovered(
        &mut self,
        hovered: bool,
        cx: &mut Context<Self>,
    ) {
        self.chrome.menu_submenu_bridge_hovered = hovered;
        if hovered {
            self.chrome.menu_close_task = None;
        } else if !self.chrome.menu_bar_hovered
            && !self.chrome.menu_panel_hovered
            && !self.chrome.menu_submenu_panel_hovered
        {
            self.schedule_menu_bar_close(cx);
        }
    }

    pub(crate) fn dismiss_menu_bar_from_body(&mut self, cx: &mut Context<Self>) {
        if self.chrome.menu_bar_open.is_some() {
            self.close_menu_bar(cx);
        }
    }

    pub(crate) fn request_save_document(&mut self, cx: &mut Context<Self>) {
        if !self.file.pending_save {
            self.file.pending_save = true;
            cx.notify();
        }
    }

    pub(crate) fn request_save_document_as(&mut self, cx: &mut Context<Self>) {
        if !self.file.pending_save_as {
            self.file.pending_save_as = true;
            cx.notify();
        }
    }

    pub(crate) fn request_open_link_prompt(
        &mut self,
        prompt_target: String,
        open_target: String,
        cx: &mut Context<Self>,
    ) {
        self.file.pending_open_link = Some(PendingOpenLink {
            prompt_target,
            open_target,
        });
        cx.notify();
    }

    pub(crate) fn close_menu_bar(&mut self, cx: &mut Context<Self>) {
        let had_open_menu = self.chrome.menu_bar_open.take().is_some();
        let had_open_submenu = self.chrome.menu_submenu_open.take().is_some();
        let had_hover_state = self.chrome.menu_bar_hovered
            || self.chrome.menu_panel_hovered
            || self.chrome.menu_submenu_panel_hovered
            || self.chrome.menu_submenu_bridge_hovered;
        let had_pending_close = self.chrome.menu_close_task.take().is_some();
        self.chrome.menu_bar_hovered = false;
        self.chrome.menu_panel_hovered = false;
        self.chrome.menu_submenu_panel_hovered = false;
        self.chrome.menu_submenu_bridge_hovered = false;
        if had_open_menu || had_open_submenu || had_hover_state || had_pending_close {
            cx.notify();
        }
    }
}
