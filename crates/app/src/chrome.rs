//! Window-level chrome for Shell-rooted windows: the in-window menu bar's
//! open/hover state machine ([`MenuBarState`]) plus the chrome renderers —
//! the custom system titlebar ([`titlebar`]), the inline menu bar buttons
//! ([`menu_bar`]), and the menu dropdown panels ([`menu_dropdown`]).

pub mod menu_bar;
pub mod menu_dropdown;
pub mod titlebar;

use gpui::*;
use std::time::Duration;

use crate::shell::Shell;

/// Open/hover state for the in-window titlebar menu bar.
#[derive(Default)]
pub(crate) struct MenuBarState {
    /// Open top-level menu in the in-window menu bar.
    pub(crate) open: Option<usize>,
    pub(crate) expanded: bool,
    /// Open child submenu inside the in-window menu panel.
    pub(crate) submenu_open: Option<usize>,
    pub(crate) panel_hovered: bool,
    pub(crate) submenu_panel_hovered: bool,
    /// Hover state for the invisible bridge spanning the gap between the menu
    /// panel and an open submenu.
    pub(crate) submenu_bridge_hovered: bool,
    pub(crate) close_task: Option<Task<()>>,
}

impl Shell {
    // ── Menu-bar state machine ────────────────────────────────────────────

    pub(crate) fn toggle_menu_bar_expanded(&mut self, cx: &mut Context<Self>) {
        self.menu_bar.expanded = !self.menu_bar.expanded;
        if !self.menu_bar.expanded {
            self.menu_bar.open = None;
            self.menu_bar.submenu_open = None;
        }
        cx.notify();
    }

    pub(crate) fn on_menu_panel_hover(
        &mut self,
        hovered: &bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_menu_panel_hovered(*hovered, window, cx);
    }

    pub(crate) fn on_menu_submenu_panel_hover(
        &mut self,
        hovered: &bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_menu_submenu_panel_hovered(*hovered, window, cx);
    }

    pub(crate) fn on_menu_submenu_bridge_hover(
        &mut self,
        hovered: &bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_menu_submenu_bridge_hovered(*hovered, window, cx);
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

    pub(crate) fn schedule_menu_bar_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.menu_bar.open.is_none() {
            return;
        }

        let weak_shell = cx.entity().downgrade();
        let window_handle = window.window_handle();
        self.menu_bar.close_task = Some(cx.spawn(
            async move |_this: WeakEntity<Shell>, cx: &mut AsyncApp| {
                cx.background_executor()
                    .timer(Duration::from_millis(120))
                    .await;
                let _ = window_handle.update(cx, |_view, _window, cx| {
                    let _ = weak_shell.update(cx, |shell, cx| {
                        shell.menu_bar.close_task = None;
                        if !shell.menu_bar.panel_hovered
                            && !shell.menu_bar.submenu_panel_hovered
                            && !shell.menu_bar.submenu_bridge_hovered
                        {
                            shell.close_menu_bar(cx);
                        }
                    });
                });
            },
        ));
    }

    pub(crate) fn set_menu_panel_hovered(
        &mut self,
        hovered: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.menu_bar.panel_hovered = hovered;
        if hovered {
            self.menu_bar.close_task = None;
        } else if !self.menu_bar.submenu_panel_hovered && !self.menu_bar.submenu_bridge_hovered {
            self.schedule_menu_bar_close(window, cx);
        }
    }

    pub(crate) fn set_menu_submenu_panel_hovered(
        &mut self,
        hovered: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.menu_bar.submenu_panel_hovered = hovered;
        if hovered {
            self.menu_bar.close_task = None;
        } else if !self.menu_bar.panel_hovered && !self.menu_bar.submenu_bridge_hovered {
            self.schedule_menu_bar_close(window, cx);
        }
    }

    pub(crate) fn set_menu_submenu_bridge_hovered(
        &mut self,
        hovered: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.menu_bar.submenu_bridge_hovered = hovered;
        if hovered {
            self.menu_bar.close_task = None;
        } else if !self.menu_bar.panel_hovered && !self.menu_bar.submenu_panel_hovered {
            self.schedule_menu_bar_close(window, cx);
        }
    }

    /// Closes the menu bar and every panel's transient overlays when the
    /// window body (outside the titlebar and any open menu panel) receives
    /// a mouse-down.
    pub(crate) fn on_body_mouse_down(
        &mut self,
        _: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_menu_bar(cx);
        self.dismiss_panel_overlays(cx);
    }

    pub(crate) fn close_menu_bar(&mut self, cx: &mut Context<Self>) {
        let had_open_menu = self.menu_bar.open.take().is_some();
        let had_open_submenu = self.menu_bar.submenu_open.take().is_some();
        let had_hover_state = self.menu_bar.panel_hovered
            || self.menu_bar.submenu_panel_hovered
            || self.menu_bar.submenu_bridge_hovered;
        let had_pending_close = self.menu_bar.close_task.take().is_some();
        self.menu_bar.panel_hovered = false;
        self.menu_bar.submenu_panel_hovered = false;
        self.menu_bar.submenu_bridge_hovered = false;
        if had_open_menu || had_open_submenu || had_hover_state || had_pending_close {
            cx.notify();
        }
    }
}
