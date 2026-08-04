//! Table block rendering.
// Migrated from blocks/render.rs

use gpui::*;

use crate::model::syntax::table::TableCellPosition;
use crate::model::syntax::table::TableColumnAlignment;
use crate::ui::blocks::block_view::Block;
use crate::editor::actions::BlockAction;
use crate::model::syntax::table::{TableAxisKind, TableAxisMarker, TableColumnLayout};
use crate::ui::blocks::block_view::EditMode;
use crate::model::syntax::table::TableAxisHighlight;
use crate::ui::blocks::render::effective_table_width;
use crate::ui::theme::Theme;

/// Render a native table block.
pub(crate) fn render_table(
    block: &mut Block,
    focused: bool,
    is_placeholder: bool,
    focused_base: Stateful<Div>,
    theme: &Theme,
    window: &mut Window,
    cx: &mut Context<Block>,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;

    let Some(runtime) = block.table_runtime.clone() else {
        return focused_base
            .text_size(px(t.text_size))
            .text_color(c.text_default)
            .line_height(rems(t.text_line_height))
            .child(block.render_text_or_mixed_inline_visuals(
                theme,
                focused,
                is_placeholder,
                None,
                None,
                c.text_default,
                t.text_size,
                FontWeight::NORMAL,
                cx,
            ))
            .into_any_element();
    };

    let viewport_width = f32::from(window.viewport_size().width.max(px(1.0)));
    let table_width = effective_table_width(block, viewport_width, d);
    let column_layout = block
        .record
        .table
        .as_ref()
        .map(|table| TableColumnLayout::measure(table, table_width, window, theme))
        .unwrap_or_else(|| TableColumnLayout::equal(runtime.header.len()));
    let preview_marker = block.table_axis_preview;
    let selected_marker = block.table_axis_selection;
    let body_row_count = runtime.rows.len();
    let right_gutter = px(18.0);
    let bottom_gutter = px(18.0);
    let column_control_visible = block.table_append_column_hovered
        || block.table_append_column_zone_hovered
        || block.table_append_column_button_hovered;
    let row_control_visible = block.table_append_row_hovered
        || block.table_append_row_zone_hovered
        || block.table_append_row_button_hovered;
    let column_button_hovered = block.table_append_column_button_hovered;
    let row_button_hovered = block.table_append_row_button_hovered;
    let weak_table_block = cx.entity().downgrade();

    let header_cells = runtime.header;

    let header_hover_block = weak_table_block.clone();
    let header_select_block = weak_table_block.clone();
    let header_menu_block = weak_table_block.clone();
    let header_marker = TableAxisMarker {
        kind: TableAxisKind::Row,
        index: 0,
    };
    let is_header_selected = selected_marker == Some(header_marker);
    let is_header_preview = preview_marker == Some(header_marker);
    let show_header_handle = is_header_selected || is_header_preview;

    let column_count = header_cells.len();
    let header_row = div()
        .relative()
        .w_full()
        .flex()
        .gap(px(0.0))
        // Anytype Left Row Handle on left border
        .child(
            div()
                .id(ElementId::Name(
                    format!("table-header-axis-band-{}", block.record.id).into(),
                ))
                .absolute()
                .left(px(-6.0))
                .top_0()
                .h_full()
                .flex()
                .items_center()
                .cursor_pointer()
                .on_hover(move |hovered, _window, cx| {
                    let _ = header_hover_block.update(cx, |_block, cx| {
                        cx.emit(BlockAction::RequestTableAxisPreview {
                            kind: TableAxisKind::Row,
                            index: 0,
                            hovered: *hovered,
                        });
                    });
                })
                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                    let _ = header_select_block.update(cx, |_block, cx| {
                        cx.stop_propagation();
                        cx.emit(BlockAction::RequestSelectTableAxis {
                            kind: TableAxisKind::Row,
                            index: 0,
                        });
                    });
                })
                .on_mouse_down(MouseButton::Right, move |event, _window, cx| {
                    let _ = header_menu_block.update(cx, |_block, cx| {
                        cx.stop_propagation();
                        cx.emit(BlockAction::RequestOpenTableAxisMenu {
                            kind: TableAxisKind::Row,
                            index: 0,
                            position: event.position,
                        });
                    });
                })
                .block_mouse_except_scroll()
                .child(
                    div()
                        .w(px(d.table_handle_width))
                        .h(relative(0.60))
                        .rounded(px(2.0))
                        .bg(if is_header_selected {
                            c.table_selection_border
                        } else {
                            c.table_handle_bg
                        })
                        .opacity(if show_header_handle { 1.0 } else { 0.0 })
                        .hover(|this| this.opacity(1.0)),
                ),
        )
        .children(header_cells.into_iter().enumerate().map(|(column, cell)| {
            let hover_block = weak_table_block.clone();
            let select_block = weak_table_block.clone();
            let menu_block = weak_table_block.clone();
            let col_hover_block = weak_table_block.clone();
            let is_last_col = column == column_count - 1;
            let col_marker = TableAxisMarker {
                kind: TableAxisKind::Column,
                index: column,
            };
            let is_col_selected = selected_marker == Some(col_marker);
            let is_col_preview = preview_marker == Some(col_marker);
            let show_col_handle = is_col_selected || is_col_preview;

            div()
                .id(ElementId::Name(
                    format!("table-header-cell-wrap-{}-{}", block.record.id, column).into(),
                ))
                .relative()
                .flex_none()
                .flex_basis(relative(column_layout.fraction(column)))
                .w(relative(column_layout.fraction(column)))
                .h_full()
                .min_w(px(0.0))
                .on_hover(move |hovered, _window, cx| {
                    if is_last_col {
                        let _ = col_hover_block.update(cx, |block, cx| {
                            block.table_append_column_hovered = *hovered;
                            cx.notify();
                        });
                    }
                })
                // Anytype Top Column Handle on top border of column header
                .child(
                    div()
                        .id(ElementId::Name(
                            format!(
                                "table-column-axis-activation-{}-{}",
                                block.record.id, column
                            )
                            .into(),
                        ))
                        .absolute()
                        .top(px(-6.0))
                        .left_0()
                        .w_full()
                        .flex()
                        .justify_center()
                        .cursor_pointer()
                        .on_hover(move |hovered, _window, cx| {
                            let _ = hover_block.update(cx, |_block, cx| {
                                cx.emit(BlockAction::RequestTableAxisPreview {
                                    kind: TableAxisKind::Column,
                                    index: column,
                                    hovered: *hovered,
                                });
                            });
                        })
                        .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                            let _ = select_block.update(cx, |_block, cx| {
                                cx.stop_propagation();
                                cx.emit(BlockAction::RequestSelectTableAxis {
                                    kind: TableAxisKind::Column,
                                    index: column,
                                });
                            });
                        })
                        .on_mouse_down(MouseButton::Right, move |event, _window, cx| {
                            let _ = menu_block.update(cx, |_block, cx| {
                                cx.stop_propagation();
                                cx.emit(BlockAction::RequestOpenTableAxisMenu {
                                    kind: TableAxisKind::Column,
                                    index: column,
                                    position: event.position,
                                });
                            });
                        })
                        .block_mouse_except_scroll()
                        .child(
                            div()
                                .h(px(d.table_handle_width))
                                .w(relative(0.40))
                                .rounded(px(2.0))
                                .bg(if is_col_selected {
                                    c.table_selection_border
                                } else {
                                    c.table_handle_bg
                                })
                                .opacity(if show_col_handle { 1.0 } else { 0.0 })
                                .hover(|this| this.opacity(1.0)),
                        ),
                )
                .child(cell)
        }));

    let body_rows = runtime
        .rows
        .into_iter()
        .enumerate()
        .map(|(body_row_index, row)| {
            let hover_block = weak_table_block.clone();
            let select_block = weak_table_block.clone();
            let menu_block = weak_table_block.clone();
            let row_hover_block = weak_table_block.clone();
            let is_last_body_row = body_row_index == body_row_count - 1;
            let visual_row = body_row_index + 1;
            let marker = TableAxisMarker {
                kind: TableAxisKind::Row,
                index: visual_row,
            };
            let is_row_selected = selected_marker == Some(marker);
            let is_row_preview = preview_marker == Some(marker);
            let show_row_handle = is_row_selected || is_row_preview;

            div()
                .id(ElementId::Name(
                    format!("table-body-row-wrap-{}-{}", block.record.id, body_row_index).into(),
                ))
                .relative()
                .w_full()
                .flex()
                .gap(px(0.0))
                .on_hover(move |hovered, _window, cx| {
                    if is_last_body_row {
                        let _ = row_hover_block.update(cx, |block, cx| {
                            block.table_append_row_hovered = *hovered;
                            cx.notify();
                        });
                    }
                })
                // Anytype Left Row Handle on left border
                .child(
                    div()
                        .id(ElementId::Name(
                            format!("table-row-axis-band-{}-{}", block.record.id, body_row_index)
                                .into(),
                        ))
                        .absolute()
                        .left(px(-6.0))
                        .top_0()
                        .h_full()
                        .flex()
                        .items_center()
                        .cursor_pointer()
                        .on_hover(move |hovered, _window, cx| {
                            let _ = hover_block.update(cx, |_block, cx| {
                                cx.emit(BlockAction::RequestTableAxisPreview {
                                    kind: TableAxisKind::Row,
                                    index: visual_row,
                                    hovered: *hovered,
                                });
                            });
                        })
                        .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                            let _ = select_block.update(cx, |_block, cx| {
                                cx.stop_propagation();
                                cx.emit(BlockAction::RequestSelectTableAxis {
                                    kind: TableAxisKind::Row,
                                    index: visual_row,
                                });
                            });
                        })
                        .on_mouse_down(MouseButton::Right, move |event, _window, cx| {
                            let _ = menu_block.update(cx, |_block, cx| {
                                cx.stop_propagation();
                                cx.emit(BlockAction::RequestOpenTableAxisMenu {
                                    kind: TableAxisKind::Row,
                                    index: visual_row,
                                    position: event.position,
                                });
                            });
                        })
                        .block_mouse_except_scroll()
                        .child(
                            div()
                                .w(px(d.table_handle_width))
                                .h(relative(0.60))
                                .rounded(px(2.0))
                                .bg(if is_row_selected {
                                    c.table_selection_border
                                } else {
                                    c.table_handle_bg
                                })
                                .opacity(if show_row_handle { 1.0 } else { 0.0 })
                                .hover(|this| this.opacity(1.0)),
                        ),
                )
                .children(row.into_iter().enumerate().map(|(column, cell)| {
                    let col_hover_block = weak_table_block.clone();
                    let is_last_col = column == column_count - 1;

                    div()
                        .id(ElementId::Name(
                            format!(
                                "table-body-cell-wrap-{}-{}-{}",
                                block.record.id, body_row_index, column
                            )
                            .into(),
                        ))
                        .flex_none()
                        .flex_basis(relative(column_layout.fraction(column)))
                        .w(relative(column_layout.fraction(column)))
                        .h_full()
                        .min_w(px(0.0))
                        .on_hover(move |hovered, _window, cx| {
                            if is_last_col {
                                let _ = col_hover_block.update(cx, |block, cx| {
                                    block.table_append_column_hovered = *hovered;
                                    cx.notify();
                                });
                            }
                        })
                        .child(cell)
                }))
        });

    let block_id = ElementId::Name(format!("block-{}", block.record.id).into());

    {
        let mut rows = Vec::with_capacity(1 + body_row_count);
        rows.push(header_row.into_any_element());
        rows.extend(body_rows.map(|row| row.into_any_element()));

        let column_edge_band = div()
            .id(ElementId::Name(
                format!("table-append-column-edge-{}", block.record.id).into(),
            ))
            .absolute()
            .top_0()
            .bottom_0()
            .right(px(-18.0))
            .w(px(18.0))
            .on_hover(cx.listener(Block::on_table_append_column_edge_hover));

        let row_edge_band = div()
            .id(ElementId::Name(
                format!("table-append-row-edge-{}", block.record.id).into(),
            ))
            .absolute()
            .left_0()
            .right_0()
            .bottom(px(-18.0))
            .h(px(18.0))
            .on_hover(cx.listener(Block::on_table_append_row_edge_hover));

        let column_control = div()
            .id(ElementId::Name(
                format!("table-append-column-zone-{}", block.record.id).into(),
            ))
            .absolute()
            .top_0()
            .bottom_0()
            .right(px(-18.0))
            .w(px(18.0))
            .on_hover(cx.listener(Block::on_table_append_column_zone_hover))
            .child(
                div()
                    .id(ElementId::Name(
                        format!("table-append-column-button-{}", block.record.id).into(),
                    ))
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .right_0()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(0.0))
                    .border(px(1.0))
                    .border_color(if column_button_hovered {
                        c.table_append_button_hover
                    } else {
                        c.table_border
                    })
                    .bg(if column_button_hovered {
                        c.table_append_button_hover
                    } else {
                        gpui::transparent_black()
                    })
                    .cursor_pointer()
                    .opacity(if column_control_visible { 1.0 } else { 0.0 })
                    .block_mouse_except_scroll()
                    .on_hover(cx.listener(Block::on_table_append_column_button_hover))
                    .on_click(cx.listener(Block::on_append_table_column))
                    .child(svg().path("icon/table/plus.svg").size(px(10.0)).text_color(
                        if column_button_hovered {
                            c.table_append_button_text
                        } else {
                            c.table_border
                        },
                    )),
            );

        let row_control = div()
            .id(ElementId::Name(
                format!("table-append-row-zone-{}", block.record.id).into(),
            ))
            .absolute()
            .left_0()
            .right_0()
            .bottom(px(-18.0))
            .h(px(18.0))
            .on_hover(cx.listener(Block::on_table_append_row_zone_hover))
            .child(
                div()
                    .id(ElementId::Name(
                        format!("table-append-row-button-{}", block.record.id).into(),
                    ))
                    .absolute()
                    .left_0()
                    .right_0()
                    .bottom_0()
                    .h_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(0.0))
                    .border(px(1.0))
                    .border_color(if row_button_hovered {
                        c.table_append_button_hover
                    } else {
                        c.table_border
                    })
                    .bg(if row_button_hovered {
                        c.table_append_button_hover
                    } else {
                        gpui::transparent_black()
                    })
                    .cursor_pointer()
                    .opacity(if row_control_visible { 1.0 } else { 0.0 })
                    .block_mouse_except_scroll()
                    .on_hover(cx.listener(Block::on_table_append_row_button_hover))
                    .on_click(cx.listener(Block::on_append_table_row))
                    .child(svg().path("icon/table/plus.svg").size(px(10.0)).text_color(
                        if row_button_hovered {
                            c.table_append_button_text
                        } else {
                            c.table_border
                        },
                    )),
            );

        let expand_control = div()
            .id(ElementId::Name(
                format!("table-expand-button-{}", block.record.id).into(),
            ))
            .absolute()
            .right(px(-18.0))
            .bottom(px(-18.0))
            .w(px(18.0))
            .h(px(18.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(0.0))
            .border(px(1.0))
            .border_color(c.table_border)
            .bg(gpui::transparent_black())
            .hover(|this| {
                this.bg(c.table_append_button_hover)
                    .border_color(c.table_append_button_hover)
            })
            .cursor_pointer()
            .opacity(if column_control_visible && row_control_visible {
                1.0
            } else {
                0.0
            })
            .block_mouse_except_scroll()
            .on_click(cx.listener(Block::on_expand_table))
            .child(
                svg()
                    .path("icon/table/plus.svg")
                    .size(px(10.0))
                    .text_color(c.table_border),
            );

        let table_grid = div()
            .relative()
            .w_full()
            .flex()
            .flex_col()
            .children(rows)
            .child(column_edge_band)
            .child(row_edge_band)
            .child(column_control)
            .child(row_control)
            .child(expand_control);

        div()
            .id(block_id)
            .w_full()
            .relative()
            .flex()
            .flex_col()
            .pt(px(10.0))
            .pl(px(10.0))
            .pr(right_gutter)
            .pb(bottom_gutter)
            .gap(px(0.0))
            .child(table_grid)
            .into_any_element()
    }
}

