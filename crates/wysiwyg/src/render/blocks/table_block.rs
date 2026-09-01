use gpui::*;

use crate::model::block::Block;
use crate::model::protocol::BlockEvent;
use crate::render::effective_table_width;
use crate::table::measure::measure_table_column_layout;
use markdown_parser::block::table::{TableAxis, TableAxisMarker, TableColumnLayout};
use theme::Theme;

/// Visual highlight priority state for table row and column axes: Dragging > Selected > Hovered > Editing > None.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TableAxisVisualState {
    /// Default: no visual highlight.
    None = 0,
    /// Priority 1 (Lowest): cell within this axis is being actively edited (1.5px green indicator on outer edge).
    Editing = 1,
    /// Priority 2: cursor is hovering over cell or axis handle (1.5px gray indicator on outer edge).
    Hovered = 2,
    /// Priority 3: axis is clicked and selected (2.0px blue full bounding box).
    Selected = 3,
    /// Priority 4 (Highest): axis is actively being dragged (2.0px blue full bounding box on source axis).
    Dragging = 4,
}

/// Unified highlight priority resolver for table row and column axes.
pub struct TableHighlightResolver {
    pub active_cell: Option<(usize, usize)>, // (row, col)
    pub hovered_row: Option<usize>,
    pub hovered_column: Option<usize>,
    pub selected_axis: Option<TableAxisMarker>,
    pub is_dragging_axis: bool,
}

impl TableHighlightResolver {
    pub fn new(
        active_cell: Option<(usize, usize)>,
        hovered_row: Option<usize>,
        hovered_column: Option<usize>,
        selected_axis: Option<TableAxisMarker>,
        is_dragging_axis: bool,
    ) -> Self {
        Self {
            active_cell,
            hovered_row,
            hovered_column,
            selected_axis,
            is_dragging_axis,
        }
    }

    /// Resolve visual highlight priority for a specific column.
    pub fn resolve_column(&self, col: usize) -> TableAxisVisualState {
        // Priority 4 (Highest): actively dragging this column
        if self.is_dragging_axis {
            if let Some(sel) = self.selected_axis {
                if sel.kind == TableAxis::Column && sel.index == col {
                    return TableAxisVisualState::Dragging;
                }
            }
        }
        // Priority 3: clicked/selected column
        if let Some(sel) = self.selected_axis {
            if sel.kind == TableAxis::Column && sel.index == col {
                return TableAxisVisualState::Selected;
            }
        }
        // Priority 2: cursor hovering over column
        if self.hovered_column == Some(col) {
            return TableAxisVisualState::Hovered;
        }
        // Priority 1 (Lowest): active cell in column
        if let Some((_, c)) = self.active_cell {
            if c == col {
                return TableAxisVisualState::Editing;
            }
        }
        TableAxisVisualState::None
    }

    /// Resolve visual highlight priority for a specific row (0 for header row, 1+ for body rows).
    pub fn resolve_row(&self, row: usize) -> TableAxisVisualState {
        // Priority 4 (Highest): actively dragging this row
        if self.is_dragging_axis {
            if let Some(sel) = self.selected_axis {
                if sel.kind == TableAxis::Row && sel.index == row {
                    return TableAxisVisualState::Dragging;
                }
            }
        }
        // Priority 3: clicked/selected row
        if let Some(sel) = self.selected_axis {
            if sel.kind == TableAxis::Row && sel.index == row {
                return TableAxisVisualState::Selected;
            }
        }
        // Priority 2: cursor hovering over row
        if self.hovered_row == Some(row) {
            return TableAxisVisualState::Hovered;
        }
        // Priority 1 (Lowest): active cell in row
        if let Some((r, _)) = self.active_cell {
            if r == row {
                return TableAxisVisualState::Editing;
            }
        }
        TableAxisVisualState::None
    }
}

/// Layout parameters and column/row geometry for a table block.
#[derive(Clone, Debug)]
pub struct TableLayoutParameters {
    pub column_count: usize,
    pub total_rows: usize,
    pub column_fractions: Vec<f32>,
    pub column_cumulative_fractions: Vec<f32>,
}

