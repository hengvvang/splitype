//! Unit tests for search and replace subsystem.

use gpui::{AppContext, TestAppContext};

use crate::editor::engine::controller::Editor;
use crate::editor::search::state::SearchScope;

#[gpui::test]
fn test_search_in_document_exact_matches(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| {
        Editor::from_markdown(
            cx,
            "# Heading Alpha\n\nThis is paragraph with Alpha word.\n\nAnother alpha in lowercase.".to_string(),
            None,
        )
    });

    editor.update(cx, |editor, cx| {
        editor.search.search_input.set_text("Alpha".to_string());
        editor.search.match_case = true;
        editor.execute_search(cx);

        assert_eq!(editor.search.matches.len(), 2);
        assert_eq!(editor.search.matches[0].preview_match, "Alpha");
        assert_eq!(editor.search.matches[0].line_number, 1);
        assert_eq!(editor.search.matches[1].preview_match, "Alpha");

        // Case insensitive
        editor.search.match_case = false;
        editor.execute_search(cx);
        assert_eq!(editor.search.matches.len(), 3);
    });
}

#[gpui::test]
fn test_search_whole_word_filter(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| {
        Editor::from_markdown(
            cx,
            "cat concatenate cat_dog cat.".to_string(),
            None,
        )
    });

    editor.update(cx, |editor, cx| {
        editor.search.search_input.set_text("cat".to_string());
        editor.search.whole_word = true;
        editor.execute_search(cx);

        // "cat" and "cat." match; "concatenate" and "cat_dog" do not
        assert_eq!(editor.search.matches.len(), 2);
    });
}

#[gpui::test]
fn test_search_regex_filter(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| {
        Editor::from_markdown(
            cx,
            "Item 100, Item 200, Item ABC".to_string(),
            None,
        )
    });

    editor.update(cx, |editor, cx| {
        editor.search.search_input.set_text(r"Item \d+".to_string());
        editor.search.use_regex = true;
        editor.execute_search(cx);

        assert_eq!(editor.search.matches.len(), 2);
        assert_eq!(editor.search.matches[0].preview_match, "Item 100");
        assert_eq!(editor.search.matches[1].preview_match, "Item 200");
    });
}

#[gpui::test]
fn test_search_next_and_prev_navigation(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| {
        Editor::from_markdown(
            cx,
            "one two one three one".to_string(),
            None,
        )
    });

    editor.update(cx, |editor, cx| {
        editor.search.search_input.set_text("one".to_string());
        editor.execute_search(cx);

        assert_eq!(editor.search.matches.len(), 3);
        assert_eq!(editor.search.active_match_index, Some(0));

        editor.search.next_match();
        assert_eq!(editor.search.active_match_index, Some(1));

        editor.search.next_match();
        assert_eq!(editor.search.active_match_index, Some(2));

        // Wrap around
        editor.search.next_match();
        assert_eq!(editor.search.active_match_index, Some(0));

        // Prev wrap around
        editor.search.prev_match();
        assert_eq!(editor.search.active_match_index, Some(2));
    });
}

#[gpui::test]
fn test_replace_all_and_atomic_undo(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| {
        Editor::from_markdown(
            cx,
            "foo bar foo baz foo".to_string(),
            None,
        )
    });

    editor.update(cx, |editor, cx| {
        editor.search.search_input.set_text("foo".to_string());
        editor.search.replace_input.set_text("qux".to_string());
        editor.execute_search(cx);
        assert_eq!(editor.search.matches.len(), 3);

        editor.replace_all_search_matches(cx);

        let doc_text = editor.doc().serialize_markdown(cx);
        assert_eq!(doc_text.trim(), "qux bar qux baz qux");

        // Undo rollback
        editor.undo_document(cx);
        let restored_text = editor.doc().serialize_markdown(cx);
        assert_eq!(restored_text.trim(), "foo bar foo baz foo");
    });
}

