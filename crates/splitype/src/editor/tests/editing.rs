use gpui::{AppContext, KeyDownEvent, Keystroke, TestAppContext};
use std::time::{Duration, Instant};

use crate::editor::engine::controller::{Editor, EditorPaneKind};
use crate::editor::input::actions::{FocusNext, Newline};
use crate::editor::panes::document_pane::dialogs::TableInsertDialogState;
use splitype_model::parse::BlockKind;

use super::*;

#[gpui::test]
async fn toggle_pane_kind_preserves_paragraph_caret_position(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "alpha\n\nbeta".to_string(), None));

    editor.update(cx, |editor, cx| {
        let target = editor.doc().blocks()[2].entity.clone();
        target.update(cx, |block, _cx| {
            block.selected_range = 2..2;
        });
        editor.active_pane_state().as_wysiwyg_mut().unwrap().focus.active_entity = Some(target.entity_id());

        editor.toggle_pane_kind(cx);
        assert!(matches!(editor.active_pane_kind(), EditorPaneKind::SourceCode));
        let pane_id = editor.active_pane_id();
        let state = editor.pane_state_ref(pane_id).unwrap();
        assert_eq!(state.as_source_code().unwrap().text, "alpha\n\nbeta");

        editor.toggle_pane_kind(cx);
        assert!(matches!(editor.active_pane_kind(), EditorPaneKind::Wysiwyg));
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[2].entity.read(cx).display_text(), "beta");
        assert_eq!(entries[2].entity.read(cx).selected_range, 2..2);
    });
}

#[gpui::test]
async fn toggle_pane_kind_ends_stale_code_block_pointer_selection(cx: &mut TestAppContext) {
    let editor =
        cx.new(|cx| Editor::from_markdown(cx, "```rust\nfn main() {}\n```".to_string(), None));

    editor.update(cx, |editor, cx| {
        let target = editor.doc().blocks()[0].entity.clone();
        target.update(cx, |block, _cx| {
            block.selected_range = 3..7;
            block.is_selecting = true;
            block.code_toolbar.picker.is_selecting = true;
        });
        editor.active_pane_state().as_wysiwyg_mut().unwrap().focus.active_entity = Some(target.entity_id());

        editor.toggle_pane_kind(cx);

        assert!(matches!(editor.active_pane_kind(), EditorPaneKind::SourceCode));
        target.read_with(cx, |block, _cx| {
            assert!(!block.is_selecting);
            assert!(!block.code_toolbar.picker.is_selecting);
            assert_eq!(block.selected_range, 3..7);
        });
    });
}

#[gpui::test]
async fn ctrl_tab_toggles_pane_kind(cx: &mut TestAppContext) {
    init_editor_test_app(cx);
    let (editor, cx) =
        cx.add_window_view(|_window, cx| Editor::from_markdown(cx, "alpha".to_string(), None));

    focus_first_block(&editor, cx);
    cx.simulate_keystrokes("ctrl-tab");
    redraw(cx);

    editor.update(cx, |editor, _cx| {
        assert!(matches!(editor.active_pane_kind(), EditorPaneKind::SourceCode));
    });

    cx.simulate_keystrokes("ctrl-tab");
    redraw(cx);

    editor.update(cx, |editor, _cx| {
        assert!(matches!(editor.active_pane_kind(), EditorPaneKind::Wysiwyg));
    });
}

#[gpui::test]
async fn ctrl_a_selects_entire_source_document_in_source_mode(cx: &mut TestAppContext) {
    init_editor_test_app(cx);
    let (editor, cx) = cx.add_window_view(|_window, cx| {
        Editor::from_markdown(cx, "alpha\n\nbeta".to_string(), None)
    });

    editor.update(cx, |editor, cx| {
        editor.toggle_pane_kind(cx);
        assert!(matches!(editor.active_pane_kind(), EditorPaneKind::SourceCode));
        let pane_id = editor.active_pane_id();
        editor.sync_source_pane(pane_id, cx);
        if let Some(source) = editor.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
            source.move_to(1, false);
            source.move_to(3, true);
        }
        editor.execute_edit_command(crate::editor::commands::DocumentEditCommand::SelectAll, None, cx);
        let source = editor.pane_state_ref(pane_id).unwrap().as_source_code().unwrap();
        assert_eq!(source.selection, Some(0..source.text.len()));
    });
}

#[gpui::test]
async fn ctrl_a_selects_only_focused_block_text_in_wysiwyg_mode(cx: &mut TestAppContext) {
    init_editor_test_app(cx);
    let (editor, cx) = cx.add_window_view(|_window, cx| {
        Editor::from_markdown(cx, "alpha\n\nbeta".to_string(), None)
    });

    let block = editor.update(cx, |editor, cx| {
        let block = editor.doc().blocks()[1].entity.clone();
        block.update(cx, |block, _cx| {
            block.selected_range = 1..1;
        });
        block
    });
    focus_block(&editor, &block, cx);

    cx.simulate_keystrokes("ctrl-a");
    redraw(cx);

    editor.read_with(cx, |editor, cx| {
        let first = editor.doc().blocks()[0].entity.read(cx);
        let second = editor.doc().blocks()[1].entity.read(cx);
        assert_eq!(first.selected_range, 0..0);
        assert_eq!(second.selected_range, 0..second.display_len());
        assert!(editor.active_pane_selection().cross_block.is_none());
    });
}

