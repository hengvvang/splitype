//! Native application menu, app-level actions, and window close routing.

use gpui::*;

use crate::actions::{
    AddLanguageConfig, AddThemeConfig, CheckForUpdates, CloseExplorerFolder, CloseWindow,
    NewWindow, NoRecentFiles, OpenBugReport, OpenDiscussions, OpenFeatureRequest, OpenFile,
    OpenRecentFile, OpenSettings, OpenSplitypeRepository, QuitApplication, SelectLanguage,
    SelectTheme, ShowAbout, ToggleExplorer,
};
use config::language::I18nManager;
use editor::actions::{ExportHtml, ExportPdf, SaveDocument, SaveDocumentAs};
use theme::ThemeManager;

pub mod builder;
pub mod dispatch;
pub mod prompts;

pub(crate) use dispatch::{MenuDispatchTarget, dispatch_menu_action, request_quit_application};

/// Global app-menu state for platform menu lifecycle hooks.
#[derive(Default)]
pub(crate) struct AppMenuState {
    window_closed_subscription: Option<Subscription>,
}

impl Global for AppMenuState {}

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
        dispatch::dispatch_menu_action(&NewWindow, None, cx);
    });
    cx.on_action(|_: &OpenFile, cx| {
        dispatch::dispatch_menu_action(&OpenFile, None, cx);
    });
    cx.on_action(|_: &OpenSettings, cx| {
        dispatch::dispatch_menu_action(&OpenSettings, None, cx);
    });
    cx.on_action(|action: &OpenRecentFile, cx| {
        dispatch::dispatch_menu_action(action, None, cx);
    });
    cx.on_action(|_: &NoRecentFiles, cx| {
        dispatch::dispatch_menu_action(&NoRecentFiles, None, cx);
    });
    cx.on_action(|_: &AddLanguageConfig, cx| {
        dispatch::dispatch_menu_action(&AddLanguageConfig, None, cx);
    });
    cx.on_action(|_: &AddThemeConfig, cx| {
        dispatch::dispatch_menu_action(&AddThemeConfig, None, cx);
    });
    cx.on_action(|_: &SaveDocument, cx| {
        dispatch::dispatch_menu_action(&SaveDocument, None, cx);
    });
    cx.on_action(|_: &SaveDocumentAs, cx| {
        dispatch::dispatch_menu_action(&SaveDocumentAs, None, cx);
    });
    cx.on_action(|_: &ExportHtml, cx| {
        dispatch::dispatch_menu_action(&ExportHtml, None, cx);
    });
    cx.on_action(|_: &ExportPdf, cx| {
        dispatch::dispatch_menu_action(&ExportPdf, None, cx);
    });
    cx.on_action(|action: &SelectTheme, cx| {
        dispatch::dispatch_menu_action(action, None, cx);
    });
    cx.on_action(|action: &SelectLanguage, cx| {
        dispatch::dispatch_menu_action(action, None, cx);
    });
    cx.on_action(|_: &CheckForUpdates, cx| {
        dispatch::dispatch_menu_action(&CheckForUpdates, None, cx);
    });
    cx.on_action(|_: &ShowAbout, cx| {
        dispatch::dispatch_menu_action(&ShowAbout, None, cx);
    });
    cx.on_action(|_: &ToggleExplorer, cx| {
        dispatch::dispatch_menu_action(&ToggleExplorer, None, cx);
    });
    cx.on_action(|_: &CloseExplorerFolder, cx| {
        dispatch::dispatch_menu_action(&CloseExplorerFolder, None, cx);
    });
    cx.on_action(|_: &QuitApplication, cx| {
        dispatch::dispatch_menu_action(&QuitApplication, None, cx);
    });
    cx.on_action(|_: &CloseWindow, cx| {
        dispatch::dispatch_menu_action(&CloseWindow, None, cx);
    });
    cx.on_action(|_: &OpenSplitypeRepository, cx| {
        dispatch::dispatch_menu_action(&OpenSplitypeRepository, None, cx);
    });
    cx.on_action(|_: &OpenBugReport, cx| {
        dispatch::dispatch_menu_action(&OpenBugReport, None, cx);
    });
    cx.on_action(|_: &OpenFeatureRequest, cx| {
        dispatch::dispatch_menu_action(&OpenFeatureRequest, None, cx);
    });
    cx.on_action(|_: &OpenDiscussions, cx| {
        dispatch::dispatch_menu_action(&OpenDiscussions, None, cx);
    });

    install_menus(cx);
    cx.activate(true);
}
