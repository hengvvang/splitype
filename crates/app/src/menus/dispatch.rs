//! Menu action routing, target window lookup, and global menu action handlers.

use gpui::*;
use std::path::PathBuf;

use super::install_menus;
use super::prompts;
use crate::actions::{
    AddLanguageConfig, AddThemeConfig, CheckForUpdates, CloseExplorerFolder, CloseWindow,
    InstallCliTool, NewWindow, NoRecentFiles, OpenBugReport, OpenDiscussions, OpenFeatureRequest,
    OpenFile, OpenRecentFile, OpenSettings, OpenSplitypeRepository, QuitApplication,
    SelectLanguage, SelectTheme, ShowAbout, ToggleExplorer, UninstallCliTool,
};
use crate::dialogs::InfoDialogKind;
use crate::shell::Shell;
use crate::window::open_editor_window;
use config::language::{I18nManager, apply_language_selection};
use editor::actions::{ExportHtml, ExportPdf, SaveDocument, SaveDocumentAs};
use editor_contracts::{DocumentPanel, ExportFormat};
use platform_contracts::PanelId;
use crate::chrome::settings_window::open_settings_window;
use splitype_installer::{install_cli_tool, uninstall_cli_tool};
use theme::apply_theme_selection;

pub(crate) fn show_window_prompt(
    window: Option<AnyWindowHandle>,
    title: &str,
    detail: &str,
    cx: &mut App,
) {
    if let Some(window) = window {
        let ok = cx.global::<I18nManager>().strings().info_dialog_ok.clone();
        let _ = window.update(cx, |_view, window, cx| {
            let buttons = [ok.as_str()];
            let _ = window.prompt(PromptLevel::Critical, title, Some(detail), &buttons, cx);
        });
    } else {
        tracing::error!(title, detail, "menu action prompt error");
    }
}

pub(crate) fn with_active_window<R>(
    cx: &mut App,
    update: impl FnOnce(&mut Shell, &mut Window, &mut Context<Shell>) -> R,
) -> Option<R> {
    let window = cx.active_window()?.downcast::<Shell>()?;
    window.update(cx, update).ok()
}

pub(crate) fn with_shell_window<R>(
    cx: &mut App,
    update: impl Fn(&mut Shell, &mut Window, &mut Context<Shell>) -> R,
) -> Option<R> {
    for window in current_window_candidates(cx) {
        if let Some(shell) = window.downcast::<Shell>() {
            if let Ok(result) = shell.update(cx, |shell, window, cx| update(shell, window, cx)) {
                return Some(result);
            }
        }
    }
    None
}

pub(crate) fn with_primary_document_panel<R>(
    cx: &mut App,
    update: impl FnOnce(&mut dyn DocumentPanel, &mut Window, &mut App) -> R,
) -> Option<R> {
    let window = cx.active_window()?.downcast::<Shell>()?;
    let result = window.update(cx, |shell, window, cx| {
        let panel_id = shell.primary_document_panel_id()?;
        let panel = shell.document_panel_mut_for(panel_id)?;
        Some(update(panel, window, cx))
    });
    result.ok().flatten()
}

pub(crate) fn current_window_candidates(cx: &mut App) -> Vec<AnyWindowHandle> {
    let mut candidates = Vec::new();
    let mut push_unique = |window: AnyWindowHandle| {
        if candidates
            .iter()
            .all(|candidate: &AnyWindowHandle| candidate.window_id() != window.window_id())
        {
            candidates.push(window);
        }
    };

    if let Some(window) = cx.active_window() {
        push_unique(window);
    }
    if let Some(windows) = cx.window_stack() {
        for window in windows {
            push_unique(window);
        }
    }
    for window in cx.windows() {
        push_unique(window);
    }

    candidates
}

