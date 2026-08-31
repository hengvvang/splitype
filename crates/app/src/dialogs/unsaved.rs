//! Unsaved changes confirmation dialog overlay and actions.

use gpui::*;

use crate::shell::{Shell, UnsavedDialogScope};
use config::language::I18nManager;
use theme::Theme;
use ui::button::{compact_danger_button, compact_primary_button, compact_secondary_button};
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
        if let Some(dialog) = self.unsaved_dialog.take() {
            let target_editor = dialog
                .scope
                .panel_id()
                .and_then(|id| self.editor_for(id))
                .or_else(|| self.active_editor());
            if let Some(editor) = target_editor {
                editor.update(cx, |editor, cx| {
                    if let Some(_restore) = dialog.restore_focus {
                        let active_pane = editor.active_pane_id();
                        if let Some(state) = editor.pane_state_mut(active_pane) {
                            let _ = state.pane.focus_handle(cx);
                        }
                    }
                    cx.notify();
                });
            }
        }
        if let Some(editor) = self.editor_with_dialog(cx, |file| file.show_unsaved_changes_dialog) {
            editor.update(cx, |editor, cx| editor.cancel_close_dialog(cx));
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
                    window.remove_window();
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
                    if let Some(editor) = self.editor_for(panel_id) {
                        editor.update(cx, |ed, cx| {
                            ed.save_tab_at(index, window, cx);
                            ed.close_tab(index, cx);
                        });
                    }
                }
            }
            cx.notify();
            return;
        }

        if let Some(editor) = self.editor_with_dialog(cx, |file| file.show_unsaved_changes_dialog) {
            editor.update(cx, |editor, cx| editor.save_and_close(window, cx));
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
                            window::PanelRegistry::registered(retained.kind)
                        {
                            descriptor.discard_retained(&mut retained.state, cx);
                        }
                    }
                    for view in self.panel_views.values_mut() {
                        view.discard_changes(cx);
                    }
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
                    if let Some(editor) = self.editor_for(panel_id) {
                        editor.update(cx, |ed, cx| {
                            if let Some(tab) = ed.session.tab_mut(index) {
                                tab.file.dirty = false;
                            }
                            ed.close_tab(index, cx);
                        });
                    }
                }
            }
            cx.notify();
            return;
        }

        if let Some(editor) = self.editor_with_dialog(cx, |file| file.show_unsaved_changes_dialog) {
            editor.update(cx, |editor, cx| editor.discard_and_close(cx));
            if self.has_unsaved_changes(cx) {
                self.prompt_close_window(cx);
            } else {
                window.remove_window();
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
                            .w(px(400.0))
                            .border(px(1.0))
                            .border_color(c.dialog_border)
                            .rounded(px(d.dialog_radius))
                            .shadow_xl()
                            .p(px(20.0))
                            .gap(px(16.0))
                            .occlude()
                            .on_click(|_event, _window, _cx| {})
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.0))
                                    .child(
                                        div()
                                            .text_size(px(16.0))
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .text_color(c.dialog_title)
                                            .child(title),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .line_height(rems(1.4))
                                            .text_color(c.dialog_body)
                                            .child(message),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .justify_end()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        compact_secondary_button("cancel-close-dialog", c, d)
                                            .text_size(px(13.0))
                                            .font_weight(t.dialog_button_weight.to_font_weight())
                                            .text_color(c.dialog_secondary_button_text)
                                            .child(strings.unsaved_changes_cancel.clone())
                                            .on_click(cx.listener(Self::on_cancel_close_dialog)),
                                    )
                                    .child(
                                        compact_danger_button("discard-and-close-dialog", c, d)
                                            .text_size(px(13.0))
                                            .font_weight(t.dialog_button_weight.to_font_weight())
                                            .text_color(c.dialog_danger_button_text)
                                            .child(strings.unsaved_changes_discard.clone())
                                            .on_click(cx.listener(Self::on_discard_and_close)),
                                    )
                                    .child(
                                        compact_primary_button("save-and-close-dialog", c, d)
                                            .text_size(px(13.0))
                                            .font_weight(t.dialog_button_weight.to_font_weight())
                                            .text_color(c.dialog_primary_button_text)
                                            .child(strings.unsaved_changes_save.clone())
                                            .on_click(cx.listener(Self::on_save_and_close)),
                                    ),
                            ),
                    ),
            )
    }
}
