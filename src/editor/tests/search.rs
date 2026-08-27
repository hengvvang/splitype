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
        assert!(!editor.search.is_match_expanded(0));

        // Sync highlights to document check
        editor.search.visible = true;
        editor.sync_search_highlights_to_document(cx);
        let first_block = editor.doc().blocks()[0].entity.read(cx);
        assert_eq!(first_block.search_matches.len(), 1);
        assert_eq!(first_block.search_matches[0].1, true); // Active match is at index 0
    });
}



