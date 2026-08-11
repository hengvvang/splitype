//! Window-level flows: menu actions, close guards, quit,
//! pane-click panel activation.

use std::fs;

use gpui::{AppContext, TestAppContext};

use crate::app::actions::{CloseWindow, QuitApplication};
use crate::editor::controller::Editor;

use super::*;

#[gpui::test]
async fn close_window_menu_action_closes_only_active_editor_window(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let first_window =
        cx.update(|cx| crate::app::window::open_editor_window(cx, "first".to_string(), None));
    cx.run_until_parked();
    let second_window =
        cx.update(|cx| crate::app::window::open_editor_window(cx, "second".to_string(), None));
    cx.run_until_parked();

    assert_ne!(first_window.window_id(), second_window.window_id());
    assert_eq!(cx.update(|cx| cx.windows().len()), 2);

    cx.update(|cx| {
        crate::app::menus::dispatch_menu_action(&CloseWindow, cx);
    });
    cx.run_until_parked();

    let remaining = cx.update(|cx| cx.windows());
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].window_id(), first_window.window_id());
    assert_ne!(remaining[0].window_id(), second_window.window_id());
}

#[gpui::test]
async fn app_menu_opened_windows_activate_and_close_independently(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let first_window =
        cx.update(|cx| crate::app::window::open_editor_window(cx, "first".to_string(), None));
    cx.run_until_parked();
    let second_window =
        cx.update(|cx| crate::app::window::open_editor_window(cx, "second".to_string(), None));
    cx.run_until_parked();

    let active_window = cx.update(|cx| cx.active_window().expect("window should be active"));
    assert_eq!(active_window.window_id(), second_window.window_id());
    assert_ne!(first_window.window_id(), second_window.window_id());
    assert_eq!(cx.update(|cx| cx.windows().len()), 2);

    assert!(
        second_window
            .update(cx, |shell, _window, _cx| shell.close_guard_installed)
            .expect("second editor window should be open")
    );

    cx.update(|cx| {
        crate::app::menus::dispatch_menu_action(&CloseWindow, cx);
    });
    cx.run_until_parked();

    let remaining = cx.update(|cx| cx.windows());
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].window_id(), first_window.window_id());

    cx.update(|cx| {
        crate::app::menus::dispatch_menu_action(&CloseWindow, cx);
    });
    cx.run_until_parked();

    assert!(cx.update(|cx| cx.windows().is_empty()));
}

#[gpui::test]
async fn app_menu_opened_file_window_reinstalls_close_guard_after_registration(
    cx: &mut TestAppContext,
) {
    init_editor_test_app(cx);

    let opened_path = temp_markdown_path("app-menu-opened-file-window-close");
    fs::write(&opened_path, "opened from file").expect("write opened markdown");

    let first_window =
        cx.update(|cx| crate::app::window::open_editor_window(cx, "first".to_string(), None));
    cx.run_until_parked();
    let second_window = cx.update(|cx| {
        crate::app::window::open_editor_window(
            cx,
            fs::read_to_string(&opened_path).expect("read opened markdown"),
            Some(opened_path.clone()),
        )
    });
    cx.run_until_parked();

    let active_window = cx.update(|cx| cx.active_window().expect("window should be active"));
    assert_eq!(active_window.window_id(), second_window.window_id());
    assert_ne!(first_window.window_id(), second_window.window_id());

    second_window
        .update(cx, |shell, window, cx| {
            assert!(shell.close_guard_installed);
            assert!(shell.on_window_should_close(window, cx));
        })
        .expect("second editor window should be open");

    cx.update(|cx| {
        crate::app::menus::dispatch_menu_action(&CloseWindow, cx);
    });
    cx.run_until_parked();

    let remaining = cx.update(|cx| cx.windows());
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].window_id(), first_window.window_id());
    assert_ne!(remaining[0].window_id(), second_window.window_id());

    let _ = fs::remove_file(opened_path);
}

