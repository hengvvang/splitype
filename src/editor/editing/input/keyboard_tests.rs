//! Keyboard pipeline tests — full end-to-end input exercises.
//!
//! Split from `keyboard.rs` so the implementation file stays small;
//! these tests drive the whole block-editing pipeline.

use crate::editor::block_protocol::BlockAction;
use crate::editor::controller::Editor;
use crate::editor::editing::input::actions::ExitCodeBlock;
use crate::editor::editing::input::actions::{Delete, DeleteBack, Newline};
use crate::editor::tree::block::Block;
use crate::model::parse::{BlockData, BlockKind};
use crate::model::block::CalloutKind;
use crate::model::inline::text::BlockText;
use gpui::{App, AppContext, Entity, TestAppContext};

#[gpui::test]
async fn request_quote_break_creates_new_root_leaf_quote_group(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "> first".to_string(), None));

    editor.update(cx, |editor, cx| {
        let quote = editor.doc().first_root().expect("root quote").clone();
        editor.on_block_event(quote, &BlockAction::RequestQuoteBreak, cx);

        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].entity.read(cx).kind(), BlockKind::Blockquote);
        assert_eq!(entries[0].entity.read(cx).display_text(), "first");
        assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::Blockquote);
        assert_eq!(entries[1].entity.read(cx).display_text(), "");
        assert_eq!(entries[1].entity.read(cx).quote_depth, 1);
        assert_eq!(editor.doc().serialize_markdown(cx), "> first\n\n> ");
        assert_eq!(
            editor.tab().focus.pending,
            Some(entries[1].entity.entity_id())
        );
    });
}

#[gpui::test]
async fn typing_quote_shortcut_immediately_refreshes_rendered_quote_metadata(
    cx: &mut TestAppContext,
) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, String::new(), None));

    editor.update(cx, |editor, cx| {
        let paragraph = editor.doc().first_root().expect("root paragraph").clone();
        paragraph.update(cx, |block, cx| {
            block.prepare_undo_capture(
                crate::editor::block_protocol::UndoCaptureKind::CoalescibleText,
                cx,
            );
            block.replace_text_in_display_range(0..0, "> ", None, false, cx);
        });
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].entity.read(cx).kind(), BlockKind::Blockquote);
        assert_eq!(entries[0].entity.read(cx).display_text(), "");
        assert_eq!(entries[0].entity.read(cx).quote_depth, 1);
        assert_eq!(editor.doc().serialize_markdown(cx), "> ");
    });
}

#[gpui::test]
async fn footnote_reference_jump_and_backref_follow_in_place_definition(cx: &mut TestAppContext) {
    let markdown = "alpha[^note]\n\n[^note]: Footnote body".to_string();
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        let paragraph = editor
            .doc()
            .first_root()
            .expect("reference paragraph")
            .clone();
        let definition = editor
            .doc()
            .blocks()
            .iter()
            .find(|entries| entries.entity.read(cx).kind() == BlockKind::FootnoteDefinition)
            .expect("footnote definition block")
            .entity
            .clone();

        editor.on_block_event(
            paragraph.clone(),
            &BlockAction::RequestJumpToFootnoteDefinition {
                id: "note".to_string(),
            },
            cx,
        );
        assert_eq!(editor.tab().focus.pending, Some(definition.entity_id()));
        assert_eq!(definition.read(cx).selected_range, 0..0);

        let expected_backref_range = paragraph
            .read(cx)
            .display_range_for_footnote_occurrence(0)
            .expect("resolved footnote occurrence");
        editor.on_block_event(
            definition.clone(),
            &BlockAction::RequestJumpToFootnoteBackref {
                id: "note".to_string(),
            },
            cx,
        );
        assert_eq!(editor.tab().focus.pending, Some(paragraph.entity_id()));
        assert_eq!(paragraph.read(cx).selected_range, expected_backref_range);
    });
}

#[gpui::test]
async fn image_block_insert_preserves_surrounding_paragraph_text(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "beforeafter".to_string(), None));

    editor.update(cx, |editor, cx| {
        let paragraph = editor.doc().first_root().expect("paragraph").clone();
        editor.insert_image_block_after_paragraph(
            &paragraph,
            &BlockText::plain("before"),
            "![image](./assets/image.png)",
            &BlockText::plain("after"),
            cx,
        );

        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].entity.read(cx).display_text(), "before");
        assert_eq!(
            entries[1].entity.read(cx).display_text(),
            "![image](./assets/image.png)"
        );
        assert!(entries[1].entity.read(cx).image_handle().is_some());
        assert_eq!(entries[2].entity.read(cx).display_text(), "after");
    });
}

#[gpui::test]
async fn image_paste_text_in_code_block_stays_inside_block(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "```\nbeforeafter\n```".to_string(), None));

    editor.update(cx, |editor, cx| {
        let block = editor.doc().first_root().expect("code block").clone();
        editor.replace_current_block_selection_with_image_text(
            &block,
            &BlockText::plain("before"),
            "![image](./assets/image.png)",
            &BlockText::plain("after"),
            cx,
        );

        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].entity.read(cx).kind(),
            BlockKind::CodeBlock { language: None }
        );
        assert_eq!(
            entries[0].entity.read(cx).display_text(),
            "before![image](./assets/image.png)after"
        );
    });
}

#[gpui::test]
async fn typing_callout_shortcut_materializes_body_and_focuses_it(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, String::new(), None));

    editor.update(cx, |editor, cx| {
        let paragraph = editor.doc().first_root().expect("root paragraph").clone();
        paragraph.update(cx, |block, cx| {
            block.prepare_undo_capture(
                crate::editor::block_protocol::UndoCaptureKind::CoalescibleText,
                cx,
            );
            block.replace_text_in_display_range(0..0, "> [!NOTE]", None, false, cx);
        });
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[0].entity.read(cx).kind(),
            BlockKind::Callout(CalloutKind::Note)
        );
        assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[1].entity.read(cx).display_text(), "");
        assert_eq!(entries[1].entity.read(cx).quote_depth, 1);
        assert_eq!(editor.doc().serialize_markdown(cx), "> [!NOTE]\n> ");
        assert_eq!(
            editor.tab().focus.pending,
            Some(entries[1].entity.entity_id())
        );
    });
}

#[gpui::test]
async fn typing_numbered_list_shortcut_after_separator_preserves_group_boundary(
    cx: &mut TestAppContext,
) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "1. aa\n2. bb\n3. cc".to_string(), None));

    let separator_id = editor.update(cx, |editor, cx| {
        let separator = Editor::new_block(cx, BlockData::paragraph(String::new()));
        let root_count = editor.doc().root_count();
        editor
            .doc_mut()
            .insert_blocks_at(None, root_count, vec![separator.clone()], cx);
        separator.entity_id()
    });

    editor.update(cx, |editor, cx| {
        let separator = editor
            .doc()
            .block_entity_by_id(separator_id)
            .expect("separator paragraph");
        assert!(separator.read(cx).list_group_separator_candidate);
        separator.update(cx, |block, cx| {
            block.prepare_undo_capture(
                crate::editor::block_protocol::UndoCaptureKind::CoalescibleText,
                cx,
            );
            block.replace_text_in_display_range(0..0, "1. ", None, false, cx);
        });
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 5);
        assert_eq!(entries[0].entity.read(cx).list_ordinal, Some(1));
        assert_eq!(entries[1].entity.read(cx).list_ordinal, Some(2));
        assert_eq!(entries[2].entity.read(cx).list_ordinal, Some(3));
        assert_eq!(entries[3].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[3].entity.read(cx).display_text(), "");
        assert_eq!(entries[4].entity.entity_id(), separator_id);
        assert_eq!(
            entries[4].entity.read(cx).kind(),
            BlockKind::NumberedListItem
        );
        assert_eq!(entries[4].entity.read(cx).display_text(), "");
        assert_eq!(entries[4].entity.read(cx).list_ordinal, Some(1));
        assert_eq!(editor.doc().serialize_markdown(cx), "1. aa\n2. bb\n3. cc\n\n1. ");
    });
}

#[gpui::test]
async fn request_indent_nests_non_empty_list_item(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "- a\n- b".to_string(), None));

    editor.update(cx, |editor, cx| {
        let second = editor.doc().blocks()[1].entity.clone();
        editor.on_block_event(second, &BlockAction::RequestIndent, cx);

        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].entity.read(cx).kind(), BlockKind::BulletListItem);
        assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::BulletListItem);
        assert_eq!(entries[1].entity.read(cx).render_depth, 1);
        assert_eq!(editor.doc().serialize_markdown(cx), "- a\n  - b");
    });
}

