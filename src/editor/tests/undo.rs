//! Undo / redo history across rendered typing.

use gpui::{AppContext, TestAppContext};

use crate::editor::controller::Editor;


#[gpui::test]
async fn undo_reverts_recent_rendered_typing(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "alpha".to_string(), None));

    editor.update(cx, |editor, cx| {
        let block = editor.doc().first_root().expect("root").clone();
        editor.tab_mut().focus.active_entity = Some(block.entity_id());
        block.update(cx, |block, cx| {
            block.prepare_undo_capture(
                crate::editor::block_protocol::UndoCaptureKind::CoalescibleText,
                cx,
            );
            block.replace_text_in_visible_range(5..5, " beta", None, false, cx);
        });
    });

    editor.update(cx, |editor, cx| {
        assert_eq!(editor.doc().to_markdown(cx), "alpha beta");
        assert_eq!(editor.tab().undo.undo_entries.len(), 1);
        editor.undo_document(cx);
        assert_eq!(editor.doc().to_markdown(cx), "alpha");
    });
}

#[gpui::test]
async fn consecutive_text_edits_within_window_coalesce_into_one_undo(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "a".to_string(), None));

    editor.update(cx, |editor, cx| {
        let block = editor.doc().first_root().expect("root").clone();
        editor.tab_mut().focus.active_entity = Some(block.entity_id());

        block.update(cx, |block, cx| {
            block.prepare_undo_capture(
                crate::editor::block_protocol::UndoCaptureKind::CoalescibleText,
                cx,
            );
            block.replace_text_in_visible_range(1..1, "b", None, false, cx);
        });
        block.update(cx, |block, cx| {
            block.prepare_undo_capture(
                crate::editor::block_protocol::UndoCaptureKind::CoalescibleText,
                cx,
            );
            block.replace_text_in_visible_range(2..2, "c", None, false, cx);
        });
    });

    editor.update(cx, |editor, cx| {
        assert_eq!(editor.doc().to_markdown(cx), "abc");
        assert_eq!(editor.tab().undo.undo_entries.len(), 1);

        editor.undo_document(cx);
        assert_eq!(editor.doc().to_markdown(cx), "a");
    });
}

#[gpui::test]
async fn redo_restores_text_reverted_by_undo(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "alpha".to_string(), None));

    editor.update(cx, |editor, cx| {
        let block = editor.doc().first_root().expect("root").clone();
        editor.tab_mut().focus.active_entity = Some(block.entity_id());
        block.update(cx, |block, cx| {
            block.prepare_undo_capture(
                crate::editor::block_protocol::UndoCaptureKind::CoalescibleText,
                cx,
            );
            block.replace_text_in_visible_range(5..5, " beta", None, false, cx);
        });
    });

    editor.update(cx, |editor, cx| {
        editor.undo_document(cx);
        assert_eq!(editor.doc().to_markdown(cx), "alpha");
        assert_eq!(editor.tab().undo.redo_entries.len(), 1);

        editor.redo_document(cx);
        assert_eq!(editor.doc().to_markdown(cx), "alpha beta");
        assert!(editor.tab().undo.redo_entries.is_empty());
    });
}

#[gpui::test]
async fn fresh_edit_clears_pending_redo_history(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "alpha".to_string(), None));

    editor.update(cx, |editor, cx| {
        let block = editor.doc().first_root().expect("root").clone();
        editor.tab_mut().focus.active_entity = Some(block.entity_id());
        block.update(cx, |block, cx| {
            block.prepare_undo_capture(
                crate::editor::block_protocol::UndoCaptureKind::CoalescibleText,
                cx,
            );
            block.replace_text_in_visible_range(5..5, " beta", None, false, cx);
        });
    });

    editor.update(cx, |editor, cx| {
        editor.undo_document(cx);
        assert_eq!(editor.tab().undo.redo_entries.len(), 1);

        // A new edit invalidates the redo stack so it cannot revive stale text.
        let block = editor.doc().first_root().expect("root").clone();
        block.update(cx, |block, cx| {
            block.prepare_undo_capture(
                crate::editor::block_protocol::UndoCaptureKind::CoalescibleText,
                cx,
            );
            block.replace_text_in_visible_range(5..5, " gamma", None, false, cx);
        });
    });

    editor.update(cx, |editor, cx| {
        editor.finalize_pending_undo_capture(cx);
        assert!(editor.tab().undo.redo_entries.is_empty());

        editor.redo_document(cx);
        assert_eq!(editor.doc().to_markdown(cx), "alpha gamma");
    });
}

