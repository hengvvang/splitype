//! Keyboard event handling for the Editor frame.

use gpui::*;

use crate::editor::Editor;

impl Editor {
    pub(crate) fn on_editor_key_down_capture(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.search.visible {
            let is_query_focused = self.search.search_focus_handle.is_focused(window);
            let is_replace_focused = self.search.replace_focus_handle.is_focused(window);
            if is_query_focused || is_replace_focused {
                self.handle_search_key_down(event, window, cx);
                cx.stop_propagation();
                return;
            }
        }

        let active_pane = self.active_pane_id();
        let handled = self.handle_pane_key_down(active_pane, event, window, cx);
        if handled {
            cx.stop_propagation();
        }
    }

    /// Esc dismissal, invoked by the shell's global `DismissTransientUi`
    /// action through [`platform_contracts::PanelView::dismiss_overlays`].
    ///
    /// Cancels in-progress pane split operations: drag gestures (without
    /// applying them), the border context menu, and open pane-kind
    /// dropdowns. Returns whether anything was dismissed.
    pub fn dismiss_transient_ui(&mut self, cx: &mut Context<Self>) -> bool {
        let cancelled_drag = self.session.root.cancel_drag_gesture();
        let closed_menu = self.session.root.active_border_menu.take().is_some();
        let closed_dropdown = self.session.root.clear_dropdowns();
        let handled = cancelled_drag || closed_menu || closed_dropdown;
        if handled {
            cx.notify();
        }
        handled
    }
}