#[gpui::test]
async fn repeated_ctrl_a_selects_all_rendered_blocks(cx: &mut TestAppContext) {
    init_editor_test_app(cx);
    let markdown =
        "alpha\n\n| a | b |\n| --- | --- |\n| 1 | 2 |\n\n```rust\nfn main() {}\n```\n\ngamma";
    let (editor, cx) = cx.add_window_view({
        let markdown = markdown.to_string();
        move |_window, cx| Editor::from_markdown(cx, markdown.clone(), None)
    });

    let block = editor.update(cx, |editor, cx| {
        let block = editor.doc().blocks()[0].entity.clone();
        block.update(cx, |block, block_cx| {
            block.move_to(0, block_cx);
        });
        block
    });
    focus_block(&editor, &block, cx);

    cx.simulate_keystrokes("ctrl-a");
    redraw(cx);

    editor.read_with(cx, |editor, cx| {
        let first = editor.doc().blocks()[0].entity.read(cx);
        assert_eq!(first.selected_range, 0..first.display_len());
        assert!(editor.active_pane_selection().cross_block.is_none());
    });

    cx.simulate_keystrokes("ctrl-a");
    redraw(cx);

    editor.read_with(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        let first_id = entries[0].entity.entity_id();
        let last = entries.last().expect("block entries");
        let last_id = last.entity.entity_id();
        let last_len = last.entity.read(cx).display_len();
        let selection = editor
            .active_pane_selection()
            .cross_block
            .expect("second Ctrl+A should select the rendered document");
        assert_eq!(selection.anchor.entity_id, first_id);
        assert_eq!(selection.anchor.offset, 0);
        assert_eq!(selection.focus.entity_id, last_id);
        assert_eq!(selection.focus.offset, last_len);
        for entries in entries {
            let block = entries.entity.read(cx);
            let len = block.display_len();
            if len > 0 {
                assert_eq!(block.editor_selection_range, Some(0..len));
            }
        }
    });

    let selected_after_second =
        editor.read_with(cx, |editor, _cx| editor.active_pane_selection().cross_block);
    cx.simulate_keystrokes("ctrl-a");
    redraw(cx);

    editor.read_with(cx, |editor, cx| {
        assert_eq!(
            editor.active_pane_selection().cross_block,
            selected_after_second,
            "third Ctrl+A should keep the full rendered document selected"
        );
        for entries in editor.doc().blocks() {
            let block = entries.entity.read(cx);
            let len = block.display_len();
            if len > 0 {
                assert_eq!(block.editor_selection_range, Some(0..len));
            }
        }
    });
}

#[gpui::test]
async fn rendered_ctrl_a_cycle_expires_before_second_press(cx: &mut TestAppContext) {
    init_editor_test_app(cx);
    let (editor, cx) = cx.add_window_view(|_window, cx| {
        Editor::from_markdown(cx, "alpha\n\nbeta".to_string(), None)
    });

    let block = editor.update(cx, |editor, cx| {
        let block = editor.doc().blocks()[1].entity.clone();
        block.update(cx, |block, block_cx| {
            block.move_to(1, block_cx);
        });
        block
    });
    focus_block(&editor, &block, cx);

    cx.simulate_keystrokes("ctrl-a");
    redraw(cx);

    editor.update(cx, |editor, cx| {
        let block = editor.doc().blocks()[1].entity.clone();
        block.update(cx, |block, _cx| {
            block.selected_range = 1..1;
        });
        let cycle = editor
            .active_pane_state()
            .selection
            .select_all_cycle
            .as_mut()
            .expect("first Ctrl+A should arm the rendered select-all cycle");
        cycle.last_pressed_at =
            Instant::now() - (Editor::WYSIWYG_SELECT_ALL_CYCLE_WINDOW + Duration::from_millis(1));
    });

    cx.simulate_keystrokes("ctrl-a");
    redraw(cx);

    editor.read_with(cx, |editor, cx| {
        let second = editor.doc().blocks()[1].entity.read(cx);
        assert_eq!(second.selected_range, 0..second.display_len());
        assert!(editor.active_pane_selection().cross_block.is_none());
        assert_eq!(
            editor
                .active_pane_selection()
                .select_all_cycle
                .expect("cycle should be reset by expired second press")
                .count,
            1
        );
    });
}

#[gpui::test]
async fn tab_key_inserts_tab_in_focused_paragraph(cx: &mut TestAppContext) {
    init_editor_test_app(cx);
    let (editor, cx) =
        cx.add_window_view(|_window, cx| Editor::from_markdown(cx, "ab".to_string(), None));

    let block = editor.update(cx, |editor, cx| {
        let block = editor.doc().blocks()[0].entity.clone();
        block.update(cx, |block, block_cx| {
            block.move_to(1, block_cx);
        });
        block
    });
    focus_block(&editor, &block, cx);

    cx.simulate_keystrokes("tab");
    redraw(cx);

    editor.update(cx, |editor, cx| {
        let block = editor.doc().blocks()[0].entity.clone();
        assert_eq!(block.read(cx).display_text(), "a    b");
        assert_eq!(editor.doc().serialize_markdown(cx), "a    b");
    });
}