#[gpui::test]
fn test_search_worktree_files(cx: &mut TestAppContext) {
    let temp_dir = std::env::temp_dir().join(format!("splitype-test-search-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);
    let file_a = temp_dir.join("a.md");
    let file_b = temp_dir.join("b.txt");
    let _ = std::fs::write(&file_a, "Hello special keyword in markdown\nAnother line");
    let _ = std::fs::write(&file_b, "Another file with special keyword inside");

    let editor = cx.new(|cx| {
        Editor::from_markdown(
            cx,
            "Active document without that term".to_string(),
            Some(file_a.clone()),
        )
    });

    editor.update(cx, |editor, cx| {
        editor.search.search_input.set_text("special keyword".to_string());
        editor.search.scope = SearchScope::Worktree;
        editor.execute_search(cx);

        assert_eq!(editor.search.matches.len(), 2);
    });

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[gpui::test]
fn test_search_in_empty_editor_never_panics(cx: &mut TestAppContext) {
    let editor = cx.new(Editor::empty);

    editor.update(cx, |editor, cx| {
        assert!(!editor.has_active_tab());

        editor.search.search_input.set_text("test query".to_string());
        editor.execute_search(cx);
        assert_eq!(editor.search.matches.len(), 0);

        editor.search.scope = SearchScope::Worktree;
        editor.execute_search(cx);
    });
}

#[gpui::test]
fn test_search_multibyte_chinese_characters_and_symbols(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| {
        Editor::from_markdown(
            cx,
            "开始重构，重构过程中：我要求代码库里只能有一套设计，严禁向后做任何兼容。".to_string(),
            None,
        )
    });

    editor.update(cx, |editor, cx| {
        // Search multi-byte Chinese word "开"
        editor.search.search_input.set_text("开".to_string());
        editor.execute_search(cx);
        assert_eq!(editor.search.matches.len(), 1);
        assert_eq!(editor.search.matches[0].preview_match, "开");

        // Search "重构"
        editor.search.search_input.set_text("重构".to_string());
        editor.execute_search(cx);
        assert_eq!(editor.search.matches.len(), 2);

        // Search special symbols without regex mode
        editor.search.search_input.set_text("：".to_string());
        editor.search.use_regex = false;
        editor.execute_search(cx);
        assert_eq!(editor.search.matches.len(), 1);

        // Test replace on Chinese characters
        editor.search.search_input.set_text("严禁".to_string());
        editor.search.replace_input.set_text("绝不".to_string());
        editor.execute_search(cx);
        assert_eq!(editor.search.matches.len(), 1);

        editor.replace_all_search_matches(cx);
        let doc_text = editor.doc().serialize_markdown(cx);
        assert!(doc_text.contains("绝不向后做任何兼容"));
    });
}

#[gpui::test]
fn test_search_results_drawer_expansion_and_item_toggle(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| {
        Editor::from_markdown(
            cx,
            "First matching item\nSecond matching item\nThird matching item".to_string(),
            None,
        )
    });

    editor.update(cx, |editor, cx| {
        editor.search.search_input.set_text("matching".to_string());
        editor.execute_search(cx);

        assert_eq!(editor.search.matches.len(), 3);
        assert!(editor.search.results_expanded);

        assert!(!editor.search.is_match_expanded(0));
        editor.search.toggle_match_expanded(0);
        assert!(editor.search.is_match_expanded(0));
        editor.search.toggle_match_expanded(0);
        // Sync highlights to document check
        editor.search.visible = true;
        editor.sync_search_highlights_to_document(cx);
        let first_block = editor.doc().blocks()[0].entity.read(cx);
        assert_eq!(first_block.search_matches.len(), 1);
        assert_eq!(first_block.search_matches[0].1, true); // Active match is at index 0
    });
}

#[gpui::test]
fn test_search_query_engine_and_invalid_regex_safety() {
    use crate::editor::search::query::SearchQuery;

    // Normal query
    let q = SearchQuery::new("hello", false, false, false);
    assert!(q.is_valid());
    let matches = q.find_matches("Hello world\nhello universe", 1);
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].line_number, 1);
    assert_eq!(matches[0].column_number, 1);
    assert_eq!(matches[1].line_number, 2);
    assert_eq!(matches[1].column_number, 1);

    // Case sensitive
    let q_case = SearchQuery::new("Hello", true, false, false);
    let matches_case = q_case.find_matches("Hello world\nhello universe", 1);
    assert_eq!(matches_case.len(), 1);

    // Whole word
    let q_word = SearchQuery::new("cat", false, true, false);
    let matches_word = q_word.find_matches("cat concatenate cat_dog cat", 1);
    assert_eq!(matches_word.len(), 2);

    // Invalid regex should safely not panic and match nothing
    let q_invalid = SearchQuery::new("([a-z", false, false, true);
    assert!(!q_invalid.is_valid());
    let matches_invalid = q_invalid.find_matches("abc def", 1);
    assert_eq!(matches_invalid.len(), 0);
}

#[gpui::test]
fn test_search_action_dispatch_and_lifecycle(cx: &mut TestAppContext) {
    super::init_editor_test_app(cx);

    let editor = cx.new(|cx| {
        Editor::from_markdown(
            cx,
            "First target line\nSecond target line\nThird target line".to_string(),
            None,
        )
    });

    let window = cx.update(|cx| cx.open_window(gpui::WindowOptions::default(), |_window, _cx| editor.clone())).unwrap();

    window.update(cx, |ed, window, cx| {
        assert!(!ed.search.visible);

        // Toggle search on
        ed.toggle_search(window, cx);
        assert!(ed.search.visible);
        assert_eq!(ed.search.active_field, crate::editor::search::state::SearchActiveField::Query);

        ed.search.search_input.set_text("target".to_string());
        ed.execute_search(cx);
        assert_eq!(ed.search.matches.len(), 3);
        assert_eq!(ed.search.active_match_index, Some(0));

        // Find next
        ed.find_next(window, cx);
        assert_eq!(ed.search.active_match_index, Some(1));

        // Find previous
        ed.find_previous(window, cx);
        assert_eq!(ed.search.active_match_index, Some(0));

        // Toggle replace on
        ed.toggle_replace(window, cx);
        assert!(ed.search.show_replace);
        assert_eq!(ed.search.active_field, crate::editor::search::state::SearchActiveField::Replace);

        // Toggle search off
        ed.toggle_search(window, cx);
        assert!(!ed.search.visible);
    }).unwrap();
}

