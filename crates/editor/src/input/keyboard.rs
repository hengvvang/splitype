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
        let active_pane = self.active_pane_id();
        let handled = self.handle_pane_key_down(active_pane, event, window, cx);
        if handled {
            cx.stop_propagation();
        }
    }
}