#[gpui::test]
async fn tab_key_inserts_tab_in_focused_code_block(cx: &mut TestAppContext) {
    init_editor_test_app(cx);
    let (editor, cx) = cx.add_window_view(|_window, cx| {
        Editor::from_markdown(cx, "```rust\nab\n```".to_string(), None)
    });

    let block = editor.update(cx, |editor, cx| {
        let block = editor.doc().blocks()[0].entity.clone();
        block.update(cx, |block, block_cx| {
            block.move_to(1, block_cx);
        });
        block
    });
    focus_block(&editor, &block, cx);

    cx.simulate_keystrokes("tab");
    redraw(cx);

    editor.update(cx, |editor, cx| {
        let block = editor.doc().blocks()[0].entity.clone();
        assert_eq!(block.read(cx).display_text(), "a    b");
        assert_eq!(editor.doc().serialize_markdown(cx), "```rust\na    b\n```");
    });
}

#[gpui::test]
async fn captured_tab_key_inserts_visible_indent_in_paragraph(cx: &mut TestAppContext) {
    init_editor_test_app(cx);
    let (editor, cx) =
        cx.add_window_view(|_window, cx| Editor::from_markdown(cx, "ab".to_string(), None));

    editor.update(cx, |editor, cx| {
        let block = editor.doc().blocks()[0].entity.clone();
        editor.focus_block(block.entity_id());
        block.update(cx, |block, block_cx| {
            block.move_to(1, block_cx);
        });
    });
    redraw(cx);

    let event = KeyDownEvent {
        keystroke: Keystroke::parse("tab").expect("valid tab keystroke"),
        is_held: false,
        prefer_character_input: false,
    };
    editor.update_in(cx, |editor, window, cx| {
        editor.on_editor_key_down_capture(&event, window, cx);
    });
    redraw(cx);

    editor.update(cx, |editor, cx| {
        let block = editor.doc().blocks()[0].entity.clone();
        assert_eq!(block.read(cx).display_text(), "a    b");
    });
}

#[gpui::test]
async fn down_from_code_content_focuses_language_input(cx: &mut TestAppContext) {
    init_editor_test_app(cx);
    let (editor, cx) = cx.add_window_view(|_window, cx| {
        Editor::from_markdown(cx, "```rust\nab\n```".to_string(), None)
    });

    // Settle focus on the code content first (and clear any pending focus that a
    // later redraw would otherwise re-apply and steal back).
    editor.update_in(cx, |editor, _window, _cx| {
        let block = editor.doc().blocks()[0].entity.clone();
        editor.focus_block(block.entity_id());
    });
    redraw(cx);

    editor.update_in(cx, |editor, window, cx| {
        let block = editor.doc().blocks()[0].entity.clone();
        block.update(cx, |block, block_cx| {
            block.move_to(block.display_len(), block_cx);
            block.on_focus_next(&FocusNext, window, block_cx);
        });
        assert!(
            block.read(cx).code_language_focus_handle.is_focused(window),
            "Down from the last code line should focus the language field"
        );
    });
}

