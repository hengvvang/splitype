//! Table grid installation and table manipulation actions.

use gpui::{AppContext, TestAppContext};

use crate::editor::engine::controller::Editor;
use crate::model::block::table::{TableAxis, TableColumnAlignment};
use crate::model::parse::BlockKind;

#[gpui::test]
async fn parsed_table_grid_installs_column_alignment_on_cells(cx: &mut TestAppContext) {
    let markdown = [
        "| Left | Center | Right |",
        "| :--- | :---: | ---: |",
        "| a | b | c |",
    ]
    .join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.read_with(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root").clone();
        assert_eq!(table.read(cx).kind(), BlockKind::Table);
        let grid = table.read(cx).table_grid.as_ref().expect("table grid");
        assert_eq!(
            grid.header[0].read(cx).table_cell_alignment(),
            Some(TableColumnAlignment::Left)
        );
        assert_eq!(
            grid.header[1].read(cx).table_cell_alignment(),
            Some(TableColumnAlignment::Center)
        );
        assert_eq!(
            grid.rows[0][2].read(cx).table_cell_alignment(),
            Some(TableColumnAlignment::Right)
        );
    });
}

#[gpui::test]
async fn append_column_updates_table_and_focuses_new_header_cell(cx: &mut TestAppContext) {
    let markdown = ["| A | B |", "| --- | ---: |", "| 1 | 2 |"].join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root").clone();
        editor.append_table_column(&table, cx);

        let table_data = table
            .read(cx)
            .data
            .table
            .as_ref()
            .expect("table data after append");
        assert_eq!(table_data.header.len(), 3);
        assert_eq!(table_data.rows[0].len(), 3);
        assert_eq!(
            table_data.alignments,
            vec![
                TableColumnAlignment::Default,
                TableColumnAlignment::Right,
                TableColumnAlignment::Right,
            ]
        );

        let grid = table.read(cx).table_grid.as_ref().expect("rebuilt grid");
        let focused = grid.header[2].entity_id();
        assert_eq!(editor.active_pane_focus().pending, Some(focused));
    });
}

#[gpui::test]
async fn append_row_updates_table_and_focuses_first_cell_of_new_row(cx: &mut TestAppContext) {
    let markdown = ["| A | B |", "| --- | :---: |", "| 1 | 2 |"].join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root").clone();
        editor.append_table_row(&table, cx);

        let table_data = table
            .read(cx)
            .data
            .table
            .as_ref()
            .expect("table data after append");
        assert_eq!(table_data.rows.len(), 2);
        assert_eq!(table_data.rows[1].len(), 2);
        assert!(
            table_data.rows[1]
                .iter()
                .all(|cell| cell.serialize_markdown().is_empty())
        );

        let grid = table.read(cx).table_grid.as_ref().expect("rebuilt grid");
        let focused = grid.rows[1][0].entity_id();
        assert_eq!(editor.active_pane_focus().pending, Some(focused));
    });
}

#[gpui::test]
async fn setting_column_alignment_updates_table_data_and_selection(cx: &mut TestAppContext) {
    let markdown = ["| A | B |", "| --- | --- |", "| 1 | 2 |"].join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root").clone();
        editor.set_table_column_alignment(&table, 1, TableColumnAlignment::Right, cx);

        let table_data = table.read(cx).data.table.as_ref().expect("table data");
        assert_eq!(
            table_data.alignments,
            vec![TableColumnAlignment::Default, TableColumnAlignment::Right]
        );
        assert_eq!(
            editor.tab().tables.axis_selection,
            Some(crate::editor::engine::controller::TableAxisSelection {
                table_block_id: table.entity_id(),
                kind: crate::model::block::table::TableAxis::Column,
                index: 1,
            })
        );
    });
}

