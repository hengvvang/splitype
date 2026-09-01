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
        pane_id: PaneId,
        index: usize,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) {
        self.outline.active_index = Some(index);
        let kind = self
            .pane_kind(pane_id)
            .unwrap_or_else(|| self.default_pane_kind());
        if let Some(state) = self.pane_state_mut(pane_id) {
            if state.pane.capabilities().outline {
                state.pane.navigate_to_outline(index, theme, cx);
                let headings = state.pane.outline_headings(cx);
                if let Some(node) = headings.get(index) {
                    let target_y = if kind.as_str() == "splitype.pane.source_code" {
                        let font_size = theme.typography.code_size.max(12.0);
                        let line_height = (font_size * theme.typography.text_line_height).round();
                        let padding = theme.dimensions.editor_padding;
                        (node.block_index as f32 * line_height) - padding
                    } else if kind.as_str() == "splitype.pane.preview" {
                        let font_size = theme.typography.text_size.max(14.0);
                        let line_height = (font_size * theme.typography.text_line_height)
                            .round()
                            .max(22.0);
                        (node.block_index as f32 * line_height * 2.0).max(0.0)
                    } else {
                        let font_size = theme.typography.text_size.max(14.0);
                        let line_height = (font_size * theme.typography.text_line_height)
                            .round()
                            .max(24.0);
                        (node.block_index as f32 * line_height * 1.5) - 40.0
                    };
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