#[gpui::test]
async fn request_outdent_lifts_list_child_paragraph_after_parent(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "- item\n\n  child text".to_string(), None));

    let child_id = editor.update(cx, |editor, cx| {
        let child = editor.doc().blocks()[1].entity.clone();
        editor.on_block_event(child.clone(), &BlockAction::RequestOutdent, cx);
        child.entity_id()
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].entity.read(cx).kind(), BlockKind::BulletListItem);
        assert_eq!(entries[0].entity.read(cx).display_text(), "item");
        assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[1].entity.read(cx).display_text(), "child text");
        assert_eq!(entries[1].entity.read(cx).render_depth, 0);
        assert_eq!(entries[1].entity.entity_id(), child_id);
        assert_eq!(editor.doc().serialize_markdown(cx), "- item\n\nchild text");
    });
}

#[gpui::test]
async fn empty_list_child_paragraph_backspace_outdents_to_root(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let editor = cx.new(|cx| Editor::from_markdown(cx, "- item\n\n  child".to_string(), None));

    let child_id = editor.update(cx, |editor, _cx| {
        editor.doc().blocks()[1].entity.entity_id()
    });

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let child = editor.doc().blocks()[1].entity.clone();
            child.update(cx, |block, block_cx| {
                block.prepare_undo_capture(
                    crate::editor::block_protocol::UndoCaptureKind::NonCoalescible,
                    block_cx,
                );
                block.replace_text_in_display_range(
                    0..block.display_len(),
                    "",
                    None,
                    false,
                    block_cx,
                );
                block.move_to(0, block_cx);
                block.on_delete_back(&DeleteBack, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].entity.read(cx).kind(), BlockKind::BulletListItem);
        assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[1].entity.read(cx).display_text(), "");
        assert_eq!(entries[1].entity.entity_id(), child_id);
        assert_eq!(entries[1].entity.read(cx).render_depth, 0);
        assert_eq!(editor.doc().serialize_markdown(cx), "- item\n\n");
    });
}

#[gpui::test]
async fn empty_list_child_paragraph_enter_continues_same_level(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let editor = cx.new(|cx| Editor::from_markdown(cx, "- item\n\n  child".to_string(), None));

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let child = editor.doc().blocks()[1].entity.clone();
            child.update(cx, |block, block_cx| {
                block.prepare_undo_capture(
                    crate::editor::block_protocol::UndoCaptureKind::NonCoalescible,
                    block_cx,
                );
                block.replace_text_in_display_range(
                    0..block.display_len(),
                    "",
                    None,
                    false,
                    block_cx,
                );
                block.move_to(0, block_cx);
                block.on_newline(&Newline, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].entity.read(cx).kind(), BlockKind::BulletListItem);
        assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[1].entity.read(cx).display_text(), "");
        // Enter splits into a fresh block; the structure (two empty
        // list-child paragraphs) is what matters.
        assert_eq!(entries[1].entity.read(cx).render_depth, 1);
        assert_eq!(entries[2].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[2].entity.read(cx).display_text(), "");
        assert_eq!(entries[2].entity.read(cx).render_depth, 1);
        assert_eq!(editor.doc().serialize_markdown(cx), "- item\n  \n  ");
    });
}

#[gpui::test]
async fn enter_inside_script_paragraph_creates_new_block(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let editor = cx.new(|cx| Editor::from_markdown(cx, "H~2~O".to_string(), None));

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let block = editor.doc().blocks()[0].entity.clone();
            block.update(cx, |block, block_cx| {
                block.move_to(block.display_len(), block_cx);
                block.on_newline(&Newline, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].entity.read(cx).display_text(), "H2O");
        assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[1].entity.read(cx).display_text(), "");
        assert_eq!(editor.doc().serialize_markdown(cx), "H~2~O\n\n");
    });
}

#[gpui::test]
async fn enter_inside_inline_math_paragraph_creates_new_block(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let editor = cx.new(|cx| Editor::from_markdown(cx, "$n^2$".to_string(), None));

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let block = editor.doc().blocks()[0].entity.clone();
            block.update(cx, |block, block_cx| {
                block.move_to(block.display_len(), block_cx);
                block.on_newline(&Newline, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[0].entity.read(cx).display_text(), "$n^2$");
        assert!(!entries[0].entity.read(cx).edits_verbatim_text());
        assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[1].entity.read(cx).display_text(), "");
        assert_eq!(editor.doc().serialize_markdown(cx), "$n^2$\n\n");
    });
}

#[gpui::test]
async fn trailing_fence_line_enter_closes_code_block(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let editor =
        cx.new(|cx| Editor::from_markdown(cx, "```rust\nlet x = 1;\n```".to_string(), None));

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let block = editor.doc().blocks()[0].entity.clone();
            block.update(cx, |block, block_cx| {
                // Type a closing fence on a fresh last line, then Enter.
                let end = block.display_len();
                block.replace_text_in_display_range(end..end, "\n```", None, false, block_cx);
                block.move_to(block.display_len(), block_cx);
                block.on_newline(&Newline, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[0].entity.read(cx).kind(),
            BlockKind::CodeBlock {
                language: Some("rust".into())
            }
        );
        assert_eq!(entries[0].entity.read(cx).display_text(), "let x = 1;");
        assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[1].entity.read(cx).display_text(), "");
        assert_eq!(editor.doc().serialize_markdown(cx), "```rust\nlet x = 1;\n```\n\n");
    });
}

#[gpui::test]
async fn setext_equals_underline_enter_promotes_previous_paragraph_to_h1(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let editor = cx.new(|cx| Editor::from_markdown(cx, "Title\n\n=====".to_string(), None));

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let underline = editor.doc().blocks()[1].entity.clone();
            underline.update(cx, |block, block_cx| {
                block.move_to(block.display_len(), block_cx);
                block.on_newline(&Newline, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[0].entity.read(cx).kind(),
            BlockKind::Heading { level: 1 }
        );
        assert_eq!(entries[0].entity.read(cx).display_text(), "Title");
        assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[1].entity.read(cx).display_text(), "");
        assert_eq!(editor.doc().serialize_markdown(cx), "# Title\n\n");
    });

    // Reversible: undo restores the two original paragraphs.
    editor.update(cx, |editor, cx| {
        editor.undo_document(cx);
        assert_eq!(editor.doc().serialize_markdown(cx), "Title\n\n=====");
    });
}

#[gpui::test]
async fn setext_dash_underline_enter_promotes_previous_paragraph_to_h2(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    // A bare "-----" in source parses as a thematic break, so simulate the
    // user typing the underline into the paragraph below the title instead.
    let editor = cx.new(|cx| Editor::from_markdown(cx, "Title\n\nx".to_string(), None));

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let underline = editor.doc().blocks()[1].entity.clone();
            underline.update(cx, |block, block_cx| {
                let end = block.display_len();
                block.replace_text_in_display_range(0..end, "-----", None, false, block_cx);
                block.move_to(block.display_len(), block_cx);
                block.on_newline(&Newline, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(
            entries[0].entity.read(cx).kind(),
            BlockKind::Heading { level: 2 }
        );
        assert_eq!(entries[0].entity.read(cx).display_text(), "Title");
        assert_eq!(editor.doc().serialize_markdown(cx), "## Title\n\n");
    });
}

#[gpui::test]
async fn dash_underline_without_heading_target_stays_a_separator(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let editor = cx.new(|cx| Editor::from_markdown(cx, String::new(), None));

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let block = editor.doc().blocks()[0].entity.clone();
            block.update(cx, |block, block_cx| {
                block.replace_text_in_display_range(0..0, "-----", None, false, block_cx);
                block.move_to(block.display_len(), block_cx);
                block.on_newline(&Newline, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries[0].entity.read(cx).kind(), BlockKind::ThematicBreak);
    });
}

#[gpui::test]
async fn equals_underline_without_heading_target_stays_a_paragraph(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let editor = cx.new(|cx| Editor::from_markdown(cx, String::new(), None));

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let block = editor.doc().blocks()[0].entity.clone();
            block.update(cx, |block, block_cx| {
                block.replace_text_in_display_range(0..0, "=====", None, false, block_cx);
                block.move_to(block.display_len(), block_cx);
                block.on_newline(&Newline, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries[0].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[0].entity.read(cx).display_text(), "=====");
    });
}

#[gpui::test]
async fn delimiter_row_enter_forms_native_table(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let editor = cx
        .new(|cx| Editor::from_markdown(cx, "| Name | Score |\n\n| --- | --- |".to_string(), None));

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let delimiter = editor.doc().root_blocks()[1].clone();
            delimiter.update(cx, |block, block_cx| {
                block.move_to(block.display_len(), block_cx);
                block.on_newline(&Newline, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let roots = editor.doc().root_blocks();
        assert_eq!(roots.len(), 2);
        assert_eq!(roots[0].read(cx).kind(), BlockKind::Table);
        let table = roots[0].read(cx).data.table.clone().expect("table");
        assert_eq!(table.header.len(), 2);
        assert_eq!(table.header[0].serialize_markdown(), "Name");
        assert!(table.rows.is_empty());
        assert_eq!(roots[1].read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(
            editor.doc().serialize_markdown(cx),
            "| Name | Score |\n| --- | --- |\n\n"
        );
    });

    // Reversible in one step back to the two source paragraphs.
    editor.update(cx, |editor, cx| {
        editor.undo_document(cx);
        assert_eq!(
            editor.doc().serialize_markdown(cx),
            "| Name | Score |\n\n| --- | --- |"
        );
    });
}

#[gpui::test]
async fn pipe_row_below_table_is_absorbed_as_a_row(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let editor = cx
        .new(|cx| Editor::from_markdown(cx, "| Name | Score |\n\n| --- | --- |".to_string(), None));

    // Form the table.
    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let delimiter = editor.doc().root_blocks()[1].clone();
            delimiter.update(cx, |block, block_cx| {
                block.move_to(block.display_len(), block_cx);
                block.on_newline(&Newline, window, block_cx);
            });
        });
    });

    // Type a body row into the paragraph below the table and press Enter.
    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let row = editor.doc().root_blocks()[1].clone();
            row.update(cx, |block, block_cx| {
                block.replace_text_in_display_range(0..0, "| Alice | 10 |", None, false, block_cx);
                block.move_to(block.display_len(), block_cx);
                block.on_newline(&Newline, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let roots = editor.doc().root_blocks();
        assert_eq!(roots[0].read(cx).kind(), BlockKind::Table);
        let table = roots[0].read(cx).data.table.clone().expect("table");
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.rows[0][0].serialize_markdown(), "Alice");
        assert_eq!(table.rows[0][1].serialize_markdown(), "10");
        assert_eq!(roots[1].read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(roots[1].read(cx).display_text(), "");
    });
}

