//! Preview table rendering — header and body grid from the persisted
//! `TableData`, with no editing chrome. Column widths fall back to equal
//! splits since the preview has no window measurement.

use gpui::*;

use crate::editor::tree::block::Block;
use crate::editor::windows::preview::render::inline;
use crate::theme::Theme;

/// Renders a native table block read-only.
pub(crate) fn render_preview_table(
    block: &Block,
    base: Div,
    theme: &Theme,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;

    let Some(table) = block.record.table.as_ref() else {
        return base
            .text_size(px(t.text_size))
            .text_color(c.text_default)
            .line_height(rems(t.text_line_height))
            .child(inline::render_preview_inline(
                &block.record.text,
                c.text_default,
                t.text_size,
                FontWeight::NORMAL,
                theme,
            ))
            .into_any_element();
    };

    let column_count = table.column_count();
    let _ = column_count;

    let header_cells = table
        .header
        .iter()
        .map(|cell| render_preview_table_cell(cell, true, theme))
        .collect::<Vec<_>>();

    let header_row = div()
        .w_full()
        .flex()
        .gap(px(0.0))
        .child(
            div()
                .w_full()
                .flex()
                .children(header_cells)
                .border_b(px(d.table_selection_border_width.max(1.0)))
                .border_color(c.table_border),
        );

    let body_rows = table
        .rows
        .iter()
        .map(|row| {
            let cells = row
                .iter()
                .map(|cell| render_preview_table_cell(cell, false, theme))
                .collect::<Vec<_>>();
            div().w_full().flex().children(cells).into_any_element()
        })
        .collect::<Vec<_>>();

    base.w_full()
        .child(
            div()
                .w_full()
                .border(px(1.0))
                .border_color(c.table_border)
                .rounded(px(4.0))
                .overflow_hidden()
                .flex()
                .flex_col()
                .child(header_row)
                .children(body_rows),
        )
        .into_any_element()
}

/// Renders one table cell with the header/body background styles.
fn render_preview_table_cell(
    cell: &crate::model::inline::text::RichText,
    is_header: bool,
    theme: &Theme,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;

    div()
        .flex_1()
        .min_w(px(0.0))
        .px(px(d.table_cell_padding_x))
        .py(px(d.table_cell_padding_y))
        .border_r_1()
        .border_color(c.table_border)
        .text_size(px(t.text_size))
        .text_color(c.text_default)
        .line_height(rems(t.text_line_height))
        .font_weight(if is_header {
            FontWeight::MEDIUM
        } else {
            FontWeight::NORMAL
        })
        .bg(if is_header {
            c.table_header_bg
        } else {
            c.table_cell_bg
        })
        .child(inline::render_preview_inline(
            cell,
            c.text_default,
            t.text_size,
            if is_header {
                FontWeight::MEDIUM
            } else {
                FontWeight::NORMAL
            },
            theme,
        ))
        .into_any_element()
}
