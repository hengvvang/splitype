//! Rendering of the editor's context menus and the table insert dialog:
//! the axis menu items, the overlay panel, and the insert-size dialog.

use crate::ui::button::{primary_button, secondary_button};
use crate::ui::dialog::dialog_card;
use crate::ui::menu_item::menu_item;
use crate::ui::popover::menu_panel;
use crate::ui::popover::overlay;

use gpui::*;

use crate::editor::controller::Editor;
use crate::editor::window::context_menu::ContextMenuState;
use crate::infra::i18n::I18nManager;
use crate::infra::theme::Theme;
use crate::model::syntax::table::TableAxisKind;
impl Editor {
    pub(crate) fn render_axis_menu_item(
        theme: &Theme,
        id: &'static str,
        label: String,
        enabled: bool,
        danger: bool,
        on_click: fn(&mut Editor, &ClickEvent, &mut Window, &mut Context<Editor>),
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        if enabled {
            menu_item(id, c, d)
                .text_size(px(d.menu_text_size))
                .font_weight(t.dialog_body_weight.to_font_weight())
                .text_color(if danger {
                    c.dialog_danger_button_bg
                } else {
                    c.dialog_secondary_button_text
                })
                .child(label)
                .on_click(cx.listener(on_click))
                .into_any_element()
        } else {
            div()
                .id(id)
                .h(px(d.menu_item_height))
                .px(px(d.menu_item_padding_x))
                .flex()
                .items_center()
                .rounded(px(d.menu_item_radius))
                .bg(c.dialog_surface)
                .text_size(px(d.menu_text_size))
                .font_weight(t.dialog_body_weight.to_font_weight())
                .text_color(if danger {
                    c.dialog_danger_button_bg
                } else {
                    c.dialog_muted
                })
                .child(label)
                .into_any_element()
        }
    }

