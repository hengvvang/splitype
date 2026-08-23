use gpui::*;

use crate::editor::block_protocol::BlockEvent;
use crate::editor::editing::table_grid::TableGrid;
use crate::editor::geometry::table_measure::measure_table_column_layout;
use crate::editor::tree::block::Block;
use crate::editor::tree::block_edit_mode::BlockEditMode;
use crate::editor::wysiwyg::render::effective_table_width;
use crate::infra::theme::Theme;
use crate::model::block::table::TableCellPosition;
use crate::model::block::table::TableColumnAlignment;
use crate::model::block::table::{TableAxis, TableAxisMarker, TableColumnLayout};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DraggedTableAxis {
    pub(crate) table_block_id: EntityId,
    pub(crate) kind: TableAxis,
    pub(crate) index: usize,
}

pub(crate) struct DraggedTableAxisView {
    pub(crate) theme: Theme,
    pub(crate) kind: TableAxis,
    pub(crate) cells: Vec<String>,
    pub(crate) width: Pixels,
    pub(crate) col_widths: Vec<Pixels>,
    pub(crate) cell_heights: Vec<Option<Pixels>>,
}

impl Render for DraggedTableAxisView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let c = &self.theme.colors;
        let d = &self.theme.dimensions;
        let t = &self.theme.typography;
        match self.kind {
            TableAxis::Column => div()
                .flex()
                .flex_col()
                .w(self.width)
                .bg(c.table_cell_bg)
                .border(px(2.0))
                .border_color(c.table_selection_border)
                .rounded(px(4.0))
                .shadow_lg()
                .opacity(0.95)
                .children(self.cells.iter().enumerate().map(|(i, text)| {
                    let mut cell_div = div()
                        .w_full()
                        .px(px(d.table_cell_padding_x))
                        .py(px(d.table_cell_padding_y))
                        .border_b(if i + 1 < self.cells.len() {
                            px(1.0)
                        } else {
                            px(0.0)
                        })
                        .border_color(c.table_border)
                        .bg(if i == 0 {
                            c.table_header_bg
                        } else {
                            c.table_cell_bg
                        })
                        .text_size(px(t.text_size))
                        .text_color(c.text_default)
                        .line_height(rems(t.text_line_height))
                        .font_weight(if i == 0 {
                            FontWeight::MEDIUM
                        } else {
                            FontWeight::NORMAL
                        })
                        .flex()
                        .items_center()
                        .child(if text.is_empty() { " " } else { text.as_str() }.to_string());
                    if let Some(Some(h)) = self.cell_heights.get(i) {
                        cell_div = cell_div.h(*h);
                    } else {
                        cell_div = cell_div.min_h(px(d.table_cell_min_height));
                    }
                    cell_div
                })),
            TableAxis::Row => div()
                .flex()
                .flex_row()
                .w(self.width)
                .bg(c.table_cell_bg)
                .border(px(2.0))
                .border_color(c.table_selection_border)
                .rounded(px(4.0))
                .shadow_lg()
                .opacity(0.95)
                .children(self.cells.iter().enumerate().map(|(i, text)| {
                    let cell_w = self.col_widths.get(i).copied().unwrap_or(px(100.0));
                    let mut cell_div = div()
                        .w(cell_w)
                        .px(px(d.table_cell_padding_x))
                        .py(px(d.table_cell_padding_y))
                        .border_r(if i + 1 < self.cells.len() {
                            px(1.0)
                        } else {
                            px(0.0)
                        })
                        .border_color(c.table_border)
                        .bg(c.table_cell_bg)
                        .text_size(px(t.text_size))
                        .text_color(c.text_default)
                        .line_height(rems(t.text_line_height))
                        .flex()
                        .items_center()
                        .child(if text.is_empty() { " " } else { text.as_str() }.to_string());
                    if let Some(Some(h)) = self.cell_heights.first() {
                        cell_div = cell_div.h(*h);
                    } else {
                        cell_div = cell_div.min_h(px(d.table_cell_min_height));
                    }
                    cell_div
                })),
        }
    }
}

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

    let Some(runtime) = block.table_grid.clone() else {
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
        .data
        .table
        .as_ref()
        .map(|table| measure_table_column_layout(table, table_width, window, theme))
        .unwrap_or_else(|| TableColumnLayout::equal(runtime.header.len()));
    let body_row_count = runtime.rows.len();
    let column_count = runtime.header.len();
    let column_control_visible = block.table_interaction.column_append.is_visible();
    let row_control_visible = block.table_interaction.row_append.is_visible();
    let column_button_hovered = block.table_interaction.column_append.button_hovered;
    let row_button_hovered = block.table_interaction.row_append.button_hovered;
    let block_entity_id = cx.entity().entity_id();
    let weak_table_block = cx.entity().downgrade();

    let exact_col_widths: Vec<Pixels> = (0..column_count)
        .map(|col| {
            runtime
                .header
                .get(col)
                .and_then(|cell| cell.read(cx).last_paint().map(|p| p.bounds.size.width))
                .unwrap_or_else(|| px(table_width * column_layout.fraction(col)))
        })
        .collect();

    let header_row_height: Option<Pixels> = runtime
        .header
        .first()
        .and_then(|cell| cell.read(cx).last_paint().map(|p| p.bounds.size.height));

    let body_row_heights: Vec<Option<Pixels>> = runtime
        .rows
        .iter()
        .map(|row| {
            row.first()
                .and_then(|cell| cell.read(cx).last_paint().map(|p| p.bounds.size.height))
        })
        .collect();

    let all_column_cell_heights: Vec<Vec<Option<Pixels>>> = (0..column_count)
        .map(|col| {
            let mut heights = Vec::with_capacity(1 + body_row_count);
            heights.push(
                runtime
                    .header
                    .get(col)
                    .and_then(|cell| cell.read(cx).last_paint().map(|p| p.bounds.size.height)),
            );
            for row in &runtime.rows {
                heights.push(
                    row.get(col)
                        .and_then(|cell| cell.read(cx).last_paint().map(|p| p.bounds.size.height)),
                );
            }
            heights
        })
        .collect();

    let col_widths = exact_col_widths.clone();

    let active_cell = {
        let mut active = None;
        for (col_idx, cell) in runtime.header.iter().enumerate() {
            if cell.read(cx).focus_handle.is_focused(window) {
                active = Some((0, col_idx));
                break;
            }
        }
        if active.is_none() {
            'outer: for (row_idx, row) in runtime.rows.iter().enumerate() {
                for (col_idx, cell) in row.iter().enumerate() {
                    if cell.read(cx).focus_handle.is_focused(window) {
                        active = Some((row_idx + 1, col_idx));
                        break 'outer;
                    }
                }
            }
        }
        active
    };

    let header_cell_texts: Vec<String> = runtime
        .header
        .iter()
        .map(|c| c.read(cx).display_text().to_string())
        .collect();

    let body_cell_texts: Vec<Vec<String>> = runtime
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|c| c.read(cx).display_text().to_string())
                .collect()
        })
        .collect();

    let header_cells = runtime.header;
    let header_select_block = weak_table_block.clone();
    let header_menu_block = weak_table_block.clone();
    let header_drop_block = weak_table_block.clone();
    let header_drag_move_block = weak_table_block.clone();
    let header_axis_hover_block = weak_table_block.clone();

    let is_header_selected = block.table_axis_selection
        == Some(TableAxisMarker {
            kind: TableAxis::Row,
            index: 0,
        });
    let header_selection_overlay = if is_header_selected {
        Some(
            div()
                .absolute()
                .inset_0()
                .border(px(2.0))
                .border_color(c.table_selection_border),
        )
    } else {
        None
    };

    let header_indicator_line = if let Some(prev) = block.table_axis_preview {
        if prev.kind == TableAxis::Row && prev.index == 0 {
            Some(
                div()
                    .absolute()
                    .top(px(-2.0))
                    .left_0()
                    .right_0()
                    .h(px(3.0))
                    .rounded(px(1.5))
                    .bg(c.table_selection_border),
            )
        } else {
            None
        }
    } else {
        None
    };

    let is_header_hovered = block.table_interaction.hovered_row == Some(0);
    let is_header_editing = active_cell.map(|(r, _)| r) == Some(0);

    let header_left_border_color = if is_header_hovered {
        c.table_handle_icon
    } else if is_header_editing {
        Hsla::from(rgba(0x22c55eff))
    } else {
        c.table_border
    };

    let header_axis_row_texts = header_cell_texts.clone();
    let header_axis_theme = theme.clone();
    let header_axis_col_widths = col_widths.clone();
    let header_axis_total_width = header_axis_col_widths
        .iter()
        .fold(px(0.0), |acc, w| acc + *w);

    let header_axis_band = div()
        .id(ElementId::Name(
            format!("table-header-axis-band-{}", block.data.id).into(),
        ))
        .absolute()
        .left(px(-10.0))
        .w(px(10.0))
        .top_0()
        .h_full()
        .cursor(CursorStyle::ResizeUpDown)
        .on_hover(move |hovered, _window, cx| {
            let _ = header_axis_hover_block.update(cx, |block, cx| {
                if *hovered {
                    block.table_interaction.hovered_row = Some(0);
                } else if block.table_interaction.hovered_row == Some(0) {
                    block.table_interaction.hovered_row = None;
                }
                cx.notify();
            });
        })
        .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
            let _ = header_select_block.update(cx, |_block, cx| {
                cx.emit(BlockEvent::RequestSelectTableAxis {
                    kind: TableAxis::Row,
                    index: 0,
                });
            });
        })
        .on_mouse_down(MouseButton::Right, move |event, _window, cx| {
            let _ = header_menu_block.update(cx, |_block, cx| {
                cx.stop_propagation();
                cx.emit(BlockEvent::RequestOpenTableAxisMenu {
                    kind: TableAxis::Row,
                    index: 0,
                    position: event.position,
                });
            });
        })
        .on_drag(
            DraggedTableAxis {
                table_block_id: block_entity_id,
                kind: TableAxis::Row,
                index: 0,
            },
            move |_drag, _point, _window, cx| {
                let cells = header_axis_row_texts.clone();
                let theme = header_axis_theme.clone();
                let col_widths = header_axis_col_widths.clone();
                cx.new(|_| DraggedTableAxisView {
                    theme,
                    kind: TableAxis::Row,
                    cells,
                    width: header_axis_total_width,
                    col_widths,
                    cell_heights: vec![header_row_height],
                })
            },
        );

    let header_row = div()
        .relative()
        .w_full()
        .flex()
        .gap(px(0.0))
        .border_l(px(2.0))
        .border_color(header_left_border_color)
        .child(header_axis_band)
        .on_drag_move::<DraggedTableAxis>(move |drag, _window, cx| {
            let kind = drag.drag(cx).kind;
            let from = drag.drag(cx).index;
            let _ = header_drag_move_block.update(cx, |_block, cx| {
                if kind == TableAxis::Row {
                    if from == 0 {
                        cx.emit(BlockEvent::RequestTableAxisPreview {
                            kind: TableAxis::Row,
                            index: 0,
                            hovered: false,
                        });
                    } else {
                        cx.emit(BlockEvent::RequestTableAxisPreview {
                            kind: TableAxis::Row,
                            index: 0,
                            hovered: true,
                        });
                    }
                }
            });
        })
        .on_drop::<DraggedTableAxis>(move |drag, _window, cx| {
            if drag.table_block_id == block_entity_id
                && drag.kind == TableAxis::Row
                && drag.index != 0
            {
                let _ = header_drop_block.update(cx, |_block, cx| {
                    cx.emit(BlockEvent::RequestReorderTableAxis {
                        kind: TableAxis::Row,
                        from: drag.index,
                        to: 0,
                    });
                });
            }
        })
        .children(header_cells.into_iter().enumerate().map(|(column, cell)| {
            let select_block = weak_table_block.clone();
            let menu_block = weak_table_block.clone();
            let drop_block = weak_table_block.clone();
            let drop_drag_block = weak_table_block.clone();
            let col_hover_block = weak_table_block.clone();
            let cell_hover_block = weak_table_block.clone();
            let col_axis_hover_block = weak_table_block.clone();
            let is_last_col = column == column_count - 1;

            let is_col_hovered = block.table_interaction.hovered_column == Some(column);
            let is_col_editing = active_cell.map(|(_, col)| col) == Some(column);

            let col_top_border_color = if is_col_hovered {
                c.table_handle_icon
            } else if is_col_editing {
                Hsla::from(rgba(0x22c55eff))
            } else {
                c.table_border
            };

            let mut column_cell_texts = Vec::with_capacity(1 + body_row_count);
            if let Some(h) = header_cell_texts.get(column) {
                column_cell_texts.push(h.clone());
            }
            for r in &body_cell_texts {
                if let Some(c) = r.get(column) {
                    column_cell_texts.push(c.clone());
                }
            }
            let col_axis_theme = theme.clone();
            let col_axis_col_widths = col_widths.clone();
            let col_axis_cell_heights = all_column_cell_heights
                .get(column)
                .cloned()
                .unwrap_or_default();
            let col_width = col_widths.get(column).copied().unwrap_or(px(100.0));

            div()
                .id(ElementId::Name(
                    format!("table-header-cell-wrap-{}-{}", block.data.id, column).into(),
                ))
                .relative()
                .flex_none()
                .flex_basis(relative(column_layout.fraction(column)))
                .w(relative(column_layout.fraction(column)))
                .h_full()
                .min_w(px(0.0))
                .border_t(px(2.0))
                .border_color(col_top_border_color)
                .on_hover(move |hovered, _window, cx| {
                    if is_last_col {
                        let _ = col_hover_block.update(cx, |block, cx| {
                            block.table_interaction.column_append.is_active = *hovered;
                            cx.notify();
                        });
                    }
                    let _ = cell_hover_block.update(cx, |block, cx| {
                        if *hovered {
                            block.table_interaction.hovered_row = Some(0);
                            block.table_interaction.hovered_column = Some(column);
                        } else if block.table_interaction.hovered_row == Some(0)
                            && block.table_interaction.hovered_column == Some(column)
                        {
                            block.table_interaction.hovered_row = None;
                            block.table_interaction.hovered_column = None;
                        }
                        cx.notify();
                    });
                })
                .on_drag_move::<DraggedTableAxis>(move |drag, _window, cx| {
                    let kind = drag.drag(cx).kind;
                    let from = drag.drag(cx).index;
                    let _ = drop_drag_block.update(cx, |_block, cx| {
                        match kind {
                            TableAxis::Column => {
                                if from == column {
                                    cx.emit(BlockEvent::RequestTableAxisPreview {
                                        kind: TableAxis::Column,
                                        index: from,
                                        hovered: false,
                                    });
                                } else {
                                    let target_slot = if from > column {
                                        column
                                    } else {
                                        column + 1
                                    };
                                    cx.emit(BlockEvent::RequestTableAxisPreview {
                                        kind: TableAxis::Column,
                                        index: target_slot,
                                        hovered: true,
                                    });
                                }
                            }
                            TableAxis::Row => {
                                if from == 0 {
                                    cx.emit(BlockEvent::RequestTableAxisPreview {
                                        kind: TableAxis::Row,
                                        index: 0,
                                        hovered: false,
                                    });
                                } else {
                                    cx.emit(BlockEvent::RequestTableAxisPreview {
                                        kind: TableAxis::Row,
                                        index: 0,
                                        hovered: true,
                                    });
                                }
                            }
                        }
                    });
                })
                .on_drop::<DraggedTableAxis>(move |drag, _window, cx| {
                    if drag.table_block_id == block_entity_id {
                        match drag.kind {
                            TableAxis::Column => {
                                if drag.index != column {
                                    let _ = drop_block.update(cx, |_block, cx| {
                                        cx.emit(BlockEvent::RequestReorderTableAxis {
                                            kind: TableAxis::Column,
                                            from: drag.index,
                                            to: column,
                                        });
                                    });
                                }
                            }
                            TableAxis::Row => {
                                if drag.index != 0 {
                                    let _ = drop_block.update(cx, |_block, cx| {
                                        cx.emit(BlockEvent::RequestReorderTableAxis {
                                            kind: TableAxis::Row,
                                            from: drag.index,
                                            to: 0,
                                        });
                                    });
                                }
                            }
                        }
                    }
                })
                // Top Column Edge Interaction (Indicator bar, ResizeLeftRight cursor, drag/drop, left-click select, right-click menu)
                .child(
                    div()
                        .id(ElementId::Name(
                            format!("table-column-axis-activation-{}-{}", block.data.id, column)
                                .into(),
                        ))
                        .absolute()
                        .top(px(-10.0))
                        .h(px(10.0))
                        .left_0()
                        .w_full()
                        .cursor(CursorStyle::ResizeLeftRight)
                        .on_hover(move |hovered, _window, cx| {
                            let _ = col_axis_hover_block.update(cx, |block, cx| {
                                if *hovered {
                                    block.table_interaction.hovered_column = Some(column);
                                } else if block.table_interaction.hovered_column == Some(column) {
                                    block.table_interaction.hovered_column = None;
                                }
                                cx.notify();
                            });
                        })
                        .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                            let _ = select_block.update(cx, |_block, cx| {
                                cx.emit(BlockEvent::RequestSelectTableAxis {
                                    kind: TableAxis::Column,
                                    index: column,
                                });
                            });
                        })
                        .on_mouse_down(MouseButton::Right, move |event, _window, cx| {
                            let _ = menu_block.update(cx, |_block, cx| {
                                cx.stop_propagation();
                                cx.emit(BlockEvent::RequestOpenTableAxisMenu {
                                    kind: TableAxis::Column,
                                    index: column,
                                    position: event.position,
                                });
                            });
                        })
                        .on_drag(
                            DraggedTableAxis {
                                table_block_id: block_entity_id,
                                kind: TableAxis::Column,
                                index: column,
                            },
                            move |_drag, _point, _window, cx| {
                                let cells = column_cell_texts.clone();
                                let theme = col_axis_theme.clone();
                                let col_widths = col_axis_col_widths.clone();
                                let cell_heights = col_axis_cell_heights.clone();
                                cx.new(|_| DraggedTableAxisView {
                                    theme,
                                    kind: TableAxis::Column,
                                    cells,
                                    width: col_width,
                                    col_widths,
                                    cell_heights,
                                })
                            },
                        ),
                )
                .child(cell)
        }))
        .children(header_selection_overlay)
        .children(header_indicator_line);

    let body_rows = runtime
        .rows
        .into_iter()
        .enumerate()
        .map(|(body_row_index, row)| {
            let select_block = weak_table_block.clone();
            let menu_block = weak_table_block.clone();
            let drop_block = weak_table_block.clone();
            let row_drag_move_block = weak_table_block.clone();
            let row_hover_block = weak_table_block.clone();
            let row_axis_hover_block = weak_table_block.clone();
            let is_last_body_row = body_row_index == body_row_count - 1;
            let visual_row = body_row_index + 1;

            let is_row_selected = block.table_axis_selection
                == Some(TableAxisMarker {
                    kind: TableAxis::Row,
                    index: visual_row,
                });
            let row_selection_overlay = if is_row_selected {
                Some(
                    div()
                        .absolute()
                        .inset_0()
                        .border(px(2.0))
                        .border_color(c.table_selection_border),
                )
            } else {
                None
            };

            let row_indicator_line = if let Some(prev) = block.table_axis_preview {
                if prev.kind == TableAxis::Row {
                    if prev.index == visual_row {
                        Some(
                            div()
                                .absolute()
                                .top(px(-1.5))
                                .left_0()
                                .right_0()
                                .h(px(3.0))
                                .rounded(px(1.5))
                                .bg(c.table_selection_border),
                        )
                    } else if is_last_body_row && prev.index == visual_row + 1 {
                        Some(
                            div()
                                .absolute()
                                .bottom(px(-1.5))
                                .left_0()
                                .right_0()
                                .h(px(3.0))
                                .rounded(px(1.5))
                                .bg(c.table_selection_border),
                        )
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            let is_row_hovered = block.table_interaction.hovered_row == Some(visual_row);
            let is_row_editing = active_cell.map(|(r, _)| r) == Some(visual_row);

            let row_left_border_color = if is_row_hovered {
                c.table_handle_icon
            } else if is_row_editing {
                Hsla::from(rgba(0x22c55eff))
            } else {
                c.table_border
            };

            let row_axis_texts = body_cell_texts
                .get(body_row_index)
                .cloned()
                .unwrap_or_default();
            let row_axis_theme = theme.clone();
            let row_axis_col_widths = col_widths.clone();
            let row_axis_total_width = row_axis_col_widths
                .iter()
                .fold(px(0.0), |acc, w| acc + *w);
            let row_axis_cell_heights = vec![body_row_heights.get(body_row_index).copied().flatten()];

            let row_axis_band = div()
                .id(ElementId::Name(
                    format!("table-row-axis-band-{}-{}", block.data.id, body_row_index)
                        .into(),
                ))
                .absolute()
                .left(px(-10.0))
                .w(px(10.0))
                .top_0()
                .h_full()
                .cursor(CursorStyle::ResizeUpDown)
                .on_hover(move |hovered, _window, cx| {
                    let _ = row_axis_hover_block.update(cx, |block, cx| {
                        if *hovered {
                            block.table_interaction.hovered_row = Some(visual_row);
                        } else if block.table_interaction.hovered_row == Some(visual_row) {
                            block.table_interaction.hovered_row = None;
                        }
                        cx.notify();
                    });
                })
                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                    let _ = select_block.update(cx, |_block, cx| {
                        cx.emit(BlockEvent::RequestSelectTableAxis {
                            kind: TableAxis::Row,
                            index: visual_row,
                        });
                    });
                })
                .on_mouse_down(MouseButton::Right, move |event, _window, cx| {
                    let _ = menu_block.update(cx, |_block, cx| {
                        cx.stop_propagation();
                        cx.emit(BlockEvent::RequestOpenTableAxisMenu {
                            kind: TableAxis::Row,
                            index: visual_row,
                            position: event.position,
                        });
                    });
                })
                .on_drag(
                    DraggedTableAxis {
                        table_block_id: block_entity_id,
                        kind: TableAxis::Row,
                        index: visual_row,
                    },
                    move |_drag, _point, _window, cx| {
                        let cells = row_axis_texts.clone();
                        let theme = row_axis_theme.clone();
                        let col_widths = row_axis_col_widths.clone();
                        let cell_heights = row_axis_cell_heights.clone();
                        cx.new(|_| DraggedTableAxisView {
                            theme,
                            kind: TableAxis::Row,
                            cells,
                            width: row_axis_total_width,
                            col_widths,
                            cell_heights,
                        })
                    },
                );

            div()
                .id(ElementId::Name(
                    format!("table-body-row-wrap-{}-{}", block.data.id, body_row_index).into(),
                ))
                .relative()
                .w_full()
                .flex()
                .gap(px(0.0))
                .border_l(px(2.0))
                .border_color(row_left_border_color)
                .child(row_axis_band)
                .on_hover(move |hovered, _window, cx| {
                    if is_last_body_row {
                        let _ = row_hover_block.update(cx, |block, cx| {
                            block.table_interaction.row_append.is_active = *hovered;
                            cx.notify();
                        });
                    }
                })
                .on_drag_move::<DraggedTableAxis>(move |drag, _window, cx| {
                    let kind = drag.drag(cx).kind;
                    let from = drag.drag(cx).index;
                    let _ = row_drag_move_block.update(cx, |_block, cx| {
                        if kind == TableAxis::Row {
                            if from == visual_row {
                                cx.emit(BlockEvent::RequestTableAxisPreview {
                                    kind: TableAxis::Row,
                                    index: visual_row,
                                    hovered: false,
                                });
                            } else {
                                let target_slot = if from > visual_row {
                                    visual_row
                                } else {
                                    visual_row + 1
                                };
                                cx.emit(BlockEvent::RequestTableAxisPreview {
                                    kind: TableAxis::Row,
                                    index: target_slot,
                                    hovered: true,
                                });
                            }
                        }
                    });
                })
                .on_drop::<DraggedTableAxis>(move |drag, _window, cx| {
                    if drag.table_block_id == block_entity_id
                        && drag.kind == TableAxis::Row
                        && drag.index != visual_row
                    {
                        let _ = drop_block.update(cx, |_block, cx| {
                            cx.emit(BlockEvent::RequestReorderTableAxis {
                                kind: TableAxis::Row,
                                from: drag.index,
                                to: visual_row,
                            });
                        });
                    }
                })
                .children(row.into_iter().enumerate().map(|(column, cell)| {
                    let col_hover_block = weak_table_block.clone();
                    let cell_hover_block = weak_table_block.clone();
                    let cell_drop_block = weak_table_block.clone();
                    let cell_drop_drag_block = weak_table_block.clone();
                    let is_last_col = column == column_count - 1;

                    div()
                        .id(ElementId::Name(
                            format!(
                                "table-body-cell-wrap-{}-{}-{}",
                                block.data.id, body_row_index, column
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
                                    block.table_interaction.column_append.is_active = *hovered;
                                    cx.notify();
                                });
                            }
                            let _ = cell_hover_block.update(cx, |block, cx| {
                                if *hovered {
                                    block.table_interaction.hovered_row = Some(visual_row);
                                    block.table_interaction.hovered_column = Some(column);
                                } else if block.table_interaction.hovered_row == Some(visual_row)
                                    && block.table_interaction.hovered_column == Some(column)
                                {
                                    block.table_interaction.hovered_row = None;
                                    block.table_interaction.hovered_column = None;
                                }
                                cx.notify();
                            });
                        })
                        .on_drag_move::<DraggedTableAxis>(move |drag, _window, cx| {
                            let kind = drag.drag(cx).kind;
                            let from = drag.drag(cx).index;
                            let _ = cell_drop_drag_block.update(cx, |_block, cx| {
                                match kind {
                                    TableAxis::Column => {
                                        if from == column {
                                            cx.emit(BlockEvent::RequestTableAxisPreview {
                                                kind: TableAxis::Column,
                                                index: from,
                                                hovered: false,
                                            });
                                        } else {
                                            let target_slot = if from > column {
                                                column
                                            } else {
                                                column + 1
                                            };
                                            cx.emit(BlockEvent::RequestTableAxisPreview {
                                                kind: TableAxis::Column,
                                                index: target_slot,
                                                hovered: true,
                                            });
                                        }
                                    }
                                    TableAxis::Row => {
                                        if from == visual_row {
                                            cx.emit(BlockEvent::RequestTableAxisPreview {
                                                kind: TableAxis::Row,
                                                index: visual_row,
                                                hovered: false,
                                            });
                                        } else {
                                            let target_slot = if from > visual_row {
                                                visual_row
                                            } else {
                                                visual_row + 1
                                            };
                                            cx.emit(BlockEvent::RequestTableAxisPreview {
                                                kind: TableAxis::Row,
                                                index: target_slot,
                                                hovered: true,
                                            });
                                        }
                                    }
                                }
                            });
                        })
                        .on_drop::<DraggedTableAxis>(move |drag, _window, cx| {
                            if drag.table_block_id == block_entity_id {
                                match drag.kind {
                                    TableAxis::Column => {
                                        if drag.index != column {
                                            let _ = cell_drop_block.update(cx, |_block, cx| {
                                                cx.emit(BlockEvent::RequestReorderTableAxis {
                                                    kind: TableAxis::Column,
                                                    from: drag.index,
                                                    to: column,
                                                });
                                            });
                                        }
                                    }
                                    TableAxis::Row => {
                                        if drag.index != visual_row {
                                            let _ = cell_drop_block.update(cx, |_block, cx| {
                                                cx.emit(BlockEvent::RequestReorderTableAxis {
                                                    kind: TableAxis::Row,
                                                    from: drag.index,
                                                    to: visual_row,
                                                });
                                            });
                                        }
                                    }
                                }
                            }
                        })
                        .child(cell)
                }))
                .children(row_selection_overlay)
                .children(row_indicator_line)
        });

    let block_id = ElementId::Name(format!("block-{}", block.data.id).into());

    {
        let mut rows = Vec::with_capacity(1 + body_row_count);
        rows.push(header_row.into_any_element());
        rows.extend(body_rows.map(|row| row.into_any_element()));

        let column_edge_band = div()
            .id(ElementId::Name(
                format!("table-append-column-edge-band-{}", block.data.id).into(),
            ))
            .absolute()
            .top_0()
            .bottom_0()
            .right(px(-18.0))
            .w(px(26.0))
            .cursor_pointer()
            .on_hover(cx.listener(Block::on_table_append_column_edge_hover));

        let row_edge_band = div()
            .id(ElementId::Name(
                format!("table-append-row-edge-band-{}", block.data.id).into(),
            ))
            .absolute()
            .left_0()
            .right_0()
            .bottom(px(-18.0))
            .h(px(26.0))
            .cursor_pointer()
            .on_hover(cx.listener(Block::on_table_append_row_edge_hover));

        let (column_edge_band, row_edge_band) = if block.table_axis_selection.is_none() {
            (Some(column_edge_band), Some(row_edge_band))
        } else {
            (None, None)
        };

        let column_control = div()
            .id(ElementId::Name(
                format!("table-append-column-zone-{}", block.data.id).into(),
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
                        format!("table-append-column-button-{}", block.data.id).into(),
                    ))
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .right_0()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_t(px(1.0))
                    .border_r(px(1.0))
                    .border_b(px(1.0))
                    .border_color(c.table_border)
                    .bg(gpui::transparent_black())
                    .cursor_pointer()
                    .opacity(if column_control_visible { 1.0 } else { 0.0 })
                    .block_mouse_except_scroll()
                    .on_hover(cx.listener(Block::on_table_append_column_button_hover))
                    .on_click(cx.listener(Block::on_append_table_column))
                    .child(
                        svg()
                            .path("icons/editor/wysiwyg/table/plus.svg")
                            .size(px(12.0))
                            .text_color(if column_button_hovered {
                                c.text_default
                            } else {
                                c.table_border
                            }),
                    ),
            );

        let row_control = div()
            .id(ElementId::Name(
                format!("table-append-row-zone-{}", block.data.id).into(),
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
                        format!("table-append-row-button-{}", block.data.id).into(),
                    ))
                    .absolute()
                    .left_0()
                    .right_0()
                    .bottom_0()
                    .h_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_l(px(1.0))
                    .border_r(px(1.0))
                    .border_b(px(1.0))
                    .border_color(c.table_border)
                    .bg(gpui::transparent_black())
                    .cursor_pointer()
                    .opacity(if row_control_visible { 1.0 } else { 0.0 })
                    .block_mouse_except_scroll()
                    .on_hover(cx.listener(Block::on_table_append_row_button_hover))
                    .on_click(cx.listener(Block::on_append_table_row))
                    .child(
                        svg()
                            .path("icons/editor/wysiwyg/table/plus.svg")
                            .size(px(12.0))
                            .text_color(if row_button_hovered {
                                c.text_default
                            } else {
                                c.table_border
                            }),
                    ),
            );

        let expand_control_visible = column_control_visible || row_control_visible;
        let expand_button_hovered = column_button_hovered && row_button_hovered;

        let expand_control = div()
            .id(ElementId::Name(
                format!("table-expand-button-{}", block.data.id).into(),
            ))
            .absolute()
            .right(px(-18.0))
            .bottom(px(-18.0))
            .w(px(18.0))
            .h(px(18.0))
            .flex()
            .items_center()
            .justify_center()
            .border_r(px(1.0))
            .border_b(px(1.0))
            .border_color(c.table_border)
            .bg(gpui::transparent_black())
            .cursor_pointer()
            .opacity(if expand_control_visible {
                1.0
            } else {
                0.0
            })
            .block_mouse_except_scroll()
            .on_hover(cx.listener(Block::on_table_append_expand_hover))
            .on_click(cx.listener(Block::on_expand_table))
            .child(
                svg()
                    .path("icons/editor/wysiwyg/table/plus.svg")
                    .size(px(12.0))
                    .text_color(if expand_button_hovered {
                        c.text_default
                    } else {
                        c.table_border
                    }),
            );

        let col_selection_overlay = if let Some(selection) = block.table_axis_selection {
            if selection.kind == TableAxis::Column && selection.index < column_count {
                let left_frac = (0..selection.index).map(|i| column_layout.fraction(i)).sum::<f32>();
                let width_frac = column_layout.fraction(selection.index);
                Some(
                    div()
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left(relative(left_frac))
                        .w(relative(width_frac))
                        .border(px(2.0))
                        .border_color(c.table_selection_border),
                )
            } else {
                None
            }
        } else {
            None
        };

        let col_indicator_line = if let Some(prev) = block.table_axis_preview {
            if prev.kind == TableAxis::Column && prev.index <= column_count {
                let x_frac = (0..prev.index).map(|i| column_layout.fraction(i)).sum::<f32>();
                Some(
                    div()
                        .absolute()
                        .top(px(-4.0))
                        .bottom(px(-4.0))
                        .left(relative(x_frac))
                        .w(px(3.0))
                        .ml(px(-1.5))
                        .rounded(px(1.5))
                        .bg(c.table_selection_border),
                )
            } else {
                None
            }
        } else {
            None
        };

        let table_box = div()
            .relative()
            .w_full()
            .flex()
            .flex_col()
            .children(rows)
            .children(col_selection_overlay)
            .children(col_indicator_line);

        let table_grid = div()
            .relative()
            .w_full()
            .child(table_box)
            .children(column_edge_band)
            .children(row_edge_band)
            .child(column_control)
            .child(row_control)
            .child(expand_control);

        div()
            .id(block_id)
            .w_full()
            .relative()
            .flex()
            .flex_col()
            .pt(px(20.0))
            .pl(px(20.0))
            .pr(px(18.0))
            .pb(px(18.0))
            .gap(px(0.0))
            .child(table_grid)
            .into_any_element()
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
        self.edit_mode = BlockEditMode::RenderedRich;
        self.clear_inline_projection();
        self.sync_render_cache();
    }

    pub(crate) fn set_table_grid(&mut self, runtime: TableGrid) {
        self.table_grid = Some(runtime);
    }

    pub(crate) fn clear_table_grid(&mut self) {
        self.table_grid = None;
        self.table_axis_preview = None;
        self.table_axis_selection = None;
        self.table_axis_highlight = crate::model::block::table::TableAxisHighlight::None;
        self.table_interaction.clear();
    }

    pub(crate) fn set_table_axis_visual_state(
        &mut self,
        preview: Option<TableAxisMarker>,
        selection: Option<TableAxisMarker>,
    ) {
        self.table_axis_preview = preview;
        self.table_axis_selection = selection;
    }
}