#[gpui::test]
async fn down_from_code_language_at_document_end_creates_trailing_paragraph(
    cx: &mut TestAppContext,
) {
    init_editor_test_app(cx);
    let (editor, cx) = cx.add_window_view(|_window, cx| {
        Editor::from_markdown(cx, "```rust\nab\n```".to_string(), None)
    });

    editor.update_in(cx, |editor, window, cx| {
        let block = editor.doc().blocks()[0].entity.clone();
        editor.focus_block(block.entity_id());
        block.update(cx, |block, block_cx| {
            block.code_language_focus_handle.focus(window, block_cx);
            block.on_code_language_focus_next(&FocusNext, window, block_cx);
        });
    });
    redraw(cx);

    editor.update(cx, |editor, cx| {
        let roots = editor.doc().root_blocks();
        assert_eq!(roots.len(), 2, "a trailing paragraph should be created");
        assert_eq!(roots[1].read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(roots[1].read(cx).display_text(), "");
    });
}

#[gpui::test]
async fn enter_in_code_language_does_not_exit_block(cx: &mut TestAppContext) {
    init_editor_test_app(cx);
    let (editor, cx) = cx.add_window_view(|_window, cx| {
        Editor::from_markdown(cx, "```rust\nab\n```".to_string(), None)
    });

    editor.update_in(cx, |editor, window, cx| {
        let block = editor.doc().blocks()[0].entity.clone();
        editor.focus_block(block.entity_id());
        block.update(cx, |block, block_cx| {
            block.code_language_focus_handle.focus(window, block_cx);
            block.on_code_language_newline(&Newline, window, block_cx);
        });
    });
    redraw(cx);

    editor.update(cx, |editor, _cx| {
        // Enter must not leave the block, so no trailing paragraph appears.
        assert_eq!(editor.doc().root_count(), 1);
    });
}

#[gpui::test]
async fn newline_at_start_of_heading_moves_entire_heading_down(cx: &mut TestAppContext) {
    init_editor_test_app(cx);
    let (editor, cx) =
        cx.add_window_view(|_window, cx| Editor::from_markdown(cx, "## 1111".to_string(), None));

    editor.update_in(cx, |editor, _window, cx| {
        let block = editor.doc().blocks()[0].entity.clone();
        block.update(cx, |block, block_cx| {
            block.move_to(0, block_cx);
        });
        editor.on_block_event(
            block,
            &crate::editor::document::protocol::BlockEvent::RequestNewlineAbove,
            cx,
        );
    });
    redraw(cx);

    editor.update(cx, |editor, cx| {
        assert_eq!(editor.doc().root_count(), 2);
        let blocks = editor.doc().blocks();
        assert_eq!(
            blocks[0].entity.read(cx).kind(),
            splitype_model::parse::BlockKind::Paragraph
        );
        assert_eq!(blocks[0].entity.read(cx).display_text(), "");
        assert_eq!(
            blocks[1].entity.read(cx).kind(),
            splitype_model::parse::BlockKind::Heading { level: 2 }
        );
        assert_eq!(blocks[1].entity.read(cx).display_text(), "1111");
    });
}

#[gpui::test]
async fn captured_tab_key_does_not_modify_code_language_input(cx: &mut TestAppContext) {
    init_editor_test_app(cx);
    let (editor, cx) = cx.add_window_view(|_window, cx| {
        Editor::from_markdown(cx, "```rust\nab\n```".to_string(), None)
    });

    editor.update_in(cx, |editor, window, cx| {
        let block = editor.doc().blocks()[0].entity.clone();
        editor.focus_block(block.entity_id());
        block.update(cx, |block, block_cx| {
            block.move_to(1, block_cx);
        });
        block.update(cx, |block, block_cx| {
            block.code_language_focus_handle.focus(window, block_cx);
        });
    });
    redraw(cx);

    editor.update_in(cx, |editor, window, cx| {
        let block = editor.doc().blocks()[0].entity.clone();
        block.update(cx, |block, block_cx| {
            block.code_language_focus_handle.focus(window, block_cx);
        });
    });

    let event = KeyDownEvent {
        keystroke: Keystroke::parse("tab").expect("valid tab keystroke"),
        is_held: false,
        prefer_character_input: false,
    };
    editor.update_in(cx, |editor, window, cx| {
        editor.on_editor_key_down_capture(&event, window, cx);
    });
    redraw(cx);

    editor.update(cx, |editor, cx| {
        let block = editor.doc().blocks()[0].entity.clone();
        let block = block.read(cx);
        assert_eq!(block.code_language_text(), "rust");
        assert_eq!(block.display_text(), "ab");
    });
}

#[gpui::test]
async fn tab_key_keeps_list_indent_semantics(cx: &mut TestAppContext) {
    init_editor_test_app(cx);
    let (editor, cx) =
        cx.add_window_view(|_window, cx| Editor::from_markdown(cx, "- a\n- b".to_string(), None));

    let second = editor.update(cx, |editor, cx| {
        let second = editor.doc().blocks()[1].entity.clone();
        second.update(cx, |block, block_cx| {
            block.move_to(block.display_len(), block_cx);
        });
        second
    });
    focus_block(&editor, &second, cx);

    cx.simulate_keystrokes("tab");
    redraw(cx);

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[1].entity.read(cx).render_depth, 1);
        assert_eq!(editor.doc().serialize_markdown(cx), "- a\n  - b");
    });
}

#[gpui::test]
async fn tab_key_keeps_table_cell_navigation(cx: &mut TestAppContext) {
    init_editor_test_app(cx);
    let markdown = ["| A | B |", "| --- | --- |", "| 1 | 2 |"].join("\n");
    let (editor, cx) =
        cx.add_window_view(move |_window, cx| Editor::from_markdown(cx, markdown, None));

    let (second_cell_id, first) = editor.update(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root").clone();
        let grid = table
            .read(cx)
            .table_grid
            .as_ref()
            .expect("table grid")
            .clone();
        let first = grid.rows[0][0].clone();
        let second = grid.rows[0][1].clone();
        first.update(cx, |block, block_cx| {
            block.move_to(block.display_len(), block_cx);
        });
        (second.entity_id(), first)
    });
    focus_block(&editor, &first, cx);

    cx.simulate_keystrokes("tab");
    redraw(cx);

    editor.update(cx, |editor, _cx| {
        assert_eq!(
            editor.active_pane_focus().active_entity,
            Some(second_cell_id)
        );
    });
}

#[gpui::test]
async fn right_arrow_at_cell_end_moves_to_next_cell(cx: &mut TestAppContext) {
    init_editor_test_app(cx);
    let markdown = ["| A | B |", "| --- | --- |", "| 1 | 2 |"].join("\n");
    let (editor, cx) =
        cx.add_window_view(move |_window, cx| Editor::from_markdown(cx, markdown, None));

    let (second_cell_id, first) = editor.update(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root").clone();
        let grid = table
            .read(cx)
            .table_grid
            .as_ref()
            .expect("table grid")
            .clone();
        let first = grid.rows[0][0].clone();
        let second = grid.rows[0][1].clone();
        first.update(cx, |block, block_cx| {
            block.move_to(block.display_len(), block_cx);
        });
        (second.entity_id(), first)
    });
    focus_block(&editor, &first, cx);

    cx.simulate_keystrokes("right");
    redraw(cx);

    editor.update(cx, |editor, _cx| {
        assert_eq!(
            editor.active_pane_focus().active_entity,
            Some(second_cell_id)
        );
    });
}

