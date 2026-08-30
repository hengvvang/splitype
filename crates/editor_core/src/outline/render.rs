//! Outline HUD coordination — navigation and hover event bridging for the active pane.

use std::time::Duration;

use gpui::*;

use crate::editor::Editor;
use crate::session::PaneKindId;
use theme::Theme;

impl Editor {
    /// Sets whether the outline HUD popover is hovered with a debounce on exit.
    pub(crate) fn set_outline_hovered(
        &mut self,
        hovered: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.outline.close_token = self.outline.close_token.wrapping_add(1);
        if hovered {
            if !self.outline.is_hovered {
                self.outline.is_hovered = true;
                cx.notify();
            }
        } else {
            let token = self.outline.close_token;
            let weak_editor = cx.entity().downgrade();
            let window_handle = window.window_handle();
            cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
                cx.background_executor()
                    .timer(Duration::from_millis(200))
                    .await;
                let _ = window_handle.update(cx, |_view, _window, cx| {
                    let _ = weak_editor.update(cx, |editor, cx| {
                        if editor.outline.close_token == token && editor.outline.is_hovered {
                            editor.outline.is_hovered = false;
                            cx.notify();
                        }
                    });
                });
            })
            .detach();
        }

        cx.notify();
    }

    /// Navigates the editor to the specified heading in the outline.
    pub(crate) fn navigate_to_outline_index(
        &mut self,
        index: usize,
        _kind: PaneKindId,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) {
        self.outline.active_index = Some(index);
        let pane_id = self.active_pane_id();
        if let Some(state) = self.pane_state_mut(pane_id) {
            state.pane.navigate_to_outline(index, theme, cx);
        }
        cx.notify();
    }
}
