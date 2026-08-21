//! Editor-side dialog state and actions: the table-insert dialog (opened
//! from the editor's context menu) and the document-facing half of the
//! window-close confirmation.
//!
//! The window-level dialogs themselves (unsaved changes, drop-replace,
//! Help-menu info) render on the Shell (`crate::app::window_dialogs`);
//! this module keeps the per-tab state those dialogs show and the editor
//! actions their buttons route to.

use crate::ui::popover::overlay;

use crate::ui::dialog::dialog_card;

use crate::ui::button::{primary_button, secondary_button};

use gpui::*;

use crate::editor::controller::Editor;
use crate::editor::view::context_menu::TableInsertTarget;
use crate::infra::i18n::I18nManager;
use crate::infra::theme::Theme;

/// State for the table insertion dialog opened from the context menu.
pub(crate) struct TableInsertDialogState {
    pub target: TableInsertTarget,
    pub body_rows: usize,
    pub columns: usize,
}

impl Editor {
    /// Cancel the unsaved-changes dialog without closing the window
    /// (routed from the Shell's dialog overlay).
    pub(crate) fn cancel_close_dialog(&mut self, cx: &mut Context<Self>) {
        let mut restore_entity = None;
        for tab in &mut self.session.tab_list.tabs {
            if tab.file.show_unsaved_changes_dialog {
                tab.file.show_unsaved_changes_dialog = false;
                tab.file.pending_close_after_save = false;
                if let Some(restore) = tab.file.close_dialog_restore_focus.take() {
                    restore_entity = Some(restore);
                }
            }
        }
        if let Some(restore) = restore_entity {
            let pane = self.active_pane_state();
            pane.focus.active_entity = Some(restore);
        }
        cx.notify();
    }