pub(crate) fn request_close_editor_window(window: AnyWindowHandle, cx: &mut App) -> bool {
    let Some(window) = window.downcast::<Shell>() else {
        return false;
    };
    window
        .update(cx, |shell, window, cx| {
            shell.request_close_current_window(window, cx);
        })
        .is_ok()
}

pub(crate) fn request_close_current_editor_window(cx: &mut App) {
    cx.defer(|cx| {
        let candidates = current_window_candidates(cx);
        if candidates.is_empty() {
            cx.quit();
            return;
        }

        for window in candidates {
            if request_close_editor_window(window, cx) {
                return;
            }
        }
    });
}

pub(crate) fn request_quit_application(cx: &mut App) {
    cx.defer(|cx| {
        let candidates = current_window_candidates(cx);
        if candidates.is_empty() {
            cx.quit();
            return;
        }

        for window in candidates {
            let Some(window) = window.downcast::<Shell>() else {
                continue;
            };

            let should_close = window
                .update(cx, |shell, window, cx| {
                    shell.on_window_should_close(window, cx)
                })
                .unwrap_or(true);
            if !should_close {
                return;
            }
        }

        cx.quit();
    });
}

/// Target context of an in-window menu dispatch: the originating shell,
/// the panel the menu was rendered for, and the window itself (needed for
/// per-window document operations and dialogs).
pub(crate) struct MenuDispatchTarget<'a> {
    pub shell: &'a WeakEntity<Shell>,
    pub panel_id: Option<PanelId>,
    pub window: &'a mut Window,
}

impl MenuDispatchTarget<'_> {
    fn window_handle(&self) -> AnyWindowHandle {
        self.window.window_handle()
    }
}