#[gpui::test]
async fn pipeless_delimiter_row_enter_forms_native_table(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let editor =
        cx.new(|cx| Editor::from_markdown(cx, "Name | Score\n\n---- | ----".to_string(), None));

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let delimiter = editor.doc().root_blocks()[1].clone();
            delimiter.update(cx, |block, block_cx| {
                block.move_to(block.display_len(), block_cx);
                block.on_newline(&Newline, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let roots = editor.doc().root_blocks();
        assert_eq!(roots.len(), 2);
        assert_eq!(roots[0].read(cx).kind(), BlockKind::Table);
        let table = roots[0].read(cx).data.table.clone().expect("table");
        assert_eq!(table.header.len(), 2);
        assert_eq!(table.header[0].serialize_markdown(), "Name");
        assert_eq!(table.header[1].serialize_markdown(), "Score");
        assert!(table.rows.is_empty());
        assert_eq!(roots[1].read(cx).kind(), BlockKind::Paragraph);
    });
}

#[gpui::test]
async fn pipeless_row_below_table_is_absorbed_as_a_row(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let editor =
        cx.new(|cx| Editor::from_markdown(cx, "Name | Score\n\n---- | ----".to_string(), None));

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let delimiter = editor.doc().root_blocks()[1].clone();
            delimiter.update(cx, |block, block_cx| {
                block.move_to(block.display_len(), block_cx);
                block.on_newline(&Newline, window, block_cx);
            });
        });
    });

    // A pipeless body row with the table's column count is absorbed.
    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let row = editor.doc().root_blocks()[1].clone();
            row.update(cx, |block, block_cx| {
                block.replace_text_in_display_range(0..0, "Alice | 10", None, false, block_cx);
                block.move_to(block.display_len(), block_cx);
                block.on_newline(&Newline, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let roots = editor.doc().root_blocks();
        assert_eq!(roots[0].read(cx).kind(), BlockKind::Table);
        let table = roots[0].read(cx).data.table.clone().expect("table");
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.rows[0][0].serialize_markdown(), "Alice");
        assert_eq!(table.rows[0][1].serialize_markdown(), "10");
        assert_eq!(roots[1].read(cx).kind(), BlockKind::Paragraph);
    });
}

#[gpui::test]
async fn ragged_pipeless_row_below_table_is_padded_to_width(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let editor =
        cx.new(|cx| Editor::from_markdown(cx, "A | B | C\n\n--- | --- | ---".to_string(), None));

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let delimiter = editor.doc().root_blocks()[1].clone();
            delimiter.update(cx, |block, block_cx| {
                block.move_to(block.display_len(), block_cx);
                block.on_newline(&Newline, window, block_cx);
            });
        });
    });

    // Two cells typed under a three-column table: absorbed as a row and
    // padded to the header width, matching how pasted ragged rows behave.
    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let row = editor.doc().root_blocks()[1].clone();
            row.update(cx, |block, block_cx| {
                block.replace_text_in_display_range(0..0, "one | two", None, false, block_cx);
                block.move_to(block.display_len(), block_cx);
                block.on_newline(&Newline, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let table = editor.doc().root_blocks()[0]
            .read(cx)
            .data
            .table
            .clone()
            .expect("table");
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.rows[0].len(), 3);
        assert_eq!(table.rows[0][0].serialize_markdown(), "one");
        assert_eq!(table.rows[0][1].serialize_markdown(), "two");
        assert_eq!(table.rows[0][2].serialize_markdown(), "");
    });
}

#[gpui::test]
async fn lone_pipe_row_without_table_context_stays_a_paragraph(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let editor = cx.new(|cx| Editor::from_markdown(cx, String::new(), None));

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let block = editor.doc().root_blocks()[0].clone();
            block.update(cx, |block, block_cx| {
                block.replace_text_in_display_range(0..0, "| a | b |", None, false, block_cx);
                block.move_to(block.display_len(), block_cx);
                block.on_newline(&Newline, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let roots = editor.doc().root_blocks();
        assert_eq!(roots[0].read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(roots[0].read(cx).display_text(), "| a | b |");
    });
}

#[gpui::test]
async fn math_block_exit_shortcut_creates_plain_text_block(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let editor = cx.new(|cx| Editor::from_markdown(cx, "$$n^2$$".to_string(), None));

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let block = editor.doc().blocks()[0].entity.clone();
            block.update(cx, |block, block_cx| {
                block.on_exit_code_block(&ExitCodeBlock, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].entity.read(cx).kind(), BlockKind::MathBlock);
        assert_eq!(entries[0].entity.read(cx).display_text(), "n^2");
        assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[1].entity.read(cx).display_text(), "");
        assert_eq!(editor.doc().serialize_markdown(cx), "$$n^2$$\n\n");
    });
}

#[gpui::test]
async fn dollar_dollar_enter_creates_editable_math_block(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let editor = cx.new(|cx| Editor::from_markdown(cx, String::new(), None));

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let block = editor.doc().blocks()[0].entity.clone();
            block.update(cx, |block, block_cx| {
                block.replace_text_in_display_range(
                    0..block.display_len(),
                    "$$",
                    None,
                    false,
                    block_cx,
                );
                block.move_to(block.display_len(), block_cx);
                block.on_newline(&Newline, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 1);
        let block = entries[0].entity.read(cx);
        assert_eq!(block.kind(), BlockKind::MathBlock);
        // The delimiters are stripped; only the formula body is stored.
        assert_eq!(block.display_text(), "");
        assert_eq!(block.selected_range, 0..0);
        assert!(block.edits_verbatim_text());
        assert_eq!(editor.doc().serialize_markdown(cx), "$$\n\n$$");
    });
}

#[gpui::test]
async fn dollar_dollar_prefix_then_enter_wraps_existing_line(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let editor = cx.new(|cx| Editor::from_markdown(cx, "E = mc^2".to_string(), None));

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let block = editor.doc().blocks()[0].entity.clone();
            block.update(cx, |block, block_cx| {
                // Home, type the fence in front of the formula, then Enter.
                block.move_to(0, block_cx);
                block.replace_text_in_display_range(0..0, "$$", None, false, block_cx);
                block.move_to("$$".len(), block_cx);
                block.on_newline(&Newline, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 1);
        let block = entries[0].entity.read(cx);
        assert_eq!(block.kind(), BlockKind::MathBlock);
        // The pre-existing text is kept as the formula body.
        assert_eq!(block.display_text(), "E = mc^2");
        assert_eq!(block.selected_range, 0..0);
        assert_eq!(editor.doc().serialize_markdown(cx), "$$E = mc^2$$");
    });
}