#[gpui::test]
async fn left_arrow_at_cell_start_moves_to_previous_cell(cx: &mut TestAppContext) {
    init_editor_test_app(cx);
    let markdown = ["| A | B |", "| --- | --- |", "| 1 | 2 |"].join("\n");
    let (editor, cx) =
        cx.add_window_view(move |_window, cx| Editor::from_markdown(cx, markdown, None));

    let (first_cell_id, second) = editor.update(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root").clone();
        let grid = table
            .read(cx)
            .table_grid
            .as_ref()
            .expect("table grid")
            .clone();
        let first = grid.rows[0][0].clone();
        let second = grid.rows[0][1].clone();
        second.update(cx, |block, block_cx| {
            block.move_to(0, block_cx);
        });
        (first.entity_id(), second)
    });
    focus_block(&editor, &second, cx);

    cx.simulate_keystrokes("left");
    redraw(cx);

    editor.update(cx, |editor, _cx| {
        assert_eq!(
            editor.active_pane_focus().active_entity,
            Some(first_cell_id)
        );
    });
}

#[gpui::test]
async fn inserting_table_at_document_end_adds_trailing_paragraph(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let editor = cx.new(|cx| Editor::from_markdown(cx, String::new(), None));

    cx.update(|_window, cx| {
        editor.update(cx, |editor, cx| {
            editor.table_insert_dialog = Some(TableInsertDialogState::new(
                None,
                3,
                2,
                None,
            ));
            editor.insert_table_from_dialog(3, 2, cx);
        });
    });

    editor.update(cx, |editor, cx| {
        let block = editor.doc().blocks().first().unwrap().entity.clone();
        let text = block.read(cx).display_text();
        assert!(text.contains("| --- | --- |"));
        assert!(text.contains("|  |  |"));
    });
}

#[gpui::test]
async fn ctrl_enter_exits_focused_math_block(cx: &mut TestAppContext) {
    init_editor_test_app(cx);
    let (editor, cx) =
        cx.add_window_view(|_window, cx| Editor::from_markdown(cx, "$$n^2$$".to_string(), None));

    let block = editor.update(cx, |editor, cx| {
        let block = editor.doc().blocks()[0].entity.clone();
        block.update(cx, |block, block_cx| {
            block.move_to(block.display_len(), block_cx);
        });
        block
    });
    focus_block(&editor, &block, cx);

    cx.simulate_keystrokes("ctrl-enter");
    redraw(cx);

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].entity.read(cx).kind(), BlockKind::MathBlock);
        assert_eq!(entries[0].entity.read(cx).display_text(), "n^2");
        assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[1].entity.read(cx).display_text(), "");
        assert_eq!(editor.doc().serialize_markdown(cx), "$$n^2$$\n");
    });
}

#[gpui::test]
async fn ctrl_enter_exits_focused_table_cell(cx: &mut TestAppContext) {
    init_editor_test_app(cx);
    let markdown = ["| A | B |", "| --- | --- |", "| 1 | 2 |"].join("\n");
    let (editor, cx) =
        cx.add_window_view(move |_window, cx| Editor::from_markdown(cx, markdown, None));

    let cell = editor.update(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root").clone();
        let cell = table.read(cx).table_grid.as_ref().expect("table grid").rows[0][0].clone();
        cell.update(cx, |block, block_cx| {
            block.move_to(block.display_len(), block_cx);
        });
        cell
    });
    focus_block(&editor, &cell, cx);

    cx.simulate_keystrokes("ctrl-enter");
    redraw(cx);

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].entity.read(cx).kind(), BlockKind::Table);
        assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[1].entity.read(cx).display_text(), "");
        assert_eq!(
            editor.active_pane_focus().active_entity,
            Some(entries[1].entity.entity_id())
        );
    });
}

#[gpui::test]
async fn ending_editor_pointer_selection_sessions_keeps_normal_selection(cx: &mut TestAppContext) {
    let editor =
        cx.new(|cx| Editor::from_markdown(cx, "```rust\nfn main() {}\n```".to_string(), None));

    editor.update(cx, |editor, cx| {
        let target = editor.doc().blocks()[0].entity.clone();
        target.update(cx, |block, _cx| {
            block.selected_range = 3..7;
            block.marked_range = Some(4..6);
            block.is_selecting = true;
        });
        editor.active_pane_state().as_wysiwyg_mut().unwrap().focus.active_entity = Some(target.entity_id());

        assert!(editor.end_block_pointer_selection_sessions(cx));
        target.read_with(cx, |block, _cx| {
            assert!(!block.is_selecting);
            assert_eq!(block.selected_range, 3..7);
            assert_eq!(block.marked_range, Some(4..6));
        });

        assert!(!editor.end_block_pointer_selection_sessions(cx));
    });
}

#[gpui::test]
async fn toggle_pane_kind_preserves_table_cell_position(cx: &mut TestAppContext) {
    let markdown = ["| Name | Value |", "| --- | --- |", "| alpha | beta |"].join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        let table = editor.doc().first_root().expect("table root").clone();
        let cell = table.read(cx).table_grid.as_ref().expect("table grid").rows[0][1].clone();
        cell.update(cx, |block, _cx| {
            block.selected_range = 2..2;
        });
        editor.active_pane_state().as_wysiwyg_mut().unwrap().focus.active_entity = Some(cell.entity_id());

        editor.toggle_pane_kind(cx);
        assert!(matches!(editor.active_pane_kind(), EditorPaneKind::SourceCode));

        editor.toggle_pane_kind(cx);
        assert!(matches!(editor.active_pane_kind(), EditorPaneKind::Wysiwyg));
        let restored_table = editor.doc().first_root().expect("restored table").clone();
        let restored_cell = restored_table
            .read(cx)
            .table_grid
            .as_ref()
            .expect("restored grid")
            .rows[0][1]
            .clone();
        assert_eq!(restored_cell.read(cx).display_text(), "beta");
        assert_eq!(restored_cell.read(cx).selected_range, 2..2);
        assert_eq!(
            editor.active_pane_focus().pending,
            Some(restored_cell.entity_id())
        );
    });
}

