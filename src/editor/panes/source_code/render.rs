//! Source code panel — raw Markdown buffer editing view.

use gpui::*;

use crate::editor::engine::controller::*;
use crate::infra::theme::Theme;

impl Editor {
    pub(crate) fn render_source_pane(
        &mut self,
        pane_id: PaneId,
        theme: &Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;

        if pane_id == self.active_pane_id() {
            self.apply_pending_focus(pane_id, window, cx);
            self.apply_pending_autoscroll(pane_id, window, cx);
        }

        let scroll_handle = self
            .pane_state_ref(pane_id)
            .map(|state| state.scroll.handle.clone())
            .unwrap_or_default();

        let content: AnyElement = if let Some(block) = self
            .pane_state_ref(pane_id)
            .and_then(|state| state.source_block.clone())
        {
            div()
                .w_full()
                .flex_shrink_0()
                .child(block)
                .into_any_element()
        } else {
            div().into_any_element()
        };

        div()
            .id(ElementId::Name(
                format!("tiled-source-editor-{pane_id}").into(),
            ))
            .w_full()
            .h_full()
            .relative()
            .bg(c.editor_background)
            .child(
                div()
                    .id(ElementId::Name(
                        format!("tiled-source-scroll-{pane_id}").into(),
                    ))
                    .w_full()
                    .h_full()
                    .flex()
                    .flex_col()
                    .overflow_y_scroll()
                    .track_scroll(&scroll_handle)
                    .p(px(d.editor_padding))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, event, window, cx| {
                            this.defer_shell_action(cx, move |shell, cx| {
                                shell.activate_panel(pane_id.0, cx)
                            });
                            this.on_source_context_menu_mouse_down(event, window, cx);
                        }),
                    )
                    .child(content),
            )
            .into_any_element()
    }
}