impl TableLayoutParameters {
    pub fn new(column_layout: &TableColumnLayout, total_rows: usize) -> Self {
        let column_count = column_layout.column_count();
        let column_fractions: Vec<f32> = (0..column_count)
            .map(|i| column_layout.fraction(i))
            .collect();
        let mut column_cumulative_fractions = Vec::with_capacity(column_count + 1);
        column_cumulative_fractions.push(0.0);
        for &f in &column_fractions {
            column_cumulative_fractions
                .push(column_cumulative_fractions.last().copied().unwrap_or(0.0) + f);
        }

        Self {
            column_count,
            total_rows,
            column_fractions,
            column_cumulative_fractions,
        }
    }

    /// Resolve column insertion line slot from local X fraction.
    /// Zone 0: x < mid_0 (including all x < 0 outside left edge) -> Line 0 (leftmost border)
    /// Zone c (1..C-1): mid_{c-1} <= x < mid_c -> Line c (centered on Line c spanning 50% of left and right column)
    /// Zone C: x >= mid_{C-1} (including all x > 1.0 outside right edge) -> Line C (rightmost border)
    pub fn resolve_column_slot(&self, x_frac: f32) -> usize {
        if self.column_count == 0 {
            return 0;
        }
        if self.column_count == 1 {
            return if x_frac < 0.5 { 0 } else { 1 };
        }

        let mid_0 =
            (self.column_cumulative_fractions[0] + self.column_cumulative_fractions[1]) / 2.0;
        if x_frac < mid_0 {
            return 0;
        }

        for c in 0..(self.column_count - 1) {
            let mid_c = (self.column_cumulative_fractions[c]
                + self.column_cumulative_fractions[c + 1])
                / 2.0;
            let mid_next = (self.column_cumulative_fractions[c + 1]
                + self.column_cumulative_fractions[c + 2])
                / 2.0;
            if x_frac >= mid_c && x_frac < mid_next {
                return c + 1;
            }
        }

        self.column_count
    }

    /// Resolve row insertion line slot from local Y fraction.
    /// Zone 0: y < mid_0 (including all y < 0 outside top edge) -> Line 0 (topmost border)
    /// Zone r (1..R-1): mid_{r-1} <= y < mid_r -> Line r (centered on Line r spanning 50% of above and below row)
    /// Zone R: y >= mid_{R-1} (including all y > 1.0 outside bottom edge) -> Line R (bottommost border)
    pub fn resolve_row_slot(&self, y_frac: f32) -> usize {
        if self.total_rows == 0 {
            return 0;
        }
        if self.total_rows == 1 {
            return if y_frac < 0.5 { 0 } else { 1 };
        }

        let total_r = self.total_rows as f32;
        let mid_0 = 0.5 / total_r;
        if y_frac < mid_0 {
            return 0;
        }

        for r in 0..(self.total_rows - 1) {
            let mid_r = (r as f32 + 0.5) / total_r;
            let mid_next = (r as f32 + 1.5) / total_r;
            if y_frac >= mid_r && y_frac < mid_next {
                return r + 1;
            }
        }

        self.total_rows
    }

