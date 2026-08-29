//! View-mode toggling: preserved image handles and positions.

use gpui::{AppContext, TestAppContext};

use crate::editor::engine::controller::{Editor, EditorPaneKind};

#[gpui::test]
async fn toggling_source_mode_preserves_root_image_handle(cx: &mut TestAppContext) {
    let markdown = "![diagram](./assets/diagram.png)".to_string();
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        editor.toggle_pane_kind(cx);
        assert!(matches!(editor.active_pane_kind(), EditorPaneKind::SourceCode));
        editor.toggle_pane_kind(cx);
        assert!(matches!(editor.active_pane_kind(), EditorPaneKind::Wysiwyg));
    });

    editor.read_with(cx, |editor, cx| {
        let block = editor.doc().first_root().expect("root block").clone();
        assert!(block.read(cx).image_handle().is_some());
    });
}

#[gpui::test]
async fn toggling_source_mode_preserves_reference_style_root_image_handle(cx: &mut TestAppContext) {
    let markdown = "![diagram][ref]\n\n[ref]: ./assets/diagram.png".to_string();
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        editor.toggle_pane_kind(cx);
        assert!(matches!(editor.active_pane_kind(), EditorPaneKind::SourceCode));
        editor.toggle_pane_kind(cx);
        assert!(matches!(editor.active_pane_kind(), EditorPaneKind::Wysiwyg));
    });

    editor.read_with(cx, |editor, cx| {
        let block = editor.doc().first_root().expect("root block").clone();
        let handle = block.read(cx).image_handle().expect("image handle");
        assert_eq!(handle.src, "./assets/diagram.png");
    });
}

#[gpui::test]
async fn toggling_source_mode_preserves_quote_child_image_handle(cx: &mut TestAppContext) {
    let markdown = "> ![diagram](./assets/diagram.png)".to_string();
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        editor.toggle_pane_kind(cx);
        assert!(matches!(editor.active_pane_kind(), EditorPaneKind::SourceCode));
        editor.toggle_pane_kind(cx);
        assert!(matches!(editor.active_pane_kind(), EditorPaneKind::Wysiwyg));
    });

    editor.read_with(cx, |editor, cx| {
        let quote = editor.doc().first_root().expect("quote root").clone();
        let image_block = quote
            .read(cx)
            .children
            .first()
            .expect("quote image child")
            .clone();
        assert!(image_block.read(cx).image_handle().is_some());
    });
}

#[gpui::test]
async fn toggling_source_mode_preserves_list_item_image_handle(cx: &mut TestAppContext) {
    let markdown = "- ![diagram](./assets/diagram.png)".to_string();
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        editor.toggle_pane_kind(cx);
        assert!(matches!(editor.active_pane_kind(), EditorPaneKind::SourceCode));
        editor.toggle_pane_kind(cx);
        assert!(matches!(editor.active_pane_kind(), EditorPaneKind::Wysiwyg));
    });

    editor.read_with(cx, |editor, cx| {
        let block = editor.doc().first_root().expect("list item root").clone();
        assert!(block.read(cx).image_handle().is_some());
    });
}

#[gpui::test]
async fn toggling_source_mode_preserves_list_child_image_handle(cx: &mut TestAppContext) {
    let markdown = "- item\n  ![diagram](./assets/diagram.png)".to_string();
    let editor = cx.new(|cx| Editor::from_markdown(cx, markdown, None));

    editor.update(cx, |editor, cx| {
        editor.toggle_pane_kind(cx);
        assert!(matches!(editor.active_pane_kind(), EditorPaneKind::SourceCode));
        editor.toggle_pane_kind(cx);
        assert!(matches!(editor.active_pane_kind(), EditorPaneKind::Wysiwyg));
    });

    editor.read_with(cx, |editor, cx| {
        let list_item = editor.doc().first_root().expect("list item root").clone();
        let image_block = list_item
            .read(cx)
            .children
            .first()
            .expect("list child image")
            .clone();
        assert!(image_block.read(cx).image_handle().is_some());
    });
}