#[gpui::test]
async fn moving_table_row_updates_focus_and_selection(cx: &mut TestAppContext) {
    let markdown = ["| A | B |", "| --- | --- |", "| 1 | 2 |", "| 3 | 4 |"].join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root").clone();
        // Visual row 2 is the second body row; move it up above the first.
        editor.move_table_row(&table, 2, -1, cx);

        let table_data = table.read(cx).data.table.as_ref().expect("table data");
        assert_eq!(table_data.rows[0][0].serialize_markdown(), "3");
        assert_eq!(
            editor.tab().tables.axis_selection,
            Some(crate::editor::engine::controller::TableAxisSelection {
                table_block_id: table.entity_id(),
                kind: crate::model::block::table::TableAxis::Row,
                index: 1,
            })
        );

        let grid = table.read(cx).table_grid.as_ref().expect("rebuilt grid");
        assert_eq!(
            editor.active_pane_focus().pending,
            Some(grid.rows[0][0].entity_id())
        );
    });
}

#[gpui::test]
async fn moving_first_body_row_up_swaps_with_header(cx: &mut TestAppContext) {
    let markdown = ["| A | B |", "| --- | --- |", "| 1 | 2 |", "| 3 | 4 |"].join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root").clone();
        // Visual row 1 (first body row) moves up into the header position.
        editor.move_table_row(&table, 1, -1, cx);

        let table_data = table.read(cx).data.table.as_ref().expect("table data");
        assert_eq!(table_data.header[0].serialize_markdown(), "1");
        assert_eq!(table_data.rows[0][0].serialize_markdown(), "A");
        assert_eq!(
            editor.tab().tables.axis_selection,
            Some(crate::editor::engine::controller::TableAxisSelection {
                table_block_id: table.entity_id(),
                kind: crate::model::block::table::TableAxis::Row,
                index: 0,
            })
        );
    });
}

#[gpui::test]
async fn moving_header_row_down_swaps_with_first_body(cx: &mut TestAppContext) {
    let markdown = ["| A | B |", "| --- | --- |", "| 1 | 2 |", "| 3 | 4 |"].join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root").clone();
        // Visual row 0 (header) moves down, swapping with the first body row.
        editor.move_table_row(&table, 0, 1, cx);

        let table_data = table.read(cx).data.table.as_ref().expect("table data");
        assert_eq!(table_data.header[0].serialize_markdown(), "1");
        assert_eq!(table_data.rows[0][0].serialize_markdown(), "A");
        assert_eq!(
            editor.tab().tables.axis_selection,
            Some(crate::editor::engine::controller::TableAxisSelection {
                table_block_id: table.entity_id(),
                kind: crate::model::block::table::TableAxis::Row,
                index: 1,
            })
        );
    });
}

#[gpui::test]
async fn selecting_first_body_row_does_not_highlight_header(cx: &mut TestAppContext) {
    use crate::model::block::table::{TableAxis, TableAxisMarker};
    let markdown = ["| A | B |", "| --- | --- |", "| 1 | 2 |", "| 3 | 4 |"].join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root").clone();
        // Visual row 1 is the first body row; the header (row 0) must stay clear.
        editor.select_table_axis(table.entity_id(), TableAxis::Row, 1, cx);

        assert_eq!(
            table.read(cx).table_axis_selection,
            Some(TableAxisMarker {
                kind: TableAxis::Row,
                index: 1,
            })
        );
    });
}

#[gpui::test]
async fn selecting_header_row_highlights_only_header(cx: &mut TestAppContext) {
    use crate::model::block::table::{TableAxis, TableAxisMarker};
    let markdown = ["| A | B |", "| --- | --- |", "| 1 | 2 |"].join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root").clone();
        editor.select_table_axis(table.entity_id(), TableAxis::Row, 0, cx);

        assert_eq!(
            table.read(cx).table_axis_selection,
            Some(TableAxisMarker {
                kind: TableAxis::Row,
                index: 0,
            })
        );
    });
}

