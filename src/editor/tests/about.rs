//! About dialog body and repository link opening.

use gpui::TestAppContext;

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