// ── TableGrid & Block cell helpers ───────────────────────────────────────────

/// Runtime cell editors attached to one native table block.
#[derive(Clone)]
pub struct TableGrid {
    pub header: Vec<Entity<Block>>,
    pub rows: Vec<Vec<Entity<Block>>>,
}

impl TableGrid {
    pub fn cell(&self, position: TableCellPosition) -> Option<Entity<Block>> {
        if position.is_header() {
            self.header.get(position.column).cloned()
        } else {
            self.rows
                .get(position.body_row_index()?)
                .and_then(|row| row.get(position.column))
                .cloned()
        }
    }
}

impl Block {
    pub(crate) fn is_table_cell(&self) -> bool {
        self.table_cell_position.is_some()
    }

    pub(crate) fn table_cell_position(&self) -> Option<TableCellPosition> {
        self.table_cell_position
    }

    pub(crate) fn table_cell_alignment(&self) -> Option<TableColumnAlignment> {
        self.table_cell_alignment
    }

    pub(crate) fn text_align(&self) -> TextAlign {
        match self
            .table_cell_alignment()
            .unwrap_or(TableColumnAlignment::Default)
        {
            TableColumnAlignment::Default | TableColumnAlignment::Left => TextAlign::Left,
            TableColumnAlignment::Center => TextAlign::Center,
            TableColumnAlignment::Right => TextAlign::Right,
        }
    }