    /// Save the current document and then close the window (routed from
    /// the Shell's dialog overlay).
    pub(crate) fn save_and_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for tab in &mut self.session.tab_list.tabs {
            if tab.file.show_unsaved_changes_dialog {
                tab.file.show_unsaved_changes_dialog = false;
                tab.file.pending_close_after_save = true;
            }
        }
        self.save_document(window, cx);
    }

    /// Discard unsaved changes and close the window immediately (routed
    /// from the Shell's dialog overlay). Clears the dirty flag so close can proceed.
    pub(crate) fn discard_and_close(&mut self, cx: &mut Context<Self>) {
        for tab in &mut self.session.tab_list.tabs {
            if tab.file.show_unsaved_changes_dialog {
                tab.file.show_unsaved_changes_dialog = false;
                tab.file.pending_close_after_save = false;
                tab.file.dirty = false;
                tab.file.pending_window_edited = false;
                tab.file.pending_window_title_refresh = true;
                tab.file.close_dialog_restore_focus = None;
            }
        }
        cx.notify();
    }

    /// Cancel the pending-close-after-save flag (called when save fails or is
    /// cancelled, or when the save completes but close is no longer desired).
    pub(crate) fn abort_pending_close_after_save(&mut self, cx: &mut Context<Self>) {
        self.tab_mut().file.pending_close_after_save = false;
        self.tab_mut().file.close_dialog_restore_focus = None;
        cx.notify();
    }

    /// The table-insert dialog opened from the context menu: a stepper for
    /// rows and columns, plus confirm / cancel buttons. Centered within
    /// this editor's tile — it targets this editor's document.
    pub(crate) fn render_table_insert_dialog_overlay(
        &self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let dialog = self.table_insert_dialog.as_ref()?;
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let s = cx.global::<I18nManager>().strings().clone();

        let stepper =
            |id_prefix: &'static str,
             label: String,
             value: usize,
             on_dec: fn(&mut Editor, &ClickEvent, &mut Window, &mut Context<Editor>),
             on_inc: fn(&mut Editor, &ClickEvent, &mut Window, &mut Context<Editor>)| {
                div()
                    .flex()
                    .flex_col()
                    .gap(px(d.table_insert_stepper_gap))
                    .child(
                        div()
                            .text_size(px(t.dialog_body_size))
                            .font_weight(t.dialog_button_weight.to_font_weight())
                            .text_color(c.dialog_body)
                            .child(label),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(d.table_insert_stepper_gap))
                            .child(
                                div()
                                    .id((id_prefix, 0usize))
                                    .size(px(d.table_insert_stepper_button_size))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(d.stepper_radius))
                                    .border(px(d.dialog_border_width))
                                    .border_color(c.dialog_border)
                                    .bg(c.dialog_secondary_button_bg)
                                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                                    .cursor_pointer()
                                    .text_color(c.dialog_secondary_button_text)
                                    .on_click(cx.listener(on_dec))
                                    .child(
                                        svg()
                                            .path("icons/editor/context_menu/minus.svg")
                                            .size(px(12.0))
                                            .text_color(c.dialog_secondary_button_text),
                                    ),
                            )
                            .child(
                                div()
                                    .min_w(px(d.table_insert_stepper_value_min_width))
                                    .h(px(d.table_insert_stepper_button_size))
                                    .px(px(d.table_insert_stepper_value_padding_x))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(d.stepper_radius))
                                    .border(px(d.dialog_border_width))
                                    .border_color(c.dialog_border)
                                    .bg(c.dialog_surface)
                                    .text_size(px(t.dialog_body_size))
                                    .text_color(c.dialog_title)
                                    .child(value.to_string()),
                            )
                            .child(
                                div()
                                    .id((id_prefix, 1usize))
                                    .size(px(d.table_insert_stepper_button_size))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(d.stepper_radius))
                                    .border(px(d.dialog_border_width))
                                    .border_color(c.dialog_border)
                                    .bg(c.dialog_secondary_button_bg)
                                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                                    .cursor_pointer()
                                    .text_color(c.dialog_secondary_button_text)
                                    .on_click(cx.listener(on_inc))
                                    .child(
                                        svg()
                                            .path("icons/editor/context_menu/plus.svg")
                                            .size(px(12.0))
                                            .text_color(c.dialog_secondary_button_text),
                                    ),
                            ),
                    )
            };

        Some(
            overlay()
                .id("table-insert-dialog-overlay")
                .occlude()
                .flex()
                .items_center()
                .justify_center()
                .bg(c.dialog_backdrop)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(Self::on_dismiss_context_menu_overlay),
                )
                .child(
                    div()
                        .w_full()
                        .px(px(d.editor_padding))
                        .flex()
                        .justify_center()
                        .child(
                            dialog_card(c, d)
                                .id("table-insert-dialog")
                                .w(px(d.dialog_width.min(d.table_insert_dialog_width)))
                                .border(px(d.dialog_border_width))
                                .border_color(c.dialog_border)
                                .rounded(px(d.dialog_radius))
                                .shadow_lg()
                                .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                                    cx.stop_propagation()
                                })
                                .child(
                                    div()
                                        .text_size(px(t.dialog_title_size))
                                        .font_weight(t.dialog_title_weight.to_font_weight())
                                        .text_color(c.dialog_title)
                                        .child(s.table_insert_title.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(t.dialog_body_size))
                                        .font_weight(t.dialog_body_weight.to_font_weight())
                                        .text_color(c.dialog_body)
                                        .child(s.table_insert_description.clone()),
                                )
                                .child(stepper(
                                    "table-body-rows",
                                    s.table_insert_body_rows.clone(),
                                    dialog.body_rows,
                                    Self::on_table_rows_decrement,
                                    Self::on_table_rows_increment,
                                ))
                                .child(stepper(
                                    "table-columns",
                                    s.table_insert_columns.clone(),
                                    dialog.columns,
                                    Self::on_table_columns_decrement,
                                    Self::on_table_columns_increment,
                                ))
                                .child(
                                    div()
                                        .flex()
                                        .justify_end()
                                        .gap(px(d.dialog_button_gap))
                                        .child(
                                            secondary_button("cancel-table-insert-dialog", c, d)
                                                .text_size(px(t.dialog_button_size))
                                                .font_weight(
                                                    t.dialog_button_weight.to_font_weight(),
                                                )
                                                .text_color(c.dialog_secondary_button_text)
                                                .on_click(
                                                    cx.listener(
                                                        Self::on_cancel_table_insert_dialog,
                                                    ),
                                                )
                                                .child(s.table_insert_cancel.clone()),
                                        )
                                        .child(
                                            primary_button("confirm-table-insert-dialog", c, d)
                                                .text_size(px(t.dialog_button_size))
                                                .font_weight(
                                                    t.dialog_button_weight.to_font_weight(),
                                                )
                                                .text_color(c.dialog_primary_button_text)
                                                .on_click(
                                                    cx.listener(
                                                        Self::on_confirm_table_insert_dialog,
                                                    ),
                                                )
                                                .child(s.table_insert_confirm.clone()),
                                        ),
                                ),
                        ),
                )
                .into_any_element(),
        )
    }
}
