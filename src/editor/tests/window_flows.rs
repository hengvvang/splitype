//! Window-level flows: menu actions, close guards, quit,
//! pane-click panel activation.

use std::fs;

use gpui::{AppContext, MouseButton, TestAppContext};

use crate::app::actions::{CloseWindow, QuitApplication};
use crate::app::window_panels::{DEFAULT_EDITOR_PANEL_ID, WindowPanelKind};
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
        .update(cx, |shell, _window, _cx| {
            assert!(shell.unsaved_dialog.is_none());
        })
        .expect("first editor window should be open");
    second_window
        .update(cx, |shell, _window, _cx| {
            assert!(shell.unsaved_dialog.is_some());
            let dialog = shell.unsaved_dialog.as_ref().unwrap();
            assert_eq!(dialog.scope, crate::app::shell::UnsavedDialogScope::Window);
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
        .update(cx, |shell, _window, _cx| {
            assert!(shell.unsaved_dialog.is_none());
        })
        .expect("first editor window should be open");
    second_window
        .update(cx, |shell, _window, _cx| {
            assert!(shell.unsaved_dialog.is_some());
            let dialog = shell.unsaved_dialog.as_ref().unwrap();
            assert_eq!(dialog.scope, crate::app::shell::UnsavedDialogScope::Window);
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
            assert!(shell.unsaved_dialog.is_none());
        })
        .expect("first editor window should be open");
    second_window
        .update(cx, |shell, _window, _cx| {
            assert!(shell.unsaved_dialog.is_none());
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
            assert!(shell.unsaved_dialog.is_none());
        })
        .expect("first editor window should be open");
    second_window
        .update(cx, |shell, _window, _cx| {
            assert!(shell.unsaved_dialog.is_some());
            let dialog = shell.unsaved_dialog.as_ref().unwrap();
            assert_eq!(dialog.scope, crate::app::shell::UnsavedDialogScope::Window);
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
            let tab_count = editor.read(cx).session().tab_count();
            (shell.panels.layout.active_leaf, panel_id, tab_count)
        })
        .expect("window update");
    assert_eq!(active_leaf, Some(panel_id.0));
    assert_eq!(tab_count, 0);
}

#[gpui::test]
fn editing_pane_click_defers_panel_activation_without_panic(cx: &mut TestAppContext) {
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
                editor.focused_pane_id,
                shell.panels.layout.active_leaf,
                editor.panel_id,
            )
        })
        .expect("window update");
    assert!(focused_pane.is_some());
    assert_eq!(active_leaf, Some(panel_id.0));
}

#[gpui::test]
fn starting_and_ending_scrollbar_drag_updates_editor_state(cx: &mut TestAppContext) {
    let editor = cx.new(|cx| Editor::from_markdown(cx, "alpha".to_string(), None));

    editor.update(cx, |editor, cx| {
        editor
            .active_pane_state()
            .focus
            .pending_scroll_active_block_into_view = true;
        editor
            .active_pane_state()
            .focus
            .pending_scroll_recheck_after_layout = true;

        let pane_id = editor.active_pane_id();
        editor.start_scrollbar_drag(pane_id, 12.0, 320.0, 64.0, 500.0, cx);
        assert_eq!(
            editor.active_pane_scroll().scrollbar_drag,
            Some(crate::editor::controller::ScrollbarDragSession {
                pointer_offset_y: 12.0,
                track_height: 320.0,
                thumb_height: 64.0,
                max_scroll_y: 500.0,
            })
        );
        assert!(
            !editor
                .active_pane_focus()
                .pending_scroll_active_block_into_view
        );
        assert!(
            !editor
                .active_pane_focus()
                .pending_scroll_recheck_after_layout
        );

        editor.update_scrollbar_drag(pane_id, 172.0, cx);
        let offset_y = -f32::from(editor.active_pane_scroll().handle.offset().y);
        assert!(offset_y > 0.0);

        editor.end_scrollbar_drag(pane_id, cx);
        assert!(editor.active_pane_scroll().scrollbar_drag.is_none());
    });
}