#[gpui::test]
async fn enter_inside_math_block_keeps_local_formula_editing(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let editor = cx.new(|cx| Editor::from_markdown(cx, "$$n^2$$".to_string(), None));

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let block = editor.doc().blocks()[0].entity.clone();
            block.update(cx, |block, block_cx| {
                // Body text is "n^2"; the caret sits between "n" and "^".
                block.move_to(1, block_cx);
                block.on_newline(&Newline, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].entity.read(cx).kind(), BlockKind::MathBlock);
        assert_eq!(entries[0].entity.read(cx).display_text(), "n\n^2");
        assert_eq!(editor.doc().serialize_markdown(cx), "$$\nn\n^2\n$$");
    });
}

#[gpui::test]
async fn auto_created_math_block_exit_shortcut_creates_plain_text_block(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let editor = cx.new(|cx| Editor::from_markdown(cx, String::new(), None));

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let block = editor.doc().blocks()[0].entity.clone();
            block.update(cx, |block, block_cx| {
                block.replace_text_in_display_range(
                    0..block.display_len(),
                    "$$",
                    None,
                    false,
                    block_cx,
                );
                block.move_to(block.display_len(), block_cx);
                block.on_newline(&Newline, window, block_cx);
                block.on_exit_code_block(&ExitCodeBlock, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].entity.read(cx).kind(), BlockKind::MathBlock);
        assert_eq!(entries[0].entity.read(cx).display_text(), "");
        assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[1].entity.read(cx).display_text(), "");
        assert_eq!(editor.doc().serialize_markdown(cx), "$$\n\n$$\n\n");
    });
}

#[gpui::test]
async fn raw_like_block_exit_shortcut_creates_plain_text_block(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let cases = [
        (
            BlockData::new(
                BlockKind::HtmlBlock,
                BlockText::plain("<div>\ncontent\n</div>".to_string()),
            ),
            BlockKind::HtmlBlock,
            "<div>\ncontent\n</div>",
        ),
        (
            BlockData::new(
                BlockKind::MermaidBlock,
                BlockText::plain("```mermaid\nflowchart LR\nA-->B\n```".to_string()),
            ),
            BlockKind::MermaidBlock,
            "```mermaid\nflowchart LR\nA-->B\n```",
        ),
        (
            BlockData::new(
                BlockKind::RawMarkdown,
                BlockText::plain("::: custom\ncontent\n:::".to_string()),
            ),
            BlockKind::RawMarkdown,
            "::: custom\ncontent\n:::",
        ),
        (
            BlockData::new(
                BlockKind::HtmlComment,
                BlockText::plain("<!--\ncomment\n-->".to_string()),
            ),
            BlockKind::HtmlComment,
            "<!--\ncomment\n-->",
        ),
    ];

    for (data, kind, text) in cases {
        let editor = cx.new(|cx| {
            let mut editor = Editor::from_markdown(cx, String::new(), None);
            let block = Editor::new_block(cx, data.clone());
            editor.doc_mut().replace_blocks(vec![block], cx);
            editor
        });

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let block = editor.doc().blocks()[0].entity.clone();
                block.update(cx, |block, block_cx| {
                    block.on_exit_code_block(&ExitCodeBlock, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let entries = editor.doc().blocks();
            assert_eq!(entries.len(), 2);
            assert_eq!(entries[0].entity.read(cx).kind(), kind);
            assert_eq!(entries[0].entity.read(cx).display_text(), text);
            assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(entries[1].entity.read(cx).display_text(), "");
        });
    }
}

#[gpui::test]
async fn table_cell_enter_still_moves_to_next_row(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let markdown = ["| A | B |", "| --- | --- |", "| 1 | 2 |", "| 3 | 4 |"].join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    let mut next_cell_id = None;
    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let table = editor.doc().first_root().expect("table root").clone();
            let (cell, expected_next_cell_id) = {
                let table = table.read(cx);
                let grid = table.table_grid.as_ref().expect("table grid");
                (grid.rows[0][0].clone(), grid.rows[1][0].entity_id())
            };
            next_cell_id = Some(expected_next_cell_id);
            cell.update(cx, |block, block_cx| {
                block.on_newline(&Newline, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, _cx| {
        assert_eq!(editor.doc().blocks().len(), 1);
        assert_eq!(editor.tab().focus.pending, next_cell_id);
    });
}

#[gpui::test]
async fn table_cell_exit_shortcut_inserts_sibling_after_table(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let markdown = ["> [!NOTE]", "> | A | B |", "> | --- | --- |", "> | 1 | 2 |"].join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let callout = editor.doc().first_root().expect("callout root").clone();
            let table = callout
                .read(cx)
                .children
                .iter()
                .find(|child| child.read(cx).kind() == BlockKind::Table)
                .expect("nested table")
                .clone();
            let cell = table
                .read(cx)
                .table_grid
                .as_ref()
                .expect("table grid")
                .rows[0][0]
                .clone();
            cell.update(cx, |block, block_cx| {
                block.on_exit_code_block(&ExitCodeBlock, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let callout = editor.doc().first_root().expect("callout root").clone();
        let children = callout.read(cx).children.clone();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].read(cx).kind(), BlockKind::Table);
        assert_eq!(children[1].read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(children[1].read(cx).display_text(), "");
        assert_eq!(editor.tab().focus.pending, Some(children[1].entity_id()));
    });
}

pub(crate) fn table_root(editor: &Editor, cx: &App) -> Entity<Block> {
    editor
        .doc()
        .blocks()
        .iter()
        .map(|entries| entries.entity.clone())
        .find(|block| block.read(cx).kind() == BlockKind::Table)
        .expect("table root")
}

#[gpui::test]
async fn arrow_down_from_last_row_exits_table_to_following_block(cx: &mut TestAppContext) {
    let markdown = ["| A | B |", "| --- | --- |", "| 1 | 2 |", "", "after"].join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        let table = table_root(editor, cx);
        let cell = table
            .read(cx)
            .table_grid
            .as_ref()
            .expect("table grid")
            .rows
            .last()
            .and_then(|row| row.first())
            .cloned()
            .expect("last row cell");
        editor.on_block_event(
            cell,
            &BlockAction::RequestTableCellMoveVertical { delta: 1 },
            cx,
        );

        let following = editor.doc().blocks()[1].entity.clone();
        assert_eq!(following.read(cx).display_text(), "after");
        assert_eq!(editor.tab().focus.pending, Some(following.entity_id()));
    });
}

#[gpui::test]
async fn arrow_up_from_header_exits_table_to_preceding_block(cx: &mut TestAppContext) {
    let markdown = ["before", "", "| A | B |", "| --- | --- |", "| 1 | 2 |"].join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        let table = table_root(editor, cx);
        let cell = table
            .read(cx)
            .table_grid
            .as_ref()
            .expect("table grid")
            .header
            .first()
            .cloned()
            .expect("header cell");
        editor.on_block_event(
            cell,
            &BlockAction::RequestTableCellMoveVertical { delta: -1 },
            cx,
        );

        let preceding = editor.doc().blocks()[0].entity.clone();
        assert_eq!(preceding.read(cx).display_text(), "before");
        assert_eq!(editor.tab().focus.pending, Some(preceding.entity_id()));
    });
}

#[gpui::test]
async fn arrow_down_into_table_focuses_header_cell(cx: &mut TestAppContext) {
    let markdown = ["before", "", "| A | B |", "| --- | --- |", "| 1 | 2 |"].join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        let paragraph = editor.doc().first_root().expect("paragraph root").clone();
        editor.on_block_event(
            paragraph,
            &BlockAction::RequestFocusNext { preferred_x: None },
            cx,
        );

        let header_cell = table_root(editor, cx)
            .read(cx)
            .table_grid
            .as_ref()
            .expect("table grid")
            .header
            .first()
            .map(|cell| cell.entity_id());
        assert_eq!(editor.tab().focus.pending, header_cell);
    });
}

#[gpui::test]
async fn arrow_up_into_table_focuses_last_row_cell(cx: &mut TestAppContext) {
    let markdown = ["| A | B |", "| --- | --- |", "| 1 | 2 |", "", "after"].join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        let paragraph = editor.doc().blocks()[1].entity.clone();
        assert_eq!(paragraph.read(cx).display_text(), "after");
        editor.on_block_event(
            paragraph,
            &BlockAction::RequestFocusPrev { preferred_x: None },
            cx,
        );

        let last_row_cell = table_root(editor, cx)
            .read(cx)
            .table_grid
            .as_ref()
            .expect("table grid")
            .rows
            .last()
            .and_then(|row| row.first())
            .map(|cell| cell.entity_id());
        assert_eq!(editor.tab().focus.pending, last_row_cell);
    });
}