#[gpui::test]
async fn body_row_preview_survives_stale_header_leave(cx: &mut TestAppContext) {
    use crate::model::block::table::TableAxis;
    let markdown = ["| A | B |", "| --- | --- |", "| 1 | 2 |"].join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root").clone();
        let id = table.entity_id();

        // Pointer crosses from the header handle down onto the first body row.
        // The body handle's enter arrives first, then the header handle's leave;
        // the stale leave must not clear the preview the pointer moved onto.
        editor.preview_table_axis(id, TableAxis::Row, 1, true, cx);
        editor.preview_table_axis(id, TableAxis::Row, 0, false, cx);
        assert_eq!(
            editor.tab().tables.axis_preview,
            Some(crate::editor::engine::controller::TableAxisSelection {
                table_block_id: id,
                kind: TableAxis::Row,
                index: 1,
            }),
            "body row preview must survive the header's stale leave"
        );

        // Leaving the body handle that owns the preview still clears it.
        editor.preview_table_axis(id, TableAxis::Row, 1, false, cx);
        assert_eq!(editor.tab().tables.axis_preview, None);
    });
}

#[gpui::test]
async fn deleting_table_column_moves_selection_to_nearest_survivor(cx: &mut TestAppContext) {
    let markdown = ["| A | B | C |", "| --- | --- | --- |", "| 1 | 2 | 3 |"].join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root").clone();
        editor.delete_table_column(&table, 2, cx);

        let table_data = table.read(cx).data.table.as_ref().expect("table data");
        assert_eq!(table_data.header.len(), 2);
        assert_eq!(
            editor.tab().tables.axis_selection,
            Some(crate::editor::engine::controller::TableAxisSelection {
                table_block_id: table.entity_id(),
                kind: crate::model::block::table::TableAxis::Column,
                index: 1,
            })
        );
    });
}

#[gpui::test]
async fn deleting_table_header_promotes_next_row(cx: &mut TestAppContext) {
    let markdown = ["| A | B |", "| --- | --- |", "| 1 | 2 |"].join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root").clone();
        editor.delete_table_header_row(&table, cx);

        let table_data = table.read(cx).data.table.as_ref().expect("table data");
        assert_eq!(table_data.header[0].serialize_markdown(), "1");
        assert_eq!(table_data.header[1].serialize_markdown(), "2");
        assert!(table_data.rows.is_empty());

        let grid = table.read(cx).table_grid.as_ref().expect("rebuilt grid");
        assert_eq!(
            editor.active_pane_focus().pending,
            Some(grid.header[0].entity_id())
        );
    });
}

#[gpui::test]
async fn deleting_last_body_row_leaves_header_only_table(cx: &mut TestAppContext) {
    let markdown = ["| A | B |", "| --- | --- |", "| 1 | 2 |"].join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root").clone();
        // Deleting the only body row used to be blocked; now it leaves a
        // header-only table behind.
        editor.delete_table_row(&table, 0, cx);

        let table_data = table.read(cx).data.table.as_ref().expect("table data");
        assert!(table_data.rows.is_empty());
        assert_eq!(table_data.header[0].serialize_markdown(), "A");
        assert_eq!(editor.doc().root_count(), 1);
        assert_eq!(table.read(cx).kind(), BlockKind::Table);
    });
}

#[gpui::test]
async fn removing_table_block_replaces_it_with_empty_paragraph(cx: &mut TestAppContext) {
    let markdown = [
        "intro",
        "| A | B |",
        "| --- | --- |",
        "| 1 | 2 |",
        "outro",
    ]
    .join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        let table = editor.doc().root_blocks()[1].clone();
        assert_eq!(table.read(cx).kind(), BlockKind::Table);
        editor.remove_table_block(&table, cx);

        let roots = editor.doc().root_blocks();
        assert_eq!(roots.len(), 3);
        assert_eq!(roots[0].read(cx).display_text(), "intro");
        assert_eq!(roots[1].read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(roots[1].read(cx).display_text(), "");
        assert_eq!(roots[2].read(cx).display_text(), "outro");
        assert_eq!(
            editor.active_pane_focus().pending,
            Some(roots[1].entity_id())
        );
    });
}

#[gpui::test]
async fn removing_the_only_table_leaves_one_empty_paragraph(cx: &mut TestAppContext) {
    let markdown = ["| A | B |", "| --- | --- |", "| 1 | 2 |"].join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root").clone();
        editor.remove_table_block(&table, cx);

        let roots = editor.doc().root_blocks();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(roots[0].read(cx).display_text(), "");
    });
}