#[gpui::test]
fn editor_tile_corner_drag_starts_outer_split(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let window = cx.update(|cx| crate::app::window::open_editor_window(cx, String::new(), None));
    cx.run_until_parked();
    let window_any: gpui::AnyWindowHandle = window.into();
    let mut cx = gpui::VisualTestContext::from_window(window_any, cx);
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();

    // The default layout is Explorer (left) + Editor (right). The Editor
    // tile must carry the same outer corner handles as Explorer/Settings:
    // a mouse-down on its top-left corner starts a window-level corner
    // drag on the outer layout.
    let editor_rect = window
        .update(&mut cx.cx, |shell, window, _cx| {
            let viewport = window.viewport_size();
            let mut rects = Vec::new();
            shell.panels.layout.tree.collect_leaf_rects(
                0.0,
                0.0,
                f32::from(viewport.width),
                f32::from(viewport.height),
                &mut rects,
            );
            rects
                .into_iter()
                .find(|r| r.id == DEFAULT_EDITOR_PANEL_ID)
                .expect("editor leaf rect")
        })
        .expect("window update");
    // The tiled layout sits below the custom titlebar; leaf rects are
    // layout-local, so hit-test coordinates need the titlebar offset.
    let titlebar_height = window
        .update(&mut cx.cx, |_shell, window, _cx| {
            let theme = _cx
                .global::<crate::infra::theme::ThemeManager>()
                .current_arc();
            crate::ui::custom_titlebar::custom_titlebar_height(window, &theme.dimensions)
        })
        .expect("window update");

    let corner = gpui::Point {
        x: gpui::px(editor_rect.x + 10.0),
        y: gpui::px(editor_rect.y + titlebar_height + 10.0),
    };
    cx.simulate_mouse_down(corner, MouseButton::Left, gpui::Modifiers::none());
    cx.run_until_parked();

    let drag_panel = window
        .update(&mut cx.cx, |shell, _window, _cx| {
            shell.panels.layout.corner_drag_panel()
        })
        .expect("window update");
    assert_eq!(drag_panel, Some(DEFAULT_EDITOR_PANEL_ID));
}

#[gpui::test]
fn editor_type_dropdown_switches_panel_kind(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let window = cx.update(|cx| crate::app::window::open_editor_window(cx, String::new(), None));
    cx.run_until_parked();
    let window_any: gpui::AnyWindowHandle = window.into();
    let mut cx = gpui::VisualTestContext::from_window(window_any, cx);
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();

    // Open the Editor tile's type dropdown (the flag lives on the panel).
    window
        .update(&mut cx.cx, |shell, _window, cx| {
            shell.panels.layout.toggle_dropdown(DEFAULT_EDITOR_PANEL_ID);
            cx.notify();
        })
        .expect("window update");
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();

    // Click the "Settings" entry of the floating menu (all() order is
    // [Editor, Explorer, Settings]). The menu renders inside the tile
    // wrapper at top(28) left(8); each entry is menu_item_height tall.
    let editor_rect = window
        .update(&mut cx.cx, |shell, window, _cx| {
            let viewport = window.viewport_size();
            let mut rects = Vec::new();
            shell.panels.layout.tree.collect_leaf_rects(
                0.0,
                0.0,
                f32::from(viewport.width),
                f32::from(viewport.height),
                &mut rects,
            );
            rects
                .into_iter()
                .find(|r| r.id == DEFAULT_EDITOR_PANEL_ID)
                .expect("editor leaf rect")
        })
        .expect("window update");
    let dims = cx.cx.read(|cx| {
        let theme = cx
            .global::<crate::infra::theme::ThemeManager>()
            .current_arc();
        theme.dimensions.clone()
    });
    let titlebar_height = window
        .update(&mut cx.cx, |_shell, window, cx| {
            let theme = cx
                .global::<crate::infra::theme::ThemeManager>()
                .current_arc();
            crate::ui::custom_titlebar::custom_titlebar_height(window, &theme.dimensions)
        })
        .expect("window update");
    let settings_y = titlebar_height
        + 28.0
        + dims.menu_panel_padding
        + 2.0 * dims.menu_item_height
        + dims.menu_item_height / 2.0;
    let settings_point = gpui::Point {
        x: gpui::px(editor_rect.x + 8.0 + dims.menu_panel_width / 2.0),
        y: gpui::px(editor_rect.y + settings_y),
    };
    cx.simulate_click(settings_point, gpui::Modifiers::none());
    cx.run_until_parked();

    let kind = window
        .update(&mut cx.cx, |shell, _window, _cx| {
            shell
                .panels
                .layout
                .tree
                .find_leaf_kind(DEFAULT_EDITOR_PANEL_ID)
        })
        .expect("window update");
    assert_eq!(kind, Some(WindowPanelKind::Settings));
}

