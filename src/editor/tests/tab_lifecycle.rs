use std::fs;
use gpui::TestAppContext;

use super::{init_editor_test_app, temp_markdown_path};
use crate::editor::engine::controller::{Editor, OpenFileMode, TabKind};

#[gpui::test]
async fn test_transient_tab_created_and_replaced_on_next_transient(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let path_a = temp_markdown_path("tab-life-a");
    let path_b = temp_markdown_path("tab-life-b");
    fs::write(&path_a, "# Doc A").unwrap();
    fs::write(&path_b, "# Doc B").unwrap();

    let (editor, cx) = cx.add_window_view(|_window, cx| Editor::empty(cx));

    // 1. Open File A as Transient
    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            editor.open_file_in_panel(&path_a, OpenFileMode::Transient, window, cx);
        });
    });

    editor.update(cx, |editor, _cx| {
        assert_eq!(editor.session().tab_count(), 1);
        let tab = editor.tab();
        assert_eq!(tab.kind, TabKind::Transient);
        assert!(tab.is_transient());
        assert_eq!(tab.file.path.as_deref(), Some(path_a.as_path()));
    });

    // 2. Open File B as Transient -> Should replace File A in-place
    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            editor.open_file_in_panel(&path_b, OpenFileMode::Transient, window, cx);
        });
    });

    editor.update(cx, |editor, _cx| {
        assert_eq!(editor.session().tab_count(), 1, "Transient tab must be replaced in-place");
        let tab = editor.tab();
        assert_eq!(tab.kind, TabKind::Transient);
        assert_eq!(tab.file.path.as_deref(), Some(path_b.as_path()));
    });

    let _ = fs::remove_file(&path_a);
    let _ = fs::remove_file(&path_b);
}

#[gpui::test]
async fn test_persistent_tab_not_replaced_by_transient(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let path_a = temp_markdown_path("tab-perm-a");
    let path_b = temp_markdown_path("tab-perm-b");
    let path_c = temp_markdown_path("tab-perm-c");
    fs::write(&path_a, "# Doc A").unwrap();
    fs::write(&path_b, "# Doc B").unwrap();
    fs::write(&path_c, "# Doc C").unwrap();

    let (editor, cx) = cx.add_window_view(|_window, cx| Editor::empty(cx));

    // 1. Open File A as Persistent (e.g. double click)
    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            editor.open_file_in_panel(&path_a, OpenFileMode::Persistent, window, cx);
        });
    });

    editor.update(cx, |editor, _cx| {
        assert_eq!(editor.session().tab_count(), 1);
        assert_eq!(editor.tab().kind, TabKind::Persistent);
    });

    // 2. Open File B as Transient -> Persistent A is preserved, B is appended
    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            editor.open_file_in_panel(&path_b, OpenFileMode::Transient, window, cx);
        });
    });

    editor.update(cx, |editor, _cx| {
        assert_eq!(editor.session().tab_count(), 2);
        assert_eq!(editor.session().tab(0).unwrap().kind, TabKind::Persistent);
        assert_eq!(editor.session().tab(1).unwrap().kind, TabKind::Transient);
        assert_eq!(editor.session().active_tab_index(), 1);
    });

    // 3. Open File C as Transient -> Replaces B (Transient), A (Persistent) remains untouched
    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            editor.open_file_in_panel(&path_c, OpenFileMode::Transient, window, cx);
        });
    });

    editor.update(cx, |editor, _cx| {
        assert_eq!(editor.session().tab_count(), 2);
        assert_eq!(editor.session().tab(0).unwrap().kind, TabKind::Persistent);
        assert_eq!(editor.session().tab(0).unwrap().file.path.as_deref(), Some(path_a.as_path()));
        assert_eq!(editor.session().tab(1).unwrap().kind, TabKind::Transient);
        assert_eq!(editor.session().tab(1).unwrap().file.path.as_deref(), Some(path_c.as_path()));
    });

    let _ = fs::remove_file(&path_a);
    let _ = fs::remove_file(&path_b);
    let _ = fs::remove_file(&path_c);
}

#[gpui::test]
async fn test_mark_dirty_persists_transient_tab_to_persistent(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let path_a = temp_markdown_path("tab-edit-a");
    let path_b = temp_markdown_path("tab-edit-b");
    fs::write(&path_a, "# Doc A").unwrap();
    fs::write(&path_b, "# Doc B").unwrap();

    let (editor, cx) = cx.add_window_view(|_window, cx| Editor::empty(cx));

    // 1. Open File A as Transient
    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            editor.open_file_in_panel(&path_a, OpenFileMode::Transient, window, cx);
        });
    });

    editor.update(cx, |editor, cx| {
        assert!(editor.tab().is_transient());
        // Edit document
        editor.mark_dirty(cx);
        assert_eq!(editor.tab().kind, TabKind::Persistent, "mark_dirty must auto-persist transient tab to persistent");
    });

    // 2. Now open File B as Transient -> Since A is dirty/persistent, B does not overwrite A
    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            editor.open_file_in_panel(&path_b, OpenFileMode::Transient, window, cx);
        });
    });

    editor.update(cx, |editor, _cx| {
        assert_eq!(editor.session().tab_count(), 2);
        assert_eq!(editor.session().tab(0).unwrap().kind, TabKind::Persistent);
        assert_eq!(editor.session().tab(1).unwrap().kind, TabKind::Transient);
    });

    let _ = fs::remove_file(&path_a);
    let _ = fs::remove_file(&path_b);
}

#[gpui::test]
async fn test_reopening_existing_tab_as_persistent_promotes_it(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let path_a = temp_markdown_path("tab-promote-a");
    fs::write(&path_a, "# Doc A").unwrap();

    let (editor, cx) = cx.add_window_view(|_window, cx| Editor::empty(cx));

    // 1. Open File A as Transient
    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            editor.open_file_in_panel(&path_a, OpenFileMode::Transient, window, cx);
        });
    });

    editor.update(cx, |editor, _cx| {
        assert!(editor.tab().is_transient());
    });

    // 2. Re-open File A as Persistent (e.g. double click explorer or keep open)
    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            editor.open_file_in_panel(&path_a, OpenFileMode::Persistent, window, cx);
        });
    });

    editor.update(cx, |editor, _cx| {
        assert_eq!(editor.session().tab_count(), 1);
        assert_eq!(editor.tab().kind, TabKind::Persistent);
    });

    let _ = fs::remove_file(&path_a);
}

#[gpui::test]
async fn test_empty_editor_mouse_events_do_not_panic(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let (editor, cx) = cx.add_window_view(|_window, cx| Editor::empty(cx));

    cx.update(|_window, cx| {
        editor.update(cx, |editor, cx| {
            let active_pane = editor.active_pane_id();
            // Mouse events and scrollbar drag end on an empty editor with 0 tabs
            editor.end_scrollbar_drag(active_pane, cx);
            editor.bump_scrollbar_visibility(active_pane, cx);
            editor.update_scrollbar_drag(active_pane, 100.0, cx);
        });
    });
}

