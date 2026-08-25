//! External file drops: clean replace and dirty-drop decisions.

use std::fs;

use crate::editor::engine::controller::{Editor, EditorPaneKind};
use crate::model::inline::text::BlockText;
use crate::model::parse::BlockKind;

use super::*;

#[gpui::test]
async fn dropped_markdown_replaces_clean_editor_in_current_window(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let dropped_path = temp_markdown_path("drop-clean-replace");
    fs::write(
        &dropped_path,
        "# Dropped\n\n| A | B |\n| --- | --- |\n| 1 | 2 |\n",
    )
    .expect("write dropped markdown");
    let cleanup_path = dropped_path.clone();
    cx.on_quit(move || {
        let _ = fs::remove_file(&cleanup_path);
    });

    let (editor, cx) =
        cx.add_window_view(|_window, cx| Editor::from_markdown(cx, "old".to_string(), None));

    editor.update(cx, |editor, cx| {
        editor.toggle_view_mode(cx);
        assert!(editor.tab().mode == EditorPaneKind::SourceCode);
    });

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            editor.request_dropped_markdown_replace(dropped_path.clone(), window, cx);
        });
    });
    redraw(cx);

    editor.read_with(cx, |editor, cx| {
        assert_eq!(editor.tab().file.path.as_ref(), Some(&dropped_path));
        assert!(editor.tab().mode == EditorPaneKind::Wysiwyg);
        assert!(!editor.tab().file.dirty);
        assert!(!editor.tab().file.show_drop_replace_dialog);
        assert_eq!(editor.doc().root_count(), 3);
        assert_eq!(
            editor
                .doc()
                .root_blocks()
                .last()
                .expect("table block")
                .read(cx)
                .kind(),
            BlockKind::Table
        );
        assert!(editor.doc().serialize_markdown(cx).contains("# Dropped"));
    });
    assert_eq!(cx.cx.windows().len(), 1);
}

#[gpui::test]
async fn dropped_paths_pick_first_valid_markdown_file(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let text_path = temp_export_path("drop-ignore-non-markdown", "txt");
    let markdown_path = temp_export_path("drop-pick-markdown", "markdown");
    fs::write(&text_path, "plain").expect("write text");
    fs::write(&markdown_path, "markdown").expect("write markdown");
    let cleanup_text = text_path.clone();
    let cleanup_markdown = markdown_path.clone();
    cx.on_quit(move || {
        let _ = fs::remove_file(&cleanup_text);
        let _ = fs::remove_file(&cleanup_markdown);
    });

    assert_eq!(
        crate::editor::input::drop::first_dropped_markdown_path(&[
            text_path,
            markdown_path.clone()
        ]),
        Some(markdown_path)
    );
}

#[gpui::test]
async fn dirty_drop_waits_for_replace_decision_and_cancel_preserves_document(
    cx: &mut TestAppContext,
) {
    init_editor_test_app(cx);

    let dropped_path = temp_markdown_path("drop-dirty-cancel");
    fs::write(&dropped_path, "dropped").expect("write dropped markdown");
    let cleanup_path = dropped_path.clone();
    cx.on_quit(move || {
        let _ = fs::remove_file(&cleanup_path);
    });

    let (editor, cx) =
        cx.add_window_view(|_window, cx| Editor::from_markdown(cx, "current".to_string(), None));
    editor.update(cx, |editor, cx| editor.mark_dirty(cx));

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            editor.request_dropped_markdown_replace(dropped_path, window, cx);
        });
    });
    redraw(cx);

    editor.read_with(cx, |editor, cx| {
        assert!(editor.tab().file.dirty);
        assert!(editor.tab().file.show_drop_replace_dialog);
        assert_eq!(editor.doc().serialize_markdown(cx), "current");
        assert!(editor.tab().file.pending_drop_replace_path.is_some());
    });

    editor.update(cx, |editor, cx| editor.cancel_drop_replace_dialog(cx));

    editor.read_with(cx, |editor, cx| {
        assert!(editor.tab().file.dirty);
        assert!(!editor.tab().file.show_drop_replace_dialog);
        assert!(editor.tab().file.pending_drop_replace_path.is_none());
        assert_eq!(editor.doc().serialize_markdown(cx), "current");
    });
}

#[gpui::test]
async fn dirty_drop_can_replace_without_saving(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let dropped_path = temp_markdown_path("drop-dirty-discard");
    fs::write(&dropped_path, "dropped").expect("write dropped markdown");
    let cleanup_path = dropped_path.clone();
    cx.on_quit(move || {
        let _ = fs::remove_file(&cleanup_path);
    });

    let (editor, cx) =
        cx.add_window_view(|_window, cx| Editor::from_markdown(cx, "current".to_string(), None));
    editor.update(cx, |editor, cx| editor.mark_dirty(cx));

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            editor.request_dropped_markdown_replace(dropped_path.clone(), window, cx);
            editor.discard_pending_drop_replace(window, cx);
        });
    });
    redraw(cx);

    editor.read_with(cx, |editor, cx| {
        assert_eq!(editor.tab().file.path.as_ref(), Some(&dropped_path));
        assert_eq!(editor.doc().serialize_markdown(cx), "dropped");
        assert!(!editor.tab().file.dirty);
        assert!(!editor.tab().file.show_drop_replace_dialog);
    });
}

#[gpui::test]
async fn dirty_drop_saves_existing_document_before_replace(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let current_path = temp_markdown_path("drop-save-current");
    let dropped_path = temp_markdown_path("drop-save-replace");
    fs::write(&current_path, "original").expect("write current markdown");
    fs::write(&dropped_path, "dropped").expect("write dropped markdown");
    let cleanup_current = current_path.clone();
    let cleanup_dropped = dropped_path.clone();
    cx.on_quit(move || {
        let _ = fs::remove_file(&cleanup_current);
        let _ = fs::remove_file(&cleanup_dropped);
    });

    let (editor, cx) = cx.add_window_view({
        let current_path = current_path.clone();
        move |_window, cx| Editor::from_markdown(cx, "original".to_string(), Some(current_path))
    });

    editor.update(cx, |editor, cx| {
        let first = editor.doc().first_root().expect("current root").clone();
        first.update(cx, |block, _cx| {
            block.data.set_text(BlockText::plain("edited".to_string()));
            block.sync_render_cache();
        });
        editor.mark_dirty(cx);
    });

    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            editor.request_dropped_markdown_replace(dropped_path.clone(), window, cx);
            editor.save_and_replace_pending_drop(window, cx);
        });
    });
    redraw(cx);

    assert_eq!(
        fs::read_to_string(&current_path).expect("read saved current"),
        "edited"
    );
    editor.read_with(cx, |editor, cx| {
        assert_eq!(editor.tab().file.path.as_ref(), Some(&dropped_path));
        assert_eq!(editor.doc().serialize_markdown(cx), "dropped");
        assert!(!editor.tab().file.dirty);
        assert!(!editor.tab().file.pending_drop_replace_after_save);
    });
}