#[gpui::test]
fn sole_editor_fallback_and_multi_editor_activation_routing(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let window = cx.update(|cx| crate::app::window::open_editor_window(cx, String::new(), None));
    cx.run_until_parked();

    // 1. Single editor without explicit click: active_editor_panel should resolve to DEFAULT_EDITOR_PANEL_ID
    let target = window
        .update(cx, |shell, _window, _cx| shell.active_editor_panel())
        .expect("window update");
    assert_eq!(target, Some(DEFAULT_EDITOR_PANEL_ID));

    // 2. Split into a second editor: active_editor_panel should resolve to the new split leaf
    let new_panel_id = window
        .update(cx, |shell, _window, cx| {
            let split_id = shell.split_panel(
                DEFAULT_EDITOR_PANEL_ID,
                crate::splitter::SplitAxis::Horizontal,
                0.5,
                false,
                cx,
            );
            if let Some(id) = split_id {
                shell.panels.layout.activate_leaf(id.0);
            }
            split_id
        })
        .expect("window update")
        .expect("split panel created");

    let active_panel = window
        .update(cx, |shell, _window, _cx| shell.active_editor_panel())
        .expect("window update");
    assert_eq!(active_panel, Some(new_panel_id.0));

    // 3. Switch active leaf back to the first editor: active_editor_panel should follow
    window
        .update(cx, |shell, _window, _cx| {
            shell.panels.layout.activate_leaf(DEFAULT_EDITOR_PANEL_ID);
        })
        .expect("window update");

    let switched_panel = window
        .update(cx, |shell, _window, _cx| shell.active_editor_panel())
        .expect("window update");
    assert_eq!(switched_panel, Some(DEFAULT_EDITOR_PANEL_ID));
}

#[gpui::test]
fn window_close_prompts_window_scope_and_discards_all_panels(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let window = cx.update(|cx| crate::app::window::open_editor_window(cx, "panel1".to_string(), None));
    cx.run_until_parked();

    let split_id = window
        .update(cx, |shell, _window, cx| {
            shell.split_panel(
                DEFAULT_EDITOR_PANEL_ID,
                crate::splitter::SplitAxis::Horizontal,
                0.5,
                true,
                cx,
            )
        })
        .expect("window update")
        .expect("split panel created");

    // Mark both editor panels dirty
    window
        .update(cx, |shell, _window, cx| {
            let ed1 = shell.editor_for(DEFAULT_EDITOR_PANEL_ID).unwrap();
            ed1.update(cx, |e, cx| e.mark_dirty(cx));
            let ed2 = shell.editor_for(split_id).unwrap();
            ed2.update(cx, |e, cx| e.mark_dirty(cx));
        })
        .expect("mark dirty");

    // Trigger window close
    window
        .update(cx, |shell, window, cx| {
            shell.request_close_current_window(window, cx);
        })
        .expect("request close window");

    // Check dialog scope is Window
    window
        .update(cx, |shell, _window, _cx| {
            assert!(shell.unsaved_dialog.is_some());
            let dialog = shell.unsaved_dialog.as_ref().unwrap();
            assert_eq!(dialog.scope, crate::app::shell::UnsavedDialogScope::Window);
        })
        .expect("check dialog");

    // Discard and close: window should close
    window
        .update(cx, |shell, window, cx| {
            let event = gpui::ClickEvent::default();
            shell.on_discard_and_close(&event, window, cx);
        })
        .expect("discard and close");
    cx.run_until_parked();

    assert!(cx.update(|cx| cx.windows().is_empty()));
}

#[gpui::test]
fn editor_panel_close_prompts_editor_panel_scope_and_discards_panel_only(
    cx: &mut TestAppContext,
) {
    init_editor_test_app(cx);

    let window = cx.update(|cx| crate::app::window::open_editor_window(cx, "panel1".to_string(), None));
    cx.run_until_parked();

    let split_id = window
        .update(cx, |shell, _window, cx| {
            shell.split_panel(
                DEFAULT_EDITOR_PANEL_ID,
                crate::splitter::SplitAxis::Horizontal,
                0.5,
                true,
                cx,
            )
        })
        .expect("window update")
        .expect("split panel created");

    // Mark both editor panels dirty
    window
        .update(cx, |shell, _window, cx| {
            let ed1 = shell.editor_for(DEFAULT_EDITOR_PANEL_ID).unwrap();
            ed1.update(cx, |e, cx| e.mark_dirty(cx));
            let ed2 = shell.editor_for(split_id).unwrap();
            ed2.update(cx, |e, cx| e.mark_dirty(cx));
        })
        .expect("mark dirty");

    // Request closing only split_id editor panel
    window
        .update(cx, |shell, _window, cx| {
            shell.request_close_panel(split_id, cx);
        })
        .expect("request close panel");

    // Check dialog scope is EditorPanel(split_id)
    window
        .update(cx, |shell, _window, _cx| {
            assert!(shell.unsaved_dialog.is_some());
            let dialog = shell.unsaved_dialog.as_ref().unwrap();
            assert_eq!(
                dialog.scope,
                crate::app::shell::UnsavedDialogScope::EditorPanel(split_id)
            );
        })
        .expect("check dialog");

    // Discard and close: only split_id should be removed, window and panel1 remain!
    window
        .update(cx, |shell, window, cx| {
            let event = gpui::ClickEvent::default();
            shell.on_discard_and_close(&event, window, cx);
        })
        .expect("discard and close");
    cx.run_until_parked();

    // Window must still exist!
    assert_eq!(cx.update(|cx| cx.windows().len()), 1);

    // split_id editor panel must be gone; DEFAULT_EDITOR_PANEL_ID must remain dirty!
    window
        .update(cx, |shell, _window, cx| {
            assert!(shell.editor_for(split_id).is_none());
            assert!(shell.editor_for(DEFAULT_EDITOR_PANEL_ID).is_some());
            let ed1 = shell.editor_for(DEFAULT_EDITOR_PANEL_ID).unwrap();
            assert!(ed1.read(cx).tab().file.dirty);
        })
        .expect("verify remaining panel");
}

