//! About dialog body and repository link opening.

use gpui::{AppContext, TestAppContext};

use crate::infra::i18n::I18nStrings;

#[test]
fn about_dialog_body_lines_include_repository_and_star_message() {
    let strings = I18nStrings::zh_cn();
    let lines = crate::app::shell::Shell::about_dialog_body_lines(&strings);

    assert_eq!(lines[0], format!("Splitype {}", env!("CARGO_PKG_VERSION")));
    assert_eq!(
        lines[2],
        format!("GitHub: {}", crate::editor::view::SPLITYPE_REPOSITORY_URL)
    );
    assert_eq!(
        lines[3],
        "如果本项目对您有帮助，那不妨给本项目一颗 Star⭐，十分感谢！"
    );
}

#[gpui::test]
async fn about_github_link_uses_gpui_url_opening(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::editor::view::open_splitype_repository(cx);
    });

    assert_eq!(
        cx.opened_url(),
        Some(crate::editor::view::SPLITYPE_REPOSITORY_URL.to_string())
    );
}

#[gpui::test]
async fn test_show_about_dialog_rendering(cx: &mut TestAppContext) {
    super::init_editor_test_app(cx);
    let window = cx.update(|cx| crate::app::window::open_editor_window(cx, "".to_string(), None));
    cx.run_until_parked();

    cx.update(|cx| {
        crate::app::menus::dispatch_menu_action(&crate::app::actions::ShowAbout, cx);
    });
    cx.run_until_parked();

    let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);
    super::redraw(&mut cx);
}

#[gpui::test]
async fn test_show_about_from_app_menu_for_editor(cx: &mut TestAppContext) {
    super::init_editor_test_app(cx);
    let window = cx.update(|cx| crate::app::window::open_editor_window(cx, "".to_string(), None));
    cx.run_until_parked();

    let editor = window
        .update(cx, |shell, _window, _cx| {
            shell.primary_editor().unwrap().downgrade()
        })
        .unwrap();

    cx.update_window(window.into(), |_view, window, cx| {
        crate::app::menus::dispatch_menu_action_for_editor(
            &crate::app::actions::ShowAbout,
            &editor,
            window,
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();

    let info_dialog = window
        .update(cx, |shell, _window, _cx| shell.info_dialog)
        .unwrap();
    assert_eq!(info_dialog, Some(crate::editor::controller::InfoDialogKind::About));

    let mut visual_cx = gpui::VisualTestContext::from_window(window.into(), cx);
    super::redraw(&mut visual_cx);
}




