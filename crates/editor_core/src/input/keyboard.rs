//! Keyboard event handling for the Editor frame.

use gpui::*;

use crate::editor::Editor;

impl Editor {
    pub(crate) fn on_editor_key_down_capture(
        &mut self,
        _event: &KeyDownEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
    }
}
