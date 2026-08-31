//! Table row operations: append, insert, move, delete, duplicate.

use crate::markdown::block::table::TableData;

pub fn append_table_row(table: &mut TableData) {
    table.append_row();
}

pub fn swap_table_rows(table: &mut TableData, visual_a: usize, visual_b: usize) {
    let total_rows = table.rows.len() + 1;
    if visual_a < total_rows && visual_b < total_rows {
        table.swap_visual_rows(visual_a, visual_b);
    }
}

pub fn delete_table_row(table: &mut TableData, row_index: usize) -> bool {
    if row_index < table.rows.len() {
        table.remove_body_row(row_index);
        true
    } else {
        false
    }
}

pub fn delete_table_header_row(table: &mut TableData) -> bool {
    if !table.rows.is_empty() {
        table.remove_header_row()
    } else {
        false
    }
}

pub fn insert_table_row_at(table: &mut TableData, visual_row: usize) {
    table.insert_row_at(visual_row);
}

pub fn duplicate_table_row(table: &mut TableData, visual_row: usize) {
    table.duplicate_row(visual_row);
}