#[gpui::test]
async fn toggle_pane_kind_preserves_callout_table_cell_position(cx: &mut TestAppContext) {
    let markdown = [
        "> [!NOTE]",
        "> | Name | Value |",
        "> | --- | --- |",
        "> | alpha | beta |",
    ]
    .join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        let callout = editor.doc().first_root().expect("callout root").clone();
        let table = callout
            .read(cx)
            .children
            .iter()
            .find(|child| child.read(cx).kind() == BlockKind::Table)
            .expect("nested table child")
            .clone();
        let cell = table.read(cx).table_grid.as_ref().expect("table grid").rows[0][1].clone();
        cell.update(cx, |block, _cx| {
            block.selected_range = 2..2;
        });
        editor.active_pane_state().as_wysiwyg_mut().unwrap().focus.active_entity = Some(cell.entity_id());

        editor.toggle_pane_kind(cx);
        assert!(matches!(editor.active_pane_kind(), EditorPaneKind::SourceCode));

        editor.toggle_pane_kind(cx);
        assert!(matches!(editor.active_pane_kind(), EditorPaneKind::Wysiwyg));
        let restored_callout = editor.doc().first_root().expect("restored callout").clone();
        let restored_table = restored_callout
            .read(cx)
            .children
            .iter()
            .find(|child| child.read(cx).kind() == BlockKind::Table)
            .expect("restored nested table")
            .clone();
        let restored_cell = restored_table
            .read(cx)
            .table_grid
            .as_ref()
            .expect("restored grid")
            .rows[0][1]
            .clone();
        assert_eq!(restored_cell.read(cx).display_text(), "beta");
        assert_eq!(restored_cell.read(cx).selected_range, 2..2);
        assert_eq!(
            editor.active_pane_focus().pending,
            Some(restored_cell.entity_id())
        );
    });
}

#[gpui::test]
async fn callout_header_unfocused_label_and_focus_projection_offset(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "> [!WARNING]".to_string(), None));

    editor.update(cx, |editor, cx| {
        let callout = editor.doc().first_root().expect("callout root").clone();
        callout.update(cx, |block, _cx| {
            assert_eq!(block.kind(), BlockKind::Callout(splitype_model::block::CalloutKind::Warning));
            // In transparent delimiter design, text remains "[!warning]" in both unfocused and focused states
            // with [! and ] rendered transparently in unfocused mode
            assert_eq!(block.display_text(), "[!warning]");

            // Click at character offset 4 ('r' in "[!warning]")
            block.selected_range = 4..4;
            block.sync_inline_projection_for_focus(true);

            // When focused, text is "[!warning]" and offset remains precisely 4
            assert_eq!(block.display_text(), "[!warning]");
            assert_eq!(block.selected_range, 4..4);

            // Crucial: Subsequent render passes stably preserve the caret at 4 ('r'), not reset
            block.sync_inline_projection_for_focus(true);
            assert_eq!(block.selected_range, 4..4);

            // While focused, moving/clicking inside the prefix (e.g. offset 7 'i') remains stable
            block.selected_range = 7..7;
            block.sync_inline_projection_for_focus(true);
            assert_eq!(block.selected_range, 7..7);

            // Unfocusing keeps display text as "[!warning]"
            block.sync_inline_projection_for_focus(false);
            assert_eq!(block.display_text(), "[!warning]");
        });
    });
}

#[gpui::test]
async fn callout_header_with_custom_title_projection_offset(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "> [!WARNING] Custom Title".to_string(), None));

    editor.update(cx, |editor, cx| {
        let callout = editor.doc().first_root().expect("callout root").clone();
        callout.update(cx, |block, _cx| {
            assert_eq!(block.kind(), BlockKind::Callout(splitype_model::block::CalloutKind::Warning));
            // When unfocused, displays custom title
            assert_eq!(block.display_text(), "Custom Title");

            // Click at character offset 7 (' ' in "Custom Title")
            block.selected_range = 7..7;
            block.sync_inline_projection_for_focus(true);

            // When focused, expands to "[!warning] Custom Title"
            assert_eq!(block.display_text(), "[!warning] Custom Title");
            // Prefix "[!warning] " has length 11, so 11 + 7 = 18
            assert_eq!(block.selected_range, 18..18);

            // Subsequent render passes preserve caret in body text
            block.sync_inline_projection_for_focus(true);
            assert_eq!(block.selected_range, 18..18);

            // While focused, clicking inside the prefix (e.g. offset 4 'r') preserves prefix caret position
            block.selected_range = 4..4;
            block.sync_inline_projection_for_focus(true);
            assert_eq!(block.selected_range, 4..4);
        });
    });
}

