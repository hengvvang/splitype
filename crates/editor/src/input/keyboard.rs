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
        if event.keystroke.key.as_str() == "escape" && cx.has_active_drag() {
            cx.stop_active_drag(window);
            self.tab_drag_hover = None;
            self.tab_reorder_target = None;
            cx.stop_propagation();
            cx.notify();
            return;
        }

        if self.search.visible {
            let is_query_focused = self.search.search_focus_handle.is_focused(window);
            let is_replace_focused = self.search.replace_focus_handle.is_focused(window);
            if is_query_focused || is_replace_focused {
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
        let had_tab_drag = self.tab_drag_hover.take().is_some();
        let had_reorder = self.tab_reorder_target.take().is_some();
        let cancelled_drag = self.session.root.cancel_drag_gesture();
        let closed_menu = self.session.root.interaction.clear_border_menu();
        let closed_dropdown = self.session.root.interaction.clear_dropdowns();
        let handled =
            had_tab_drag || had_reorder || cancelled_drag || closed_menu || closed_dropdown;
        if handled {
            cx.notify();
        }
        handled
    }
}
