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

pub(crate) struct DraggedTableAxisView;

impl Render for DraggedTableAxisView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().w(px(0.0)).h(px(0.0))
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
    let column_control_visible = block.table_interaction.column_append.is_visible();
    let row_control_visible = block.table_interaction.row_append.is_visible();
    let column_button_hovered = block.table_interaction.column_append.button_hovered;
    let row_button_hovered = block.table_interaction.row_append.button_hovered;
    let block_entity_id = cx.entity().entity_id();
    let weak_table_block = cx.entity().downgrade();

    let header_cells = runtime.header;
    let header_select_block = weak_table_block.clone();
    let header_menu_block = weak_table_block.clone();
    let header_drag_block = weak_table_block.clone();
    let header_drop_block = weak_table_block.clone();
    let header_drag_move_block = weak_table_block.clone();

    let hovered_insert_row = if block.table_axis_selection.is_none() {
        block.table_interaction.hovered_insert_row
    } else {
        None
    };
    let hovered_insert_col = if block.table_axis_selection.is_none() {
        block.table_interaction.hovered_insert_column
    } else {
        None
    };

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

    let header_indicator_line = if let (Some(sel), Some(prev)) =
        (block.table_axis_selection, block.table_axis_preview)
    {
        if sel.kind == TableAxis::Row && prev.kind == TableAxis::Row && sel.index != prev.index && prev.index == 0 {
            Some(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .h(px(3.0))
                    .mt(px(-1.5))
                    .bg(c.table_selection_border),
            )
        } else {
            None
        }
    } else {
        None
    };

    let header_insert_hover_block = weak_table_block.clone();
    let header_insert_click_block = weak_table_block.clone();
    let header_top_insert_band = div()
        .id(ElementId::Name(
            format!("row-insert-band-top-{}", block.data.id).into(),
        ))
        .absolute()
        .top(px(-8.0))
        .h(px(16.0))
        .left(px(-20.0))
        .w(px(28.0))
        .cursor_pointer()
        .on_hover(move |hovered, _window, cx| {
            let _ = header_insert_hover_block.update(cx, |block, cx| {
                block.table_interaction.hovered_insert_row = if *hovered { Some(0) } else { None };
                cx.notify();
            });
        })
        .on_click(move |_event, _window, cx| {
            let _ = header_insert_click_block.update(cx, |_block, cx| {
                cx.emit(BlockEvent::RequestInsertTableAxisAt {
                    kind: TableAxis::Row,
                    index: 0,
                });
            });
        })
        .block_mouse_except_scroll();

    let header_insert_visuals = if hovered_insert_row == Some(0) {
        let btn_click_block = weak_table_block.clone();
        vec![
            div()
                .absolute()
                .top(px(-1.0))
                .left(px(-14.0))
                .right_0()
                .h(px(2.0))
                .bg(c.text_default)
                .into_any_element(),
            div()
                .id(ElementId::Name(
                    format!("row-insert-btn-top-{}", block.data.id).into(),
                ))
                .absolute()
                .top(px(-8.0))
                .left(px(-20.0))
                .size(px(16.0))
                .rounded_full()
                .bg(c.text_default)
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .on_click(move |_event, _window, cx| {
                    let _ = btn_click_block.update(cx, |_block, cx| {
                        cx.emit(BlockEvent::RequestInsertTableAxisAt {
                            kind: TableAxis::Row,
                            index: 0,
                        });
                    });
                })
                .block_mouse_except_scroll()
                .child(
                    svg()
                        .path("icons/editor/wysiwyg/table/plus.svg")
                        .size(px(10.0))
                        .text_color(c.editor_background),
                )
                .into_any_element(),
        ]
    } else {
        Vec::new()
    };

    let column_count = header_cells.len();
    let total_rows = 1 + body_row_count;

    let header_row = div()
        .relative()
        .w_full()
        .flex()
        .gap(px(0.0))
        .children(header_selection_overlay)
        .children(header_indicator_line)
        .child(header_top_insert_band)
        .children(header_insert_visuals)
        .on_drag_move::<DraggedTableAxis>(move |_event, _window, cx| {
            let _ = header_drag_move_block.update(cx, |_block, cx| {
                cx.emit(BlockEvent::RequestTableAxisPreview {
                    kind: TableAxis::Row,
                    index: 0,
                    hovered: true,
                });
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
        // Left Row Edge Interaction (ResizeUpDown cursor, drag/drop, left-click select, right-click menu)
        .child(
            div()
                .id(ElementId::Name(
                    format!("table-header-axis-band-{}", block.data.id).into(),
                ))
                .absolute()
                .left(px(-10.0))
                .w(px(12.0))
                .top_0()
                .h_full()
                .cursor(CursorStyle::ResizeUpDown)
                .on_click(move |_event, _window, cx| {
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
                        let _ = header_drag_block.update(cx, |_block, cx| {
                            cx.emit(BlockEvent::RequestSelectTableAxis {
                                kind: TableAxis::Row,
                                index: 0,
                            });
                        });
                        cx.new(|_| DraggedTableAxisView)
                    },
                ),
        )
        .children(header_cells.into_iter().enumerate().map(|(column, cell)| {
            let select_block = weak_table_block.clone();
            let menu_block = weak_table_block.clone();
            let drag_block = weak_table_block.clone();
            let drop_block = weak_table_block.clone();
            let drop_drag_block = weak_table_block.clone();
            let col_hover_block = weak_table_block.clone();
            let is_last_col = column == column_count - 1;

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
                .on_hover(move |hovered, _window, cx| {
                    if is_last_col {
                        let _ = col_hover_block.update(cx, |block, cx| {
                            block.table_interaction.column_append.is_active = *hovered;
                            cx.notify();
                        });
                    }
                })
                .on_drag_move::<DraggedTableAxis>(move |drag, _window, cx| {
                    let kind = drag.drag(cx).kind;
                    let _ = drop_drag_block.update(cx, |_block, cx| {
                        match kind {
                            TableAxis::Column => {
                                cx.emit(BlockEvent::RequestTableAxisPreview {
                                    kind: TableAxis::Column,
                                    index: column,
                                    hovered: true,
                                });
                            }
                            TableAxis::Row => {
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
                // Top Column Edge Interaction (ResizeLeftRight cursor, drag/drop, left-click select, right-click menu)
                .child(
                    div()
                        .id(ElementId::Name(
                            format!("table-column-axis-activation-{}-{}", block.data.id, column)
                                .into(),
                        ))
                        .absolute()
                        .top(px(-10.0))
                        .h(px(12.0))
                        .left_0()
                        .w_full()
                        .cursor(CursorStyle::ResizeLeftRight)
                        .on_click(move |_event, _window, cx| {
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
                                let _ = drag_block.update(cx, |_block, cx| {
                                    cx.emit(BlockEvent::RequestSelectTableAxis {
                                        kind: TableAxis::Column,
                                        index: column,
                                    });
                                });
                                cx.new(|_| DraggedTableAxisView)
                            },
                        ),
                )
                .child(cell)
        }));

    let body_rows = runtime
        .rows
        .into_iter()
        .enumerate()
        .map(|(body_row_index, row)| {
            let select_block = weak_table_block.clone();
            let menu_block = weak_table_block.clone();
            let drag_block = weak_table_block.clone();
            let drop_block = weak_table_block.clone();
            let row_drag_move_block = weak_table_block.clone();
            let row_hover_block = weak_table_block.clone();
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

            let row_indicator_line = if let (Some(sel), Some(prev)) =
                (block.table_axis_selection, block.table_axis_preview)
            {
                if sel.kind == TableAxis::Row
                    && prev.kind == TableAxis::Row
                    && sel.index != prev.index
                    && prev.index == visual_row
                {
                    if sel.index < visual_row {
                        Some(
                            div()
                                .absolute()
                                .bottom_0()
                                .left_0()
                                .right_0()
                                .h(px(3.0))
                                .mb(px(-1.5))
                                .bg(c.table_selection_border),
                        )
                    } else {
                        Some(
                            div()
                                .absolute()
                                .top_0()
                                .left_0()
                                .right_0()
                                .h(px(3.0))
                                .mt(px(-1.5))
                                .bg(c.table_selection_border),
                        )
                    }
                } else {
                    None
                }
            } else {
                None
            };

            let row_top_insert_band = {
                let insert_hover = weak_table_block.clone();
                let insert_click = weak_table_block.clone();
                div()
                    .id(ElementId::Name(
                        format!("row-insert-band-top-{}-{}", block.data.id, body_row_index).into(),
                    ))
                    .absolute()
                    .top(px(-8.0))
                    .h(px(16.0))
                    .left(px(-20.0))
                    .w(px(28.0))
                    .cursor_pointer()
                    .on_hover(move |hovered, _window, cx| {
                        let _ = insert_hover.update(cx, |block, cx| {
                            block.table_interaction.hovered_insert_row =
                                if *hovered { Some(visual_row) } else { None };
                            cx.notify();
                        });
                    })
                    .on_click(move |_event, _window, cx| {
                        let _ = insert_click.update(cx, |_block, cx| {
                            cx.emit(BlockEvent::RequestInsertTableAxisAt {
                                kind: TableAxis::Row,
                                index: visual_row,
                            });
                        });
                    })
                    .block_mouse_except_scroll()
            };

            let mut row_insert_visuals = Vec::new();
            if hovered_insert_row == Some(visual_row) {
                let btn_click = weak_table_block.clone();
                row_insert_visuals.push(
                    div()
                        .absolute()
                        .top(px(-1.0))
                        .left(px(-14.0))
                        .right_0()
                        .h(px(2.0))
                        .bg(c.text_default)
                        .into_any_element(),
                );
                row_insert_visuals.push(
                    div()
                        .id(ElementId::Name(
                            format!("row-insert-btn-top-{}-{}", block.data.id, body_row_index).into(),
                        ))
                        .absolute()
                        .top(px(-8.0))
                        .left(px(-20.0))
                        .size(px(16.0))
                        .rounded_full()
                        .bg(c.text_default)
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .on_click(move |_event, _window, cx| {
                            let _ = btn_click.update(cx, |_block, cx| {
                                cx.emit(BlockEvent::RequestInsertTableAxisAt {
                                    kind: TableAxis::Row,
                                    index: visual_row,
                                });
                            });
                        })
                        .block_mouse_except_scroll()
                        .child(
                            svg()
                                .path("icons/editor/wysiwyg/table/plus.svg")
                                .size(px(10.0))
                                .text_color(c.editor_background),
                        )
                        .into_any_element(),
                );
            }

            let last_row_bottom_insert_band = if is_last_body_row {
                let insert_hover = weak_table_block.clone();
                let insert_click = weak_table_block.clone();
                Some(
                    div()
                        .id(ElementId::Name(
                            format!("row-insert-band-bottom-{}", block.data.id).into(),
                        ))
                        .absolute()
                        .bottom(px(-8.0))
                        .h(px(16.0))
                        .left(px(-20.0))
                        .w(px(28.0))
                        .cursor_pointer()
                        .on_hover(move |hovered, _window, cx| {
                            let _ = insert_hover.update(cx, |block, cx| {
                                block.table_interaction.hovered_insert_row =
                                    if *hovered { Some(total_rows) } else { None };
                                cx.notify();
                            });
                        })
                        .on_click(move |_event, _window, cx| {
                            let _ = insert_click.update(cx, |_block, cx| {
                                cx.emit(BlockEvent::RequestInsertTableAxisAt {
                                    kind: TableAxis::Row,
                                    index: total_rows,
                                });
                            });
                        })
                        .block_mouse_except_scroll(),
                )
            } else {
                None
            };

            if is_last_body_row && hovered_insert_row == Some(total_rows) {
                let btn_click = weak_table_block.clone();
                row_insert_visuals.push(
                    div()
                        .absolute()
                        .bottom(px(-1.0))
                        .left(px(-14.0))
                        .right_0()
                        .h(px(2.0))
                        .bg(c.text_default)
                        .into_any_element(),
                );
                row_insert_visuals.push(
                    div()
                        .id(ElementId::Name(
                            format!("row-insert-btn-bottom-{}", block.data.id).into(),
                        ))
                        .absolute()
                        .bottom(px(-8.0))
                        .left(px(-20.0))
                        .size(px(16.0))
                        .rounded_full()
                        .bg(c.text_default)
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .on_click(move |_event, _window, cx| {
                            let _ = btn_click.update(cx, |_block, cx| {
                                cx.emit(BlockEvent::RequestInsertTableAxisAt {
                                    kind: TableAxis::Row,
                                    index: total_rows,
                                });
                            });
                        })
                        .block_mouse_except_scroll()
                        .child(
                            svg()
                                .path("icons/editor/wysiwyg/table/plus.svg")
                                .size(px(10.0))
                                .text_color(c.editor_background),
                        )
                        .into_any_element(),
                );
            }

            div()
                .id(ElementId::Name(
                    format!("table-body-row-wrap-{}-{}", block.data.id, body_row_index).into(),
                ))
                .relative()
                .w_full()
                .flex()
                .gap(px(0.0))
                .children(row_selection_overlay)
                .children(row_indicator_line)
                .child(row_top_insert_band)
                .children(last_row_bottom_insert_band)
                .children(row_insert_visuals)
                .on_hover(move |hovered, _window, cx| {
                    if is_last_body_row {
                        let _ = row_hover_block.update(cx, |block, cx| {
                            block.table_interaction.row_append.is_active = *hovered;
                            cx.notify();
                        });
                    }
                })
                .on_drag_move::<DraggedTableAxis>(move |_event, _window, cx| {
                    let _ = row_drag_move_block.update(cx, |_block, cx| {
                        cx.emit(BlockEvent::RequestTableAxisPreview {
                            kind: TableAxis::Row,
                            index: visual_row,
                            hovered: true,
                        });
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
                // Left Row Edge Interaction (ResizeUpDown cursor, drag/drop, left-click select, right-click menu)
                .child(
                    div()
                        .id(ElementId::Name(
                            format!("table-row-axis-band-{}-{}", block.data.id, body_row_index)
                                .into(),
                        ))
                        .absolute()
                        .left(px(-10.0))
                        .w(px(12.0))
                        .top_0()
                        .h_full()
                        .cursor(CursorStyle::ResizeUpDown)
                        .on_click(move |_event, _window, cx| {
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
                                let _ = drag_block.update(cx, |_block, cx| {
                                    cx.emit(BlockEvent::RequestSelectTableAxis {
                                        kind: TableAxis::Row,
                                        index: visual_row,
                                    });
                                });
                                cx.new(|_| DraggedTableAxisView)
                            },
                        ),
                )
                .children(row.into_iter().enumerate().map(|(column, cell)| {
                    let col_hover_block = weak_table_block.clone();
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
                        })
                        .on_drag_move::<DraggedTableAxis>(move |drag, _window, cx| {
                            let kind = drag.drag(cx).kind;
                            let _ = cell_drop_drag_block.update(cx, |_block, cx| {
                                match kind {
                                    TableAxis::Column => {
                                        cx.emit(BlockEvent::RequestTableAxisPreview {
                                            kind: TableAxis::Column,
                                            index: column,
                                            hovered: true,
                                        });
                                    }
                                    TableAxis::Row => {
                                        cx.emit(BlockEvent::RequestTableAxisPreview {
                                            kind: TableAxis::Row,
                                            index: visual_row,
                                            hovered: true,
                                        });
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
            .right(px(-6.0))
            .w(px(12.0))
            .bottom_0()
            .cursor_pointer()
            .on_hover(cx.listener(Block::on_table_append_column_edge_hover));

        let row_edge_band = div()
            .id(ElementId::Name(
                format!("table-append-row-edge-band-{}", block.data.id).into(),
            ))
            .absolute()
            .left_0()
            .bottom(px(-6.0))
            .w_full()
            .h(px(12.0))
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
                    .child(
                        svg()
                            .path("icons/editor/wysiwyg/table/plus.svg")
                            .size(px(12.0))
                            .text_color(if column_button_hovered {
                                c.table_append_button_text
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
                    .child(
                        svg()
                            .path("icons/editor/wysiwyg/table/plus.svg")
                            .size(px(12.0))
                            .text_color(if row_button_hovered {
                                c.table_append_button_text
                            } else {
                                c.table_border
                            }),
                    ),
            );

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
            .hover(|this| {
                this.bg(c.table_append_button_hover)
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
                    .path("icons/editor/wysiwyg/table/plus.svg")
                    .size(px(12.0))
                    .text_color(c.table_border),
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

        let col_indicator_line = if let (Some(sel), Some(prev)) =
            (block.table_axis_selection, block.table_axis_preview)
        {
            if sel.kind == TableAxis::Column
                && prev.kind == TableAxis::Column
                && sel.index != prev.index
                && prev.index < column_count
            {
                let x_frac = if sel.index < prev.index {
                    (0..=prev.index).map(|i| column_layout.fraction(i)).sum::<f32>()
                } else {
                    (0..prev.index).map(|i| column_layout.fraction(i)).sum::<f32>()
                };
                Some(
                    div()
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left(relative(x_frac))
                        .w(px(3.0))
                        .ml(px(-1.5))
                        .bg(c.table_selection_border),
                )
            } else {
                None
            }
        } else {
            None
        };

        let mut col_insert_bands = Vec::with_capacity(column_count + 1);
        let mut col_insert_visuals = Vec::new();

        for c_idx in 0..=column_count {
            let x_frac = if c_idx == 0 {
                0.0
            } else if c_idx == column_count {
                1.0
            } else {
                (0..c_idx).map(|i| column_layout.fraction(i)).sum::<f32>()
            };
            let insert_hover_block = weak_table_block.clone();
            let insert_click_block = weak_table_block.clone();

            col_insert_bands.push(
                div()
                    .id(ElementId::Name(
                        format!("col-insert-band-{}-{}", block.data.id, c_idx).into(),
                    ))
                    .absolute()
                    .top(px(-18.0))
                    .h(px(28.0))
                    .left(relative(x_frac))
                    .ml(px(-8.0))
                    .w(px(16.0))
                    .cursor_pointer()
                    .on_hover(move |hovered, _window, cx| {
                        let _ = insert_hover_block.update(cx, |block, cx| {
                            block.table_interaction.hovered_insert_column =
                                if *hovered { Some(c_idx) } else { None };
                            cx.notify();
                        });
                    })
                    .on_click(move |_event, _window, cx| {
                        let _ = insert_click_block.update(cx, |_block, cx| {
                            cx.emit(BlockEvent::RequestInsertTableAxisAt {
                                kind: TableAxis::Column,
                                index: c_idx,
                            });
                        });
                    })
                    .block_mouse_except_scroll(),
            );

            if hovered_insert_col == Some(c_idx) {
                let btn_click_block = weak_table_block.clone();
                col_insert_visuals.push(
                    div()
                        .absolute()
                        .top(px(-14.0))
                        .bottom_0()
                        .left(relative(x_frac))
                        .w(px(2.0))
                        .ml(px(-1.0))
                        .bg(c.text_default)
                        .into_any_element(),
                );
                col_insert_visuals.push(
                    div()
                        .id(ElementId::Name(
                            format!("col-insert-btn-{}-{}", block.data.id, c_idx).into(),
                        ))
                        .absolute()
                        .top(px(-18.0))
                        .left(relative(x_frac))
                        .ml(px(-8.0))
                        .size(px(16.0))
                        .rounded_full()
                        .bg(c.text_default)
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .on_click(move |_event, _window, cx| {
                            let _ = btn_click_block.update(cx, |_block, cx| {
                                cx.emit(BlockEvent::RequestInsertTableAxisAt {
                                    kind: TableAxis::Column,
                                    index: c_idx,
                                });
                            });
                        })
                        .block_mouse_except_scroll()
                        .child(
                            svg()
                                .path("icons/editor/wysiwyg/table/plus.svg")
                                .size(px(10.0))
                                .text_color(c.editor_background),
                        )
                        .into_any_element(),
                );
            }
        }

        let table_box = div()
            .relative()
            .w_full()
            .border_t(px(1.0))
            .border_l(px(1.0))
            .border_color(c.table_border)
            .flex()
            .flex_col()
            .children(rows)
            .children(col_selection_overlay)
            .children(col_indicator_line)
            .children(col_insert_bands)
            .children(col_insert_visuals);

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
