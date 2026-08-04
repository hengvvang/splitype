//! Preview panel — read-only rendered snapshot of the document.

use gpui::*;

use crate::editor::controller::*;
use crate::infra::i18n::I18nStrings;
use crate::theme::Theme;

impl Editor {
    pub(crate) fn render_tiled_preview_panel(
        &mut self,
        _primary_content: &mut Option<AnyElement>,
        theme: &Theme,
        _strings: &I18nStrings,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;

        self.refresh_preview_blocks(cx);

        // Render each preview block inside a read-only shell that captures all
        // interaction events. The visual rendering is identical to the Block
        // panel, but mouse clicks, keyboard input, and focus are suppressed so
        // the Preview channel remains a truly read-only view.
        let editor = cx.entity().downgrade();
        let block_elements: Vec<AnyElement> = self
            .preview
            .blocks
            .iter()
            .map(|entity| {
                let block_id = entity.entity_id();
                let preview_editor = editor.clone();
                div()
                    .w_full()
                    .flex_shrink_0()
                    .mt(px(d.block_gap))
                    .cursor_default()
                    .capture_any_mouse_down(move |_event, _window, cx| {
                        cx.stop_propagation();
                    })
                    .capture_key_down(move |_event, _window, cx| {
                        cx.stop_propagation();
                    })
                    .on_mouse_down(MouseButton::Right, move |event, window, cx| {
                        let _ = preview_editor.update(cx, |editor, cx| {
                            editor.on_block_context_menu_mouse_down(block_id, event, window, cx);
                        });
                    })
                    .child(entity.clone())
                    .into_any_element()
            })
            .collect();

        div()
            .w_full()
            .h_full()
            .relative()
            .bg(c.editor_background)
            .child(
                div()
                    .id("tiled-preview-scroll")
                    .w_full()
                    .h_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .overflow_y_scroll()
                    .p(px(d.editor_padding))
                    .children(block_elements),
            )
            .into_any_element()
    }
}
