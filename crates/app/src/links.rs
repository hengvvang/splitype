//! Project links — the repository, issue, and community URLs opened from
//! the Help menu and the About dialog.

use gpui::App;

pub const REPOSITORY_URL: &str = "https://github.com/hengvvang/splitype";
pub const BUG_REPORT_URL: &str =
    "https://github.com/hengvvang/splitype/issues/new?template=bug_report.yml";
pub const FEATURE_REQUEST_URL: &str =
    "https://github.com/hengvvang/splitype/issues/new?template=feature_request.yml";
pub const DISCUSSIONS_URL: &str = "https://github.com/hengvvang/splitype/discussions";
pub const WIKI_URL: &str = "https://github.com/hengvvang/splitype/wiki";
pub const RELEASES_URL: &str = "https://github.com/hengvvang/splitype/releases";

pub fn open_repository(cx: &mut App) {
    cx.open_url(REPOSITORY_URL);
}

pub fn open_bug_report(cx: &mut App) {
    cx.open_url(BUG_REPORT_URL);
}

pub fn open_feature_request(cx: &mut App) {
    cx.open_url(FEATURE_REQUEST_URL);
}

pub fn open_discussions(cx: &mut App) {
    cx.open_url(DISCUSSIONS_URL);
}
