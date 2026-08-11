//! Native application menu, app-level actions, and window close routing.
//!
//! This module owns menu construction dispatch and the actions that
//! operate on the active editor window. The Quit action is routed to the
//! current window so the existing unsaved-changes dialog remains
//! authoritative for that window.
//!
//! Submodules: `menu_build` (the platform menu tree), `menu_prompts`
//! (file-open and config-import prompt flows).

use std::path::{Path, PathBuf};

use gpui::*;

use crate::app::actions::{
    AddLanguageConfig, AddThemeConfig, CheckForUpdates, CloseExplorerFolder, CloseWindow,
    InstallCliTool, NewWindow, NoRecentFiles, OpenBugReport, OpenDiscussions, OpenFeatureRequest,
    OpenFile, OpenRecentFile, OpenSettings, OpenSplitypeRepository, QuitApplication,
    SelectLanguage, SelectTheme, ShowAbout, ToggleExplorer, UninstallCliTool,
};
#[cfg(target_os = "macos")]
use crate::app::cli_install::{install_cli_tool, uninstall_cli_tool};
#[cfg(not(target_os = "macos"))]
use crate::app::cli_install::{install_cli_tool, uninstall_cli_tool};
use crate::app::shell::Shell;
use crate::app::window::{open_editor_window, record_recent_file_and_refresh};
use crate::editor::actions::{ExportHtml, ExportPdf, SaveDocument, SaveDocumentAs};
use crate::editor::controller::{Editor, InfoDialogKind};
use crate::editor::render::export::ExportFormat;
use crate::editor::view::{
    open_bug_report, open_discussions, open_feature_request, open_splitype_repository,
};
use crate::infra::config::settings::{apply_configured_language, apply_configured_theme};
use crate::infra::i18n::I18nManager;
use crate::infra::theme::ThemeManager;
use crate::settings::open_settings_window;

pub(crate) mod menu_build;
pub(crate) mod menu_prompts;

/// Global app-menu state for platform menu lifecycle hooks.
#[derive(Default)]
pub(crate) struct AppMenuState {
    window_closed_subscription: Option<Subscription>,
}

impl Global for AppMenuState {}

pub(crate) fn record_recent_file_from_editor(path: &Path, cx: &mut App) {
    record_recent_file_and_refresh(path, cx);
}

fn show_window_prompt(window: Option<AnyWindowHandle>, title: &str, detail: &str, cx: &mut App) {
    if let Some(window) = window {
        let ok = cx.global::<I18nManager>().strings().info_dialog_ok.clone();
        let _ = window.update(cx, |_view, window, cx| {
            let buttons = [ok.as_str()];
            let _ = window.prompt(PromptLevel::Critical, title, Some(detail), &buttons, cx);
        });
    } else {
        eprintln!("{title}: {detail}");
    }
}

fn with_active_window<R>(
    cx: &mut App,
    update: impl FnOnce(&mut Shell, &mut Window, &mut Context<Shell>) -> R,
) -> Option<R> {
    let window = cx.active_window()?.downcast::<Shell>()?;
    window.update(cx, update).ok()
}

fn with_primary_editor<R>(
    cx: &mut App,
    update: impl FnOnce(&mut Editor, &mut Window, &mut Context<Editor>) -> R,
) -> Option<R> {
    let window = cx.active_window()?.downcast::<Shell>()?;
    let result = window.update(cx, |shell, window, cx| {
        let Some(editor) = shell.primary_editor().cloned() else {
            return None;
        };
        Some(editor.update(cx, |editor, cx| update(editor, window, cx)))
    });
    result.ok().flatten()
}

fn show_info_dialog_on_active_window(cx: &mut App, kind: InfoDialogKind) {
    let _ = with_active_window(cx, move |shell, _window, cx| {
        shell.show_info_dialog(kind, cx);
    });
}

fn request_update_check_on_active_window(cx: &mut App) {
    let _ = with_active_window(cx, |shell, window, cx| {
        shell.request_check_updates(window, cx);
    });
}

fn is_editor_scoped_menu_action(action: &dyn Action) -> bool {
    action.as_any().is::<SaveDocument>()
        || action.as_any().is::<SaveDocumentAs>()
        || action.as_any().is::<ExportHtml>()
        || action.as_any().is::<ExportPdf>()
        || action.as_any().is::<QuitApplication>()
        || action.as_any().is::<CloseWindow>()
        || action.as_any().is::<CheckForUpdates>()
        || action.as_any().is::<ShowAbout>()
        || action.as_any().is::<InstallCliTool>()
        || action.as_any().is::<UninstallCliTool>()
        || action.as_any().is::<ToggleExplorer>()
        || action.as_any().is::<CloseExplorerFolder>()
}

