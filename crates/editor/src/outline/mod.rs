//! Outline HUD coordination — navigation and hover event bridging for the
//! active pane. The HUD state lives in `editor_contracts::OutlineHudState`;
//! the floating HUD rendering lives in the `ui` crate.

use std::time::Duration;

use gpui::*;

use crate::editor::Editor;
use editor_contracts::PaneId;
use theme::Theme;

impl Editor {
    /// Sets whether the outline HUD popover is hovered with a debounce on exit.
    pub(crate) fn set_outline_hovered(
        &mut self,
        hovered: bool,
        _window: &mut Window,
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
            cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
                cx.background_executor()
                    .timer(Duration::from_millis(250))
                    .await;
                let _ = weak_editor.update(cx, |editor, cx| {
                    if editor.outline.close_token == token && editor.outline.is_hovered {
                        editor.outline.is_hovered = false;
                        cx.notify();
                    }
                });
            })
            .detach();
        }

        cx.notify();
    }

    /// Navigates the editor to the specified heading in the outline.
    pub(crate) fn navigate_to_outline_index(
        &mut self,
        pane_id: PaneId,
        index: usize,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) {
        self.outline.active_index = Some(index);
        if let Some(state) = self.pane_state_mut(pane_id) {
            if state.pane.capabilities().outline {
                let target_y = state.pane.navigate_to_outline(index, theme, cx);
                if let Some(target_y) = target_y {
                    state
                        .scroll
                        .handle
                        .set_offset(point(px(0.0), px(-target_y.max(0.0))));
                }
            }
        }
        cx.notify();
    }
}
