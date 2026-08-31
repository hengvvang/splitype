//! Table column operations: append, insert, align, move, delete, duplicate.

use crate::markdown::block::table::{TableColumnAlignment, TableData};

pub fn append_table_column(table: &mut TableData) {
    let alignment = table
        .alignments
        .last()
        .copied()
        .unwrap_or(TableColumnAlignment::Default);
    table.append_column(alignment);
}

pub fn set_table_column_alignment(
    table: &mut TableData,
    column: usize,
    alignment: TableColumnAlignment,
) {
    table.set_column_alignment(column, alignment);
}

pub fn swap_table_columns(table: &mut TableData, a: usize, b: usize) {
    if a < table.column_count() && b < table.column_count() {
        table.swap_columns(a, b);
    }
}

pub fn delete_table_column(table: &mut TableData, column: usize) -> bool {
    if table.column_count() > 1 && column < table.column_count() {
        table.remove_column(column);
        true
    } else {
        false
    }
}

pub fn insert_table_column_at(table: &mut TableData, column: usize) {
    table.insert_column_at(column, TableColumnAlignment::Default);
}

pub fn duplicate_table_column(table: &mut TableData, column: usize) {
    table.duplicate_column(column);
}
