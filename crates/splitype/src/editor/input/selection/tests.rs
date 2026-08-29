//! Integration tests for cross-block selection, CJK character boundaries, and table deletion.

#[cfg(test)]
mod tests {
    use gpui::{AppContext, Bounds, Context, TestAppContext, point, px, size};

    use crate::editor::document::protocol::UndoCaptureKind;
    use crate::editor::engine::controller::{
        CrossBlockSelection, CrossBlockSelectionEndpoint, Editor, EditorSelection,
    };
    use crate::editor::input::actions::{Cut, Undo};
    use i18n::I18nManager;
    use theme::ThemeManager;
    use markdown::parse::BlockKind;

    fn init_editor_test_app(cx: &mut TestAppContext) {
        cx.update(|cx| {
            I18nManager::init(cx);
            ThemeManager::init(cx);
            crate::keybindings::init(cx);
        });
    }

    fn redraw(cx: &mut gpui::VisualTestContext) {
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.run_until_parked();
    }

    fn set_selection(
        editor: &mut Editor,
        start_index: usize,
        start_offset: usize,
        end_index: usize,
        end_offset: usize,
        cx: &mut Context<Editor>,
    ) {
        let entries = editor.doc().blocks().to_vec();
        let start = entries[start_index].entity.entity_id();
        let end = entries[end_index].entity.entity_id();
        editor.active_pane_state().selection.cross_block = Some(CrossBlockSelection {
            anchor: CrossBlockSelectionEndpoint {
                entity_id: start,
                offset: start_offset,
            },
            focus: CrossBlockSelectionEndpoint {
                entity_id: end,
                offset: end_offset,
            },
        });
        editor.sync_cross_block_selection_visuals(cx);
    }

    fn assign_visible_block_bounds(editor: &mut Editor, cx: &mut Context<Editor>) {
        for (index, entries) in editor.doc().blocks().to_vec().into_iter().enumerate() {
            entries.entity.update(cx, move |block, _cx| {
                block.push_last_paint(
                    Bounds::new(
                        point(px(0.0), px(index as f32 * 32.0)),
                        size(px(400.0), px(24.0)),
                    ),
                    Vec::new(),
                    px(24.0),
                );
            });
        }
    }

    #[test]
    fn mouse_down_starts_cross_block_drag_after_clearing_old_selection() {
        let mut cx = TestAppContext::single();
        init_editor_test_app(&mut cx);
        let editor =
            cx.new(|cx| Editor::from_markdown(cx, "alpha\n\nbeta\n\ngamma".to_string(), None));

        editor.update(&mut cx, |editor, cx| {
            assign_visible_block_bounds(editor, cx);
            let last_index = editor.doc().blocks().len() - 1;
            set_selection(editor, 0, 0, last_index, 2, cx);
            assert!(editor.active_pane_selection().cross_block.is_some());
            assert!(
                editor.doc().blocks().iter().any(|entries| entries
                    .entity
                    .read(cx)
                    .editor_selection_range
                    .is_some())
            );

            editor.begin_cross_block_drag_at_point(
                editor.active_pane_id(),
                point(px(8.0), px(4.0)),
                cx,
            );

            assert!(editor.active_pane_selection().cross_block.is_none());
            assert!(editor.active_pane_selection().cross_block_drag.is_some());
            assert!(
                editor.doc().blocks().iter().all(|entries| entries
                    .entity
                    .read(cx)
                    .editor_selection_range
                    .is_none())
            );
        });
        cx.quit();
    }

    #[test]
    fn typing_replaces_cross_block_selection_with_plain_text() {
        let mut cx = TestAppContext::single();
        init_editor_test_app(&mut cx);
        let editor =
            cx.new(|cx| Editor::from_markdown(cx, "alpha\n\nbeta\n\ngamma".to_string(), None));

        editor.update(&mut cx, |editor, cx| {
            let last_index = editor.doc().blocks().len() - 1;
            set_selection(editor, 0, 2, last_index, 2, cx);
            assert!(editor.replace_cross_block_selection_with_text(
                "X",
                None,
                false,
                UndoCaptureKind::CoalescibleText,
                cx
            ));

            assert_eq!(editor.doc().serialize_markdown(cx), "alXmma");
            assert!(editor.active_pane_selection().cross_block.is_none());
            assert!(editor.active_pane_selection().cross_block_drag.is_none());
            let block = editor.doc().blocks()[0].entity.read(cx);
            assert_eq!(block.selected_range, 3..3);
            assert!(block.marked_range.is_none());
        });
        cx.quit();
    }

