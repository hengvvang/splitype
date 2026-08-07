//! In-window titlebar menu-bar state machine.
//!
//! Tracks which menu/submenu is open, hover deferral and the delayed-close
//! task, plus the info dialog and save/open-link prompts that menu actions
//! initiate. The hover/expand input handlers are co-located with the state
//! they drive; rendering lives in [`super`].

use gpui::Task;

use crate::editor::controller::*;

/// Open/hover state for the in-window titlebar menu bar.
#[derive(Default)]
pub(crate) struct MenuBarState {
    /// Open top-level menu in the in-window fallback menu bar.
    pub(crate) open: Option<usize>,
    pub(crate) expanded: bool,
    /// Open child submenu inside the in-window fallback menu panel.
    pub(crate) submenu_open: Option<usize>,
    pub(crate) bar_hovered: bool,
    pub(crate) panel_hovered: bool,
    pub(crate) submenu_panel_hovered: bool,
    /// Hover state for the invisible bridge spanning the gap between the menu
    /// panel and an open submenu. Tracked separately from
    /// `submenu_panel_hovered` so the handoff between the two regions cannot
    /// clobber a single shared flag and tear the menu down.
    pub(crate) submenu_bridge_hovered: bool,
    pub(crate) close_task: Option<Task<()>>,
}

impl Editor {
    pub(crate) fn toggle_menu_bar_expanded(&mut self, cx: &mut Context<Self>) {
        self.menu_bar.expanded = !self.menu_bar.expanded;
        if !self.menu_bar.expanded {
            self.menu_bar.open = None;
            self.menu_bar.submenu_open = None;
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
        if self
            .active_editor_tab()
            .is_some_and(|tab| tab.file.show_unsaved_changes_dialog)
        {
            return;
        }

        self.menu_bar.open = None;
        self.menu_bar.submenu_open = None;
        self.menu_bar.submenu_panel_hovered = false;
        self.menu_bar.submenu_bridge_hovered = false;
        self.info_dialog = Some(kind);
        cx.notify();
    }

    pub(crate) fn hide_info_dialog(&mut self, cx: &mut Context<Self>) {
        if self.info_dialog.take().is_some() {
            cx.notify();
        }
    }

    pub(crate) fn open_menu_bar(&mut self, index: usize, cx: &mut Context<Self>) {
        self.menu_bar.close_task = None;
        if self.menu_bar.open != Some(index) {
            self.menu_bar.open = Some(index);
            self.menu_bar.submenu_open = None;
            self.menu_bar.submenu_panel_hovered = false;
            self.menu_bar.submenu_bridge_hovered = false;
            cx.notify();
        }
    }

    pub(crate) fn open_menu_submenu(&mut self, index: usize, cx: &mut Context<Self>) {
        self.menu_bar.close_task = None;
        if self.menu_bar.submenu_open != Some(index) {
            self.menu_bar.submenu_open = Some(index);
            cx.notify();
        }
    }

    pub(crate) fn close_menu_submenu(&mut self, cx: &mut Context<Self>) {
        let had_open_submenu = self.menu_bar.submenu_open.take().is_some();
        let had_submenu_hover =
            self.menu_bar.submenu_panel_hovered || self.menu_bar.submenu_bridge_hovered;
        self.menu_bar.submenu_panel_hovered = false;
        self.menu_bar.submenu_bridge_hovered = false;
        if had_open_submenu || had_submenu_hover {
            cx.notify();
        }
    }

    pub(crate) fn schedule_menu_bar_close(&mut self, cx: &mut Context<Self>) {
        if self.menu_bar.open.is_none() {
            return;
        }

        let weak_editor = cx.entity().downgrade();
        self.menu_bar.close_task = Some(cx.spawn(
            async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
                cx.background_executor()
                    .timer(Duration::from_millis(120))
                    .await;
                let _ = weak_editor.update(cx, |editor, cx| {
                    editor.menu_bar.close_task = None;
                    if !editor.menu_bar.bar_hovered
                        && !editor.menu_bar.panel_hovered
                        && !editor.menu_bar.submenu_panel_hovered
                        && !editor.menu_bar.submenu_bridge_hovered
                    {
                        editor.close_menu_bar(cx);
                    }
                });
            },
        ));
    }

    pub(crate) fn set_menu_bar_hovered(&mut self, hovered: bool, cx: &mut Context<Self>) {
        self.menu_bar.bar_hovered = hovered;
        if hovered {
            self.menu_bar.close_task = None;
        } else if !self.menu_bar.panel_hovered
            && !self.menu_bar.submenu_panel_hovered
            && !self.menu_bar.submenu_bridge_hovered
        {
            self.schedule_menu_bar_close(cx);
        }
    }

    pub(crate) fn set_menu_panel_hovered(&mut self, hovered: bool, cx: &mut Context<Self>) {
        self.menu_bar.panel_hovered = hovered;
        if hovered {
            self.menu_bar.close_task = None;
        } else if !self.menu_bar.bar_hovered
            && !self.menu_bar.submenu_panel_hovered
            && !self.menu_bar.submenu_bridge_hovered
        {
            self.schedule_menu_bar_close(cx);
        }
    }

    pub(crate) fn set_menu_submenu_panel_hovered(&mut self, hovered: bool, cx: &mut Context<Self>) {
        self.menu_bar.submenu_panel_hovered = hovered;
        if hovered {
            self.menu_bar.close_task = None;
        } else if !self.menu_bar.bar_hovered
            && !self.menu_bar.panel_hovered
            && !self.menu_bar.submenu_bridge_hovered
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
        self.menu_bar.submenu_bridge_hovered = hovered;
        if hovered {
            self.menu_bar.close_task = None;
        } else if !self.menu_bar.bar_hovered
            && !self.menu_bar.panel_hovered
            && !self.menu_bar.submenu_panel_hovered
        {
            self.schedule_menu_bar_close(cx);
        }
    }

    pub(crate) fn dismiss_menu_bar_from_body(&mut self, cx: &mut Context<Self>) {
        if self.menu_bar.open.is_some() {
            self.close_menu_bar(cx);
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

    pub(crate) fn close_menu_bar(&mut self, cx: &mut Context<Self>) {
        let had_open_menu = self.menu_bar.open.take().is_some();
        let had_open_submenu = self.menu_bar.submenu_open.take().is_some();
        let had_hover_state = self.menu_bar.bar_hovered
            || self.menu_bar.panel_hovered
            || self.menu_bar.submenu_panel_hovered
            || self.menu_bar.submenu_bridge_hovered;
        let had_pending_close = self.menu_bar.close_task.take().is_some();
        self.menu_bar.bar_hovered = false;
        self.menu_bar.panel_hovered = false;
        self.menu_bar.submenu_panel_hovered = false;
        self.menu_bar.submenu_bridge_hovered = false;
        if had_open_menu || had_open_submenu || had_hover_state || had_pending_close {
            cx.notify();
        }
    }
}
