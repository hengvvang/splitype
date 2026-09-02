//! Unsaved changes confirmation dialog overlay and actions.

use gpui::*;

use crate::shell::{Shell, UnsavedDialogScope};
use config::language::I18nManager;
use theme::Theme;
use ui::button::{compact_primary_button, compact_secondary_button};
use ui::dialog::dialog_card;
use ui::popover::overlay;

impl Shell {
    /// Cancel the unsaved-changes dialog without closing the window or panel.
    pub(crate) fn on_cancel_close_dialog(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.unsaved_dialog.take();
        if let Some(panel) = self.document_panel_with_unsaved_dialog_mut(cx) {
            panel.cancel_close_dialog(cx);
        }
        cx.notify();
    }

    /// Save the target document(s) and then close the window, panel, or tab depending on scope.
    pub(crate) fn on_save_and_close(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(dialog) = self.unsaved_dialog.take() {
            match dialog.scope {
                UnsavedDialogScope::Window => {
                    for view in self.panel_views.values_mut() {
                        let _ = view.save_all(window, cx);
                    }
                    self.close_window_now(window, cx);
                }
                UnsavedDialogScope::Panel(panel_id) => {
                    if let Some(view) = self.panel_views.get_mut(&panel_id) {
                        let _ = view.save_all(window, cx);
                    }
                    if self.layout_leaf_count() > 1 {
                        self.close_panel(panel_id, cx);
                    }
                }
                UnsavedDialogScope::Tab { panel_id, index } => {
                    if let Some(panel) = self.document_panel_mut_for(panel_id) {
                        panel.save_tab_at(index, window, cx);
                        panel.close_tab(index, cx);
                    }
                }
            }
            cx.notify();
            return;
        }

        if let Some(panel) = self.document_panel_with_unsaved_dialog_mut(cx) {
            panel.save_and_close_dialog(window, cx);
        }
    }

    /// Discard unsaved changes and close the window, panel, or tab depending on scope.
    pub(crate) fn on_discard_and_close(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(dialog) = self.unsaved_dialog.take() {
            match dialog.scope {
                UnsavedDialogScope::Window => {
                    for retained in self.retained_panel_states.values_mut() {
                        if let Ok(Some(descriptor)) =
                            window::PanelRegistry::registered(retained.kind.clone())
                        {
                            descriptor.discard_retained(&mut retained.state, cx);
                        }
                    }
                    for view in self.panel_views.values_mut() {
                        view.discard_changes(cx);
                    }
                    self.sweep_orphaned_dirty_buffers(cx);
                    self.snapshot_window_state(cx);
                    window.remove_window();
                }
                UnsavedDialogScope::Panel(panel_id) => {
                    if let Some(view) = self.panel_views.get_mut(&panel_id) {
                        view.discard_changes(cx);
                    }
                    if self.layout_leaf_count() > 1 {
                        self.close_panel(panel_id, cx);
                    }
                }
                UnsavedDialogScope::Tab { panel_id, index } => {
                    if let Some(panel) = self.document_panel_mut_for(panel_id) {
                        panel.discard_tab_at(index, cx);
                    }
                }
            }
            cx.notify();
            return;
        }

        if let Some(panel) = self.document_panel_with_unsaved_dialog_mut(cx) {
            panel.discard_and_close_dialog(cx);
            if self.has_unsaved_changes(cx) {
                self.prompt_close_window(cx);
            } else {
                self.close_window_now(window, cx);
            }
        }
    }

    pub(crate) fn render_unsaved_changes_overlay(
        &self,
        theme: &Theme,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let strings = cx.global::<I18nManager>().strings();

        let (title, message) = if let Some(dialog) = &self.unsaved_dialog {
            match &dialog.scope {
                UnsavedDialogScope::Window => (
                    strings.unsaved_changes_window_title.clone(),
                    strings.unsaved_changes_window_message.clone(),
                ),
                UnsavedDialogScope::Panel(_) => (
                    strings.unsaved_changes_editor_title.clone(),
                    strings.unsaved_changes_editor_message.clone(),
                ),
                UnsavedDialogScope::Tab { .. } => (
                    strings.unsaved_changes_tab_title.clone(),
                    strings
                        .unsaved_changes_tab_message_template
                        .replace("{name}", &dialog.document_name),
                ),
            }
        } else {
            (
                strings.unsaved_changes_title.clone(),
                strings.unsaved_changes_message.clone(),
            )
        };

        overlay()
            .id("unsaved-changes-overlay")
            .flex()
            .items_center()
            .justify_center()
            .on_click(cx.listener(Self::on_cancel_close_dialog))
            .child(
                div()
                    .w_full()
                    .px(px(d.editor_padding))
                    .flex()
                    .justify_center()
                    .child(
                        dialog_card(c, d)
                            .id("unsaved-changes-dialog")
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
                                            .child(title),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .line_height(rems(1.4))
                                            .text_color(c.dialog_body)
                                            .child(message),
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
                                        compact_primary_button("save-and-close-dialog", c, d)
                                            .flex_1()
                                            .h(px(32.0))
                                            .text_size(px(13.0))
                                            .font_weight(t.dialog_button_weight.to_font_weight())
                                            .text_color(c.dialog_primary_button_text)
                                            .child(strings.unsaved_changes_save.clone())
                                            .on_click(cx.listener(Self::on_save_and_close)),
                                    )
                                    .child(
                                        compact_secondary_button("discard-and-close-dialog", c, d)
                                            .flex_1()
                                            .h(px(32.0))
                                            .bg(c.dialog_surface)
                                            .text_size(px(13.0))
                                            .font_weight(t.dialog_button_weight.to_font_weight())
                                            .text_color(c.dialog_secondary_button_text)
                                            .child(strings.unsaved_changes_discard.clone())
                                            .on_click(cx.listener(Self::on_discard_and_close)),
                                    )
                                    .child(
                                        compact_secondary_button("cancel-close-dialog", c, d)
                                            .flex_1()
                                            .h(px(32.0))
                                            .bg(c.dialog_surface)
                                            .text_size(px(13.0))
                                            .font_weight(t.dialog_button_weight.to_font_weight())
                                            .text_color(c.dialog_secondary_button_text)
                                            .child(strings.unsaved_changes_cancel.clone())
                                            .on_click(cx.listener(Self::on_cancel_close_dialog)),
                                    ),
                            ),
                    ),
            )
    }
}