#[gpui::test]
async fn block_up_from_table_cell_exits_to_preceding_block(cx: &mut TestAppContext) {
    let markdown = ["before", "", "| A | B |", "| --- | --- |", "| 1 | 2 |"].join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        // Start from a body cell, not the header, to confirm Block Up leaves
        // the whole table instead of stepping to the cell above.
        let cell = table_root(editor, cx)
            .read(cx)
            .table_grid
            .as_ref()
            .expect("table grid")
            .rows
            .last()
            .and_then(|row| row.first())
            .cloned()
            .expect("body cell");
        editor.on_block_event(cell, &BlockAction::RequestBlockUp, cx);

        let preceding = editor.doc().blocks()[0].entity.clone();
        assert_eq!(preceding.read(cx).display_text(), "before");
        assert_eq!(editor.tab().focus.pending, Some(preceding.entity_id()));
    });
}

#[gpui::test]
async fn block_down_into_table_focuses_header_cell(cx: &mut TestAppContext) {
    let markdown = ["before", "", "| A | B |", "| --- | --- |", "| 1 | 2 |"].join("\n");
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        let paragraph = editor.doc().first_root().expect("paragraph root").clone();
        editor.on_block_event(paragraph, &BlockAction::RequestBlockDown, cx);

        let header_cell = table_root(editor, cx)
            .read(cx)
            .table_grid
            .as_ref()
            .expect("table grid")
            .header
            .first()
            .map(|cell| cell.entity_id());
        assert_eq!(editor.tab().focus.pending, header_cell);
    });
}

#[gpui::test]
async fn down_out_of_code_block_focuses_following_block(cx: &mut TestAppContext) {
    let editor =
        cx.new(|cx| Editor::from_markdown(cx, "```rust\nab\n```\n\nafter".to_string(), None));

    editor.update(cx, |editor, cx| {
        let code = editor.doc().first_root().expect("code root").clone();
        assert!(code.read(cx).kind().is_code_block());
        // Down from the language field emits RequestFocusNext; with a block
        // below, focus lands there rather than creating anything.
        editor.on_block_event(
            code,
            &BlockAction::RequestFocusNext { preferred_x: None },
            cx,
        );

        let following = editor.doc().blocks()[1].entity.clone();
        assert_eq!(following.read(cx).display_text(), "after");
        assert_eq!(editor.doc().root_count(), 2);
        assert_eq!(editor.tab().focus.pending, Some(following.entity_id()));
    });
}

#[gpui::test]
async fn down_out_of_trailing_code_block_creates_and_focuses_paragraph(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "```rust\nab\n```".to_string(), None));

    editor.update(cx, |editor, cx| {
        let code = editor.doc().first_root().expect("code root").clone();
        assert_eq!(editor.doc().root_count(), 1);
        editor.on_block_event(
            code,
            &BlockAction::RequestFocusNext { preferred_x: None },
            cx,
        );

        let roots = editor.doc().root_blocks();
        assert_eq!(roots.len(), 2);
        assert_eq!(roots[1].read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(roots[1].read(cx).display_text(), "");
        assert_eq!(editor.tab().focus.pending, Some(roots[1].entity_id()));
    });
}

#[gpui::test]
async fn down_out_of_trailing_math_block_creates_and_focuses_paragraph(cx: &mut TestAppContext) {
    // Same miss as code blocks, one of the other multi-line widget blocks.
    let editor = cx.new(|cx| Editor::from_markdown(cx, "$$\nx^2\n$$".to_string(), None));

    editor.update(cx, |editor, cx| {
        let math = editor.doc().first_root().expect("math root").clone();
        assert_eq!(math.read(cx).kind(), BlockKind::MathBlock);
        editor.on_block_event(
            math,
            &BlockAction::RequestFocusNext { preferred_x: None },
            cx,
        );

        let roots = editor.doc().root_blocks();
        assert_eq!(roots.len(), 2);
        assert_eq!(roots[1].read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(editor.tab().focus.pending, Some(roots[1].entity_id()));
    });
}

#[gpui::test]
async fn down_at_end_of_trailing_paragraph_creates_nothing(cx: &mut TestAppContext) {
    // Regression guard: ordinary text blocks must not sprout a paragraph.
    let editor = cx.new(|cx| Editor::from_markdown(cx, "hello".to_string(), None));

    editor.update(cx, |editor, cx| {
        let paragraph = editor.doc().first_root().expect("paragraph").clone();
        editor.on_block_event(
            paragraph,
            &BlockAction::RequestFocusNext { preferred_x: None },
            cx,
        );

        // No trailing paragraph is invented for an ordinary text block.
        assert_eq!(editor.doc().root_count(), 1);
    });
}

#[gpui::test]
async fn plain_multiline_paste_with_scripts_splits_physical_lines(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, String::new(), None));

    editor.update(cx, |editor, cx| {
        let block = editor.doc().blocks()[0].entity.clone();
        editor.on_block_event(
            block,
            &BlockAction::RequestPasteMultiline {
                leading: BlockText::plain(String::new()),
                lines: vec![
                    "H~2~O".to_string(),
                    "CO<sub>2</sub>".to_string(),
                    "x<sup>n</sup>".to_string(),
                ],
                trailing: BlockText::plain(String::new()),
                split_physical_lines: true,
            },
            cx,
        );

        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].entity.read(cx).display_text(), "H2O");
        assert_eq!(entries[1].entity.read(cx).display_text(), "CO2");
        assert_eq!(entries[2].entity.read(cx).display_text(), "xn");
        assert_eq!(editor.doc().serialize_markdown(cx), "H~2~O\n\nCO~2~\n\nx^n^");
    });
}

#[gpui::test]
async fn structural_paste_of_table_renders_native_table(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, String::new(), None));

    editor.update(cx, |editor, cx| {
        let block = editor.doc().blocks()[0].entity.clone();
        editor.on_block_event(
            block,
            &BlockAction::RequestPasteMultiline {
                leading: BlockText::plain(String::new()),
                lines: vec![
                    "| A | B |".to_string(),
                    "| --- | --- |".to_string(),
                    "| 1 | 2 |".to_string(),
                ],
                trailing: BlockText::plain(String::new()),
                split_physical_lines: false,
            },
            cx,
        );

        // The header row must survive: previously the first pasted line was
        // folded into the paragraph, leaving the alignment row to masquerade
        // as the header. The empty paste target is also dropped, and a
        // trailing paragraph is added so the document does not end on the
        // table with no line below it.
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 2);
        let table = entries[0].entity.read(cx);
        assert_eq!(table.kind(), BlockKind::Table);
        let data = table.data.table.as_ref().expect("table data");
        assert_eq!(data.header[0].serialize_markdown(), "A");
        assert_eq!(data.header[1].serialize_markdown(), "B");
        assert_eq!(data.rows.len(), 1);
        assert_eq!(data.rows[0][0].serialize_markdown(), "1");
        assert_eq!(data.rows[0][1].serialize_markdown(), "2");
        assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[1].entity.read(cx).display_text(), "");
    });
}

#[gpui::test]
async fn structural_paste_of_code_block_renders_native_code_block(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, String::new(), None));

    editor.update(cx, |editor, cx| {
        let block = editor.doc().blocks()[0].entity.clone();
        editor.on_block_event(
            block,
            &BlockAction::RequestPasteMultiline {
                leading: BlockText::plain(String::new()),
                lines: vec![
                    "```rust".to_string(),
                    "fn main() {}".to_string(),
                    "```".to_string(),
                ],
                trailing: BlockText::plain(String::new()),
                split_physical_lines: false,
            },
            cx,
        );

        // The fence is structural, so the whole paste goes through the block
        // importer rather than the plain-text path: the opening ```rust line is
        // no longer folded into a paragraph, and the empty paste target is
        // dropped. A trailing paragraph is added so the document does not end
        // on the code block with no line below it.
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 2);
        let code = entries[0].entity.read(cx);
        assert_eq!(
            code.kind(),
            BlockKind::CodeBlock {
                language: Some("rust".into())
            }
        );
        assert_eq!(code.display_text(), "fn main() {}");
        assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(
            editor.doc().serialize_markdown(cx),
            "```rust\nfn main() {}\n```\n\n"
        );
    });
}

