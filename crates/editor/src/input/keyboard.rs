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
}
