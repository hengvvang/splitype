//! Save (Ctrl-S / menu action) and HTML export flows.

use std::fs;

use crate::editor::actions::SaveDocument;
use crate::editor::controller::Editor;
use crate::editor::render::export::ExportFormat;
use crate::model::inline::text::BlockText;

use super::*;

#[gpui::test]
async fn ctrl_s_saves_wysiwyg_mode_edit_to_existing_file(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let path = temp_markdown_path("ctrl-s-rendered-save");
    fs::write(&path, "alpha").expect("write initial markdown");
    let cleanup_path = path.clone();
    cx.on_quit(move || {
        let _ = fs::remove_file(&cleanup_path);
    });

    let (editor, cx) = cx.add_window_view({
        let path = path.clone();
        move |_window, cx| Editor::from_markdown(cx, "alpha".to_string(), Some(path))
    });
    focus_first_block(&editor, cx);

    cx.simulate_input("!");
    redraw(cx);
    let expected = editor.read_with(cx, |editor, cx| {
        assert!(editor.tab().file.dirty);
        assert!(!editor.tab().file.pending_save);
        editor.doc().serialize_markdown(cx)
    });
    assert_ne!(expected, "alpha");

    cx.simulate_keystrokes("ctrl-s");
    redraw(cx);

    assert_eq!(
        fs::read_to_string(&path).expect("read saved markdown"),
        expected
    );
    editor.read_with(cx, |editor, _cx| {
        assert!(!editor.tab().file.dirty);
        assert!(!editor.tab().file.pending_save);
    });
}

#[gpui::test]
async fn window_save_action_saves_current_editor_without_global_menu_route(
    cx: &mut TestAppContext,
) {
    init_editor_test_app(cx);

    let path = temp_markdown_path("window-action-save");
    fs::write(&path, "alpha").expect("write initial markdown");
    let cleanup_path = path.clone();
    cx.on_quit(move || {
        let _ = fs::remove_file(&cleanup_path);
    });

    let (editor, cx) = cx.add_window_view({
        let path = path.clone();
        move |_window, cx| Editor::from_markdown(cx, "alpha".to_string(), Some(path))
    });
    focus_first_block(&editor, cx);

    cx.simulate_input(" action");
    redraw(cx);
    let expected = editor.read_with(cx, |editor, cx| {
        assert!(editor.tab().file.dirty);
        editor.doc().serialize_markdown(cx)
    });
    assert_ne!(expected, "alpha");

    cx.dispatch_action(SaveDocument);
    redraw(cx);

    assert_eq!(
        fs::read_to_string(&path).expect("read saved markdown"),
        expected
    );
    editor.read_with(cx, |editor, _cx| {
        assert!(!editor.tab().file.dirty);
        assert!(!editor.tab().file.pending_save);
    });
}

#[gpui::test]
async fn export_html_writes_rendered_document_without_changing_editor_state(
    cx: &mut TestAppContext,
) {
    init_editor_test_app(cx);

    let export_path = temp_export_path("rendered-export-html", "html");
    let cleanup_path = export_path.clone();
    cx.on_quit(move || {
        let _ = fs::remove_file(&cleanup_path);
    });

    let (editor, cx) = cx.add_window_view(|_window, cx| {
        Editor::from_markdown(cx, "# Title\n\nbody".to_string(), None)
    });

    editor.update(cx, |editor, cx| {
        editor.mark_dirty(cx);
        assert!(editor.tab().file.dirty);
        assert!(editor.tab().file.path.is_none());
        editor
            .export_document_to_path(ExportFormat::Html, &export_path, cx)
            .expect("html export should write");
        assert!(editor.tab().file.dirty);
        assert!(editor.tab().file.path.is_none());
    });

    let html = fs::read_to_string(&export_path).expect("read exported html");
    assert!(html.contains("<h1>Title</h1>"));
    assert!(html.contains("<p>body</p>"));
}

#[gpui::test]
async fn export_html_uses_source_mode_raw_text(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let export_path = temp_export_path("source-export-html", "html");
    let cleanup_path = export_path.clone();
    cx.on_quit(move || {
        let _ = fs::remove_file(&cleanup_path);
    });

    let (editor, cx) =
        cx.add_window_view(|_window, cx| Editor::from_markdown(cx, "rendered".to_string(), None));

    editor.update(cx, |editor, cx| {
        editor.toggle_view_mode(cx);
        let source_block = editor
            .doc()
            .first_root()
            .expect("source mode should keep one root block")
            .clone();
        source_block.update(cx, |block, _cx| {
            block.data.set_text(BlockText::plain(
                "# Source\n\n<!--\n<strong>visible</strong>\n-->".to_string(),
            ));
            block.sync_render_cache();
        });
        editor
            .export_document_to_path(ExportFormat::Html, &export_path, cx)
            .expect("source html export should write");
    });

    let html = fs::read_to_string(&export_path).expect("read exported html");
    assert!(html.contains("<h1>Source</h1>"));
    assert!(html.contains("class=\"vlt-comment\""));
    assert!(html.contains("&lt;strong&gt;visible&lt;/strong&gt;"));
}