#[gpui::test]
async fn callout_header_prefix_editing_updates_variant(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "> [!WARNING]".to_string(), None));

    editor.update(cx, |editor, cx| {
        let callout = editor.doc().first_root().expect("callout root").clone();
        callout.update(cx, |block, cx| {
            block.sync_inline_projection_for_focus(true);
            assert_eq!(block.display_text(), "[!warning]");

            // Edit "warning" to "note" (replace range 2..9 with "note")
            block.replace_text_in_display_range(2..9, "note", None, false, cx);
            assert_eq!(block.kind(), BlockKind::Callout(splitype_model::block::CalloutKind::Note));
            assert_eq!(block.display_text(), "[!note]");
        });

        assert_eq!(editor.doc().serialize_markdown(cx), "> [!NOTE]");
    });
}

#[gpui::test]
async fn callout_header_prefix_deletion_downgrades_to_quote(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "> [!WARNING]".to_string(), None));

    editor.update(cx, |editor, cx| {
        let callout = editor.doc().first_root().expect("callout root").clone();
        callout.update(cx, |block, cx| {
            block.sync_inline_projection_for_focus(true);
            assert_eq!(block.display_text(), "[!warning]");

            // Delete entire prefix and replace with "plain quote"
            block.replace_text_in_display_range(0..10, "plain quote", None, false, cx);
            assert_eq!(block.kind(), BlockKind::Blockquote);
            assert_eq!(block.display_text(), "plain quote");
        });
    });
}

#[gpui::test]
async fn callout_header_text_runs_have_purple_delimiters_and_accent_type(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "> [!WARNING]".to_string(), None));

    editor.update(cx, |editor, cx| {
        let callout = editor.doc().first_root().expect("callout root").clone();
        callout.update(cx, |block, _cx| {
            block.sync_inline_projection_for_focus(true);
            let display_text = gpui::SharedString::from(block.display_text().to_string());
            assert_eq!(display_text.as_ref(), "[!warning]");

            let accent = gpui::Hsla::from(gpui::rgba(0xf97316ff)); // orange
            let purple = gpui::Hsla::from(gpui::rgba(0xa855f7ff)); // purple
            let base_run = gpui::TextRun {
                len: display_text.len(),
                font: gpui::font("Segoe UI"),
                color: accent,
                background_color: None,
                underline: None,
                strikethrough: None,
            };

            let runs = crate::editor::panes::wysiwyg::render::inline::shaping::build_text_runs(
                block,
                &display_text,
                &base_run,
                gpui::px(1.0),
                accent,
                purple,
                purple,
                purple,
            );

            // Runs:
            // 0..2: "[!" -> purple
            // 2..9: "warning" -> accent (orange)
            // 9..10: "]" -> purple
            assert_eq!(runs.len(), 3);
            assert_eq!(runs[0].len, 2);
            assert_eq!(runs[0].color, purple);
            assert_eq!(runs[1].len, 7);
            assert_eq!(runs[1].color, accent);
            assert_eq!(runs[2].len, 1);
            assert_eq!(runs[2].color, purple);
        });
    });
}

#[gpui::test]
async fn callout_break_and_retype_syntax_reenters_callout(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "> [!WARNING] Watch out!".to_string(), None));

    editor.update(cx, |editor, cx| {
        let callout = editor.doc().first_root().expect("callout root").clone();
        callout.update(cx, |block, cx| {
            block.sync_inline_projection_for_focus(true);
            assert_eq!(block.display_text(), "[!warning] Watch out!");
            assert_eq!(block.kind(), BlockKind::Callout(splitype_model::block::CalloutKind::Warning));

            // 1. Break syntax by deleting 'g' at offset 8..9 -> "[!warnin] Watch out!"
            block.replace_text_in_display_range(8..9, "", None, false, cx);
            assert_eq!(block.kind(), BlockKind::Blockquote);
            assert_eq!(block.display_text(), "[!warnin] Watch out!");

            // 2. Type 'g' back at offset 8..8 -> "[!warning] Watch out!"
            block.replace_text_in_display_range(8..8, "g", None, false, cx);
            assert_eq!(block.kind(), BlockKind::Callout(splitype_model::block::CalloutKind::Warning));
            assert_eq!(block.display_text(), "[!warning] Watch out!");

            // 3. Change 'warning' to 'tip' (range 2..9 replaced by 'tip')
            block.replace_text_in_display_range(2..9, "tip", None, false, cx);
            assert_eq!(block.kind(), BlockKind::Callout(splitype_model::block::CalloutKind::Tip));
            assert_eq!(block.display_text(), "[!tip] Watch out!");
        });

        assert_eq!(editor.doc().serialize_markdown(cx), "> [!TIP] Watch out!");
    });
}

#[gpui::test]
async fn image_focus_expands_to_source_syntax_with_markers(cx: &mut TestAppContext) {
    let markdown = "![diagram](https://example.com/a.png)".to_string();
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown.clone(), None));

    editor.update(cx, |editor, cx| {
        let block = editor.doc().first_root().expect("first root").clone();
        block.update(cx, |block, _cx| {
            assert!(block.is_showing_rendered_image());

            block.sync_inline_projection_for_focus(true);
            assert_eq!(block.display_text(), "![diagram](https://example.com/a.png)");

            let text_color = gpui::Hsla::from(gpui::rgba(0x111111ff));
            let purple = gpui::Hsla::from(gpui::rgba(0xa855f7ff));
            let base_run = gpui::TextRun {
                len: block.display_text().len(),
                font: gpui::font("Segoe UI"),
                color: text_color,
                background_color: None,
                underline: None,
                strikethrough: None,
            };

            let display_text = gpui::SharedString::from(block.display_text().to_string());
            let runs = crate::editor::panes::wysiwyg::render::inline::shaping::build_text_runs(
                block,
                &display_text,
                &base_run,
                gpui::px(1.0),
                purple,
                purple,
                purple,
                purple,
            );

            // Delimiters "![" (0..2), "](" (9..11), ")" (36..37) are purple markers
            assert_eq!(runs[0].len, 2);
            assert_eq!(runs[0].color, purple); // "!["
            assert_eq!(runs[1].len, 7); // "diagram"
            assert_eq!(runs[2].len, 2); // "]("
            assert_eq!(runs[2].color, purple);
            assert_eq!(runs[3].len, 25); // "https://example.com/a.png"
            assert_eq!(runs[4].len, 1); // ")"
            assert_eq!(runs[4].color, purple);
        });

        assert_eq!(editor.doc().serialize_markdown(cx), markdown);
    });
}