    pub(crate) fn set_table_cell_mode(
        &mut self,
        position: TableCellPosition,
        alignment: TableColumnAlignment,
    ) {
        self.table_cell_position = Some(position);
        self.table_cell_alignment = Some(alignment);
        self.edit_mode = EditMode::RenderedRich;
        self.clear_inline_projection();
        self.sync_render_cache();
    }

    pub(crate) fn set_table_runtime(&mut self, runtime: TableGrid) {
        self.table_runtime = Some(runtime);
    }

    pub(crate) fn clear_table_runtime(&mut self) {
        self.table_runtime = None;
        self.table_axis_preview = None;
        self.table_axis_selection = None;
        self.table_axis_highlight = TableAxisHighlight::None;
        self.table_append_column_edge_hovered = false;
        self.table_append_column_hovered = false;
        self.table_append_column_zone_hovered = false;
        self.table_append_column_button_hovered = false;
        self.table_append_column_close_task = None;
        self.table_append_row_edge_hovered = false;
        self.table_append_row_hovered = false;
        self.table_append_row_zone_hovered = false;
        self.table_append_row_button_hovered = false;
        self.table_append_row_close_task = None;
        self.table_hovered_row = None;
        self.table_hovered_column = None;
    }

    pub(crate) fn set_table_axis_visual_state(
        &mut self,
        preview: Option<TableAxisMarker>,
        selection: Option<TableAxisMarker>,
    ) {
        self.table_axis_preview = preview;
        self.table_axis_selection = selection;
    }

    pub(crate) fn set_table_axis_highlight(&mut self, highlight: TableAxisHighlight) {
        self.table_axis_highlight = highlight;
    }
}