    #[test]
    fn ime_composition_replaces_cross_block_selection_and_marks_inserted_text() {
        let mut cx = TestAppContext::single();
        init_editor_test_app(&mut cx);
        let editor =
            cx.new(|cx| Editor::from_markdown(cx, "alpha\n\nbeta\n\ngamma".to_string(), None));

        editor.update(&mut cx, |editor, cx| {
            let last_index = editor.doc().blocks().len() - 1;
            set_selection(editor, 0, 2, last_index, 2, cx);
            assert!(editor.replace_cross_block_selection_with_text(
                "ni",
                Some(2..2),
                true,
                UndoCaptureKind::CoalescibleText,
                cx
            ));

            assert_eq!(editor.doc().serialize_markdown(cx), "alnimma");
            let block = editor.doc().blocks()[0].entity.read(cx);
            assert_eq!(block.selected_range, 4..4);
            assert_eq!(block.marked_range, Some(2..4));
            assert!(block.editor_selection_range.is_none());
        });
        cx.quit();
    }

    #[test]
    fn cross_block_selection_marks_visual_ranges_and_copies_markdown() {
        let mut cx = TestAppContext::single();
        init_editor_test_app(&mut cx);
        let editor = cx.new(|cx| {
            Editor::from_markdown(
                cx,
                "alpha **bold**\n\n- item\n\n![alt](image.png)".to_string(),
                None,
            )
        });

        editor.update(&mut cx, |editor, cx| {
            let entries = editor.doc().blocks().to_vec();
            let last_index = entries.len() - 1;
            let end_len = entries[last_index].entity.read(cx).display_len();
            set_selection(editor, 0, 0, last_index, end_len, cx);

            assert_eq!(
                editor.cross_block_selected_markdown(cx).as_deref(),
                Some("alpha **bold**\n\n- item\n\n![alt](image.png)")
            );
            for entries in entries {
                let block = entries.entity.read(cx);
                if block.display_len() > 0 {
                    assert_eq!(block.editor_selection_range, Some(0..block.display_len()));
                }
            }
        });
        cx.quit();
    }

    #[test]
    fn cross_block_selection_with_chinese_footnote_definitions_maps_source_correctly() {
        let mut cx = TestAppContext::single();
        init_editor_test_app(&mut cx);
        let editor = cx.new(|cx| {
            Editor::from_markdown(
                cx,
                "正文段落测试[^note]\n\n[^note]: 脚注内容测试文字".to_string(),
                None,
            )
        });

        editor.update(&mut cx, |editor, cx| {
            let entries = editor.doc().blocks().to_vec();
            let last_index = entries.len() - 1;
            let end_len = entries[last_index].entity.read(cx).display_len();
            // Full-block selection including the footnote definition: the
            // source mapping must slice the serialized source on char
            // boundaries even with multibyte (CJK) text.
            set_selection(editor, 0, 0, last_index, end_len, cx);
            assert_eq!(
                editor.cross_block_selected_markdown(cx).as_deref(),
                Some("正文段落测试[^note]\n\n[^note]: 脚注内容测试文字")
            );

            // Partial selection inside the footnote definition row.
            let def_index = entries
                .iter()
                .position(|entries| entries.entity.read(cx).kind() == BlockKind::FootnoteDefinition)
                .expect("footnote definition");
            // Selecting part of the id maps into the `[^…]` label.
            set_selection(editor, def_index, 0, def_index, 4, cx);
            assert_eq!(
                editor.cross_block_selected_markdown(cx).as_deref(),
                Some("[^note")
            );
            // Selecting part of the content maps after `]: `. Byte offsets:
            // `note: ` is 6 bytes, each CJK char is 3 bytes.
            set_selection(editor, def_index, 6, def_index, 18, cx);
            assert_eq!(
                editor.cross_block_selected_markdown(cx).as_deref(),
                Some("脚注内容")
            );
        });
        cx.quit();
    }