#[gpui::test]
async fn new_tab_table_grid_initialized_on_activation(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "Initial Tab".to_string(), None));

    editor.update(cx, |editor, cx| {
        let list = &mut editor.session_mut().tab_list;
        let table_md = ["| Header 1 | Header 2 |", "| --- | --- |", "| Row 1 | Row 2 |"].join("\n");
        list.push(Editor::new_tab_from_markdown(cx, table_md, None));
        editor.activate_tab(1, cx);
    });

    editor.read_with(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table block").clone();
        assert_eq!(table.read(cx).kind(), BlockKind::Table);
        let grid = table.read(cx).table_grid.as_ref().expect("table_grid must be initialized on tab switch");
        assert_eq!(grid.header.len(), 2);
        assert_eq!(grid.rows.len(), 1);
        assert_eq!(grid.header[0].read(cx).display_text(), "Header 1");
        assert_eq!(grid.rows[0][0].read(cx).display_text(), "Row 1");
    });
}

#[gpui::test]
async fn reordering_table_columns_swaps_columns(cx: &mut TestAppContext) {
    let markdown = ["| Col A | Col B |", "| --- | --- |", "| 1 | 2 |"].join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root").clone();
        editor.reorder_table_axis(&table, TableAxis::Column, 0, 1, cx);
    });

    editor.read_with(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root");
        let grid = table.read(cx).table_grid.as_ref().expect("grid");
        assert_eq!(grid.header[0].read(cx).display_text(), "Col B");
        assert_eq!(grid.header[1].read(cx).display_text(), "Col A");
        assert_eq!(grid.rows[0][0].read(cx).display_text(), "2");
        assert_eq!(grid.rows[0][1].read(cx).display_text(), "1");
    });
}

#[gpui::test]
async fn reordering_table_rows_swaps_rows(cx: &mut TestAppContext) {
    let markdown = ["| H1 | H2 |", "| --- | --- |", "| R1 | R2 |", "| R3 | R4 |"].join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root").clone();
        // Row 1 (R1, R2) and Row 2 (R3, R4)
        editor.reorder_table_axis(&table, TableAxis::Row, 1, 2, cx);
    });

    editor.read_with(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root");
        let grid = table.read(cx).table_grid.as_ref().expect("grid");
        assert_eq!(grid.rows[0][0].read(cx).display_text(), "R3");
        assert_eq!(grid.rows[0][1].read(cx).display_text(), "R4");
        assert_eq!(grid.rows[1][0].read(cx).display_text(), "R1");
        assert_eq!(grid.rows[1][1].read(cx).display_text(), "R2");
    });
}

#[gpui::test]
async fn inserting_table_column_at_boundary(cx: &mut TestAppContext) {
    let markdown = ["| Col A | Col B |", "| --- | --- |", "| 1 | 2 |"].join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root").clone();
        editor.insert_table_column_at(&table, 1, cx);
    });

    editor.read_with(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root");
        let grid = table.read(cx).table_grid.as_ref().expect("grid");
        assert_eq!(grid.header.len(), 3);
        assert_eq!(grid.header[0].read(cx).display_text(), "Col A");
        assert_eq!(grid.header[1].read(cx).display_text(), "");
        assert_eq!(grid.header[2].read(cx).display_text(), "Col B");
        assert_eq!(grid.rows[0][0].read(cx).display_text(), "1");
        assert_eq!(grid.rows[0][1].read(cx).display_text(), "");
        assert_eq!(grid.rows[0][2].read(cx).display_text(), "2");
    });
}

#[gpui::test]
async fn inserting_table_row_at_boundary(cx: &mut TestAppContext) {
    let markdown = ["| H1 | H2 |", "| --- | --- |", "| R1 | R2 |"].join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root").clone();
        editor.insert_table_row_at(&table, 1, cx);
    });

    editor.read_with(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root");
        let grid = table.read(cx).table_grid.as_ref().expect("grid");
        assert_eq!(grid.rows.len(), 2);
        assert_eq!(grid.rows[0][0].read(cx).display_text(), "");
        assert_eq!(grid.rows[1][0].read(cx).display_text(), "R1");
    });
}