#[gpui::test]
fn test_search_navigation_in_source_code_and_preview(cx: &mut TestAppContext) {
    super::init_editor_test_app(cx);

    let editor = cx.new(|cx| {
        Editor::from_markdown(
            cx,
            "First target line\nSecond target line\nThird target line".to_string(),
            None,
        )
    });

    let window = cx
        .update(|cx| {
            cx.open_window(gpui::WindowOptions::default(), |_window, _cx| {
                editor.clone()
            })
        })
        .unwrap();

    window
        .update(cx, |ed, window, cx| {
            let active_pane = ed.active_pane_id();

            // 1. Switch active pane to SourceCode mode
            ed.change_pane_kind(active_pane, crate::editor::engine::controller::EditorPaneKind::SourceCode);
            ed.sync_source_pane(active_pane, cx);

            ed.toggle_search(window, cx);
            ed.search.search_input.set_text("target".to_string());
            ed.execute_search(cx);

            assert_eq!(ed.search.matches.len(), 3);
            assert_eq!(ed.search.active_match_index, Some(0));

            // In SourceCode mode, source_block should have search highlights and selection
            let source_block = ed
                .pane_state_ref(active_pane)
                .unwrap()
                .source_block
                .as_ref()
                .unwrap()
                .clone();
            let matches = source_block.read(cx).search_matches.clone();
            assert_eq!(matches.len(), 3);
            assert_eq!(matches[0].1, true); // Active match
            assert_eq!(matches[1].1, false);

            // Find next in SourceCode mode
            ed.find_next(window, cx);
            assert_eq!(ed.search.active_match_index, Some(1));
            let matches_after_next = source_block.read(cx).search_matches.clone();
            assert_eq!(matches_after_next[0].1, false);
            assert_eq!(matches_after_next[1].1, true); // Second match is now active

            // 2. Switch active pane to Preview mode
            ed.change_pane_kind(
                active_pane,
                crate::editor::engine::controller::EditorPaneKind::Preview,
            );
            ed.refresh_preview_blocks(active_pane, cx);
            ed.sync_search_highlights_to_document(cx);

            let preview_blocks = ed
                .pane_state_ref(active_pane)
                .unwrap()
                .preview
                .blocks
                .clone();
            assert!(!preview_blocks.is_empty());

            // Jump to previous match in Preview mode
            ed.find_previous(window, cx);
            assert_eq!(ed.search.active_match_index, Some(0));
            let first_preview_block = preview_blocks[0].read(cx);
            assert_eq!(first_preview_block.search_matches.len(), 3);
            assert_eq!(first_preview_block.search_matches[0].1, true);
            assert_eq!(first_preview_block.search_matches[1].1, false);
        })
        .unwrap();
}

#[gpui::test]
fn test_search_multibyte_chinese_characters_in_preview_rendering(cx: &mut TestAppContext) {
    super::init_editor_test_app(cx);

    let editor = cx.new(|cx| {
        Editor::from_markdown(
            cx,
            "**第一列内容**\n\n*第二列说明* [链接列](https://example.com)\n\n| 表头1 | 表头2 |\n|---|---|\n| 第3列 | 目标列 |".to_string(),
            None,
        )
    });

    let window = cx
        .update(|cx| {
            cx.open_window(gpui::WindowOptions::default(), |_window, _cx| {
                editor.clone()
            })
        })
        .unwrap();

    window
        .update(cx, |ed, window, cx| {
            let active_pane = ed.active_pane_id();
            ed.change_pane_kind(
                active_pane,
                crate::editor::engine::controller::EditorPaneKind::Preview,
            );
            ed.refresh_preview_blocks(active_pane, cx);

            ed.toggle_search(window, cx);
            ed.search.search_input.set_text("列".to_string());
            ed.execute_search(cx);

            assert!(!ed.search.matches.is_empty());

            // Next / Prev jumping in Preview mode with Chinese multi-byte characters
            for _ in 0..ed.search.matches.len() {
                ed.find_next(window, cx);
            }
            for _ in 0..ed.search.matches.len() {
                ed.find_previous(window, cx);
            }
        })
        .unwrap();
}