    /// Map insertion slot to reorder target index `to`.
    pub fn slot_to_target_index(from: usize, slot: usize) -> Option<usize> {
        if slot == from || slot == from + 1 {
            None
        } else if slot <= from {
            Some(slot)
        } else {
            Some(slot - 1)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DraggedTableAxis {
    pub table_block_id: EntityId,
    pub kind: TableAxis,
    pub index: usize,
}

pub struct DraggedTableAxisView {
    pub theme: Theme,
    pub offset: Point<Pixels>,
}

impl Render for DraggedTableAxisView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let c = &self.theme.colors;
        let axis_len = px(432.0);
        let half_len = px(216.0);
        let ox = self.offset.x;
        let oy = self.offset.y;

        div()
            .relative()
            .size(px(0.0))
            // Horizontal solid crosshair line (432px, 1.0px thickness, centered at drag cursor)
            .child(
                div()
                    .absolute()
                    .left(ox - half_len)
                    .w(axis_len)
                    .top(oy - px(0.5))
                    .h(px(1.0))
                    .bg(c.table_selection_border),
            )
            // Vertical solid crosshair line (432px, 1.0px thickness, centered at drag cursor)
            .child(
                div()
                    .absolute()
                    .top(oy - half_len)
                    .h(axis_len)
                    .left(ox - px(0.5))
                    .w(px(1.0))
                    .bg(c.table_selection_border),
            )
    }
}

/// Render a native table block.
pub fn render_table(
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

    let layout_params = TableLayoutParameters::new(&column_layout, 1 + body_row_count);

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

    let is_dragging_axis = block.table_axis_preview.is_some();

    let resolver = TableHighlightResolver::new(
        active_cell,
        block.table_interaction.hovered_row,
        block.table_interaction.hovered_column,
        block.table_axis_selection,
        is_dragging_axis,
    );

    let header_cells = runtime.header;
    let header_select_block = weak_table_block.clone();
    let header_menu_block = weak_table_block.clone();
    let header_axis_hover_block = weak_table_block.clone();

    let row_0_state = resolver.resolve_row(0);

    let header_left_indicator = match row_0_state {
        TableAxisVisualState::Editing => Some(
            div()
                .absolute()
                .left(px(-1.0))
                .top_0()
                .bottom_0()
                .w(px(1.5))
                .bg(Hsla::from(rgba(0x22c55eff))),
        ),
        TableAxisVisualState::Hovered => Some(
            div()
                .absolute()
                .left(px(-1.0))
                .top_0()
                .bottom_0()
                .w(px(1.5))
                .bg(c.table_handle_icon),
        ),
        TableAxisVisualState::Selected
        | TableAxisVisualState::Dragging
        | TableAxisVisualState::None => None,
    };

    let row_0_selection_box = if row_0_state >= TableAxisVisualState::Selected {
        Some(
            div()
                .absolute()
                .left(px(-1.0))
                .right(px(-1.0))
                .top(px(-1.0))
                .bottom(px(-1.0))
                .border(px(2.0))
                .border_color(c.table_selection_border),
        )
    } else {
        None
    };

    let header_axis_theme = theme.clone();

    let header_axis_band = div()
        .id(ElementId::Name(
            format!("table-header-axis-band-{}", block.data.id).into(),
        ))
        .absolute()
        .left(px(-12.0))
        .w(px(16.0))
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
            move |_drag, point, _window, cx| {
                let theme = header_axis_theme.clone();
                cx.new(|_| DraggedTableAxisView {
                    theme,
                    offset: point,
                })
            },
        );

