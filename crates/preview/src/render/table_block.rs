//! Preview table rendering — header and body grid from the persisted
//! `TableData`, with no editing chrome. Column widths fall back to equal
//! splits since the preview has no window measurement.

use gpui::*;

use crate::node::PreviewBlock;
use crate::render::inline;
use crate::render::preview_centered_column_width;
use crate::settings::PreviewSettings;
use markdown_parser::block::table::TableColumnLayout;
use theme::Theme;

/// Renders a native table block read-only with content-measured column
/// widths, mirroring the WYSIWYG table layout.
pub(crate) fn render_preview_table(
    block: &PreviewBlock,
    base: Div,
    settings: PreviewSettings,
    theme: &Theme,
    window: &mut Window,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;

    let Some(table) = block.data.table.as_ref() else {
        return base
            .text_size(px(t.text_size))
            .text_color(c.text_default)
            .line_height(rems(t.text_line_height))
            .child(inline::render_preview_inline(
                &block.data.text,
                c.text_default,
                t.text_size,
                FontWeight::NORMAL,
                theme,
            ))
            .into_any_element();
    };

    let viewport_width = f32::from(window.viewport_size().width.max(px(1.0)));
    let _table_width = preview_centered_column_width(viewport_width, d);
    let column_layout = TableColumnLayout::equal(table.column_count());

    let header_cells = table
        .header
        .iter()
        .enumerate()
        .map(|(index, cell)| {
            render_preview_table_cell(
                cell,
                settings.show_table_headers,
                column_layout.fraction(index),
                theme,
            )
        })
        .collect::<Vec<_>>();

    let header_inner = div().w_full().flex().children(header_cells);
    let header_row = if settings.show_table_headers {
        header_inner
            .border_b(px(d.table_selection_border_width.max(1.0)))
            .border_color(c.table_border)
    } else {
        header_inner
    };

    let body_rows = table
        .rows
        .iter()
        .map(|row| {
            let cells = row
                .iter()
                .enumerate()
                .map(|(index, cell)| {
                    render_preview_table_cell(cell, false, column_layout.fraction(index), theme)
                })
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
                .flex()
                .flex_col()
                .child(header_row)
                .children(body_rows),
        )
        .into_any_element()
}

/// Renders one table cell with the header/body background styles.
fn render_preview_table_cell(
    cell: &markdown_parser::inline::text::BlockText,
    is_header: bool,
    fraction: f32,
    theme: &Theme,
) -> AnyElement {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;

    div()
        .w(relative(fraction))
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