#[gpui::test]
async fn app_menu_opened_dirty_file_window_prompts_only_that_window(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let opened_path = temp_markdown_path("app-menu-opened-dirty-file-window-close");
    fs::write(&opened_path, "opened from file").expect("write opened markdown");

    let first_window =
        cx.update(|cx| crate::app::window::open_editor_window(cx, "first".to_string(), None));
    let second_window = cx.update(|cx| {
        crate::app::window::open_editor_window(
            cx,
            fs::read_to_string(&opened_path).expect("read opened markdown"),
            Some(opened_path.clone()),
        )
    });
    cx.run_until_parked();

    second_window
        .update(cx, |shell, window, cx| {
            let editor = shell.primary_editor().expect("editor panel").clone();
            editor.update(cx, |editor, cx| {
                editor.mark_dirty(cx);
            });
            assert!(!shell.on_window_should_close(window, cx));
        })
        .expect("second editor window should be open");

    first_window
        .update(cx, |shell, _window, cx| {
            let editor = shell.primary_editor().expect("editor panel");
            assert!(!editor.read(cx).tab().file.show_unsaved_changes_dialog);
        })
        .expect("first editor window should be open");
    second_window
        .update(cx, |shell, _window, cx| {
            let editor = shell.primary_editor().expect("editor panel");
            assert!(editor.read(cx).tab().file.show_unsaved_changes_dialog);
        })
        .expect("second editor window should be open");

    let _ = fs::remove_file(opened_path);
}

#[gpui::test]
async fn app_menu_opened_dirty_window_close_guard_prompts_only_that_window(
    cx: &mut TestAppContext,
) {
    init_editor_test_app(cx);

    let first_window =
        cx.update(|cx| crate::app::window::open_editor_window(cx, "first".to_string(), None));
    let second_window =
        cx.update(|cx| crate::app::window::open_editor_window(cx, "second".to_string(), None));
    cx.run_until_parked();

    second_window
        .update(cx, |shell, window, cx| {
            let editor = shell.primary_editor().expect("editor panel").clone();
            editor.update(cx, |editor, cx| {
                editor.mark_dirty(cx);
            });
            assert!(!shell.on_window_should_close(window, cx));
        })
        .expect("second editor window should be open");

    first_window
        .update(cx, |shell, _window, cx| {
            let editor = shell.primary_editor().expect("editor panel");
            assert!(!editor.read(cx).tab().file.show_unsaved_changes_dialog);
        })
        .expect("first editor window should be open");
    second_window
        .update(cx, |shell, _window, cx| {
            let editor = shell.primary_editor().expect("editor panel");
            assert!(editor.read(cx).tab().file.show_unsaved_changes_dialog);
        })
        .expect("second editor window should be open");
}

#[gpui::test]
async fn quit_application_allows_clean_editor_windows_to_quit(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let first_window =
        cx.update(|cx| crate::app::window::open_editor_window(cx, "first".to_string(), None));
    cx.run_until_parked();
    let second_window =
        cx.update(|cx| crate::app::window::open_editor_window(cx, "second".to_string(), None));
    cx.run_until_parked();

    assert_eq!(cx.update(|cx| cx.windows().len()), 2);

    cx.update(|cx| {
        crate::app::menus::dispatch_menu_action(&QuitApplication, cx);
    });
    cx.run_until_parked();

    // Clean windows quit without prompting: no unsaved-changes dialog on
    // either window (the quit flow itself is asynchronous in tests).
    first_window
        .update(cx, |shell, _window, _cx| {
            let editor = shell.primary_editor().expect("editor panel");
            assert!(!editor.read(_cx).tab().file.show_unsaved_changes_dialog);
        })
        .expect("first editor window should be open");
    second_window
        .update(cx, |shell, _window, _cx| {
            let editor = shell.primary_editor().expect("editor panel");
            assert!(!editor.read(_cx).tab().file.show_unsaved_changes_dialog);
        })
        .expect("second editor window should be open");
}

