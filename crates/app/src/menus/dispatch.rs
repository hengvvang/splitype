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
use config::language::{I18nManager, apply_configured_language};
use core_contracts::{DocumentPanel, ExportFormat};
use editor::actions::{ExportHtml, ExportPdf, SaveDocument, SaveDocumentAs};
use editor::view::{
    open_bug_report, open_discussions, open_feature_request, open_splitype_repository,
};
use settings::open_settings_window;
use splitype_installer::{install_cli_tool, uninstall_cli_tool};
use theme::apply_configured_theme;
use window::PanelId;

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

pub(crate) fn show_info_dialog_on_active_window(cx: &mut App, kind: InfoDialogKind) {
    let _ = with_shell_window(cx, move |shell, _window, cx| {
        shell.show_info_dialog(kind, cx);
    });
}

pub(crate) fn request_update_check_on_active_window(cx: &mut App) {
    let _ = with_shell_window(cx, |shell, window, cx| {
        shell.request_check_updates(window, cx);
    });
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

/// Executes one of the app-menu actions against the current application state.
pub(crate) fn dispatch_menu_action(action: &dyn Action, cx: &mut App) {
    if action.as_any().is::<NewWindow>() {
        open_editor_window(cx, String::new(), None);
    } else if action.as_any().is::<OpenFile>() {
        prompts::prompt_and_open_files(cx);
    } else if action.as_any().is::<OpenSettings>() {
        open_settings_window(cx);
    } else if let Some(action) = action.as_any().downcast_ref::<OpenRecentFile>() {
        prompts::open_recent_file(cx, PathBuf::from(&action.path));
    } else if action.as_any().is::<NoRecentFiles>() {
    } else if action.as_any().is::<AddLanguageConfig>() {
        prompts::prompt_and_import_language_config(cx);
    } else if action.as_any().is::<AddThemeConfig>() {
        prompts::prompt_and_import_theme_config(cx);
    } else if action.as_any().is::<SaveDocument>() {
        let _ =
            with_primary_document_panel(cx, |panel, window, cx| panel.save_document(window, cx));
    } else if action.as_any().is::<SaveDocumentAs>() {
        let _ =
            with_primary_document_panel(cx, |panel, window, cx| panel.save_document_as(window, cx));
    } else if action.as_any().is::<ExportHtml>() {
        let _ = with_primary_document_panel(cx, |panel, window, cx| {
            panel.export_document(ExportFormat::Html, window, cx)
        });
    } else if action.as_any().is::<ExportPdf>() {
        let _ = with_primary_document_panel(cx, |panel, window, cx| {
            panel.export_document(ExportFormat::Pdf, window, cx)
        });
    } else if let Some(action) = action.as_any().downcast_ref::<SelectTheme>() {
        match apply_configured_theme(cx, &action.theme_id) {
            Ok(changed) => {
                if changed {
                    install_menus(cx);
                    cx.refresh_windows();
                }
            }
            Err(err) => {
                let title = cx
                    .global::<I18nManager>()
                    .strings()
                    .settings_save_failed_title
                    .clone();
                show_window_prompt(cx.active_window(), &title, &err.to_string(), cx);
            }
        }
    } else if let Some(action) = action.as_any().downcast_ref::<SelectLanguage>() {
        match apply_configured_language(cx, &action.language_id) {
            Ok(changed) => {
                if changed {
                    install_menus(cx);
                    cx.refresh_windows();
                }
            }
            Err(err) => {
                let title = cx
                    .global::<I18nManager>()
                    .strings()
                    .settings_save_failed_title
                    .clone();
                show_window_prompt(cx.active_window(), &title, &err.to_string(), cx);
            }
        }
    } else if action.as_any().is::<CheckForUpdates>() {
        request_update_check_on_active_window(cx);
    } else if action.as_any().is::<ShowAbout>() {
        show_info_dialog_on_active_window(cx, InfoDialogKind::About);
    } else if action.as_any().is::<InstallCliTool>() {
        install_cli_tool(cx);
        install_menus(cx);
    } else if action.as_any().is::<UninstallCliTool>() {
        uninstall_cli_tool(cx);
        install_menus(cx);
    } else if action.as_any().is::<ToggleExplorer>() {
        let _ = with_shell_window(cx, |shell, window, cx| {
            shell.toggle_sidebar_drawers(window, cx);
        });
    } else if action.as_any().is::<CloseExplorerFolder>() {
        let _ = with_shell_window(cx, |shell, _window, cx| {
            shell.close_sidebar_folders(cx);
        });
    } else if action.as_any().is::<QuitApplication>() {
        request_quit_application(cx);
    } else if action.as_any().is::<CloseWindow>() {
        request_close_current_editor_window(cx);
    } else if action.as_any().is::<OpenSplitypeRepository>() {
        open_splitype_repository(cx);
    } else if action.as_any().is::<OpenBugReport>() {
        open_bug_report(cx);
    } else if action.as_any().is::<OpenFeatureRequest>() {
        open_feature_request(cx);
    } else if action.as_any().is::<OpenDiscussions>() {
        open_discussions(cx);
    }
}

/// Executes a menu action with access to the originating shell window and
/// its document panel, identified by the panel id the menu was rendered for.
pub(crate) fn dispatch_menu_action_for_panel(
    action: &dyn Action,
    target_shell: &WeakEntity<Shell>,
    panel_id: PanelId,
    window: &mut Window,
    cx: &mut App,
) {
    window.activate_window();
    let current_window = Some(window.window_handle());

    if action.as_any().is::<NewWindow>() {
        open_editor_window(cx, String::new(), None);
    } else if action.as_any().is::<OpenFile>() {
        prompts::prompt_and_open_files_with_error_window(cx, current_window);
    } else if action.as_any().is::<OpenSettings>() {
        open_settings_window(cx);
    } else if let Some(action) = action.as_any().downcast_ref::<OpenRecentFile>() {
        prompts::open_recent_file_with_error_window(
            cx,
            PathBuf::from(&action.path),
            current_window,
        );
    } else if action.as_any().is::<NoRecentFiles>() {
        // Disabled item, do nothing
    } else if action.as_any().is::<AddLanguageConfig>() {
        prompts::prompt_and_import_language_config_with_error_window(cx, current_window);
    } else if action.as_any().is::<AddThemeConfig>() {
        prompts::prompt_and_import_theme_config_with_error_window(cx, current_window);
    } else if action.as_any().is::<SaveDocument>() {
        let _ = target_shell.update(cx, |shell, cx| {
            if let Some(panel) = shell.document_panel_mut_for(panel_id) {
                panel.request_save_document(cx);
            }
        });
    } else if action.as_any().is::<SaveDocumentAs>() {
        let _ = target_shell.update(cx, |shell, cx| {
            if let Some(panel) = shell.document_panel_mut_for(panel_id) {
                panel.request_save_document_as(cx);
            }
        });
    } else if action.as_any().is::<ExportHtml>() {
        let _ = target_shell.update(cx, |shell, cx| {
            if let Some(panel) = shell.document_panel_mut_for(panel_id) {
                panel.export_document(ExportFormat::Html, window, cx);
            }
        });
    } else if action.as_any().is::<ExportPdf>() {
        let _ = target_shell.update(cx, |shell, cx| {
            if let Some(panel) = shell.document_panel_mut_for(panel_id) {
                panel.export_document(ExportFormat::Pdf, window, cx);
            }
        });
    } else if let Some(action) = action.as_any().downcast_ref::<SelectTheme>() {
        match apply_configured_theme(cx, &action.theme_id) {
            Ok(changed) => {
                if changed {
                    install_menus(cx);
                    cx.refresh_windows();
                }
            }
            Err(err) => {
                let title = cx
                    .global::<I18nManager>()
                    .strings()
                    .settings_save_failed_title
                    .clone();
                show_window_prompt(current_window, &title, &err.to_string(), cx);
            }
        }
    } else if let Some(action) = action.as_any().downcast_ref::<SelectLanguage>() {
        match apply_configured_language(cx, &action.language_id) {
            Ok(changed) => {
                if changed {
                    install_menus(cx);
                    cx.refresh_windows();
                }
            }
            Err(err) => {
                let title = cx
                    .global::<I18nManager>()
                    .strings()
                    .settings_save_failed_title
                    .clone();
                show_window_prompt(current_window, &title, &err.to_string(), cx);
            }
        }
    } else if action.as_any().is::<QuitApplication>() {
        request_quit_application(cx);
    } else if action.as_any().is::<CloseWindow>() {
        let _ = target_shell.update(cx, |shell, cx| {
            shell.request_close_current_window(window, cx);
        });
    } else if action.as_any().is::<CheckForUpdates>() {
        let _ = target_shell.update(cx, |shell, cx| {
            shell.request_check_updates(window, cx);
        });
    } else if action.as_any().is::<ShowAbout>() {
        let _ = target_shell.update(cx, |shell, cx| {
            shell.show_info_dialog(InfoDialogKind::About, cx);
        });
    } else if action.as_any().is::<InstallCliTool>() {
        install_cli_tool(cx);
        install_menus(cx);
    } else if action.as_any().is::<UninstallCliTool>() {
        uninstall_cli_tool(cx);
        install_menus(cx);
    } else if action.as_any().is::<ToggleExplorer>() {
        let _ = target_shell.update(cx, |shell, cx| {
            shell.toggle_sidebar_drawers(window, cx);
        });
    } else if action.as_any().is::<CloseExplorerFolder>() {
        let _ = target_shell.update(cx, |shell, cx| {
            shell.close_sidebar_folders(cx);
        });
    } else if action.as_any().is::<OpenSplitypeRepository>() {
        open_splitype_repository(cx);
    } else if action.as_any().is::<OpenBugReport>() {
        open_bug_report(cx);
    } else if action.as_any().is::<OpenFeatureRequest>() {
        open_feature_request(cx);
    } else if action.as_any().is::<OpenDiscussions>() {
        open_discussions(cx);
    } else {
        let deferred_action = action.boxed_clone();
        cx.defer(move |cx| {
            dispatch_menu_action(deferred_action.as_ref(), cx);
        });
    }
}
