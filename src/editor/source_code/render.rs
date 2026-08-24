//! Source code panel — raw Markdown buffer editing view.

use gpui::*;

use crate::editor::controller::*;
use crate::infra::theme::Theme;

impl Editor {
    pub(crate) fn render_source_pane(
        &mut self,
        pane_id: PaneId,
        theme: &Theme,
        _cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;

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
                    .p(px(d.editor_padding))
                    .child(content),
            )
            .into_any_element()
    }
}