    #[test]
    fn cross_block_cut_writes_markdown_deletes_range_and_undo_restores() {
        let mut cx = TestAppContext::single();
        init_editor_test_app(&mut cx);
        let original = "alpha\n\nbeta\n\ngamma";
        let (editor, cx) = cx.add_window_view({
            let original = original.to_string();
            move |_window, cx| Editor::from_markdown(cx, original.clone(), None)
        });
        redraw(cx);

        editor.update(cx, |editor, cx| {
            let last_index = editor.doc().blocks().len() - 1;
            set_selection(editor, 0, 2, last_index, 2, cx);
            assert_eq!(
                editor.cross_block_selected_markdown(cx).as_deref(),
                Some("pha\n\nbeta\n\nga")
            );
        });
        redraw(cx);

        // Dispatch along the focused path needs a keyboard-driven focus;
        // the capture handler itself is what the test exercises, so call it
        // directly (same code path the ctrl-x binding reaches in the UI).
        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                editor.on_cut_capture(&Cut, window, cx);
            });
        });
        redraw(cx);

        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some("pha\n\nbeta\n\nga")
        );
        assert_eq!(
            editor.read_with(cx, |editor, cx| editor.doc().serialize_markdown(cx)),
            "almma"
        );

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                editor.on_undo(&Undo, window, cx);
            });
        });
        redraw(cx);

        assert_eq!(
            editor.read_with(cx, |editor, cx| editor.doc().serialize_markdown(cx)),
            original
        );
        editor.read_with(cx, |editor, cx| {
            assert_eq!(
                editor.cross_block_selected_markdown(cx).as_deref(),
                Some("pha\n\nbeta\n\nga")
            );
        });
        cx.quit();
    }

    const TABLE_DOC: &str = "alpha\n\n| a | b |\n| --- | --- |\n| 1 | 2 |\n\ngamma";

    #[test]
    fn delete_selection_spanning_table_removes_table() {
        let mut cx = TestAppContext::single();
        init_editor_test_app(&mut cx);
        let editor = cx.new(|cx| Editor::from_markdown(cx, TABLE_DOC.to_string(), None));

        editor.update(&mut cx, |editor, cx| {
            let entries = editor.doc().blocks().to_vec();
            let last_index = entries.len() - 1;
            let end_len = entries[last_index].entity.read(cx).display_len();
            // The table sits in the interior of the selection.
            set_selection(editor, 0, 0, last_index, end_len, cx);
            assert!(editor.delete_cross_block_selection(cx));

            let text = editor.doc().serialize_markdown(cx);
            assert!(!text.contains('|'), "table should be gone: {text:?}");
            assert!(!text.contains("alpha"));
            assert!(!text.contains("gamma"));
        });
        cx.quit();
    }

    #[test]
    fn delete_selection_with_trailing_table_removes_table() {
        let mut cx = TestAppContext::single();
        init_editor_test_app(&mut cx);
        let editor = cx.new(|cx| Editor::from_markdown(cx, TABLE_DOC.to_string(), None));

        editor.update(&mut cx, |editor, cx| {
            // Table is at index 2 (alpha is 0, empty is 1, table is 2).
            set_selection(editor, 0, 0, 2, 0, cx);
            assert!(editor.delete_cross_block_selection(cx));

            // The table is removed in full; only `gamma` survives
            let text = editor.doc().serialize_markdown(cx);
            assert!(
                !text.contains('|'),
                "trailing table should be gone: {text:?}"
            );
            assert_eq!(text.trim(), "gamma");
        });
        cx.quit();
    }

    #[test]
    fn delete_selection_of_only_table_removes_just_the_table() {
        let mut cx = TestAppContext::single();
        init_editor_test_app(&mut cx);
        let editor = cx.new(|cx| Editor::from_markdown(cx, TABLE_DOC.to_string(), None));

        editor.update(&mut cx, |editor, cx| {
            let entries = editor.doc().blocks().to_vec();
            let alpha_len = entries[0].entity.read(cx).display_len();
            // Drag from the end of the paragraph above onto the table: only the
            // table is removed, and re-parse normalizes the spacing.
            set_selection(editor, 0, alpha_len, 2, 0, cx);
            assert!(editor.delete_cross_block_selection(cx));

            assert_eq!(editor.doc().serialize_markdown(cx), "alpha\n\ngamma");
        });
        cx.quit();
    }

    #[test]
    fn cut_selection_including_table_serializes_and_deletes_it() {
        // Exercise cut's two halves directly (the clipboard markdown and the
        // deleted source range) rather than dispatching the action, keeping this
        // a focused unit test of the cut logic.
        let mut cx = TestAppContext::single();
        init_editor_test_app(&mut cx);
        let editor = cx.new(|cx| Editor::from_markdown(cx, TABLE_DOC.to_string(), None));

        editor.update(&mut cx, |editor, cx| {
            let entries = editor.doc().blocks().to_vec();
            let last_index = entries.len() - 1;
            let end_len = entries[last_index].entity.read(cx).display_len();
            set_selection(editor, 0, 0, last_index, end_len, cx);

            // The clipboard markdown serializes the full table, matching what
            // delete removes; otherwise cut would drop it from the clipboard.
            let markdown = editor.cross_block_selected_markdown(cx).unwrap();
            assert!(markdown.contains("| a | b |"), "clipboard: {markdown:?}");
            assert!(markdown.contains("| 1 | 2 |"), "clipboard: {markdown:?}");
            assert!(markdown.contains("alpha") && markdown.contains("gamma"));

            assert!(editor.delete_cross_block_selection(cx));
            assert!(
                !editor.doc().serialize_markdown(cx).contains('|'),
                "document should no longer contain the table"
            );
        });
        cx.quit();
    }

    #[test]
    fn delete_selection_spanning_code_block_removes_it() {
        let mut cx = TestAppContext::single();
        init_editor_test_app(&mut cx);
        // Code blocks edit their raw text, so they are deletable as an ordinary
        // text range; this documents that display_len-based behavior.
        let doc = "alpha\n\n```\ncode\n```\n\ngamma";
        let editor = cx.new(|cx| Editor::from_markdown(cx, doc.to_string(), None));

        editor.update(&mut cx, |editor, cx| {
            let entries = editor.doc().blocks().to_vec();
            let last_index = entries.len() - 1;
            let end_len = entries[last_index].entity.read(cx).display_len();
            set_selection(editor, 0, 0, last_index, end_len, cx);
            assert!(editor.delete_cross_block_selection(cx));

            let text = editor.doc().serialize_markdown(cx);
            assert!(
                !text.contains("code"),
                "code block should be gone: {text:?}"
            );
        });
        cx.quit();
    }

    #[test]
    fn delete_selection_ending_on_trailing_empty_paragraph_removes_table() {
        let mut cx = TestAppContext::single();
        init_editor_test_app(&mut cx);
        let doc = "alpha\n\n| a | b |\n| --- | --- |\n| 1 | 2 |";
        let editor = cx.new(|cx| Editor::from_markdown(cx, doc.to_string(), None));

        editor.update(&mut cx, |editor, cx| {
            // Append a trailing empty paragraph, exactly as inserting a table at
            // the end of a document does.
            let empty =
                Editor::new_block(cx, markdown::parse::BlockData::paragraph(String::new()));
            let index = editor.doc().root_count();
            editor
                .doc_mut()
                .insert_blocks_at(None, index, vec![empty], cx);

            let entries = editor.doc().blocks().to_vec();
            let alpha_len = entries[0].entity.read(cx).display_len();
            let last_index = entries.len() - 1;
            // From the end of `alpha` onto the trailing empty paragraph.
            set_selection(editor, 0, alpha_len, last_index, 0, cx);
            assert!(editor.delete_cross_block_selection(cx));

            let text = editor.doc().serialize_markdown(cx);
            assert!(!text.contains('|'), "table should be gone: {text:?}");
            assert_eq!(text.trim(), "alpha");
        });
        cx.quit();
    }

    #[test]
    fn delete_selection_starting_on_empty_paragraph_removes_table() {
        let mut cx = TestAppContext::single();
        init_editor_test_app(&mut cx);
        let doc = "| a | b |\n| --- | --- |\n| 1 | 2 |\n\ngamma";
        let editor = cx.new(|cx| Editor::from_markdown(cx, doc.to_string(), None));

        editor.update(&mut cx, |editor, cx| {
            // Prepend a leading empty paragraph; starting the highlight on it used
            // to abort deletion (the user's "drag up from the text below into an
            // empty block above the table" case).
            let empty =
                Editor::new_block(cx, markdown::parse::BlockData::paragraph(String::new()));
            editor.doc_mut().insert_blocks_at(None, 0, vec![empty], cx);

            let entries = editor.doc().blocks().to_vec();
            let last_index = entries.len() - 1;
            // From the empty paragraph (index 0) to the start of `gamma`.
            set_selection(editor, 0, 0, last_index, 0, cx);
            assert!(editor.delete_cross_block_selection(cx));

            let text = editor.doc().serialize_markdown(cx);
            assert!(!text.contains('|'), "table should be gone: {text:?}");
            assert_eq!(text.trim(), "gamma");
        });
        cx.quit();
    }

    #[test]
    fn cross_block_selected_markdown_with_cjk_characters() {
        let mut cx = TestAppContext::single();
        init_editor_test_app(&mut cx);
        let doc = "# 标题一\n\n- 项目一\n  - 嵌套项目 1.1\n- 项目二\n\n### 2.4 定义列表\n\n术语一\n: 这是术语一的定义描述\n\n术语二\n: 这是术语二的第一行定义";
        let editor = cx.new(|cx| Editor::from_markdown(cx, doc.to_string(), None));

        editor.update(&mut cx, |editor, cx| {
            let entries = editor.doc().blocks().to_vec();
            assert!(entries.len() >= 4);

            // Select across multiple blocks containing Chinese characters
            set_selection(editor, 0, 0, entries.len() - 1, 1, cx);

            let selected = editor.selected_markdown_text(cx);
            assert!(selected.is_some());
            let selected_text = selected.unwrap();
            assert!(!selected_text.is_empty());

            // Verify active_selection returns CrossBlock variant
            match editor.active_selection(cx) {
                EditorSelection::CrossBlock(_) => {}
                other => panic!("expected CrossBlock selection, got {:?}", other),
            }
        });
        cx.quit();
    }
}