/// The single menu-action dispatcher. `target` is `Some` when the action was
/// invoked from an in-window menu (giving it a shell, panel, and window);
/// native menu actions dispatch with `None` and resolve their window through
/// the active window / primary document panel instead.
pub(crate) fn dispatch_menu_action(
    action: &dyn Action,
    mut target: Option<MenuDispatchTarget<'_>>,
    cx: &mut App,
) {
    if !action.as_any().is::<OpenSettings>() && !action.as_any().is::<NewWindow>() {
        if let Some(target) = target.as_mut() {
            target.window.activate_window();
        }
    }
    let error_window = target.as_ref().map(|target| target.window_handle());

    if action.as_any().is::<NewWindow>() {
        open_editor_window(cx, String::new(), None);
    } else if action.as_any().is::<OpenFile>() {
        prompts::prompt_and_open_files_with_error_window(cx, error_window);
    } else if action.as_any().is::<OpenSettings>() {
        open_settings_window(cx);
    } else if let Some(action) = action.as_any().downcast_ref::<OpenRecentFile>() {
        prompts::open_recent_file_with_error_window(cx, PathBuf::from(&action.path), error_window);
    } else if action.as_any().is::<NoRecentFiles>() {
        // Disabled item.
    } else if action.as_any().is::<AddLanguageConfig>() {
        prompts::prompt_and_import_language_config_with_error_window(cx, error_window);
    } else if action.as_any().is::<AddThemeConfig>() {
        prompts::prompt_and_import_theme_config_with_error_window(cx, error_window);
    } else if action.as_any().is::<SaveDocument>() {
        run_document_action(target, cx, |panel, _window, cx| {
            panel.request_save_document(cx);
        });
    } else if action.as_any().is::<SaveDocumentAs>() {
        run_document_action(target, cx, |panel, _window, cx| {
            panel.request_save_document_as(cx);
        });
    } else if action.as_any().is::<ExportHtml>() {
        run_document_action(target, cx, |panel, window, cx| {
            panel.export_document(ExportFormat::Html, window, cx);
        });
    } else if action.as_any().is::<ExportPdf>() {
        run_document_action(target, cx, |panel, window, cx| {
            panel.export_document(ExportFormat::Pdf, window, cx);
        });
    } else if let Some(action) = action.as_any().downcast_ref::<SelectTheme>() {
        match apply_theme_selection(cx, &action.theme_id) {
            Ok(()) => {
                install_menus(cx);
                cx.refresh_windows();
            }
            Err(err) => {
                let title = cx
                    .global::<I18nManager>()
                    .strings()
                    .settings_save_failed_title
                    .clone();
                show_window_prompt(error_window, &title, &err.to_string(), cx);
            }
        }
    } else if let Some(action) = action.as_any().downcast_ref::<SelectLanguage>() {
        match apply_language_selection(cx, &action.language_id) {
            Ok(()) => {
                install_menus(cx);
                cx.refresh_windows();
            }
            Err(err) => {
                let title = cx
                    .global::<I18nManager>()
                    .strings()
                    .settings_save_failed_title
                    .clone();
                show_window_prompt(error_window, &title, &err.to_string(), cx);
            }
        }
    } else if action.as_any().is::<CheckForUpdates>() {
        run_shell_action(target, cx, |shell, window, cx| {
            shell.request_check_updates(window, cx);
        });
    } else if action.as_any().is::<ShowAbout>() {
        run_shell_action(target, cx, |shell, _window, cx| {
            shell.show_info_dialog(InfoDialogKind::About, cx);
        });
    } else if action.as_any().is::<InstallCliTool>() {
        install_cli_tool(cx);
        install_menus(cx);
    } else if action.as_any().is::<UninstallCliTool>() {
        uninstall_cli_tool(cx);
        install_menus(cx);
    } else if action.as_any().is::<ToggleExplorer>() {
        run_shell_action(target, cx, |shell, window, cx| {
            shell.toggle_explorer_tree(window, cx);
        });
    } else if action.as_any().is::<CloseExplorerFolder>() {
        run_shell_action(target, cx, |shell, _window, cx| {
            shell.close_explorer_folder_scope(cx);
        });
    } else if action.as_any().is::<QuitApplication>() {
        request_quit_application(cx);
    } else if action.as_any().is::<CloseWindow>() {
        if let Some(MenuDispatchTarget { shell, window, .. }) = target {
            let _ = shell.update(cx, |shell, cx| {
                shell.request_close_current_window(window, cx);
            });
        } else {
            request_close_current_editor_window(cx);
        }
    } else if action.as_any().is::<OpenSplitypeRepository>() {
        crate::links::open_repository(cx);
    } else if action.as_any().is::<OpenBugReport>() {
        crate::links::open_bug_report(cx);
    } else if action.as_any().is::<OpenFeatureRequest>() {
        crate::links::open_feature_request(cx);
    } else if action.as_any().is::<OpenDiscussions>() {
        crate::links::open_discussions(cx);
    }
}

/// Runs a document-panel operation against the menu's target panel, falling
/// back to the active window's primary document panel for native menus.
fn run_document_action(
    target: Option<MenuDispatchTarget<'_>>,
    cx: &mut App,
    op: impl FnOnce(&mut dyn DocumentPanel, &mut Window, &mut App),
) {
    if let Some(MenuDispatchTarget {
        shell,
        panel_id,
        window,
    }) = target
    {
        let _ = shell.update(cx, |shell, cx| {
            if let Some(panel_id) = panel_id {
                if let Some(panel) = shell.document_panel_mut_for(panel_id) {
                    op(panel, window, cx);
                }
            }
        });
    } else {
        let _ = with_primary_document_panel(cx, op);
    }
}

/// Runs a shell operation against the menu's target shell, falling back to
/// the first available shell window for native menus.
fn run_shell_action(
    target: Option<MenuDispatchTarget<'_>>,
    cx: &mut App,
    op: impl Fn(&mut Shell, &mut Window, &mut Context<Shell>),
) {
    if let Some(MenuDispatchTarget { shell, window, .. }) = target {
        let _ = shell.update(cx, |shell, cx| op(shell, window, cx));
    } else {
        let _ = with_shell_window(cx, op);
    }
}
