//! Dropped-file replacement confirmation dialog overlay.

use gpui::*;

use crate::shell::Shell;
use config::language::I18nManager;
use theme::Theme;
use ui::button::{compact_primary_button, compact_secondary_button};
use ui::dialog::dialog_card;
use ui::popover::overlay;

impl Shell {
    pub(crate) fn on_cancel_drop_replace_dialog(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(panel) = self.document_panel_with_drop_replace_dialog_mut(cx) else {
            return;
        };
        panel.cancel_drop_replace_dialog(cx);
    }

    pub(crate) fn on_discard_and_replace_drop(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(panel) = self.document_panel_with_drop_replace_dialog_mut(cx) else {
            return;
        };
        panel.discard_pending_drop_replace(window, cx);
    }

    pub(crate) fn on_save_and_replace_drop(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(panel) = self.document_panel_with_drop_replace_dialog_mut(cx) else {
            return;
        };
        panel.save_and_replace_pending_drop(window, cx);
    }

    pub(crate) fn render_drop_replace_overlay(
        &self,
        theme: &Theme,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let strings = cx.global::<I18nManager>().strings();

        overlay()
            .id("drop-replace-overlay")
            .flex()
            .items_center()
            .justify_center()
            .on_click(cx.listener(Self::on_cancel_drop_replace_dialog))
            .child(
                div()
                    .w_full()
                    .px(px(d.editor_padding))
                    .flex()
                    .justify_center()
                    .child(
                        dialog_card(c, d)
                            .id("drop-replace-dialog")
                            .w(px(440.0))
                            .p(px(0.0))
                            .gap(px(0.0))
                            .overflow_hidden()
                            .border(px(1.0))
                            .border_color(c.dialog_border)
                            .rounded(px(d.dialog_radius))
                            .shadow_2xl()
                            .occlude()
                            .on_click(|_event, _window, _cx| {})
                            .child(
                                div()
                                    .w_full()
                                    .p(px(24.0))
                                    .flex()
                                    .flex_col()
                                    .gap(px(12.0))
                                    .child(
                                        div()
                                            .text_size(px(20.0))
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .text_color(c.dialog_title)
                                            .child(strings.drop_replace_title.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .line_height(rems(t.text_line_height))
                                            .text_color(c.dialog_body)
                                            .child(strings.drop_replace_message.clone()),
                                    ),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .px(px(24.0))
                                    .py(px(16.0))
                                    .bg(c.dialog_secondary_button_bg)
                                    .border_t_1()
                                    .border_color(c.dialog_border)
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        compact_primary_button(
                                            "save-and-replace-drop-dialog",
                                            c,
                                            d,
                                        )
                                        .flex_1()
                                        .h(px(32.0))
                                        .text_size(px(13.0))
                                        .font_weight(t.dialog_button_weight.to_font_weight())
                                        .text_color(c.dialog_primary_button_text)
                                        .child(strings.drop_replace_save_and_replace.clone())
                                        .on_click(cx.listener(Self::on_save_and_replace_drop)),
                                    )
                                    .child(
                                        compact_secondary_button(
                                            "discard-and-replace-drop-dialog",
                                            c,
                                            d,
                                        )
                                        .flex_1()
                                        .h(px(32.0))
                                        .bg(c.dialog_surface)
                                        .text_size(px(13.0))
                                        .font_weight(t.dialog_button_weight.to_font_weight())
                                        .text_color(c.dialog_secondary_button_text)
                                        .child(strings.drop_replace_discard_and_replace.clone())
                                        .on_click(cx.listener(Self::on_discard_and_replace_drop)),
                                    )
                                    .child(
                                        compact_secondary_button(
                                            "cancel-drop-replace-dialog",
                                            c,
                                            d,
                                        )
                                        .flex_1()
                                        .h(px(32.0))
                                        .bg(c.dialog_surface)
                                        .text_size(px(13.0))
                                        .font_weight(t.dialog_button_weight.to_font_weight())
                                        .text_color(c.dialog_secondary_button_text)
                                        .child(strings.drop_replace_cancel.clone())
                                        .on_click(cx.listener(Self::on_cancel_drop_replace_dialog)),
                                    ),
                            ),
                    ),
            )
    }
}