#[gpui::test]
async fn structural_paste_of_table_preserves_surrounding_text(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "beforeafter".into(), None));

    editor.update(cx, |editor, cx| {
        let block = editor.doc().blocks()[0].entity.clone();
        editor.on_block_event(
            block,
            &BlockAction::RequestPasteMultiline {
                leading: BlockText::plain("before"),
                lines: vec![
                    "| A | B |".to_string(),
                    "| --- | --- |".to_string(),
                    "| 1 | 2 |".to_string(),
                ],
                trailing: BlockText::plain("after"),
                split_physical_lines: false,
            },
            cx,
        );

        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].entity.read(cx).display_text(), "before");

        let table = entries[1].entity.read(cx);
        assert_eq!(table.kind(), BlockKind::Table);
        let data = table.data.table.as_ref().expect("table data");
        assert_eq!(data.header[0].serialize_markdown(), "A");
        assert_eq!(data.rows[0][0].serialize_markdown(), "1");

        assert_eq!(entries[2].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[2].entity.read(cx).display_text(), "after");
    });
}

#[gpui::test]
async fn structural_paste_of_code_block_preserves_surrounding_text(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "beforeafter".into(), None));

    editor.update(cx, |editor, cx| {
        let block = editor.doc().blocks()[0].entity.clone();
        editor.on_block_event(
            block,
            &BlockAction::RequestPasteMultiline {
                leading: BlockText::plain("before"),
                lines: vec![
                    "```rust".to_string(),
                    "fn main() {}".to_string(),
                    "```".to_string(),
                ],
                trailing: BlockText::plain("after"),
                split_physical_lines: false,
            },
            cx,
        );

        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].entity.read(cx).display_text(), "before");
        assert_eq!(
            entries[1].entity.read(cx).kind(),
            BlockKind::CodeBlock {
                language: Some("rust".into())
            }
        );
        assert_eq!(entries[1].entity.read(cx).display_text(), "fn main() {}");
        assert_eq!(entries[2].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[2].entity.read(cx).display_text(), "after");
        // Text already follows the code block, so no extra trailing
        // paragraph is added mid-document.
    });
}

#[gpui::test]
async fn structural_paste_at_document_end_adds_one_trailing_paragraph(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "intro".into(), None));

    editor.update(cx, |editor, cx| {
        let block = editor.doc().blocks()[0].entity.clone();
        block.update(cx, |block, _cx| {
            block.selected_range = block.display_len()..block.display_len();
        });
        editor.on_block_event(
            block,
            &BlockAction::RequestPasteMultiline {
                leading: BlockText::plain("intro"),
                lines: vec!["***".to_string()],
                trailing: BlockText::plain(String::new()),
                split_physical_lines: false,
            },
            cx,
        );

        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].entity.read(cx).display_text(), "intro");
        assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::ThematicBreak);
        assert_eq!(entries[2].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[2].entity.read(cx).display_text(), "");
    });
}

#[gpui::test]
async fn structural_paste_of_quote_at_document_end_adds_trailing_paragraph(
    cx: &mut TestAppContext,
) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "intro".into(), None));

    editor.update(cx, |editor, cx| {
        let block = editor.doc().blocks()[0].entity.clone();
        block.update(cx, |block, _cx| {
            block.selected_range = block.display_len()..block.display_len();
        });
        editor.on_block_event(
            block,
            &BlockAction::RequestPasteMultiline {
                leading: BlockText::plain("intro"),
                lines: vec!["> quoted".to_string()],
                trailing: BlockText::plain(String::new()),
                split_physical_lines: false,
            },
            cx,
        );

        // The quote container cannot hold the caret below it, so a trailing
        // paragraph is added even though quote normalization re-parses the
        // whole document on the way.
        let roots = editor.doc().root_blocks();
        assert_eq!(roots.len(), 3);
        assert_eq!(roots[0].read(cx).display_text(), "intro");
        assert_eq!(roots[1].read(cx).kind(), BlockKind::Blockquote);
        assert_eq!(roots[2].read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(roots[2].read(cx).display_text(), "");
    });
}

#[gpui::test]
async fn structural_paste_of_callout_at_document_end_adds_trailing_paragraph(
    cx: &mut TestAppContext,
) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "intro".into(), None));

    editor.update(cx, |editor, cx| {
        let block = editor.doc().blocks()[0].entity.clone();
        block.update(cx, |block, _cx| {
            block.selected_range = block.display_len()..block.display_len();
        });
        editor.on_block_event(
            block,
            &BlockAction::RequestPasteMultiline {
                leading: BlockText::plain("intro"),
                lines: vec!["> [!NOTE]".to_string(), "> body".to_string()],
                trailing: BlockText::plain(String::new()),
                split_physical_lines: false,
            },
            cx,
        );

        let roots = editor.doc().root_blocks();
        assert_eq!(roots.len(), 3);
        assert_eq!(
            roots[1].read(cx).kind(),
            BlockKind::Callout(CalloutKind::Note)
        );
        assert_eq!(roots[2].read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(roots[2].read(cx).display_text(), "");
    });
}

#[gpui::test]
async fn structural_paste_of_footnote_definition_at_document_end_adds_trailing_paragraph(
    cx: &mut TestAppContext,
) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "intro".into(), None));

    editor.update(cx, |editor, cx| {
        let block = editor.doc().blocks()[0].entity.clone();
        block.update(cx, |block, _cx| {
            block.selected_range = block.display_len()..block.display_len();
        });
        editor.on_block_event(
            block,
            &BlockAction::RequestPasteMultiline {
                leading: BlockText::plain("intro"),
                lines: vec!["[^note]: definition body".to_string()],
                trailing: BlockText::plain(String::new()),
                split_physical_lines: false,
            },
            cx,
        );

        let roots = editor.doc().root_blocks();
        assert_eq!(roots.len(), 3);
        assert_eq!(roots[1].read(cx).kind(), BlockKind::FootnoteDefinition);
        assert_eq!(roots[2].read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(roots[2].read(cx).display_text(), "");
    });
}

#[gpui::test]
async fn structural_paste_of_standalone_image_at_document_end_adds_trailing_paragraph(
    cx: &mut TestAppContext,
) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "intro".into(), None));

    editor.update(cx, |editor, cx| {
        let block = editor.doc().blocks()[0].entity.clone();
        block.update(cx, |block, _cx| {
            block.selected_range = block.display_len()..block.display_len();
        });
        editor.on_block_event(
            block,
            &BlockAction::RequestPasteMultiline {
                leading: BlockText::plain("intro"),
                lines: vec!["![alt](pic.png)".to_string()],
                trailing: BlockText::plain(String::new()),
                split_physical_lines: false,
            },
            cx,
        );

        // A lone image renders as a self-contained widget, so it gets the
        // same trailing paragraph even though it is a paragraph block.
        let roots = editor.doc().root_blocks();
        assert_eq!(roots.len(), 3);
        assert!(roots[1].read(cx).is_standalone_image());
        assert_eq!(roots[2].read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(roots[2].read(cx).display_text(), "");
    });
}

#[gpui::test]
async fn plain_multiline_paste_with_blank_script_lines_skips_separator_blanks(
    cx: &mut TestAppContext,
) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, String::new(), None));

    editor.update(cx, |editor, cx| {
        let block = editor.doc().blocks()[0].entity.clone();
        editor.on_block_event(
            block,
            &BlockAction::RequestPasteMultiline {
                leading: BlockText::plain(String::new()),
                lines: vec![
                    "H~2~O".to_string(),
                    String::new(),
                    "CO<sub>2</sub>".to_string(),
                    String::new(),
                    "x<sup>n</sup>".to_string(),
                    String::new(),
                ],
                trailing: BlockText::plain(String::new()),
                split_physical_lines: true,
            },
            cx,
        );

        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].entity.read(cx).display_text(), "H2O");
        assert_eq!(entries[1].entity.read(cx).display_text(), "CO2");
        assert_eq!(entries[2].entity.read(cx).display_text(), "xn");
    });
}

#[gpui::test]
async fn plain_multiline_paste_with_leading_inline_html_splits_physical_lines(
    cx: &mut TestAppContext,
) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, String::new(), None));

    editor.update(cx, |editor, cx| {
        let block = editor.doc().blocks()[0].entity.clone();
        editor.on_block_event(
            block,
            &BlockAction::RequestPasteMultiline {
                leading: BlockText::plain(String::new()),
                lines: vec![
                    "<sub>2</sub>".to_string(),
                    "<sup>n</sup>".to_string(),
                    "<span style=\"color:red\">x</span>".to_string(),
                ],
                trailing: BlockText::plain(String::new()),
                split_physical_lines: true,
            },
            cx,
        );

        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].entity.read(cx).display_text(), "2");
        assert_eq!(entries[1].entity.read(cx).display_text(), "n");
        assert_eq!(entries[2].entity.read(cx).display_text(), "x");
        assert_eq!(
            editor.doc().serialize_markdown(cx),
            "<sub>2</sub>\n\n<sup>n</sup>\n\n<span style=\"color: rgba(255,0,0,1.000);\">x</span>"
        );
    });
}