#[gpui::test]
async fn quit_application_prompts_dirty_editor_without_quitting(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let first_window =
        cx.update(|cx| crate::app::window::open_editor_window(cx, "first".to_string(), None));
    cx.run_until_parked();
    let second_window =
        cx.update(|cx| crate::app::window::open_editor_window(cx, "second".to_string(), None));
    cx.run_until_parked();

    let second_editor = second_window
        .update(cx, |shell, _window, _cx| {
            shell.primary_editor().expect("editor panel").clone()
        })
        .expect("second editor window should be open");
    second_editor.update(cx, |editor, cx| editor.mark_dirty(cx));
    assert_eq!(cx.update(|cx| cx.windows().len()), 2);

    cx.update(|cx| {
        crate::app::menus::dispatch_menu_action(&QuitApplication, cx);
    });
    cx.run_until_parked();

    let open_windows = cx.update(|cx| cx.windows());
    assert_eq!(open_windows.len(), 2);
    assert!(
        open_windows
            .iter()
            .any(|window| window.window_id() == first_window.window_id())
    );
    assert!(
        open_windows
            .iter()
            .any(|window| window.window_id() == second_window.window_id())
    );
    first_window
        .update(cx, |shell, _window, _cx| {
            let editor = shell.primary_editor().expect("editor panel");
            assert!(!editor.read(_cx).tab().file.show_unsaved_changes_dialog);
        })
        .expect("first editor window should be open");
    second_window
        .update(cx, |shell, _window, _cx| {
            let editor = shell.primary_editor().expect("editor panel");
            assert!(editor.read(_cx).tab().file.show_unsaved_changes_dialog);
        })
        .expect("second editor window should be open");
}

#[gpui::test]
async fn windows_fallback_close_window_dispatch_closes_target_editor_window(
    cx: &mut TestAppContext,
) {
    init_editor_test_app(cx);

    let window =
        cx.update(|cx| crate::app::window::open_editor_window(cx, "target".to_string(), None));
    cx.run_until_parked();
    let target_window_id = window.window_id();
    let editor = window
        .update(cx, |shell, _window, _cx| {
            shell.primary_editor().expect("editor panel").downgrade()
        })
        .expect("editor window should be open");

    cx.update(|cx| {
        let window = cx.active_window().expect("window should be active");
        let editor = editor.clone();
        let _ = window.update(cx, |_view, window, cx| {
            crate::app::menus::dispatch_menu_action_for_editor(&CloseWindow, &editor, window, cx);
        });
    });
    cx.run_until_parked();

    assert!(
        cx.update(|cx| cx.windows())
            .iter()
            .all(|window| window.window_id() != target_window_id)
    );
}

#[gpui::test]
async fn window_close_action_closes_current_editor_before_global_menu_route(
    cx: &mut TestAppContext,
) {
    init_editor_test_app(cx);

    let first_window =
        cx.update(|cx| crate::app::window::open_editor_window(cx, "first".to_string(), None));
    cx.run_until_parked();
    let second_window =
        cx.update(|cx| crate::app::window::open_editor_window(cx, "second".to_string(), None));
    cx.run_until_parked();

    // The window root's own CloseWindow action handler runs before the
    // global menu route; firing it closes only the focused editor's window.
    second_window
        .update(cx, |shell, window, cx| {
            shell.on_close_window(&CloseWindow, window, cx);
        })
        .expect("second editor window should be open");
    cx.run_until_parked();

    let remaining = cx.update(|cx| cx.windows());
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].window_id(), first_window.window_id());
    assert_ne!(remaining[0].window_id(), second_window.window_id());
}