    pub(crate) fn render_context_menu_overlay(
        &self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let menu = self.context_menu.as_ref()?;
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let s = cx.global::<I18nManager>().strings().clone();
        // Mouse positions arrive in window coordinates, but this editor's
        // overlays position inside its own tile (which starts at
        // `area_rect.origin`); translate once, and paint the menu in a
        // deferred layer so panels at tile edges escape the split
        // containers' overflow clipping.
        let origin = self.area_rect.map(|rect| rect.origin).unwrap_or_default();

        match menu {
            ContextMenuState::Insert {
                position,
                submenu_open,
                ..
            } => {
                let panel_x = position.x - origin.x;
                let panel_y = position.y - origin.y;
                let panel_width = px(d.context_menu_panel_width);

                let submenu = submenu_open.then(|| {
                    menu_panel(c, d)
                        .id("editor-context-menu-submenu")
                        .absolute()
                        .left(panel_x + panel_width + px(d.context_menu_submenu_gap))
                        .top(panel_y)
                        .w(px(d.context_menu_submenu_width))
                        .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                            cx.stop_propagation()
                        })
                        .on_hover(cx.listener(Self::on_context_menu_submenu_hover))
                        .child(
                            menu_item("editor-context-menu-insert-table", c, d)
                                .text_size(px(d.menu_text_size))
                                .font_weight(t.dialog_body_weight.to_font_weight())
                                .text_color(c.dialog_secondary_button_text)
                                .child(s.context_menu_table.clone())
                                .on_click(cx.listener(Self::on_open_table_insert_dialog)),
                        )
                });

                let overlay = overlay()
                    .id("editor-context-menu-overlay")
                    .occlude()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(Self::on_dismiss_context_menu_overlay),
                    )
                    .child(
                        menu_panel(c, d)
                            .id("editor-context-menu-panel")
                            .absolute()
                            .left(panel_x)
                            .top(panel_y)
                            .w(panel_width)
                            .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                                cx.stop_propagation()
                            })
                            .child(
                                menu_item("editor-context-menu-insert", c, d)
                                    .justify_between()
                                    .bg(if *submenu_open {
                                        c.panel_row_selected
                                    } else {
                                        c.dialog_surface
                                    })
                                    .text_size(px(d.menu_text_size))
                                    .font_weight(t.dialog_body_weight.to_font_weight())
                                    .text_color(c.dialog_secondary_button_text)
                                    .child(s.context_menu_insert.clone())
                                    .child(
                                        svg()
                                            .path("icons/editor/context_menu/chevron-right.svg")
                                            .size(px(14.0))
                                            .text_color(c.dialog_secondary_button_text),
                                    )
                                    .on_hover(cx.listener(Self::on_context_menu_insert_hover)),
                            ),
                    );

                Some(
                    deferred(if let Some(submenu) = submenu {
                        overlay.child(submenu).into_any_element()
                    } else {
                        overlay.into_any_element()
                    })
                    .into_any_element(),
                )
            }
            ContextMenuState::TableAxis {
                position,
                selection,
            } => {
                let panel_x = position.x - origin.x;
                let panel_y = position.y - origin.y;
                let Some(table_block) = self.table_block_by_id(selection.table_block_id, cx) else {
                    return None;
                };
                let table = table_block.read(cx).record.table.clone()?;
                let items = match selection.kind {
                    TableAxisKind::Column => vec![
                        Self::render_axis_menu_item(
                            theme,
                            "table-axis-insert-column-left",
                            "Insert Column Left".to_string(),
                            true,
                            false,
                            Self::on_insert_table_column_left,
                            cx,
                        ),
                        Self::render_axis_menu_item(
                            theme,
                            "table-axis-insert-column-right",
                            "Insert Column Right".to_string(),
                            true,
                            false,
                            Self::on_insert_table_column_right,
                            cx,
                        ),
                        Self::render_axis_menu_item(
                            theme,
                            "table-axis-duplicate-column",
                            "Duplicate Column".to_string(),
                            true,
                            false,
                            Self::on_duplicate_table_column,
                            cx,
                        ),
                        div()
                            .mx(px(d.menu_separator_margin_x))
                            .my(px(d.menu_separator_margin_y))
                            .h(px(d.menu_separator_height))
                            .bg(c.dialog_border)
                            .into_any_element(),
                        Self::render_axis_menu_item(
                            theme,
                            "table-axis-align-column-left",
                            s.table_axis_align_column_left.clone(),
                            true,
                            false,
                            Self::on_align_table_column_left,
                            cx,
                        ),
                        Self::render_axis_menu_item(
                            theme,
                            "table-axis-align-column-center",
                            s.table_axis_align_column_center.clone(),
                            true,
                            false,
                            Self::on_align_table_column_center,
                            cx,
                        ),
                        Self::render_axis_menu_item(
                            theme,
                            "table-axis-align-column-right",
                            s.table_axis_align_column_right.clone(),
                            true,
                            false,
                            Self::on_align_table_column_right,
                            cx,
                        ),
                        div()
                            .mx(px(d.menu_separator_margin_x))
                            .my(px(d.menu_separator_margin_y))
                            .h(px(d.menu_separator_height))
                            .bg(c.dialog_border)
                            .into_any_element(),
                        Self::render_axis_menu_item(
                            theme,
                            "table-axis-move-column-left",
                            s.table_axis_move_column_left.clone(),
                            selection.index > 0,
                            false,
                            Self::on_move_table_column_left,
                            cx,
                        ),
                        Self::render_axis_menu_item(
                            theme,
                            "table-axis-move-column-right",
                            s.table_axis_move_column_right.clone(),
                            selection.index + 1 < table.column_count(),
                            false,
                            Self::on_move_table_column_right,
                            cx,
                        ),
                        div()
                            .mx(px(d.menu_separator_margin_x))
                            .my(px(d.menu_separator_margin_y))
                            .h(px(d.menu_separator_height))
                            .bg(c.dialog_border)
                            .into_any_element(),
                        Self::render_axis_menu_item(
                            theme,
                            "table-axis-delete-column",
                            s.table_axis_delete_column.clone(),
                            // Always enabled: deleting the last column removes the
                            // whole table.
                            true,
                            true,
                            Self::on_delete_table_column,
                            cx,
                        ),
                    ],
                    TableAxisKind::Row => {
                        let mut items: Vec<AnyElement> = Vec::new();
                        items.push(Self::render_axis_menu_item(
                            theme,
                            "table-axis-insert-row-above",
                            "Insert Row Above".to_string(),
                            true,
                            false,
                            Self::on_insert_table_row_above,
                            cx,
                        ));
                        items.push(Self::render_axis_menu_item(
                            theme,
                            "table-axis-insert-row-below",
                            "Insert Row Below".to_string(),
                            true,
                            false,
                            Self::on_insert_table_row_below,
                            cx,
                        ));
                        items.push(Self::render_axis_menu_item(
                            theme,
                            "table-axis-duplicate-row",
                            "Duplicate Row".to_string(),
                            true,
                            false,
                            Self::on_duplicate_table_row,
                            cx,
                        ));
                        items.push(
                            div()
                                .mx(px(d.menu_separator_margin_x))
                                .my(px(d.menu_separator_margin_y))
                                .h(px(d.menu_separator_height))
                                .bg(c.dialog_border)
                                .into_any_element(),
                        );
                        // The header row (visual index 0) shares the normal row
                        // menu, with its Header Row styling toggle added on top.
                        if selection.index == 0 {
                            let headers_shown =
                                crate::infra::config::settings::EditorSettings::show_table_headers(
                                    cx,
                                );
                            items.push(
                                menu_item("table-header-toggle", c, d)
                                    .justify_between()
                                    .gap(px(d.menu_item_padding_x))
                                    .text_size(px(d.menu_text_size))
                                    .font_weight(t.dialog_body_weight.to_font_weight())
                                    .text_color(c.dialog_secondary_button_text)
                                    .child(s.table_header_row.clone())
                                    .child(if headers_shown { "✓" } else { "" })
                                    .on_click(cx.listener(Self::on_toggle_table_headers))
                                    .into_any_element(),
                            );
                            items.push(
                                div()
                                    .mx(px(d.menu_separator_margin_x))
                                    .my(px(d.menu_separator_margin_y))
                                    .h(px(d.menu_separator_height))
                                    .bg(c.dialog_border)
                                    .into_any_element(),
                            );
                        }
                        items.push(Self::render_axis_menu_item(
                            theme,
                            "table-axis-move-row-up",
                            s.table_axis_move_row_up.clone(),
                            selection.index > 0,
                            false,
                            Self::on_move_table_row_up,
                            cx,
                        ));
                        items.push(Self::render_axis_menu_item(
                            theme,
                            "table-axis-move-row-down",
                            s.table_axis_move_row_down.clone(),
                            selection.index < table.rows.len(),
                            false,
                            Self::on_move_table_row_down,
                            cx,
                        ));
                        items.push(
                            div()
                                .mx(px(d.menu_separator_margin_x))
                                .my(px(d.menu_separator_margin_y))
                                .h(px(d.menu_separator_height))
                                .bg(c.dialog_border)
                                .into_any_element(),
                        );
                        // Always enabled: deleting the header promotes the first
                        // body row, and deleting the last remaining row removes
                        // the whole table.
                        items.push(Self::render_axis_menu_item(
                            theme,
                            "table-axis-delete-row",
                            s.table_axis_delete_row.clone(),
                            true,
                            true,
                            Self::on_delete_table_row,
                            cx,
                        ));
                        items
                    }
                };

                Some(
                    deferred(
                        overlay()
                            .id("table-axis-context-menu-overlay")
                            .occlude()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(Self::on_dismiss_context_menu_overlay),
                            )
                            .child(
                                div()
                                    .id("table-axis-context-menu-panel")
                                    .absolute()
                                    .left(panel_x)
                                    .top(panel_y)
                                    .w(px(d.context_menu_axis_panel_width))
                                    .p(px(d.menu_panel_padding))
                                    .flex()
                                    .flex_col()
                                    .gap(px(d.menu_panel_gap))
                                    .bg(c.dialog_surface)
                                    .border(px(d.dialog_border_width))
                                    .border_color(c.dialog_border)
                                    .rounded(px(d.menu_panel_radius))
                                    .shadow_lg()
                                    .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                                        cx.stop_propagation()
                                    })
                                    .children(items),
                            )
                            .into_any_element(),
                    )
                    .into_any_element(),
                )
            }
        }
    }

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
                                    .rounded(px(d.table_insert_stepper_radius))
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
                                    .rounded(px(d.table_insert_stepper_radius))
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
                                    .rounded(px(d.table_insert_stepper_radius))
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