#[gpui::test]
async fn plain_paste_preserves_tibetan_spaces(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, String::new(), None));
    let tibetan = "༄༅།།དཔལ་ལྡན་རྩ་བའི་བླ་མ་རིན་པོ་ཆེ།། བདག་གི་སྤྱི་བོར་པདྨའི་གདན་བཞུགས་ནས།། ";

    editor.update(cx, |editor, cx| {
        let block = editor.doc().blocks()[0].entity.clone();
        editor.on_block_event(
            block,
            &BlockAction::RequestPasteMultiline {
                leading: BlockText::plain(String::new()),
                lines: vec![tibetan.to_string()],
                trailing: BlockText::plain(String::new()),
                split_physical_lines: true,
            },
            cx,
        );

        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].entity.read(cx).display_text(), tibetan);
        assert!(entries[0].entity.read(cx).display_text().contains("།། བདག"));
        assert!(entries[0].entity.read(cx).display_text().ends_with(' '));
        assert_eq!(editor.doc().serialize_markdown(cx), tibetan);
    });
}

#[gpui::test]
async fn nested_list_item_backspace_downgrades_to_direct_list_child(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let editor = cx.new(|cx| Editor::from_markdown(cx, "- a\n  - b".to_string(), None));

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let nested = editor.doc().blocks()[1].entity.clone();
            nested.update(cx, |block, block_cx| {
                block.move_to(0, block_cx);
                block.on_delete_back(&DeleteBack, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].entity.read(cx).kind(), BlockKind::BulletListItem);
        assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[1].entity.read(cx).display_text(), "b");
        assert_eq!(entries[1].entity.read(cx).render_depth, 1);
        assert_eq!(editor.doc().serialize_markdown(cx), "- a\n\n  b");
    });
}

#[gpui::test]
async fn empty_nested_list_item_backspace_twice_exits_to_outer_paragraph(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let editor = cx.new(|cx| Editor::from_markdown(cx, "- a\n  - ".to_string(), None));

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let nested = editor.doc().blocks()[1].entity.clone();
            nested.update(cx, |block, block_cx| {
                block.move_to(0, block_cx);
                block.on_delete_back(&DeleteBack, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[1].entity.read(cx).display_text(), "");
        assert_eq!(entries[1].entity.read(cx).render_depth, 1);
        assert_eq!(editor.doc().serialize_markdown(cx), "- a\n  ");
    });

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let child = editor.doc().blocks()[1].entity.clone();
            child.update(cx, |block, block_cx| {
                block.move_to(0, block_cx);
                block.on_delete_back(&DeleteBack, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].entity.read(cx).kind(), BlockKind::BulletListItem);
        assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[1].entity.read(cx).display_text(), "");
        assert_eq!(entries[1].entity.read(cx).render_depth, 0);
        assert_eq!(editor.doc().serialize_markdown(cx), "- a\n\n");
    });
}

#[gpui::test]
async fn nested_list_item_downgrade_hoists_children_after_paragraph(cx: &mut TestAppContext) {
    let editor =
        cx.new(|cx| Editor::from_markdown(cx, "- a\n  - b\n    - c\n  - d".to_string(), None));

    editor.update(cx, |editor, cx| {
        let nested = editor.doc().blocks()[1].entity.clone();
        editor.on_block_event(
            nested,
            &BlockAction::RequestDowngradeNestedListItemToChildParagraph,
            cx,
        );

        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].entity.read(cx).kind(), BlockKind::BulletListItem);
        assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[1].entity.read(cx).display_text(), "b");
        assert_eq!(entries[1].entity.read(cx).render_depth, 1);
        assert_eq!(entries[2].entity.read(cx).kind(), BlockKind::BulletListItem);
        assert_eq!(entries[2].entity.read(cx).display_text(), "c");
        assert_eq!(entries[2].entity.read(cx).render_depth, 1);
        assert_eq!(entries[3].entity.read(cx).kind(), BlockKind::BulletListItem);
        assert_eq!(entries[3].entity.read(cx).display_text(), "d");
        assert_eq!(entries[3].entity.read(cx).render_depth, 1);
        assert_eq!(editor.doc().serialize_markdown(cx), "- a\n\n  b\n  - c\n  - d");
    });
}

#[gpui::test]
async fn nested_numbered_and_task_items_backspace_downgrade_to_list_child(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();

    let numbered = cx.new(|cx| Editor::from_markdown(cx, "1. a\n  1. b".to_string(), None));
    cx.update(|window, cx| {
        numbered.update(cx, |editor, cx| {
            let nested = editor.doc().blocks()[1].entity.clone();
            nested.update(cx, |block, block_cx| {
                block.move_to(0, block_cx);
                block.on_delete_back(&DeleteBack, window, block_cx);
            });
        });
    });
    numbered.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[1].entity.read(cx).display_text(), "b");
        assert_eq!(entries[1].entity.read(cx).render_depth, 1);
        assert_eq!(editor.doc().serialize_markdown(cx), "1. a\n\n  b");
    });

    let task = cx.new(|cx| Editor::from_markdown(cx, "- [ ] a\n  - [ ] b".to_string(), None));
    cx.update(|window, cx| {
        task.update(cx, |editor, cx| {
            let nested = editor.doc().blocks()[1].entity.clone();
            nested.update(cx, |block, block_cx| {
                block.move_to(0, block_cx);
                block.on_delete_back(&DeleteBack, window, block_cx);
            });
        });
    });
    task.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[1].entity.read(cx).display_text(), "b");
        assert_eq!(entries[1].entity.read(cx).render_depth, 1);
        assert_eq!(editor.doc().serialize_markdown(cx), "- [ ] a\n\n  b");
    });
}

#[gpui::test]
async fn request_quote_break_creates_nested_leaf_quote_group(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "> outer\n>> inner".to_string(), None));

    editor.update(cx, |editor, cx| {
        let nested_quote = editor.doc().blocks()[1].entity.clone();
        editor.on_block_event(nested_quote, &BlockAction::RequestQuoteBreak, cx);

        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].entity.read(cx).kind(), BlockKind::Blockquote);
        assert_eq!(entries[0].entity.read(cx).display_text(), "outer");
        assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::Blockquote);
        assert_eq!(entries[1].entity.read(cx).display_text(), "inner");
        assert_eq!(entries[1].entity.read(cx).quote_depth, 2);
        assert_eq!(entries[2].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[2].entity.read(cx).display_text(), "");
        assert_eq!(entries[2].entity.read(cx).quote_depth, 1);
        assert_eq!(entries[3].entity.read(cx).kind(), BlockKind::Blockquote);
        assert_eq!(entries[3].entity.read(cx).display_text(), "");
        assert_eq!(entries[3].entity.read(cx).quote_depth, 2);
        assert_eq!(editor.doc().serialize_markdown(cx), "> outer\n> > inner\n> \n> > ");
        assert_eq!(
            editor.tab().focus.pending,
            Some(entries[3].entity.entity_id())
        );
    });
}

#[gpui::test]
async fn imported_leaf_quote_backspace_twice_downgrades_to_text_block(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let editor = cx.new(|cx| Editor::from_markdown(cx, "> a".to_string(), None));

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let quote = editor.doc().first_root().expect("root quote").clone();
            quote.update(cx, |block, block_cx| {
                block.move_to(block.display_len(), block_cx);
                block.on_delete_back(&DeleteBack, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].entity.read(cx).kind(), BlockKind::Blockquote);
        assert_eq!(entries[0].entity.read(cx).display_text(), "");
        assert_eq!(entries[0].entity.read(cx).quote_depth, 1);
        assert_eq!(editor.doc().serialize_markdown(cx), "> ");
    });

    let empty_quote_id = editor.update(cx, |editor, _cx| {
        editor.doc().first_root().expect("empty quote").entity_id()
    });

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let quote = editor.doc().first_root().expect("empty quote").clone();
            quote.update(cx, |block, block_cx| {
                block.move_to(0, block_cx);
                block.on_delete_back(&DeleteBack, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[0].entity.read(cx).display_text(), "");
        assert_eq!(entries[0].entity.read(cx).quote_depth, 0);
        assert_eq!(entries[0].entity.entity_id(), empty_quote_id);
        assert_eq!(editor.doc().serialize_markdown(cx), "");
    });
}