fn is_window_context_menu_action(action: &dyn Action) -> bool {
    action.as_any().is::<NewWindow>()
        || action.as_any().is::<OpenFile>()
        || action.as_any().is::<OpenSettings>()
        || action.as_any().is::<OpenRecentFile>()
        || action.as_any().is::<NoRecentFiles>()
        || action.as_any().is::<AddLanguageConfig>()
        || action.as_any().is::<AddThemeConfig>()
        || action.as_any().is::<InstallCliTool>()
        || action.as_any().is::<UninstallCliTool>()
        || is_editor_scoped_menu_action(action)
}

fn current_window_candidates(cx: &mut App) -> Vec<AnyWindowHandle> {
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

fn request_close_editor_window(window: AnyWindowHandle, cx: &mut App) -> bool {
    let Some(window) = window.downcast::<crate::app::shell::Shell>() else {
        return false;
    };
    window
        .update(cx, |shell, window, cx| {
            shell.request_close_current_window(window, cx);
        })
        .is_ok()
}

fn request_close_current_editor_window(cx: &mut App) {
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
}

pub(crate) fn request_quit_application(cx: &mut App) {
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
            .unwrap_or(false);
        if !should_close {
            return;
        }
    }

    cx.quit();
}

/// Executes one of the app-menu actions against the current application state.
pub(crate) fn dispatch_menu_action(action: &dyn Action, cx: &mut App) {
    if action.as_any().is::<NewWindow>() {
        open_editor_window(cx, String::new(), None);
    } else if action.as_any().is::<OpenFile>() {
        menu_prompts::prompt_and_open_files(cx);
    } else if action.as_any().is::<OpenSettings>() {
        open_settings_window(cx);
    } else if let Some(action) = action.as_any().downcast_ref::<OpenRecentFile>() {
        menu_prompts::open_recent_file(cx, PathBuf::from(&action.path));
    } else if action.as_any().is::<NoRecentFiles>() {
    } else if action.as_any().is::<AddLanguageConfig>() {
        menu_prompts::prompt_and_import_language_config(cx);
    } else if action.as_any().is::<AddThemeConfig>() {
        menu_prompts::prompt_and_import_theme_config(cx);
    } else if action.as_any().is::<SaveDocument>() {
        let _ = with_primary_editor(cx, |editor, window, cx| editor.save_document(window, cx));
    } else if action.as_any().is::<SaveDocumentAs>() {
        let _ = with_primary_editor(cx, |editor, window, cx| editor.save_document_as(window, cx));
    } else if action.as_any().is::<ExportHtml>() {
        let _ = with_primary_editor(cx, |editor, window, cx| {
            editor.export_document_via_prompt(ExportFormat::Html, window, cx)
        });
    } else if action.as_any().is::<ExportPdf>() {
        let _ = with_primary_editor(cx, |editor, window, cx| {
            editor.export_document_via_prompt(ExportFormat::Pdf, window, cx)
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
        let _ = with_active_window(cx, |shell, window, cx| {
            shell.toggle_explorer_drawer(window, cx);
        });
    } else if action.as_any().is::<CloseExplorerFolder>() {
        let _ = with_active_window(cx, |shell, _window, cx| {
            shell.close_explorer_folder(cx);
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

/// Executes a menu action against a specific editor when the action is
/// editor-scoped, falling back to app-wide behavior for global actions.
pub(crate) fn dispatch_menu_action_for_editor(
    action: &dyn Action,
    target: &WeakEntity<Editor>,
    window: &mut Window,
    cx: &mut App,
) {
    if !is_window_context_menu_action(action) {
        let deferred_action = action.boxed_clone();
        cx.defer(move |cx| {
            dispatch_menu_action(deferred_action.as_ref(), cx);
        });
        return;
    }

    window.activate_window();
    let current_window = Some(window.window_handle());

    // Document-dependent actions are no-ops in the welcome state (no tabs);
    // the UI hides them, but menu accelerators could still fire them.
    if action.as_any().is::<SaveDocument>()
        || action.as_any().is::<SaveDocumentAs>()
        || action.as_any().is::<ExportHtml>()
        || action.as_any().is::<ExportPdf>()
    {
        let no_tab = target
            .update(cx, |editor, _cx| !editor.has_active_tab())
            .unwrap_or(true);
        if no_tab {
            return;
        }
    }

    if action.as_any().is::<NewWindow>() {
        open_editor_window(cx, String::new(), None);
    } else if action.as_any().is::<OpenFile>() {
        menu_prompts::prompt_and_open_files_with_error_window(cx, current_window);
    } else if action.as_any().is::<OpenSettings>() {
        open_settings_window(cx);
    } else if let Some(action) = action.as_any().downcast_ref::<OpenRecentFile>() {
        menu_prompts::open_recent_file_with_error_window(
            cx,
            PathBuf::from(&action.path),
            current_window,
        );
    } else if action.as_any().is::<NoRecentFiles>() {
    } else if action.as_any().is::<AddLanguageConfig>() {
        menu_prompts::prompt_and_import_language_config_with_error_window(cx, current_window);
    } else if action.as_any().is::<AddThemeConfig>() {
        menu_prompts::prompt_and_import_theme_config_with_error_window(cx, current_window);
    } else if action.as_any().is::<SaveDocument>() {
        let _ = target.update(cx, |editor, cx| editor.request_save_document(cx));
    } else if action.as_any().is::<SaveDocumentAs>() {
        let _ = target.update(cx, |editor, cx| editor.request_save_document_as(cx));
    } else if action.as_any().is::<ExportHtml>() {
        let _ = target.update(cx, |editor, cx| {
            editor.export_document_via_prompt(ExportFormat::Html, window, cx);
        });
    } else if action.as_any().is::<ExportPdf>() {
        let _ = target.update(cx, |editor, cx| {
            editor.export_document_via_prompt(ExportFormat::Pdf, window, cx);
        });
    } else if action.as_any().is::<QuitApplication>() {
        request_quit_application(cx);
    } else if action.as_any().is::<CloseWindow>() {
        // Read the editor's shell handle instead of updating through it:
        // the Shell's aggregated dirty check reads every editor entity,
        // which would collide with an editor currently being updated.
        let shell = target
            .read_with(cx, |editor, _cx| editor.shell.clone())
            .ok()
            .flatten();
        if let Some(shell) = shell {
            let _ = shell.update(cx, |shell, cx| {
                shell.request_close_current_window(window, cx);
            });
        }
    } else if action.as_any().is::<CheckForUpdates>() {
        let _ = target.update(cx, |editor, cx| {
            if let Some(shell) = editor.shell.clone() {
                let _ = shell.update(cx, |shell, cx| {
                    shell.request_check_updates(window, cx);
                });
            }
        });
    } else if action.as_any().is::<ShowAbout>() {
        let _ = target.update(cx, |editor, cx| {
            if let Some(shell) = editor.shell.clone() {
                let _ = shell.update(cx, |shell, cx| {
                    shell.show_info_dialog(InfoDialogKind::About, cx);
                });
            }
        });
    } else if action.as_any().is::<InstallCliTool>() {
        install_cli_tool(cx);
        install_menus(cx);
    } else if action.as_any().is::<UninstallCliTool>() {
        uninstall_cli_tool(cx);
        install_menus(cx);
    } else if action.as_any().is::<ToggleExplorer>() {
        let _ = target.update(cx, |editor, cx| {
            if let Some(shell) = editor.shell.clone() {
                let _ = shell.update(cx, |shell, cx| {
                    shell.toggle_explorer_drawer(window, cx);
                });
            }
        });
    } else if action.as_any().is::<CloseExplorerFolder>() {
        let _ = target.update(cx, |editor, cx| {
            if let Some(shell) = editor.shell.clone() {
                let _ = shell.update(cx, |shell, cx| shell.close_explorer_folder(cx));
            }
        });
    }
}

pub(crate) fn install_menus(cx: &mut App) {
    let recent_files = menu_prompts::recent_files_for_menu();
    let menus = menu_build::build_menus(
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
pub(crate) fn init(cx: &mut App) {
    cx.set_global(AppMenuState::default());
    let subscription = cx.on_window_closed(handle_window_closed);
    cx.global_mut::<AppMenuState>().window_closed_subscription = Some(subscription);

    cx.on_action(|_: &NewWindow, cx| {
        dispatch_menu_action(&NewWindow, cx);
    });
    cx.on_action(|_: &OpenFile, cx| {
        dispatch_menu_action(&OpenFile, cx);
    });
    cx.on_action(|_: &OpenSettings, cx| {
        dispatch_menu_action(&OpenSettings, cx);
    });
    cx.on_action(|action: &OpenRecentFile, cx| {
        dispatch_menu_action(action, cx);
    });
    cx.on_action(|_: &NoRecentFiles, cx| {
        dispatch_menu_action(&NoRecentFiles, cx);
    });
    cx.on_action(|_: &AddLanguageConfig, cx| {
        dispatch_menu_action(&AddLanguageConfig, cx);
    });
    cx.on_action(|_: &AddThemeConfig, cx| {
        dispatch_menu_action(&AddThemeConfig, cx);
    });
    cx.on_action(|_: &SaveDocument, cx| {
        dispatch_menu_action(&SaveDocument, cx);
    });
    cx.on_action(|_: &SaveDocumentAs, cx| {
        dispatch_menu_action(&SaveDocumentAs, cx);
    });
    cx.on_action(|_: &ExportHtml, cx| {
        dispatch_menu_action(&ExportHtml, cx);
    });
    cx.on_action(|_: &ExportPdf, cx| {
        dispatch_menu_action(&ExportPdf, cx);
    });
    cx.on_action(|action: &SelectTheme, cx| {
        dispatch_menu_action(action, cx);
    });
    cx.on_action(|action: &SelectLanguage, cx| {
        dispatch_menu_action(action, cx);
    });
    cx.on_action(|_: &CheckForUpdates, cx| {
        dispatch_menu_action(&CheckForUpdates, cx);
    });
    cx.on_action(|_: &ShowAbout, cx| {
        dispatch_menu_action(&ShowAbout, cx);
    });
    cx.on_action(|_: &ToggleExplorer, cx| {
        dispatch_menu_action(&ToggleExplorer, cx);
    });
    cx.on_action(|_: &CloseExplorerFolder, cx| {
        dispatch_menu_action(&CloseExplorerFolder, cx);
    });
    cx.on_action(|_: &QuitApplication, cx| {
        dispatch_menu_action(&QuitApplication, cx);
    });
    cx.on_action(|_: &CloseWindow, cx| {
        dispatch_menu_action(&CloseWindow, cx);
    });
    cx.on_action(|_: &OpenSplitypeRepository, cx| {
        dispatch_menu_action(&OpenSplitypeRepository, cx);
    });
    cx.on_action(|_: &OpenBugReport, cx| {
        dispatch_menu_action(&OpenBugReport, cx);
    });
    cx.on_action(|_: &OpenFeatureRequest, cx| {
        dispatch_menu_action(&OpenFeatureRequest, cx);
    });
    cx.on_action(|_: &OpenDiscussions, cx| {
        dispatch_menu_action(&OpenDiscussions, cx);
    });

    install_menus(cx);
    cx.activate(true);
}

#[cfg(test)]
mod tests {
    use crate::app::actions::{
        AddLanguageConfig, AddThemeConfig, CloseWindow, NewWindow, NoRecentFiles, OpenFile,
        OpenRecentFile, OpenSettings, QuitApplication, SelectLanguage, SelectTheme,
    };
    use crate::editor::actions::SaveDocument;

    #[test]
    fn fallback_menu_routes_window_context_actions_without_app_defer() {
        assert!(super::is_window_context_menu_action(&NewWindow));
        assert!(super::is_window_context_menu_action(&OpenFile));
        assert!(super::is_window_context_menu_action(&OpenSettings));
        assert!(super::is_window_context_menu_action(&OpenRecentFile {
            path: "notes.md".into(),
        }));
        assert!(super::is_window_context_menu_action(&NoRecentFiles));
        assert!(super::is_window_context_menu_action(&AddLanguageConfig));
        assert!(super::is_window_context_menu_action(&AddThemeConfig));
        assert!(super::is_window_context_menu_action(&SaveDocument));
        assert!(super::is_window_context_menu_action(&QuitApplication));
        assert!(super::is_window_context_menu_action(&CloseWindow));
        assert!(!super::is_window_context_menu_action(&SelectTheme {
            theme_id: "splitype".into(),
        }));
        assert!(!super::is_window_context_menu_action(&SelectLanguage {
            language_id: "en-US".into(),
        }));
    }
}
