//! Native application menu, app-level actions, and window close routing.

use std::path::Path;
use gpui::*;

use crate::actions::{
    AddLanguageConfig, AddThemeConfig, CheckForUpdates, CloseExplorerFolder, CloseWindow,
    NewWindow, NoRecentFiles, OpenBugReport, OpenDiscussions, OpenFeatureRequest, OpenFile,
    OpenRecentFile, OpenSettings, OpenSplitypeRepository, QuitApplication, SelectLanguage,
    SelectTheme, ShowAbout, ToggleExplorer,
};
use crate::window::record_recent_file_and_refresh;
use theme::ThemeManager;
use config::language::I18nManager;
use editor::actions::{ExportHtml, ExportPdf, SaveDocument, SaveDocumentAs};

pub mod builder;
pub mod dispatch;
pub mod prompts;

pub(crate) use dispatch::{dispatch_menu_action_for_editor, request_quit_application};

/// Global app-menu state for platform menu lifecycle hooks.
#[derive(Default)]
pub(crate) struct AppMenuState {
    window_closed_subscription: Option<Subscription>,
}

impl Global for AppMenuState {}

pub(crate) fn record_recent_file_from_editor(path: &Path, cx: &mut App) {
    record_recent_file_and_refresh(path, cx);
}

pub fn install_menus(cx: &mut App) {
    let recent_files = prompts::recent_files_for_menu();
    let menus = builder::build_menus(
        cx.global::<ThemeManager>(),
        cx.global::<I18nManager>(),
        &recent_files,
    );
    cx.set_menus(menus);
}

fn handle_window_closed(cx: &mut App) {
    if cx.windows().is_empty() {
        cx.quit();
    }
}

/// Installs menu state, action handlers, and the native menu bar.
pub fn init(cx: &mut App) {
    cx.set_global(AppMenuState::default());
    let subscription = cx.on_window_closed(|cx, _window_id| handle_window_closed(cx));
    cx.global_mut::<AppMenuState>().window_closed_subscription = Some(subscription);

    cx.on_action(|_: &NewWindow, cx| {
        dispatch::dispatch_menu_action(&NewWindow, cx);
    });
    cx.on_action(|_: &OpenFile, cx| {
        dispatch::dispatch_menu_action(&OpenFile, cx);
    });
    cx.on_action(|_: &OpenSettings, cx| {
        dispatch::dispatch_menu_action(&OpenSettings, cx);
    });
    cx.on_action(|action: &OpenRecentFile, cx| {
        dispatch::dispatch_menu_action(action, cx);
    });
    cx.on_action(|_: &NoRecentFiles, cx| {
        dispatch::dispatch_menu_action(&NoRecentFiles, cx);
    });
    cx.on_action(|_: &AddLanguageConfig, cx| {
        dispatch::dispatch_menu_action(&AddLanguageConfig, cx);
    });
    cx.on_action(|_: &AddThemeConfig, cx| {
        dispatch::dispatch_menu_action(&AddThemeConfig, cx);
    });
    cx.on_action(|_: &SaveDocument, cx| {
        dispatch::dispatch_menu_action(&SaveDocument, cx);
    });
    cx.on_action(|_: &SaveDocumentAs, cx| {
        dispatch::dispatch_menu_action(&SaveDocumentAs, cx);
    });
    cx.on_action(|_: &ExportHtml, cx| {
        dispatch::dispatch_menu_action(&ExportHtml, cx);
    });
    cx.on_action(|_: &ExportPdf, cx| {
        dispatch::dispatch_menu_action(&ExportPdf, cx);
    });
    cx.on_action(|action: &SelectTheme, cx| {
        dispatch::dispatch_menu_action(action, cx);
    });
    cx.on_action(|action: &SelectLanguage, cx| {
        dispatch::dispatch_menu_action(action, cx);
    });
    cx.on_action(|_: &CheckForUpdates, cx| {
        dispatch::dispatch_menu_action(&CheckForUpdates, cx);
    });
    cx.on_action(|_: &ShowAbout, cx| {
        dispatch::dispatch_menu_action(&ShowAbout, cx);
    });
    cx.on_action(|_: &ToggleExplorer, cx| {
        dispatch::dispatch_menu_action(&ToggleExplorer, cx);
    });
    cx.on_action(|_: &CloseExplorerFolder, cx| {
        dispatch::dispatch_menu_action(&CloseExplorerFolder, cx);
    });
    cx.on_action(|_: &QuitApplication, cx| {
        dispatch::dispatch_menu_action(&QuitApplication, cx);
    });
    cx.on_action(|_: &CloseWindow, cx| {
        dispatch::dispatch_menu_action(&CloseWindow, cx);
    });
    cx.on_action(|_: &OpenSplitypeRepository, cx| {
        dispatch::dispatch_menu_action(&OpenSplitypeRepository, cx);
    });
    cx.on_action(|_: &OpenBugReport, cx| {
        dispatch::dispatch_menu_action(&OpenBugReport, cx);
    });
    cx.on_action(|_: &OpenFeatureRequest, cx| {
        dispatch::dispatch_menu_action(&OpenFeatureRequest, cx);
    });
    cx.on_action(|_: &OpenDiscussions, cx| {
        dispatch::dispatch_menu_action(&OpenDiscussions, cx);
    });

    install_menus(cx);
    cx.activate(true);
}