#[gpui::test]
async fn welcome_pane_click_defers_panel_activation(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let window = cx.update(|cx| crate::app::window::open_editor_window(cx, String::new(), None));
    cx.run_until_parked();

    // The pane-body mouse-down runs inside the editor's own update. The
    // Shell activation it triggers must be deferred, otherwise
    // `sync_panel_states` double-leases this very entity (gpui panic).
    window
        .update(cx, |shell, window, cx| {
            let editor = shell
                .primary_editor()
                .expect("editor window has an editor")
                .clone();
            editor.update(cx, |ed, cx| {
                let mut pane_ids = Vec::new();
                ed.session().root.tree.leaf_ids(&mut pane_ids);
                ed.focus_pane(pane_ids[0], window, cx);
            });
        })
        .expect("window update");
    cx.run_until_parked();

    // The deferred activation ran: the clicked editor's panel became the
    // active leaf of the outer layout, and the welcome-mode click did not
    // create a tab.
    let (active_leaf, panel_id, tab_count) = window
        .update(cx, |shell, _window, cx| {
            let editor = shell.primary_editor().expect("editor window has an editor");
            let panel_id = editor.read(cx).panel_id;
            let tab_count = editor.read(cx).session().tab_list.tabs.len();
            (shell.panels.layout.active_leaf, panel_id, tab_count)
        })
        .expect("window update");
    assert_eq!(active_leaf, Some(panel_id));
    assert_eq!(tab_count, 0);
}

#[gpui::test]
async fn editing_pane_click_defers_panel_activation_without_panic(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let window = cx.update(|cx| crate::app::window::open_editor_window(cx, String::new(), None));
    cx.run_until_parked();

    // Enter editing (the welcome double-click flow creates a tab first).
    window
        .update(cx, |shell, _window, cx| {
            let editor = shell
                .primary_editor()
                .expect("editor window has an editor")
                .clone();
            editor.update(cx, |ed, cx| ed.new_untitled_tab(cx));
        })
        .expect("window update");
    cx.run_until_parked();

    // A pane-body click now focuses the pane; the deferred panel
    // activation must not double-lease the editor.
    window
        .update(cx, |shell, window, cx| {
            let editor = shell
                .primary_editor()
                .expect("editor window has an editor")
                .clone();
            editor.update(cx, |ed, cx| {
                let mut pane_ids = Vec::new();
                ed.session().root.tree.leaf_ids(&mut pane_ids);
                ed.focus_pane(pane_ids[0], window, cx);
            });
        })
        .expect("window update");
    cx.run_until_parked();

    let (focused_pane, active_leaf, panel_id) = window
        .update(cx, |shell, _window, cx| {
            let editor = shell.primary_editor().expect("editor window has an editor");
            let editor = editor.read(cx);
            (
                editor.focused_pane,
                shell.panels.layout.active_leaf,
                editor.panel_id,
            )
        })
        .expect("window update");
    assert!(focused_pane.is_some());
    assert_eq!(active_leaf, Some(panel_id));
}

#[gpui::test]
async fn starting_and_ending_scrollbar_drag_updates_editor_state(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "alpha".to_string(), None));

    editor.update(cx, |editor, cx| {
        editor.tab_mut().focus.pending_scroll_active_block_into_view = true;
        editor.tab_mut().focus.pending_scroll_recheck_after_layout = true;

        editor.start_scrollbar_drag(12.0, 320.0, 64.0, 500.0, cx);
        assert_eq!(
            editor.tab().scroll.scrollbar_drag,
            Some(crate::editor::controller::ScrollbarDragSession {
                pointer_offset_y: 12.0,
                track_height: 320.0,
                thumb_height: 64.0,
                max_scroll_y: 500.0,
            })
        );
        assert!(!editor.tab().focus.pending_scroll_active_block_into_view);
        assert!(!editor.tab().focus.pending_scroll_recheck_after_layout);

        editor.update_scrollbar_drag(172.0, cx);
        let offset_y = -f32::from(editor.tab().scroll.handle.offset().y);
        assert!(offset_y > 0.0);

        editor.end_scrollbar_drag(cx);
        assert!(editor.tab().scroll.scrollbar_drag.is_none());
    });
}

