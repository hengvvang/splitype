//! Source-offset mapping generators for table matrix cells and alignment rows.

use gpui::*;

use crate::markdown::block::table::serialize_table_cell_markdown;
use crate::model::block::Block;
use crate::state::SourceTargetMapping;

pub fn push_table_mappings(
    block: &Entity<Block>,
    list_depth: usize,
    quote_depth: usize,
    absolute_start: usize,
    mappings: &mut Vec<SourceTargetMapping>,
    cx: &App,
) -> usize {
    let Some(table) = block.read(cx).data.table.clone() else {
        return 0;
    };
    let Some(grid) = block.read(cx).table_grid.clone() else {
        return 0;
    };

    let lines = crate::markdown::block::table::serialize_table_markdown_lines(&table);
    let indentation = "  ".repeat(list_depth);
    let quote_prefix = "> ".repeat(quote_depth);
    let line_prefix_len = indentation.len() + quote_prefix.len();
    let mut line_start = absolute_start;

    if let Some(header_line) = lines.first() {
        let mut line_cursor = line_prefix_len + 2usize;
        for (column, cell) in grid.header.iter().enumerate() {
            let Some(tree) = table.header.get(column) else {
                continue;
            };
            let cell_markdown = serialize_table_cell_markdown(tree);
            let start = line_start + line_cursor;
            let len = cell_markdown.len();
            mappings.push(SourceTargetMapping {
                entity: cell.clone(),
                full_source_range: start..start + len,
                content_to_source: (0..=len).collect(),
                source_to_content: (0..=len).collect(),
            });
            line_cursor += len + 3;
        }
        line_start += line_prefix_len + header_line.len() + 1;
    }

    if lines.len() > 1 {
        line_start += line_prefix_len + lines[1].len() + 1;
    }

    for (body_row_index, row) in grid.rows.iter().enumerate() {
        let Some(row_line) = lines.get(body_row_index + 2) else {
            break;
        };
        let mut line_cursor = line_prefix_len + 2usize;
        for (column, cell) in row.iter().enumerate() {
            let Some(tree) = table
                .rows
                .get(body_row_index)
                .and_then(|table_row| table_row.get(column))
            else {
                continue;
            };
            let cell_markdown = serialize_table_cell_markdown(tree);
            let start = line_start + line_cursor;
            let len = cell_markdown.len();
            mappings.push(SourceTargetMapping {
                entity: cell.clone(),
                full_source_range: start..start + len,
                content_to_source: (0..=len).collect(),
                source_to_content: (0..=len).collect(),
            });
            line_cursor += len + 3;
        }
        line_start += line_prefix_len + row_line.len() + 1;
    }

    lines
        .iter()
        .map(|line| line_prefix_len + line.len())
        .sum::<usize>()
        + lines.len().saturating_sub(1)
}