#[gpui::test]
fn test_unsaved_dialog_tab_discard_and_close(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let window = cx.update(|cx| crate::app::window::open_editor_window(cx, "tab 1".to_string(), None));
    cx.run_until_parked();

    window
        .update(cx, |shell, _window, cx| {
            // Add a second dirty tab to the default editor panel
            let editor = shell.editor_for(DEFAULT_EDITOR_PANEL_ID).unwrap().clone();
            editor.update(cx, |editor, cx| {
                let tab = Editor::new_tab_from_markdown(cx, "tab 2".to_string(), None);
                let tab_idx = editor.session_mut().tab_list.push(tab);
                editor.session_mut().tab_list.get_mut(tab_idx).unwrap().file.dirty = true;
                editor.request_close_tab(1, cx);
            });
        })
        .expect("request close tab");
    cx.run_until_parked();

    // Check dialog scope is Tab { panel_id, index: 1 }
    window
        .update(cx, |shell, _window, _cx| {
            assert!(shell.unsaved_dialog.is_some());
            let dialog = shell.unsaved_dialog.as_ref().unwrap();
            assert_eq!(
                dialog.scope,
                crate::app::shell::UnsavedDialogScope::Tab {
                    panel_id: crate::app::window_panels::PanelId(DEFAULT_EDITOR_PANEL_ID),
                    index: 1,
                }
            );
        })
        .expect("check dialog");

    // Discard and close: tab 1 is closed; tab 0, editor, and window remain!
    window
        .update(cx, |shell, window, cx| {
            let event = gpui::ClickEvent::default();
            shell.on_discard_and_close(&event, window, cx);
        })
        .expect("discard and close");
    cx.run_until_parked();

    assert_eq!(cx.update(|cx| cx.windows().len()), 1);

    window
        .update(cx, |shell, _window, cx| {
            let editor = shell.editor_for(DEFAULT_EDITOR_PANEL_ID).unwrap();
            assert_eq!(editor.read(cx).session().tab_count(), 1);
        })
        .expect("verify tab count");
}

#[gpui::test]
fn unsaved_dialog_cancel_leaves_document_dirty(cx: &mut TestAppContext) {
    init_editor_test_app(cx);

    let window = cx.update(|cx| crate::app::window::open_editor_window(cx, "content".to_string(), None));
    cx.run_until_parked();

    window
        .update(cx, |shell, _window, cx| {
            let editor = shell.editor_for(DEFAULT_EDITOR_PANEL_ID).unwrap();
            editor.update(cx, |ed, cx| {
                ed.mark_dirty(cx);
            });
        })
        .expect("mark dirty");

    window
        .update(cx, |shell, _window, cx| {
            shell.prompt_close_editor_for(DEFAULT_EDITOR_PANEL_ID, cx);
        })
        .expect("prompt");

    window
        .update(cx, |shell, _window, _cx| {
            assert!(shell.unsaved_dialog.is_some());
        })
        .expect("check dialog");

    // Cancel dialog
    window
        .update(cx, |shell, window, cx| {
            let event = gpui::ClickEvent::default();
            shell.on_cancel_close_dialog(&event, window, cx);
        })
        .expect("cancel");

    // Dialog is closed, window and editor are intact and dirty
    window
        .update(cx, |shell, _window, cx| {
            assert!(shell.unsaved_dialog.is_none());
            let editor = shell.editor_for(DEFAULT_EDITOR_PANEL_ID).unwrap();
            assert!(editor.read(cx).tab().file.dirty);
        })
        .expect("verify cancel");
}
