use gpui::*;

use crate::engine::controller::*;
use editor_source_code::SourceCodeViewElement;
use theme::Theme;

impl Editor {
    pub(crate) fn render_source_pane(
        &mut self,
        pane_id: PaneId,
        theme: &Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;

        self.sync_source_pane(pane_id, cx);

        let focus_handle = {
            let state = self.pane_state(pane_id);
            if let Some(source) = state.as_source_code_mut() {
                if source.focus_handle.is_none() {
                    source.focus_handle = Some(cx.focus_handle());
                }
                source.focus_handle.clone().unwrap()
            } else {
                cx.focus_handle()
            }
        };

        if pane_id == self.active_pane_id() {
            self.apply_pending_focus(pane_id, window, cx);
            self.apply_pending_autoscroll(pane_id, window, cx);
        }

        let scroll_handle = self
            .pane_state_ref(pane_id)
            .map(|state| state.scroll.handle.clone())
            .unwrap_or_default();

        div()
            .id(ElementId::Name(
                format!("tiled-source-editor-{pane_id}").into(),
            ))
            .key_context("SourceCode")
            .track_focus(&focus_handle)
            .w_full()
            .h_full()
            .relative()
            .bg(c.editor_background)
            .font(theme::TypographyStore::default_font(
                theme::TypographyScope::Code,
            ))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                if this.handle_source_key_down(pane_id, event, window, cx) {
                    cx.stop_propagation();
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    this.focus_pane(pane_id, window, cx);
                    this.handle_source_mouse_down(pane_id, event, window, cx);
                }),
            )
            .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, window, cx| {
                this.handle_source_mouse_move(pane_id, event, window, cx);
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseUpEvent, window, cx| {
                    this.handle_source_mouse_up(pane_id, event, window, cx);
                }),
            )
            .child(
                div()
                    .id(ElementId::Name(
                        format!("tiled-source-scroll-{pane_id}").into(),
                    ))
                    .w_full()
                    .h_full()
                    .overflow_y_scroll()
                    .track_scroll(&scroll_handle)
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, event, window, cx| {
                            this.defer_host_action(cx, move |host, cx| {
                                host.activate_panel(pane_id.0.into(), cx)
                            });
                            this.on_source_context_menu_mouse_down(event, window, cx);
                        }),
                    )
                    .child(SourceCodeViewElement {
                        view: self.source_view.clone(),
                        ime: self.source_ime.clone(),
                        host: self.pane_host.clone(),
                        pane_id,
                    }),
            )
            .child(self.render_floating_outline_hud(pane_id, EditorPaneKind::SourceCode, theme, cx))
            .into_any_element()
    }
}
