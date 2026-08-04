//! Source code panel — raw Markdown buffer editing view.

use gpui::*;

use crate::editor::controller::*;
use crate::theme::Theme;

impl Editor {
    pub(crate) fn render_source_editor_panel(
        &mut self,
        theme: &Theme,
        _cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;

        let content: AnyElement = if let Some(ref block) = self.source_panel.block {
            div()
                .w_full()
                .flex_shrink_0()
                .child(block.clone())
                .into_any_element()
        } else {
            div().into_any_element()
        };

        div()
            .id("tiled-source-editor")
            .w_full()
            .h_full()
            .relative()
            .bg(c.editor_background)
            .child(
                div()
                    .id("tiled-source-scroll")
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
