//! Rendering of the editor's context menus: the axis menu items and the
//! overlay panel. The table-insert dialog moved to `dialogs.rs`.

use crate::ui::menu_item::menu_item;
use crate::ui::popover::menu_panel;
use crate::ui::popover::overlay;

use gpui::*;

use crate::editor::engine::controller::Editor;
use crate::editor::panes::document_pane::context_menu::ContextMenuState;
use crate::infra::i18n::I18nManager;
use crate::infra::theme::Theme;
use crate::model::block::table::TableAxis;
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

    /// Renders the footnote content tooltip floating near the pointer when a
    /// footnote reference or definition header is hovered.
    pub(crate) fn render_footnote_tooltip(
        &self,
        theme: &Theme,
        window: &Window,
        _cx: &App,
    ) -> Option<AnyElement> {
        let tooltip = self.footnote_tooltip.as_ref()?;
        let c = &theme.colors;
        let origin = self.panel_rect.map(|rect| rect.origin).unwrap_or_default();
        let top = (tooltip.position.y - origin.y + px(4.0)).max(px(0.0));

        let viewport_width = f32::from(window.viewport_size().width.max(px(1.0)));
        let panel_width = self
            .panel_rect
            .map(|r| f32::from(r.size.width))
            .unwrap_or(viewport_width);
        let max_width = 420.0_f32;
        let mut left_f32 = f32::from(tooltip.position.x - origin.x);
        if left_f32 + 200.0 > panel_width {
            left_f32 = (panel_width - max_width.min(panel_width) - 16.0).max(8.0);
        }
        let left = px(left_f32.max(8.0));

        Some(
            div()
                .absolute()
                .occlude()
                .left(left)
                .top(top)
                .max_w(px(max_width))
                .px(px(10.0))
                .py(px(6.0))
                .rounded(px(5.0))
                .bg(c.dialog_surface)
                .border(px(1.0))
                .border_color(c.dialog_border)
                .shadow_md()
                .text_size(px(13.0))
                .text_color(c.dialog_muted)
                .line_height(relative(1.5))
                .child(tooltip.content.clone())
                .into_any_element(),
        )
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
        // `panel_rect.origin`); translate once, and paint the menu in a
        // deferred layer so panels at tile edges escape the split
        // containers' overflow clipping.
        let origin = self.panel_rect.map(|rect| rect.origin).unwrap_or_default();

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
                let table = table_block.read(cx).data.table.clone()?;
                let items = match selection.kind {
                    TableAxis::Column => vec![
                        Self::render_axis_menu_item(
                            theme,
                            "table-axis-insert-column-left",
                            s.table_axis_insert_column_left.clone(),
                            true,
                            false,
                            Self::on_insert_table_column_left,
                            cx,
                        ),
                        Self::render_axis_menu_item(
                            theme,
                            "table-axis-insert-column-right",
                            s.table_axis_insert_column_right.clone(),
                            true,
                            false,
                            Self::on_insert_table_column_right,
                            cx,
                        ),
                        Self::render_axis_menu_item(
                            theme,
                            "table-axis-duplicate-column",
                            s.table_axis_duplicate_column.clone(),
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
                    TableAxis::Row => {
                        let mut items: Vec<AnyElement> = vec![
                            Self::render_axis_menu_item(
                                theme,
                                "table-axis-insert-row-above",
                                s.table_axis_insert_row_above.clone(),
                                true,
                                false,
                                Self::on_insert_table_row_above,
                                cx,
                            ),
                            Self::render_axis_menu_item(
                                theme,
                                "table-axis-insert-row-below",
                                s.table_axis_insert_row_below.clone(),
                                true,
                                false,
                                Self::on_insert_table_row_below,
                                cx,
                            ),
                            Self::render_axis_menu_item(
                                theme,
                                "table-axis-duplicate-row",
                                s.table_axis_duplicate_row.clone(),
                                true,
                                false,
                                Self::on_duplicate_table_row,
                                cx,
                            ),
                            div()
                                .mx(px(d.menu_separator_margin_x))
                                .my(px(d.menu_separator_margin_y))
                                .h(px(d.menu_separator_height))
                                .bg(c.dialog_border)
                                .into_any_element(),
                        ];
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

    /// Renders the interactive Table Size Matrix Picker popup.
    pub(crate) fn render_table_size_picker_overlay(
        &self,
        theme: &Theme,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let picker = self.table_size_picker.as_ref()?;
        let c = &theme.colors;
        let d = &theme.dimensions;

        let table_block_id = picker.table_block_id;
        let origin = self.panel_rect.map(|rect| rect.origin).unwrap_or_default();
        let pos_x = picker.position.x - origin.x;
        let pos_y = picker.position.y - origin.y;

        let panel_width = 162.0_f32;
        let panel_height = 240.0_f32;

        let viewport = window.viewport_size();
        let max_x = f32::from(viewport.width) - panel_width - 16.0;
        let max_y = f32::from(viewport.height) - panel_height - 16.0;

        let panel_x = px(f32::from(pos_x - px(panel_width)).clamp(8.0, max_x.max(8.0)));
        let panel_y = px(f32::from(pos_y + px(10.0)).clamp(8.0, max_y.max(8.0)));

        let max_matrix_rows = 8usize;
        let max_matrix_cols = 6usize;

        let current_rows = picker.current_rows;
        let current_cols = picker.current_cols;

        let display_rows = picker
            .hovered_rows
            .unwrap_or(picker.current_rows)
            .clamp(1, max_matrix_rows);
        let display_cols = picker
            .hovered_cols
            .unwrap_or(picker.current_cols)
            .clamp(1, max_matrix_cols);

        let is_dark = c.dialog_surface.l < 0.5;
        let (inactive_bg, current_only_bg, hover_only_bg, overlap_bg) = if is_dark {
            (
                Hsla::from(rgba(0x27272aff)), // Inactive block base (dark zinc)
                Hsla::from(rgba(0x3f3f46ff)), // Current layout only: 浅灰
                Hsla::from(rgba(0x71717aff)), // Hovered selection only: 中度灰色
                Hsla::from(rgba(0xa1a1aaff)), // Both current & hovered: 深灰
            )
        } else {
            (
                Hsla::from(rgba(0xf3f4f6ff)), // Inactive block base (very light gray)
                Hsla::from(rgba(0xd1d5dbff)), // Current layout only: 浅灰
                Hsla::from(rgba(0x9ca3afff)), // Hovered selection only: 中度灰色
                Hsla::from(rgba(0x4b5563ff)), // Both current & hovered: 深灰
            )
        };

        let top_indicator = div()
            .w_full()
            .flex()
            .items_center()
            .justify_center()
            .gap(px(6.0))
            .pb(px(8.0))
            .border_b(px(1.0))
            .border_color(c.dialog_border)
            .child(
                div()
                    .px(px(10.0))
                    .py(px(2.0))
                    .border(px(1.0))
                    .border_color(c.dialog_border)
                    .rounded(px(3.0))
                    .text_size(px(12.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(c.text_default)
                    .child(format!("{}", display_rows)),
            )
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(c.dialog_muted)
                    .child("x"),
            )
            .child(
                div()
                    .px(px(10.0))
                    .py(px(2.0))
                    .border(px(1.0))
                    .border_color(c.dialog_border)
                    .rounded(px(3.0))
                    .text_size(px(12.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(c.text_default)
                    .child(format!("{}", display_cols)),
            );

        let mut grid_rows = Vec::with_capacity(max_matrix_rows);
        for r in 0..max_matrix_rows {
            let row_num = r + 1;
            let mut row_cells = Vec::with_capacity(max_matrix_cols);
            for col in 0..max_matrix_cols {
                let col_num = col + 1;
                let is_in_current = r < current_rows && col < current_cols;
                let is_in_hovered = if let (Some(h_r), Some(h_c)) = (picker.hovered_rows, picker.hovered_cols) {
                    r < h_r && col < h_c
                } else {
                    false
                };

                let cell_bg = match (is_in_current, is_in_hovered) {
                    (true, true) => overlap_bg,
                    (false, true) => hover_only_bg,
                    (true, false) => current_only_bg,
                    (false, false) => inactive_bg,
                };

                let cell = div()
                    .id(ElementId::Name(format!("table-size-cell-{}-{}", r, col).into()))
                    .size(px(20.0))
                    .rounded(px(3.0))
                    .bg(cell_bg)
                    .cursor_pointer()
                    .on_hover(cx.listener(move |editor, hovered: &bool, _window, cx| {
                        if *hovered {
                            editor.set_table_size_picker_hover(Some(row_num), Some(col_num), cx);
                        }
                    }))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |editor, _event, _window, cx| {
                            editor.resize_table(table_block_id, row_num, col_num, cx);
                        }),
                    );
                row_cells.push(cell);
            }
            grid_rows.push(
                div()
                    .flex()
                    .gap(px(3.0))
                    .children(row_cells),
            );
        }

        let matrix_grid = div()
            .id("table-size-matrix-grid")
            .flex()
            .flex_col()
            .gap(px(3.0))
            .pt(px(6.0))
            .on_hover(cx.listener(|editor, hovered: &bool, _window, cx| {
                if !*hovered {
                    editor.set_table_size_picker_hover(None, None, cx);
                }
            }))
            .children(grid_rows);

        Some(
            deferred(
                overlay()
                    .id("table-size-picker-overlay")
                    .occlude()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|editor, _event, _window, cx| {
                            editor.close_table_size_picker(cx);
                        }),
                    )
                    .child(
                        div()
                            .id("table-size-picker-panel")
                            .absolute()
                            .left(panel_x)
                            .top(panel_y)
                            .p(px(12.0))
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .bg(c.dialog_surface)
                            .border(px(d.dialog_border_width))
                            .border_color(c.dialog_border)
                            .rounded(px(d.menu_panel_radius))
                            .shadow_lg()
                            .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                                cx.stop_propagation();
                            })
                            .child(top_indicator)
                            .child(matrix_grid),
                    )
                    .into_any_element(),
            )
            .into_any_element(),
        )
    }
}