    let header_row = div()
        .relative()
        .w_full()
        .flex()
        .gap(px(0.0))
        .border_l(px(1.0))
        .border_color(c.table_border)
        .child(header_axis_band)
        .children(header_cells.into_iter().enumerate().map(|(column, cell)| {
            let select_block = weak_table_block.clone();
            let menu_block = weak_table_block.clone();
            let col_hover_block = weak_table_block.clone();
            let cell_hover_block = weak_table_block.clone();
            let col_axis_hover_block = weak_table_block.clone();
            let is_last_col = column == column_count - 1;

            let col_state = resolver.resolve_column(column);

            let col_top_indicator = match col_state {
                TableAxisVisualState::Editing => Some(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .top(px(-1.0))
                        .h(px(1.5))
                        .bg(Hsla::from(rgba(0x22c55eff))),
                ),
                TableAxisVisualState::Hovered => Some(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .top(px(-1.0))
                        .h(px(1.5))
                        .bg(c.table_handle_icon),
                ),
                TableAxisVisualState::Selected
                | TableAxisVisualState::Dragging
                | TableAxisVisualState::None => None,
            };

            let col_axis_theme = theme.clone();

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
                .border_t(px(1.0))
                .border_color(c.table_border)
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
                .child(
                    div()
                        .id(ElementId::Name(
                            format!("table-column-axis-activation-{}-{}", block.data.id, column)
                                .into(),
                        ))
                        .absolute()
                        .top(px(-12.0))
                        .h(px(16.0))
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
                            move |_drag, point, _window, cx| {
                                let theme = col_axis_theme.clone();
                                cx.new(|_| DraggedTableAxisView {
                                    theme,
                                    offset: point,
                                })
                            },
                        ),
                )
                .child(cell)
                .children(col_top_indicator)
        }))
        .children(header_left_indicator)
        .children(row_0_selection_box);

    let body_rows = runtime
        .rows
        .into_iter()
        .enumerate()
        .map(|(body_row_index, row)| {
            let select_block = weak_table_block.clone();
            let menu_block = weak_table_block.clone();
            let row_hover_block = weak_table_block.clone();
            let row_axis_hover_block = weak_table_block.clone();
            let is_last_body_row = body_row_index == body_row_count - 1;
            let visual_row = body_row_index + 1;
            let row_state = resolver.resolve_row(visual_row);

            let row_left_indicator = match row_state {
                TableAxisVisualState::Editing => Some(
                    div()
                        .absolute()
                        .left(px(-1.0))
                        .top_0()
                        .bottom_0()
                        .w(px(1.5))
                        .bg(Hsla::from(rgba(0x22c55eff))),
                ),
                TableAxisVisualState::Hovered => Some(
                    div()
                        .absolute()
                        .left(px(-1.0))
                        .top_0()
                        .bottom_0()
                        .w(px(1.5))
                        .bg(c.table_handle_icon),
                ),
                TableAxisVisualState::Selected
                | TableAxisVisualState::Dragging
                | TableAxisVisualState::None => None,
            };

            let row_selection_box = if row_state >= TableAxisVisualState::Selected {
                Some(
                    div()
                        .absolute()
                        .left(px(-1.0))
                        .right(px(-1.0))
                        .top(px(-1.0))
                        .bottom(px(-1.0))
                        .border(px(2.0))
                        .border_color(c.table_selection_border),
                )
            } else {
                None
            };

            let row_axis_theme = theme.clone();

            let row_axis_band = div()
                .id(ElementId::Name(
                    format!("table-row-axis-band-{}-{}", block.data.id, body_row_index).into(),
                ))
                .absolute()
                .left(px(-12.0))
                .w(px(16.0))
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
                    move |_drag, point, _window, cx| {
                        let theme = row_axis_theme.clone();
                        cx.new(|_| DraggedTableAxisView {
                            theme,
                            offset: point,
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
                .border_l(px(1.0))
                .border_color(c.table_border)
                .child(row_axis_band)
                .on_hover(move |hovered, _window, cx| {
                    if is_last_body_row {
                        let _ = row_hover_block.update(cx, |block, cx| {
                            block.table_interaction.row_append.is_active = *hovered;
                            cx.notify();
                        });
                    }
                })
                .children(row.into_iter().enumerate().map(|(column, cell)| {
                    let col_hover_block = weak_table_block.clone();
                    let cell_hover_block = weak_table_block.clone();
                    let is_last_col = column == column_count - 1;

                    div()
                        .id(ElementId::Name(
                            format!(
                                "table-body-cell-wrap-{}-{}-{}",
                                block.data.id, body_row_index, column
                            )
                            .into(),
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
                        .child(cell)
                }))
                .children(row_left_indicator)
                .children(row_selection_box)
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

        let picker_weak_block = weak_table_block.clone();

        let size_picker_control = div()
            .id(ElementId::Name(
                format!("table-size-picker-button-{}", block.data.id).into(),
            ))
            .absolute()
            .right(px(-18.0))
            .bottom(px(-18.0))
            .w(px(18.0))
            .h(px(18.0))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .block_mouse_except_scroll()
            .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                cx.stop_propagation();
                let _ = picker_weak_block.update(cx, |_block, cx| {
                    cx.emit(BlockEvent::RequestOpenTableSizePicker {
                        position: event.position,
                    });
                });
            })
            .child(
                div()
                    .size(px(6.0))
                    .rounded_full()
                    .bg(c.table_selection_border),
            );

        let table_drag_move_block = weak_table_block.clone();
        let table_drop_block = weak_table_block.clone();

        let col_selection_overlay = (0..column_count)
            .find(|&c| resolver.resolve_column(c) >= TableAxisVisualState::Selected)
            .map(|col| {
                let left_frac = layout_params.column_cumulative_fractions[col];
                let width_frac = layout_params.column_fractions[col];
                div()
                    .absolute()
                    .top(px(-1.0))
                    .bottom(px(-1.0))
                    .left(relative(left_frac))
                    .w(relative(width_frac))
                    .border(px(2.0))
                    .border_color(c.table_selection_border)
            });

        let col_insertion_line = if let Some(prev) = block.table_axis_preview {
            if prev.kind == TableAxis::Column && prev.index <= column_count {
                let x_frac = layout_params.column_cumulative_fractions[prev.index];
                Some(
                    div()
                        .absolute()
                        .top(px(-1.0))
                        .bottom(px(-1.0))
                        .left(relative(x_frac))
                        .w(px(2.0))
                        .ml(px(-1.0))
                        .bg(c.table_selection_border),
                )
            } else {
                None
            }
        } else {
            None
        };

        let row_insertion_line = if let Some(prev) = block.table_axis_preview {
            if prev.kind == TableAxis::Row && prev.index <= 1 + body_row_count {
                let total_r = (1 + body_row_count) as f32;
                let y_frac = prev.index as f32 / total_r;
                Some(
                    div()
                        .absolute()
                        .left(px(-1.0))
                        .right(px(-1.0))
                        .top(relative(y_frac))
                        .h(px(2.0))
                        .mt(px(-1.0))
                        .bg(c.table_selection_border),
                )
            } else {
                None
            }
        } else {
            None
        };

        let table_box = div()
            .id(ElementId::Name(
                format!("table-box-{}", block.data.id).into(),
            ))
            .relative()
            .w_full()
            .flex()
            .flex_col()
            .children(rows)
            .children(col_selection_overlay)
            .children(col_insertion_line)
            .children(row_insertion_line);

        let drag_params = layout_params.clone();

        let table_grid = div()
            .relative()
            .w_full()
            .child(table_box)
            .children(column_edge_band)
            .children(row_edge_band)
            .child(column_control)
            .child(row_control)
            .child(size_picker_control)
            .on_drag_move::<DraggedTableAxis>(move |drag, _window, cx| {
                let kind = drag.drag(cx).kind;
                let bounds = drag.bounds;
                let pos = drag.event.position;

                let _ = table_drag_move_block.update(cx, |block, cx| match kind {
                    TableAxis::Column => {
                        let rel_x = pos.x - bounds.origin.x;
                        let x_frac = f32::from(rel_x) / f32::from(bounds.size.width.max(px(1.0)));
                        let slot = drag_params.resolve_column_slot(x_frac);

                        if block.table_axis_preview
                            != Some(TableAxisMarker {
                                kind: TableAxis::Column,
                                index: slot,
                            })
                        {
                            cx.emit(BlockEvent::RequestTableAxisPreview {
                                kind: TableAxis::Column,
                                index: slot,
                                hovered: true,
                            });
                        }
                    }
                    TableAxis::Row => {
                        let rel_y = pos.y - bounds.origin.y;
                        let y_frac = f32::from(rel_y) / f32::from(bounds.size.height.max(px(1.0)));
                        let slot = drag_params.resolve_row_slot(y_frac);

                        if block.table_axis_preview
                            != Some(TableAxisMarker {
                                kind: TableAxis::Row,
                                index: slot,
                            })
                        {
                            cx.emit(BlockEvent::RequestTableAxisPreview {
                                kind: TableAxis::Row,
                                index: slot,
                                hovered: true,
                            });
                        }
                    }
                });
            })
            .on_drop::<DraggedTableAxis>(move |drag, _window, cx| {
                if drag.table_block_id == block_entity_id {
                    let kind = drag.kind;
                    let from = drag.index;
                    let _ = table_drop_block.update(cx, |block, cx| {
                        if let Some(prev) = block.table_axis_preview {
                            if prev.kind == kind {
                                if let Some(to) =
                                    TableLayoutParameters::slot_to_target_index(from, prev.index)
                                {
                                    cx.emit(BlockEvent::RequestReorderTableAxis { kind, from, to });
                                }
                            }
                        }
                    });
                }
            });

        div()
            .id(block_id)
            .w_full()
            .relative()
            .flex()
            .flex_col()
            .pt(px(20.0))
            .pl(px(d.block_padding_x))
            .pr(px(d.block_padding_x + d.table_append_button_extent))
            .pb(px(18.0))
            .gap(px(0.0))
            .child(table_grid)
            .into_any_element()
    }
}