#[gpui::test]
async fn shortcut_created_leaf_quote_backspace_twice_downgrades_to_text_block(
    cx: &mut TestAppContext,
) {
    let cx = cx.add_empty_window();
    let editor = cx.new(|cx| Editor::from_markdown(cx, String::new(), None));

    editor.update(cx, |editor, cx| {
        let paragraph = editor.doc().first_root().expect("root paragraph").clone();
        paragraph.update(cx, |block, cx| {
            block.prepare_undo_capture(
                crate::editor::block_protocol::UndoCaptureKind::CoalescibleText,
                cx,
            );
            block.replace_text_in_display_range(0..0, "> ", None, false, cx);
            block.replace_text_in_display_range(0..0, "a", None, false, cx);
        });
    });

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let quote = editor.doc().first_root().expect("shortcut quote").clone();
            quote.update(cx, |block, block_cx| {
                block.move_to(block.display_len(), block_cx);
                block.on_delete_back(&DeleteBack, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let quote = editor.doc().first_root().expect("empty shortcut quote");
        assert_eq!(quote.read(cx).kind(), BlockKind::Blockquote);
        assert_eq!(quote.read(cx).display_text(), "");
        assert_eq!(quote.read(cx).quote_depth, 1);
        assert_eq!(editor.doc().serialize_markdown(cx), "> ");
    });

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let quote = editor
                .doc()
                .first_root()
                .expect("empty shortcut quote")
                .clone();
            quote.update(cx, |block, block_cx| {
                block.move_to(0, block_cx);
                block.on_delete_back(&DeleteBack, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let paragraph = editor
            .doc()
            .first_root()
            .expect("text block after downgrade");
        assert_eq!(paragraph.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(paragraph.read(cx).display_text(), "");
        assert_eq!(editor.doc().serialize_markdown(cx), "");
    });
}

#[gpui::test]
async fn root_quote_break_then_backspace_keeps_text_block_slot_after_group(
    cx: &mut TestAppContext,
) {
    let cx = cx.add_empty_window();
    let editor = cx.new(|cx| Editor::from_markdown(cx, "> side\n>\n> 1234".to_string(), None));

    let new_leaf_id = editor.update(cx, |editor, cx| {
        let quote = editor.doc().first_root().expect("group quote").clone();
        editor.on_block_event(quote, &BlockAction::RequestQuoteBreak, cx);
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::Blockquote);
        assert_eq!(entries[1].entity.read(cx).display_text(), "");
        entries[1].entity.entity_id()
    });

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let new_leaf = editor.doc().blocks()[1].entity.clone();
            new_leaf.update(cx, |block, block_cx| {
                block.move_to(0, block_cx);
                block.on_delete_back(&DeleteBack, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].entity.read(cx).kind(), BlockKind::Blockquote);
        assert_eq!(entries[0].entity.read(cx).display_text(), "side\n\n1234");
        assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[1].entity.read(cx).display_text(), "");
        assert_eq!(entries[1].entity.entity_id(), new_leaf_id);
        assert_eq!(entries[1].entity.read(cx).quote_depth, 0);
        assert_eq!(editor.doc().serialize_markdown(cx), "> side\n> \n> 1234\n\n");
    });
}

#[gpui::test]
async fn empty_callout_body_backspace_downgrades_parent_to_quote(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let editor = cx.new(|cx| Editor::from_markdown(cx, "> [!NOTE]\n> ".to_string(), None));

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let body = editor.doc().blocks()[1].entity.clone();
            body.update(cx, |block, block_cx| {
                block.move_to(0, block_cx);
                block.on_delete_back(&DeleteBack, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].entity.read(cx).kind(), BlockKind::Blockquote);
        assert_eq!(entries[0].entity.read(cx).display_text(), "[!NOTE]");
        assert_eq!(editor.doc().serialize_markdown(cx), "> \\[!NOTE]");
    });
}

#[gpui::test]
async fn callout_exit_break_creates_plain_text_block(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let editor = cx.new(|cx| Editor::from_markdown(cx, "> [!TIP]\n> body".to_string(), None));

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let body = editor.doc().blocks()[1].entity.clone();
            body.update(cx, |block, block_cx| {
                block.on_exit_code_block(&ExitCodeBlock, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 3);
        assert_eq!(
            entries[0].entity.read(cx).kind(),
            BlockKind::Callout(CalloutKind::Tip)
        );
        assert_eq!(entries[2].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[2].entity.read(cx).display_text(), "");
        assert_eq!(entries[2].entity.read(cx).quote_depth, 0);
        assert_eq!(editor.doc().serialize_markdown(cx), "> [!TIP]\n> body\n\n");
        assert_eq!(
            editor.tab().focus.pending,
            Some(entries[2].entity.entity_id())
        );
    });
}

#[gpui::test]
async fn delete_on_empty_leaf_quote_downgrades_to_text_block(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let editor = cx.new(|cx| Editor::from_markdown(cx, "> ".to_string(), None));

    let empty_quote_id = editor.update(cx, |editor, _cx| {
        editor.doc().first_root().expect("empty quote").entity_id()
    });

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let quote = editor.doc().first_root().expect("empty quote").clone();
            quote.update(cx, |block, block_cx| {
                block.move_to(0, block_cx);
                block.on_delete(&Delete, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[0].entity.read(cx).display_text(), "");
        assert_eq!(entries[0].entity.entity_id(), empty_quote_id);
        assert_eq!(editor.doc().serialize_markdown(cx), "");
    });
}

#[gpui::test]
async fn quote_container_with_children_does_not_collapse_from_leaf_exit_path(
    cx: &mut TestAppContext,
) {
    let cx = cx.add_empty_window();
    let editor = cx.new(|cx| Editor::from_markdown(cx, ">\n> - item".to_string(), None));

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let quote = editor.doc().first_root().expect("container quote").clone();
            quote.update(cx, |block, block_cx| {
                block.move_to(0, block_cx);
                block.on_delete_back(&DeleteBack, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].entity.read(cx).kind(), BlockKind::Blockquote);
        assert_eq!(entries[0].entity.read(cx).display_text(), "");
        assert_eq!(entries[0].entity.read(cx).quote_depth, 1);
        assert!(!entries[0].entity.read(cx).children.is_empty());
        assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::BulletListItem);
        assert_eq!(editor.doc().serialize_markdown(cx), "> - item");
    });
}

#[gpui::test]
async fn quote_newline_inside_title_stays_in_one_source_authoritative_group(
    cx: &mut TestAppContext,
) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "> firstsecond".to_string(), None));

    editor.update(cx, |editor, cx| {
        let quote = editor.doc().first_root().expect("root quote").clone();
        quote.update(cx, |block, cx| {
            block.prepare_undo_capture(
                crate::editor::block_protocol::UndoCaptureKind::NonCoalescible,
                cx,
            );
            block.replace_text_in_display_range(5..5, "\n", None, false, cx);
        });

        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].entity.read(cx).kind(), BlockKind::Blockquote);
        assert_eq!(entries[0].entity.read(cx).display_text(), "first\nsecond");
        assert_eq!(editor.doc().serialize_markdown(cx), "> first\n> second");
    });
}

#[gpui::test]
async fn root_quote_enter_stays_in_same_group(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let editor = cx.new(|cx| Editor::from_markdown(cx, "> first".to_string(), None));

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let quote = editor.doc().first_root().expect("root quote").clone();
            quote.update(cx, |block, block_cx| {
                block.move_to(block.display_len(), block_cx);
            });
            quote.update(cx, |block, block_cx| {
                block.on_newline(&Newline, window, block_cx);
            });
        });
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].entity.read(cx).kind(), BlockKind::Blockquote);
        assert_eq!(entries[0].entity.read(cx).display_text(), "first");
        assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::Paragraph);
        assert_eq!(entries[1].entity.read(cx).display_text(), "");
        assert_eq!(entries[1].entity.read(cx).quote_depth, 1);
        assert_eq!(editor.doc().serialize_markdown(cx), "> first\n> ");
    });
}

#[gpui::test]
async fn multiline_edit_inside_quote_reparses_into_child_blocks(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "> first".to_string(), None));

    editor.update(cx, |editor, cx| {
        let quote = editor.doc().first_root().expect("root quote").clone();
        quote.update(cx, |block, cx| {
            block.prepare_undo_capture(
                crate::editor::block_protocol::UndoCaptureKind::NonCoalescible,
                cx,
            );
            block.replace_text_in_display_range(5..5, "\n- item", None, false, cx);
        });
    });

    editor.update(cx, |editor, cx| {
        let entries = editor.doc().blocks();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].entity.read(cx).kind(), BlockKind::Blockquote);
        assert_eq!(entries[0].entity.read(cx).display_text(), "first");
        assert_eq!(entries[1].entity.read(cx).kind(), BlockKind::BulletListItem);
        assert_eq!(entries[1].entity.read(cx).display_text(), "item");
        assert_eq!(editor.doc().serialize_markdown(cx), "> first\n> - item");
    });
}
