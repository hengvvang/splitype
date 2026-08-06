//! Source code panel — raw Markdown buffer editing view.

use gpui::*;

use crate::editor::controller::*;
use crate::theme::Theme;

impl Editor {
    pub(crate) fn render_source_editor_panel(
        &mut self,
        area_id: usize,
        panel_id: usize,
        theme: &Theme,
        _cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;

        let content: AnyElement = if let Some(ref block) = self.tab().source_panel.block {
            div()
                .w_full()
                .flex_shrink_0()
                .child(block.clone())
                .into_any_element()
        } else {
            div().into_any_element()
        };

        div()
            .id(ElementId::Name(
                format!("tiled-source-editor-{area_id}-{panel_id}").into(),
            ))
            .w_full()
            .h_full()
            .relative()
            .bg(c.editor_background)
            .child(
                div()
                    .id(ElementId::Name(
                        format!("tiled-source-scroll-{area_id}-{panel_id}").into(),
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
