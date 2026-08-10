//! End-to-end block event tests for the editor input pipeline.
//!
//! Split from `block_events.rs` so the implementation file stays small;
//! these tests drive `Editor::on_block_event` through the full pipeline.

#[cfg(test)]
mod tests {
    use crate::editor::block_protocol::BlockAction;
    use crate::editor::controller::Editor;
    use crate::editor::editing::input::actions::ExitCodeBlock;
    use crate::editor::editing::input::actions::{DeleteBack, Newline};
    use crate::model::block::{BlockData, BlockKind, CalloutKind};
    use crate::model::inline::text::RichText;
    use gpui::{AppContext, TestAppContext};

    #[gpui::test]
    async fn request_quote_break_creates_new_root_leaf_quote_group(cx: &mut TestAppContext) {
        let editor = cx.new(|cx| Editor::from_markdown(cx, "> first".to_string(), None));

        editor.update(cx, |editor, cx| {
            let quote = editor.doc().first_root().expect("root quote").clone();
            editor.on_block_event(quote, &BlockAction::RequestQuoteBreak, cx);

            let visible = editor.doc().blocks();
            assert_eq!(visible.len(), 2);
            assert_eq!(visible[0].entity.read(cx).kind(), BlockKind::Blockquote);
            assert_eq!(visible[0].entity.read(cx).display_text(), "first");
            assert_eq!(visible[1].entity.read(cx).kind(), BlockKind::Blockquote);
            assert_eq!(visible[1].entity.read(cx).display_text(), "");
            assert_eq!(visible[1].entity.read(cx).quote_depth, 1);
            assert_eq!(editor.doc().to_markdown(cx), "> first\n\n> ");
            assert_eq!(
                editor.tab().focus.pending,
                Some(visible[1].entity.entity_id())
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
                block.replace_text_in_visible_range(0..0, "> ", None, false, cx);
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.doc().blocks();
            assert_eq!(visible.len(), 1);
            assert_eq!(visible[0].entity.read(cx).kind(), BlockKind::Blockquote);
            assert_eq!(visible[0].entity.read(cx).display_text(), "");
            assert_eq!(visible[0].entity.read(cx).quote_depth, 1);
            assert_eq!(editor.doc().to_markdown(cx), "> ");
        });
    }

    #[gpui::test]
    async fn footnote_reference_jump_and_backref_follow_in_place_definition(
        cx: &mut TestAppContext,
    ) {
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
                .find(|visible| visible.entity.read(cx).kind() == BlockKind::FootnoteDefinition)
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
                .current_range_for_footnote_occurrence(0)
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
                &RichText::plain("before"),
                "![image](./assets/image.png)",
                &RichText::plain("after"),
                cx,
            );

            let visible = editor.doc().blocks();
            assert_eq!(visible.len(), 3);
            assert_eq!(visible[0].entity.read(cx).display_text(), "before");
            assert_eq!(
                visible[1].entity.read(cx).display_text(),
                "![image](./assets/image.png)"
            );
            assert!(visible[1].entity.read(cx).image_runtime().is_some());
            assert_eq!(visible[2].entity.read(cx).display_text(), "after");
        });
    }

    #[gpui::test]
    async fn image_paste_text_in_code_block_stays_inside_block(cx: &mut TestAppContext) {
        let editor =
            cx.new(|cx| Editor::from_markdown(cx, "```\nbeforeafter\n```".to_string(), None));

        editor.update(cx, |editor, cx| {
            let block = editor.doc().first_root().expect("code block").clone();
            editor.replace_current_block_selection_with_image_text(
                &block,
                &RichText::plain("before"),
                "![image](./assets/image.png)",
                &RichText::plain("after"),
                cx,
            );

            let visible = editor.doc().blocks();
            assert_eq!(visible.len(), 1);
            assert_eq!(
                visible[0].entity.read(cx).kind(),
                BlockKind::CodeBlock { language: None }
            );
            assert_eq!(
                visible[0].entity.read(cx).display_text(),
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
                block.replace_text_in_visible_range(0..0, "> [!NOTE]", None, false, cx);
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.doc().blocks();
            assert_eq!(visible.len(), 2);
            assert_eq!(
                visible[0].entity.read(cx).kind(),
                BlockKind::Callout(CalloutKind::Note)
            );
            assert_eq!(visible[1].entity.read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(visible[1].entity.read(cx).display_text(), "");
            assert_eq!(visible[1].entity.read(cx).quote_depth, 1);
            assert_eq!(editor.doc().to_markdown(cx), "> [!NOTE]\n> ");
            assert_eq!(
                editor.tab().focus.pending,
                Some(visible[1].entity.entity_id())
            );
        });
    }

    #[gpui::test]
    async fn typing_numbered_list_shortcut_after_separator_preserves_group_boundary(
        cx: &mut TestAppContext,
    ) {
        let editor =
            cx.new(|cx| Editor::from_markdown(cx, "1. aa\n2. bb\n3. cc".to_string(), None));

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
                block.replace_text_in_visible_range(0..0, "1. ", None, false, cx);
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.doc().blocks();
            assert_eq!(visible.len(), 5);
            assert_eq!(visible[0].entity.read(cx).list_ordinal, Some(1));
            assert_eq!(visible[1].entity.read(cx).list_ordinal, Some(2));
            assert_eq!(visible[2].entity.read(cx).list_ordinal, Some(3));
            assert_eq!(visible[3].entity.read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(visible[3].entity.read(cx).display_text(), "");
            assert_eq!(visible[4].entity.entity_id(), separator_id);
            assert_eq!(
                visible[4].entity.read(cx).kind(),
                BlockKind::NumberedListItem
            );
            assert_eq!(visible[4].entity.read(cx).display_text(), "");
            assert_eq!(visible[4].entity.read(cx).list_ordinal, Some(1));
            assert_eq!(editor.doc().to_markdown(cx), "1. aa\n2. bb\n3. cc\n\n1. ");
        });
    }

    #[gpui::test]
    async fn request_indent_nests_non_empty_list_item(cx: &mut TestAppContext) {
        let editor = cx.new(|cx| Editor::from_markdown(cx, "- a\n- b".to_string(), None));

        editor.update(cx, |editor, cx| {
            let second = editor.doc().blocks()[1].entity.clone();
            editor.on_block_event(second, &BlockAction::RequestIndent, cx);

            let visible = editor.doc().blocks();
            assert_eq!(visible.len(), 2);
            assert_eq!(visible[0].entity.read(cx).kind(), BlockKind::BulletListItem);
            assert_eq!(visible[1].entity.read(cx).kind(), BlockKind::BulletListItem);
            assert_eq!(visible[1].entity.read(cx).render_depth, 1);
            assert_eq!(editor.doc().to_markdown(cx), "- a\n  - b");
        });
    }

    #[gpui::test]
    async fn request_outdent_lifts_list_child_paragraph_after_parent(cx: &mut TestAppContext) {
        let editor =
            cx.new(|cx| Editor::from_markdown(cx, "- item\n\n  child text".to_string(), None));

        let child_id = editor.update(cx, |editor, cx| {
            let child = editor.doc().blocks()[1].entity.clone();
            editor.on_block_event(child.clone(), &BlockAction::RequestOutdent, cx);
            child.entity_id()
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.doc().blocks();
            assert_eq!(visible.len(), 2);
            assert_eq!(visible[0].entity.read(cx).kind(), BlockKind::BulletListItem);
            assert_eq!(visible[0].entity.read(cx).display_text(), "item");
            assert_eq!(visible[1].entity.read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(visible[1].entity.read(cx).display_text(), "child text");
            assert_eq!(visible[1].entity.read(cx).render_depth, 0);
            assert_eq!(visible[1].entity.entity_id(), child_id);
            assert_eq!(editor.doc().to_markdown(cx), "- item\n\nchild text");
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
                    block.replace_text_in_visible_range(
                        0..block.visible_len(),
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
            let visible = editor.doc().blocks();
            assert_eq!(visible.len(), 2);
            assert_eq!(visible[0].entity.read(cx).kind(), BlockKind::BulletListItem);
            assert_eq!(visible[1].entity.read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(visible[1].entity.read(cx).display_text(), "");
            assert_eq!(visible[1].entity.entity_id(), child_id);
            assert_eq!(visible[1].entity.read(cx).render_depth, 0);
            assert_eq!(editor.doc().to_markdown(cx), "- item\n\n");
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
                    block.replace_text_in_visible_range(
                        0..block.visible_len(),
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
            let visible = editor.doc().blocks();
            assert_eq!(visible.len(), 3);
            assert_eq!(visible[0].entity.read(cx).kind(), BlockKind::BulletListItem);
            assert_eq!(visible[1].entity.read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(visible[1].entity.read(cx).display_text(), "");
            // Enter splits into a fresh block; the structure (two empty
            // list-child paragraphs) is what matters.
            assert_eq!(visible[1].entity.read(cx).render_depth, 1);
            assert_eq!(visible[2].entity.read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(visible[2].entity.read(cx).display_text(), "");
            assert_eq!(visible[2].entity.read(cx).render_depth, 1);
            assert_eq!(editor.doc().to_markdown(cx), "- item\n  \n  ");
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
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.doc().blocks();
            assert_eq!(visible.len(), 2);
            assert_eq!(visible[0].entity.read(cx).display_text(), "H2O");
            assert_eq!(visible[1].entity.read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(visible[1].entity.read(cx).display_text(), "");
            assert_eq!(editor.doc().to_markdown(cx), "H~2~O\n\n");
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
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.doc().blocks();
            assert_eq!(visible.len(), 2);
            assert_eq!(visible[0].entity.read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(visible[0].entity.read(cx).display_text(), "$n^2$");
            assert!(!visible[0].entity.read(cx).uses_raw_text_editing());
            assert_eq!(visible[1].entity.read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(visible[1].entity.read(cx).display_text(), "");
            assert_eq!(editor.doc().to_markdown(cx), "$n^2$\n\n");
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
                    let end = block.visible_len();
                    block.replace_text_in_visible_range(end..end, "\n```", None, false, block_cx);
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.doc().blocks();
            assert_eq!(visible.len(), 2);
            assert_eq!(
                visible[0].entity.read(cx).kind(),
                BlockKind::CodeBlock {
                    language: Some("rust".into())
                }
            );
            assert_eq!(visible[0].entity.read(cx).display_text(), "let x = 1;");
            assert_eq!(visible[1].entity.read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(visible[1].entity.read(cx).display_text(), "");
            assert_eq!(editor.doc().to_markdown(cx), "```rust\nlet x = 1;\n```\n\n");
        });
    }

    #[gpui::test]
    async fn setext_equals_underline_enter_promotes_previous_paragraph_to_h1(
        cx: &mut TestAppContext,
    ) {
        let cx = cx.add_empty_window();
        let editor = cx.new(|cx| Editor::from_markdown(cx, "Title\n\n=====".to_string(), None));

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let underline = editor.doc().blocks()[1].entity.clone();
                underline.update(cx, |block, block_cx| {
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.doc().blocks();
            assert_eq!(visible.len(), 2);
            assert_eq!(
                visible[0].entity.read(cx).kind(),
                BlockKind::Heading { level: 1 }
            );
            assert_eq!(visible[0].entity.read(cx).display_text(), "Title");
            assert_eq!(visible[1].entity.read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(visible[1].entity.read(cx).display_text(), "");
            assert_eq!(editor.doc().to_markdown(cx), "# Title\n\n");
        });

        // Reversible: undo restores the two original paragraphs.
        editor.update(cx, |editor, cx| {
            editor.undo_document(cx);
            assert_eq!(editor.doc().to_markdown(cx), "Title\n\n=====");
        });
    }

    #[gpui::test]
    async fn setext_dash_underline_enter_promotes_previous_paragraph_to_h2(
        cx: &mut TestAppContext,
    ) {
        let cx = cx.add_empty_window();
        // A bare "-----" in source parses as a thematic break, so simulate the
        // user typing the underline into the paragraph below the title instead.
        let editor = cx.new(|cx| Editor::from_markdown(cx, "Title\n\nx".to_string(), None));

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let underline = editor.doc().blocks()[1].entity.clone();
                underline.update(cx, |block, block_cx| {
                    let end = block.visible_len();
                    block.replace_text_in_visible_range(0..end, "-----", None, false, block_cx);
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.doc().blocks();
            assert_eq!(
                visible[0].entity.read(cx).kind(),
                BlockKind::Heading { level: 2 }
            );
            assert_eq!(visible[0].entity.read(cx).display_text(), "Title");
            assert_eq!(editor.doc().to_markdown(cx), "## Title\n\n");
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
                    block.replace_text_in_visible_range(0..0, "-----", None, false, block_cx);
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.doc().blocks();
            assert_eq!(visible[0].entity.read(cx).kind(), BlockKind::ThematicBreak);
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
                    block.replace_text_in_visible_range(0..0, "=====", None, false, block_cx);
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.doc().blocks();
            assert_eq!(visible[0].entity.read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(visible[0].entity.read(cx).display_text(), "=====");
        });
    }

    #[gpui::test]
    async fn delimiter_row_enter_forms_native_table(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let editor = cx.new(|cx| {
            Editor::from_markdown(cx, "| Name | Score |\n\n| --- | --- |".to_string(), None)
        });

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let delimiter = editor.doc().root_blocks()[1].clone();
                delimiter.update(cx, |block, block_cx| {
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let roots = editor.doc().root_blocks();
            assert_eq!(roots.len(), 2);
            assert_eq!(roots[0].read(cx).kind(), BlockKind::Table);
            let table = roots[0].read(cx).record.table.clone().expect("table");
            assert_eq!(table.header.len(), 2);
            assert_eq!(table.header[0].serialize_markdown(), "Name");
            assert!(table.rows.is_empty());
            assert_eq!(roots[1].read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(
                editor.doc().to_markdown(cx),
                "| Name | Score |\n| --- | --- |\n\n"
            );
        });

        // Reversible in one step back to the two source paragraphs.
        editor.update(cx, |editor, cx| {
            editor.undo_document(cx);
            assert_eq!(
                editor.doc().to_markdown(cx),
                "| Name | Score |\n\n| --- | --- |"
            );
        });
    }

    #[gpui::test]
    async fn pipe_row_below_table_is_absorbed_as_a_row(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let editor = cx.new(|cx| {
            Editor::from_markdown(cx, "| Name | Score |\n\n| --- | --- |".to_string(), None)
        });

        // Form the table.
        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let delimiter = editor.doc().root_blocks()[1].clone();
                delimiter.update(cx, |block, block_cx| {
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        // Type a body row into the paragraph below the table and press Enter.
        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let row = editor.doc().root_blocks()[1].clone();
                row.update(cx, |block, block_cx| {
                    block.replace_text_in_visible_range(
                        0..0,
                        "| Alice | 10 |",
                        None,
                        false,
                        block_cx,
                    );
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let roots = editor.doc().root_blocks();
            assert_eq!(roots[0].read(cx).kind(), BlockKind::Table);
            let table = roots[0].read(cx).record.table.clone().expect("table");
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
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let roots = editor.doc().root_blocks();
            assert_eq!(roots.len(), 2);
            assert_eq!(roots[0].read(cx).kind(), BlockKind::Table);
            let table = roots[0].read(cx).record.table.clone().expect("table");
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
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        // A pipeless body row with the table's column count is absorbed.
        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let row = editor.doc().root_blocks()[1].clone();
                row.update(cx, |block, block_cx| {
                    block.replace_text_in_visible_range(0..0, "Alice | 10", None, false, block_cx);
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let roots = editor.doc().root_blocks();
            assert_eq!(roots[0].read(cx).kind(), BlockKind::Table);
            let table = roots[0].read(cx).record.table.clone().expect("table");
            assert_eq!(table.rows.len(), 1);
            assert_eq!(table.rows[0][0].serialize_markdown(), "Alice");
            assert_eq!(table.rows[0][1].serialize_markdown(), "10");
            assert_eq!(roots[1].read(cx).kind(), BlockKind::Paragraph);
        });
    }

    #[gpui::test]
    async fn ragged_pipeless_row_below_table_is_padded_to_width(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let editor = cx
            .new(|cx| Editor::from_markdown(cx, "A | B | C\n\n--- | --- | ---".to_string(), None));

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let delimiter = editor.doc().root_blocks()[1].clone();
                delimiter.update(cx, |block, block_cx| {
                    block.move_to(block.visible_len(), block_cx);
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
                    block.replace_text_in_visible_range(0..0, "one | two", None, false, block_cx);
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let table = editor.doc().root_blocks()[0]
                .read(cx)
                .record
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
                    block.replace_text_in_visible_range(0..0, "| a | b |", None, false, block_cx);
                    block.move_to(block.visible_len(), block_cx);
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
            let visible = editor.doc().blocks();
            assert_eq!(visible.len(), 2);
            assert_eq!(visible[0].entity.read(cx).kind(), BlockKind::MathBlock);
            assert_eq!(visible[0].entity.read(cx).display_text(), "n^2");
            assert_eq!(visible[1].entity.read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(visible[1].entity.read(cx).display_text(), "");
            assert_eq!(editor.doc().to_markdown(cx), "$$n^2$$\n\n");
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
                    block.replace_text_in_visible_range(
                        0..block.visible_len(),
                        "$$",
                        None,
                        false,
                        block_cx,
                    );
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.doc().blocks();
            assert_eq!(visible.len(), 1);
            let block = visible[0].entity.read(cx);
            assert_eq!(block.kind(), BlockKind::MathBlock);
            // The delimiters are stripped; only the formula body is stored.
            assert_eq!(block.display_text(), "");
            assert_eq!(block.selected_range, 0..0);
            assert!(block.uses_raw_text_editing());
            assert_eq!(editor.doc().to_markdown(cx), "$$\n\n$$");
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
                    block.replace_text_in_visible_range(0..0, "$$", None, false, block_cx);
                    block.move_to("$$".len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.doc().blocks();
            assert_eq!(visible.len(), 1);
            let block = visible[0].entity.read(cx);
            assert_eq!(block.kind(), BlockKind::MathBlock);
            // The pre-existing text is kept as the formula body.
            assert_eq!(block.display_text(), "E = mc^2");
            assert_eq!(block.selected_range, 0..0);
            assert_eq!(editor.doc().to_markdown(cx), "$$E = mc^2$$");
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
            let visible = editor.doc().blocks();
            assert_eq!(visible.len(), 1);
            assert_eq!(visible[0].entity.read(cx).kind(), BlockKind::MathBlock);
            assert_eq!(visible[0].entity.read(cx).display_text(), "n\n^2");
            assert_eq!(editor.doc().to_markdown(cx), "$$\nn\n^2\n$$");
        });
    }

    #[gpui::test]
    async fn auto_created_math_block_exit_shortcut_creates_plain_text_block(
        cx: &mut TestAppContext,
    ) {
        let cx = cx.add_empty_window();
        let editor = cx.new(|cx| Editor::from_markdown(cx, String::new(), None));

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let block = editor.doc().blocks()[0].entity.clone();
                block.update(cx, |block, block_cx| {
                    block.replace_text_in_visible_range(
                        0..block.visible_len(),
                        "$$",
                        None,
                        false,
                        block_cx,
                    );
                    block.move_to(block.visible_len(), block_cx);
                    block.on_newline(&Newline, window, block_cx);
                    block.on_exit_code_block(&ExitCodeBlock, window, block_cx);
                });
            });
        });

        editor.update(cx, |editor, cx| {
            let visible = editor.doc().blocks();
            assert_eq!(visible.len(), 2);
            assert_eq!(visible[0].entity.read(cx).kind(), BlockKind::MathBlock);
            assert_eq!(visible[0].entity.read(cx).display_text(), "");
            assert_eq!(visible[1].entity.read(cx).kind(), BlockKind::Paragraph);
            assert_eq!(visible[1].entity.read(cx).display_text(), "");
            assert_eq!(editor.doc().to_markdown(cx), "$$\n\n$$\n\n");
        });
    }

    #[gpui::test]
    async fn raw_like_block_exit_shortcut_creates_plain_text_block(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let cases = [
            (
                BlockData::new(
                    BlockKind::HtmlBlock,
                    RichText::plain("<div>\ncontent\n</div>".to_string()),
                ),
                BlockKind::HtmlBlock,
                "<div>\ncontent\n</div>",
            ),
            (
                BlockData::new(
                    BlockKind::MermaidBlock,
                    RichText::plain("```mermaid\nflowchart LR\nA-->B\n```".to_string()),
                ),
                BlockKind::MermaidBlock,
                "```mermaid\nflowchart LR\nA-->B\n```",
            ),
            (
                BlockData::new(
                    BlockKind::RawMarkdown,
                    RichText::plain("::: custom\ncontent\n:::".to_string()),
                ),
                BlockKind::RawMarkdown,
                "::: custom\ncontent\n:::",
            ),
            (
                BlockData::new(
                    BlockKind::HtmlComment,
                    RichText::plain("<!--\ncomment\n-->".to_string()),
                ),
                BlockKind::HtmlComment,
                "<!--\ncomment\n-->",
            ),
        ];

        for (record, kind, text) in cases {
            let editor = cx.new(|cx| {
                let mut editor = Editor::from_markdown(cx, String::new(), None);
                let block = Editor::new_block(cx, record.clone());
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
                let visible = editor.doc().blocks();
                assert_eq!(visible.len(), 2);
                assert_eq!(visible[0].entity.read(cx).kind(), kind);
                assert_eq!(visible[0].entity.read(cx).display_text(), text);
                assert_eq!(visible[1].entity.read(cx).kind(), BlockKind::Paragraph);
                assert_eq!(visible[1].entity.read(cx).display_text(), "");
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
                    let runtime = table.table_runtime.as_ref().expect("table runtime");
                    (runtime.rows[0][0].clone(), runtime.rows[1][0].entity_id())
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
                    .table_runtime
                    .as_ref()
                    .expect("table runtime")
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
}