#[gpui::test]
async fn empty_alt_image_focus_expands_to_source_syntax(cx: &mut TestAppContext) {
    let markdown = "![](https://example.com/b.png)".to_string();
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown.clone(), None));

    editor.update(cx, |editor, cx| {
        let block = editor.doc().first_root().expect("first root").clone();
        block.update(cx, |block, _cx| {
            block.sync_inline_projection_for_focus(true);
            assert_eq!(block.display_text(), "![](https://example.com/b.png)");
        });

        assert_eq!(editor.doc().serialize_markdown(cx), markdown);
    });
}

#[gpui::test]
async fn editing_image_syntax_updates_markdown_and_handle(cx: &mut TestAppContext) {
    let markdown = "![old](https://example.com/old.png)".to_string();
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        let block = editor.doc().first_root().expect("first root").clone();
        block.update(cx, |block, cx| {
            block.sync_inline_projection_for_focus(true);
            assert_eq!(block.display_text(), "![old](https://example.com/old.png)");

            // Replace "old" alt text (range 2..5) with "new_diagram"
            block.replace_text_in_display_range(2..5, "new_diagram", None, false, cx);
            assert_eq!(block.display_text(), "![new_diagram](https://example.com/old.png)");
        });

        assert_eq!(
            editor.doc().serialize_markdown(cx),
            "![new_diagram](https://example.com/old.png)"
        );
        let handle = block.read(cx).image_handle().expect("image handle");
        assert_eq!(handle.alt, "new_diagram");
    });
}

#[gpui::test]
async fn table_insert_dialog_matrix_hover_and_confirmation(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "".to_string(), None));

    editor.update(cx, |editor, cx| {
        // Open table insert dialog with initial 4 rows, 3 columns
        editor.table_insert_dialog = Some(TableInsertDialogState::new(
            None,
            4,
            3,
            None,
        ));
        let dialog = editor.table_insert_dialog.as_ref().unwrap();
        assert_eq!(dialog.rows, 4);
        assert_eq!(dialog.columns, 3);

        // 1. Hover over 6 rows, 5 cols
        editor.set_table_insert_hover(Some(6), Some(5), cx);
        assert_eq!(editor.table_insert_dialog.as_ref().unwrap().hovered_rows, Some(6));
        assert_eq!(editor.table_insert_dialog.as_ref().unwrap().hovered_cols, Some(5));
        assert_eq!(editor.table_insert_dialog.as_ref().unwrap().rows, 6);
        assert_eq!(editor.table_insert_dialog.as_ref().unwrap().columns, 5);

        // 2. Hover over 5 rows, 4 cols
        editor.set_table_insert_hover(Some(5), Some(4), cx);
        assert_eq!(editor.table_insert_dialog.as_ref().unwrap().rows, 5);
        assert_eq!(editor.table_insert_dialog.as_ref().unwrap().columns, 4);

        // 3. Press Enter to confirm insert
        let key_enter = KeyDownEvent {
            keystroke: Keystroke::parse("enter").expect("valid keystroke enter"),
            is_held: false,
            prefer_character_input: false,
        };
        editor.handle_table_insert_key_down(&key_enter, cx);
        assert!(editor.table_insert_dialog.is_none());
        let block = editor.doc().blocks().first().unwrap().entity.clone();
        let text = block.read(cx).display_text();
        assert!(text.contains("| --- |"));
    });
}

#[gpui::test]
async fn table_insert_dialog_escape_cancels(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "".to_string(), None));

    editor.update(cx, |editor, cx| {
        editor.table_insert_dialog = Some(TableInsertDialogState::new(
            None,
            4,
            3,
            None,
        ));
        let key_escape = KeyDownEvent {
            keystroke: Keystroke::parse("escape").expect("valid keystroke escape"),
            is_held: false,
            prefer_character_input: false,
        };
        editor.handle_table_insert_key_down(&key_escape, cx);
        assert!(editor.table_insert_dialog.is_none());
    });
}

#[gpui::test]
async fn table_insert_dialog_direct_cell_click_inserts_table(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "".to_string(), None));

    editor.update(cx, |editor, cx| {
        editor.table_insert_dialog = Some(TableInsertDialogState::new(
            None,
            4,
            3,
            None,
        ));
        // Clicking cell (5, 4) directly inserts table and closes dialog
        editor.insert_table_from_dialog(5, 4, cx);
        assert!(editor.table_insert_dialog.is_none());
        let block = editor.doc().blocks().first().unwrap().entity.clone();
        let text = block.read(cx).display_text();
        assert!(text.contains("| --- |"));
    });
}


