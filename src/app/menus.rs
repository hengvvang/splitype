//! Native application menu, app-level actions, and window close routing.
//!
//! This module owns menu construction and the actions that operate on the
//! active editor window. The Quit action is routed to the current window so the
//! existing unsaved-changes dialog remains authoritative for that window.

use std::path::{Path, PathBuf};

use gpui::*;

use crate::app::windows::{
    open_editor_window, open_file_in_new_window, record_recent_file_and_refresh,
};
use crate::editor::actions::{
    AddLanguageConfig, AddThemeConfig, CheckForUpdates, CloseExplorerFolder, CloseWindow,
    ExportHtml, ExportPdf, InstallCliTool, NewWindow, NoRecentFiles, OpenBugReport,
    OpenDiscussions, OpenFeatureRequest, OpenFile, OpenRecentFile, OpenSettings,
    OpenSplitypeRepository, QuitApplication, SaveDocument, SaveDocumentAs, SelectLanguage,
    SelectTheme, ShowAbout, ToggleExplorer, UninstallCliTool,
};
use crate::editor::controller::{Editor, InfoDialogKind};
use crate::editor::render::export::ExportFormat;
use crate::infra::config::recent::{read_recent_files, remove_recent_file};
use crate::infra::config::settings::{
    apply_configured_language, apply_configured_theme, import_language_config_and_select,
    import_theme_config_and_select,
};
use crate::infra::i18n::I18nManager;
#[cfg(target_os = "macos")]
use crate::platform::cli_tool::{install_cli_tool, is_cli_symlink_current_app, uninstall_cli_tool};
#[cfg(not(target_os = "macos"))]
use crate::platform::cli_tool::{install_cli_tool, uninstall_cli_tool};
use crate::theme::ThemeManager;
use crate::windows::editor::{
    open_bug_report, open_discussions, open_feature_request, open_splitype_repository,
};
use crate::windows::settings::open_settings_window;

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

fn with_active_editor<R>(
    cx: &mut App,
    update: impl FnOnce(&mut Editor, &mut Window, &mut Context<Editor>) -> R,
) -> Option<R> {
    let window = cx.active_window()?.downcast::<Editor>()?;
    window.update(cx, update).ok()
}

fn show_info_dialog_on_active_editor(cx: &mut App, kind: InfoDialogKind) {
    let _ = with_active_editor(cx, move |editor, _window, cx| {
        editor.show_info_dialog(kind, cx);
    });
}

fn request_update_check_on_active_editor(cx: &mut App) {
    let _ = with_active_editor(cx, |editor, window, cx| {
        editor.request_check_updates(window, cx);
    });
}

fn recent_files_for_menu() -> Vec<PathBuf> {
    match read_recent_files() {
        Ok(paths) => paths,
        Err(err) => {
            eprintln!("failed to read recent file history: {err}");
            Vec::new()
        }
    }
}

fn open_recent_file(cx: &mut App, path: PathBuf) {
    let error_window = cx.active_window();
    open_recent_file_with_error_window(cx, path, error_window);
}

fn open_recent_file_with_error_window(
    cx: &mut App,
    path: PathBuf,
    error_window: Option<AnyWindowHandle>,
) {
    if !path.is_file() {
        if let Err(err) = remove_recent_file(&path) {
            eprintln!("failed to remove missing recent file: {err}");
        }
        install_menus(cx);
        cx.refresh_windows();
        let strings = cx.global::<I18nManager>().strings().clone();
        let detail = strings
            .recent_file_missing_message_template
            .replace("{path}", &path.to_string_lossy());
        show_window_prompt(
            error_window,
            &strings.recent_file_missing_title,
            &detail,
            cx,
        );
        return;
    }

    open_file_in_editor_or_new_window(cx, &path);
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
    let Some(window) = window.downcast::<Editor>() else {
        return false;
    };

    window
        .update(cx, |editor, window, cx| {
            editor.request_close_current_window(window, cx);
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
        let Some(window) = window.downcast::<Editor>() else {
            continue;
        };

        let should_close = window
            .update(cx, |editor, window, cx| {
                editor.on_window_should_close(window, cx)
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
        prompt_and_open_files(cx);
    } else if action.as_any().is::<OpenSettings>() {
        open_settings_window(cx);
    } else if let Some(action) = action.as_any().downcast_ref::<OpenRecentFile>() {
        open_recent_file(cx, PathBuf::from(&action.path));
    } else if action.as_any().is::<NoRecentFiles>() {
    } else if action.as_any().is::<AddLanguageConfig>() {
        prompt_and_import_language_config(cx);
    } else if action.as_any().is::<AddThemeConfig>() {
        prompt_and_import_theme_config(cx);
    } else if action.as_any().is::<SaveDocument>() {
        let _ = with_active_editor(cx, |editor, window, cx| editor.save_document(window, cx));
    } else if action.as_any().is::<SaveDocumentAs>() {
        let _ = with_active_editor(cx, |editor, window, cx| editor.save_document_as(window, cx));
    } else if action.as_any().is::<ExportHtml>() {
        let _ = with_active_editor(cx, |editor, window, cx| {
            editor.export_document_via_prompt(ExportFormat::Html, window, cx)
        });
    } else if action.as_any().is::<ExportPdf>() {
        let _ = with_active_editor(cx, |editor, window, cx| {
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
        request_update_check_on_active_editor(cx);
    } else if action.as_any().is::<ShowAbout>() {
        show_info_dialog_on_active_editor(cx, InfoDialogKind::About);
    } else if action.as_any().is::<InstallCliTool>() {
        install_cli_tool(cx);
        install_menus(cx);
    } else if action.as_any().is::<UninstallCliTool>() {
        uninstall_cli_tool(cx);
        install_menus(cx);
    } else if action.as_any().is::<ToggleExplorer>() {
        let _ = with_active_editor(cx, |editor, window, cx| {
            editor.toggle_explorer_drawer(window, cx);
        });
    } else if action.as_any().is::<CloseExplorerFolder>() {
        let _ = with_active_editor(cx, |editor, _window, cx| {
            editor.close_explorer_folder(cx);
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
        prompt_and_open_files_with_error_window(cx, current_window);
    } else if action.as_any().is::<OpenSettings>() {
        open_settings_window(cx);
    } else if let Some(action) = action.as_any().downcast_ref::<OpenRecentFile>() {
        open_recent_file_with_error_window(cx, PathBuf::from(&action.path), current_window);
    } else if action.as_any().is::<NoRecentFiles>() {
    } else if action.as_any().is::<AddLanguageConfig>() {
        prompt_and_import_language_config_with_error_window(cx, current_window);
    } else if action.as_any().is::<AddThemeConfig>() {
        prompt_and_import_theme_config_with_error_window(cx, current_window);
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
        let _ = target.update(cx, |editor, cx| {
            editor.request_close_current_window(window, cx);
        });
    } else if action.as_any().is::<CheckForUpdates>() {
        let _ = target.update(cx, |editor, cx| {
            editor.request_check_updates(window, cx);
        });
    } else if action.as_any().is::<ShowAbout>() {
        let _ = target.update(cx, |editor, cx| {
            editor.show_info_dialog(InfoDialogKind::About, cx)
        });
    } else if action.as_any().is::<InstallCliTool>() {
        install_cli_tool(cx);
        install_menus(cx);
    } else if action.as_any().is::<UninstallCliTool>() {
        uninstall_cli_tool(cx);
        install_menus(cx);
    } else if action.as_any().is::<ToggleExplorer>() {
        let _ = target.update(cx, |editor, cx| {
            editor.toggle_explorer_drawer(window, cx);
        });
    } else if action.as_any().is::<CloseExplorerFolder>() {
        let _ = target.update(cx, |editor, cx| {
            editor.close_explorer_folder(cx);
        });
    }
}

fn build_menus(
    theme_manager: &ThemeManager,
    i18n_manager: &I18nManager,
    recent_files: &[PathBuf],
) -> Vec<Menu> {
    let strings = i18n_manager.strings().clone();
    let mut theme_items = theme_manager
        .available_themes()
        .iter()
        .map(|entry| {
            MenuItem::action(
                entry.name.to_string(),
                SelectTheme {
                    theme_id: entry.id.to_string(),
                },
            )
        })
        .collect::<Vec<_>>();
    theme_items.push(MenuItem::separator());
    theme_items.push(MenuItem::action(
        strings.menu_add_theme_config.clone(),
        AddThemeConfig,
    ));

    let mut language_items = i18n_manager
        .available_languages()
        .iter()
        .map(|entry| {
            MenuItem::action(
                entry.name.to_string(),
                SelectLanguage {
                    language_id: entry.id.to_string(),
                },
            )
        })
        .collect::<Vec<_>>();
    language_items.push(MenuItem::separator());
    language_items.push(MenuItem::action(
        strings.menu_add_language_config.clone(),
        AddLanguageConfig,
    ));

    let recent_items = if recent_files.is_empty() {
        vec![MenuItem::action(
            strings.menu_no_recent_files.clone(),
            NoRecentFiles,
        )]
    } else {
        recent_files
            .iter()
            .map(|path| {
                // into_owned on a Cow<str> reuses the Cow::Owned variant
                // (no copy) when the OS string is valid UTF-8 — the common
                // case — and only allocates for the lossy fallback. The
                // previous .to_string_lossy().to_string() always allocated.
                let label = path.to_string_lossy().into_owned();
                MenuItem::action(label.clone(), OpenRecentFile { path: label })
            })
            .collect()
    };
    #[cfg(target_os = "macos")]
    let help_items = {
        let cli_installed = is_cli_symlink_current_app();
        let mut items = vec![
            MenuItem::action(strings.menu_repository.clone(), OpenSplitypeRepository),
            MenuItem::action(strings.menu_bug_report.clone(), OpenBugReport),
            MenuItem::action(strings.menu_feature_request.clone(), OpenFeatureRequest),
            MenuItem::action(strings.menu_discussions.clone(), OpenDiscussions),
            MenuItem::separator(),
        ];
        if cli_installed {
            items.push(MenuItem::action(
                SharedString::new(strings.menu_uninstall_cli_tool.as_str()),
                UninstallCliTool,
            ));
        } else {
            items.push(MenuItem::action(
                SharedString::new(strings.menu_install_cli_tool.as_str()),
                InstallCliTool,
            ));
        }
        items
    };
    #[cfg(not(target_os = "macos"))]
    let help_items = vec![
        MenuItem::action(strings.menu_repository.clone(), OpenSplitypeRepository),
        MenuItem::action(strings.menu_bug_report.clone(), OpenBugReport),
        MenuItem::action(strings.menu_feature_request.clone(), OpenFeatureRequest),
        MenuItem::action(strings.menu_discussions.clone(), OpenDiscussions),
    ];

    vec![
        Menu {
            name: "Splitype".into(),
            items: vec![
                MenuItem::action(strings.menu_about.clone(), ShowAbout),
                MenuItem::action(strings.menu_check_updates.clone(), CheckForUpdates),
                MenuItem::separator(),
                MenuItem::action(strings.menu_settings.clone(), OpenSettings),
                MenuItem::separator(),
                MenuItem::action(strings.menu_quit.clone(), QuitApplication),
            ],
        },
        Menu {
            name: strings.menu_file.into(),
            items: vec![
                MenuItem::action(strings.menu_new_window.clone(), NewWindow),
                MenuItem::action(strings.menu_close_window.clone(), CloseWindow),
                MenuItem::action(strings.menu_open_file.clone(), OpenFile),
                MenuItem::submenu(Menu {
                    name: strings.menu_open_recent_file.clone().into(),
                    items: recent_items,
                }),
                MenuItem::separator(),
                MenuItem::action(strings.menu_save.clone(), SaveDocument),
                MenuItem::action(strings.menu_save_as.clone(), SaveDocumentAs),
                MenuItem::separator(),
                MenuItem::submenu(Menu {
                    name: strings.menu_export.clone().into(),
                    items: vec![
                        MenuItem::action(strings.menu_export_html.clone(), ExportHtml),
                        MenuItem::action(strings.menu_export_pdf.clone(), ExportPdf),
                    ],
                }),
                MenuItem::separator(),
                MenuItem::action(strings.menu_close_explorer_folder.clone(), CloseExplorerFolder),
            ],
        },
        Menu {
            name: strings.menu_view.into(),
            items: vec![
                MenuItem::submenu(Menu {
                    name: strings.menu_theme.clone().into(),
                    items: theme_items,
                }),
                MenuItem::separator(),
                MenuItem::submenu(Menu {
                    name: strings.menu_language.clone().into(),
                    items: language_items,
                }),
            ],
        },
        Menu {
            name: strings.menu_help.into(),
            items: help_items,
        },
    ]
}

pub(crate) fn install_menus(cx: &mut App) {
    let recent_files = recent_files_for_menu();
    let menus = build_menus(
        cx.global::<ThemeManager>(),
        cx.global::<I18nManager>(),
        &recent_files,
    );
    cx.set_menus(menus);
}

fn prompt_and_open_files(cx: &mut App) {
    let error_window = cx.active_window();
    prompt_and_open_files_with_error_window(cx, error_window);
}

fn prompt_and_open_files_with_error_window(cx: &mut App, error_window: Option<AnyWindowHandle>) {
    let prompt_title = cx
        .global::<I18nManager>()
        .strings()
        .open_markdown_files_prompt
        .clone();
    let prompt = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: true,
        prompt: Some(prompt_title.into()),
    });

    cx.spawn(async move |cx| match prompt.await {
        Ok(Ok(Some(paths))) => {
            let _ = cx.update(move |cx| {
                for path in paths {
                    open_file_in_editor_or_new_window(cx, &path);
                }
            });
        }
        Ok(Err(err)) => {
            let detail = err.to_string();
            let _ = cx.update(move |cx| {
                let title = cx
                    .global::<I18nManager>()
                    .strings()
                    .open_failed_title
                    .clone();
                show_window_prompt(error_window, &title, &detail, cx);
            });
        }
        Ok(Ok(None)) | Err(_) => {}
    })
    .detach();
}

/// Opens `path` in the active editor's tab list when an editor window is
/// focused; otherwise opens a brand-new editor window. Records the
/// recent-file entry either way.
fn open_file_in_editor_or_new_window(cx: &mut App, path: &Path) {
    let opened_in_editor = with_active_editor(cx, |editor, window, cx| {
        editor.open_file_in_active_editor(path, window, cx)
    })
    .is_some_and(|opened| opened);
    if !opened_in_editor {
        if let Err(err) = open_file_in_new_window(cx, path) {
            let title = cx
                .global::<I18nManager>()
                .strings()
                .open_failed_title
                .clone();
            show_window_prompt(cx.active_window(), &title, &err.to_string(), cx);
        }
    }
    record_recent_file_and_refresh(path, cx);
}

fn prompt_and_import_language_config(cx: &mut App) {
    let error_window = cx.active_window();
    prompt_and_import_language_config_with_error_window(cx, error_window);
}

fn prompt_and_import_language_config_with_error_window(
    cx: &mut App,
    error_window: Option<AnyWindowHandle>,
) {
    let prompt_title = cx
        .global::<I18nManager>()
        .strings()
        .add_language_config_prompt
        .clone();
    let prompt = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: false,
        prompt: Some(prompt_title.into()),
    });

    cx.spawn(async move |cx| match prompt.await {
        Ok(Ok(Some(paths))) => {
            let Some(path) = paths.into_iter().next() else {
                return;
            };
            let _ = cx.update(move |cx| {
                let result = import_language_config_and_select(cx, &path);
                match result {
                    Ok(_) => {
                        install_menus(cx);
                        cx.refresh_windows();
                    }
                    Err(err) => {
                        let title = cx
                            .global::<I18nManager>()
                            .strings()
                            .config_import_failed_title
                            .clone();
                        show_window_prompt(error_window, &title, &err.to_string(), cx);
                    }
                }
            });
        }
        Ok(Err(err)) => {
            let detail = err.to_string();
            let _ = cx.update(move |cx| {
                let title = cx
                    .global::<I18nManager>()
                    .strings()
                    .config_import_failed_title
                    .clone();
                show_window_prompt(error_window, &title, &detail, cx);
            });
        }
        Ok(Ok(None)) | Err(_) => {}
    })
    .detach();
}

fn prompt_and_import_theme_config(cx: &mut App) {
    let error_window = cx.active_window();
    prompt_and_import_theme_config_with_error_window(cx, error_window);
}

fn prompt_and_import_theme_config_with_error_window(
    cx: &mut App,
    error_window: Option<AnyWindowHandle>,
) {
    let prompt_title = cx
        .global::<I18nManager>()
        .strings()
        .add_theme_config_prompt
        .clone();
    let prompt = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: false,
        prompt: Some(prompt_title.into()),
    });

    cx.spawn(async move |cx| match prompt.await {
        Ok(Ok(Some(paths))) => {
            let Some(path) = paths.into_iter().next() else {
                return;
            };
            let _ = cx.update(move |cx| {
                let result = import_theme_config_and_select(cx, &path);
                match result {
                    Ok(_) => {
                        install_menus(cx);
                        cx.refresh_windows();
                    }
                    Err(err) => {
                        let title = cx
                            .global::<I18nManager>()
                            .strings()
                            .config_import_failed_title
                            .clone();
                        show_window_prompt(error_window, &title, &err.to_string(), cx);
                    }
                }
            });
        }
        Ok(Err(err)) => {
            let detail = err.to_string();
            let _ = cx.update(move |cx| {
                let title = cx
                    .global::<I18nManager>()
                    .strings()
                    .config_import_failed_title
                    .clone();
                show_window_prompt(error_window, &title, &detail, cx);
            });
        }
        Ok(Ok(None)) | Err(_) => {}
    })
    .detach();
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
    use super::build_menus;
    use crate::editor::actions::{
        AddLanguageConfig, AddThemeConfig, CloseWindow, ExportHtml, ExportPdf,
        NewWindow, NoRecentFiles, OpenBugReport, OpenDiscussions, OpenFeatureRequest, OpenFile,
        OpenRecentFile, OpenSettings, OpenSplitypeRepository, QuitApplication, SaveDocument,
        SelectLanguage, SelectTheme,
    };
    use crate::infra::i18n::I18nManager;
    use crate::platform::cli_tool::applescript_string_literal;
    use crate::theme::ThemeManager;
    use gpui::{Action, MenuItem};
    use std::path::PathBuf;

    fn action_name(item: &MenuItem) -> &str {
        match item {
            MenuItem::Action { name, .. } => name.as_ref(),
            _ => panic!("expected action menu item"),
        }
    }

    fn submenu(item: &MenuItem) -> &gpui::Menu {
        match item {
            MenuItem::Submenu(menu) => menu,
            _ => panic!("expected submenu item"),
        }
    }

    #[test]
    fn applescript_string_literal_escapes_special_characters() {
        assert_eq!(
            applescript_string_literal(
                r#"/Applications/splitype "Test".app/Contents/MacOS/splitype"#
            ),
            r#""/Applications/splitype \"Test\".app/Contents/MacOS/splitype""#
        );
        assert_eq!(
            applescript_string_literal(r#"/Applications/O'Brien\splitype.app"#),
            r#""/Applications/O'Brien\\splitype.app""#
        );
    }

    // Menu bar: [Splitype, File, View, Help] on every platform.
    const FILE_IDX: usize = 1;
    const VIEW_IDX: usize = 2;
    const HELP_IDX: usize = 3;
    // Export is a submenu of File; Theme and Language are submenus of View.
    const EXPORT_ITEM_IDX: usize = 8;
    const THEME_SUBMENU_IDX: usize = 0;
    const LANGUAGE_SUBMENU_IDX: usize = 2;

    #[test]
    fn build_menus_uses_english_fallback_by_default() {
        let theme_manager = ThemeManager::default();
        let i18n_manager = I18nManager::default();
        let menus = build_menus(&theme_manager, &i18n_manager, &[]);

        let menu_names = menus
            .iter()
            .map(|menu| menu.name.to_string())
            .collect::<Vec<_>>();
        assert_eq!(menu_names, vec!["Splitype", "File", "View", "Help"]);

        // Splitype menu: About, Check for Updates, separator, Open Settings,
        // separator, Quit Splitype.
        assert_eq!(action_name(&menus[0].items[0]), "About Splitype");
        assert_eq!(action_name(&menus[0].items[1]), "Check for Updates");
        assert!(matches!(menus[0].items[2], MenuItem::Separator));
        assert_eq!(action_name(&menus[0].items[3]), "Open Settings");
        assert!(matches!(menus[0].items[4], MenuItem::Separator));
        assert_eq!(action_name(&menus[0].items[5]), "Quit Splitype");

        // File menu: New Window, Close Window, Open File, Open Recent File,
        // separator, Save, Save As, separator, Export submenu.
        assert_eq!(action_name(&menus[FILE_IDX].items[0]), "New Window");
        assert_eq!(action_name(&menus[FILE_IDX].items[1]), "Close Window");
        assert_eq!(action_name(&menus[FILE_IDX].items[2]), "Open File");
        assert_eq!(
            submenu(&menus[FILE_IDX].items[3]).name.to_string(),
            "Open Recent File"
        );
        assert_eq!(
            submenu(&menus[FILE_IDX].items[EXPORT_ITEM_IDX])
                .name
                .to_string(),
            "Export"
        );

        // Export submenu: HTML and PDF.
        let export_items = &submenu(&menus[FILE_IDX].items[EXPORT_ITEM_IDX]).items;
        assert_eq!(action_name(&export_items[0]), "HTML");
        assert_eq!(action_name(&export_items[1]), "PDF");

        // View menu: Theme submenu, separator, Language submenu.
        assert_eq!(
            submenu(&menus[VIEW_IDX].items[THEME_SUBMENU_IDX])
                .name
                .to_string(),
            "Theme"
        );
        let language_items = &submenu(&menus[VIEW_IDX].items[LANGUAGE_SUBMENU_IDX]).items;
        assert_eq!(action_name(&language_items[0]), "简体中文");
        assert_eq!(action_name(&language_items[1]), "English");
    }

    #[test]
    fn build_menus_uses_chinese_language_when_selected() {
        let theme_manager = ThemeManager::default();
        let i18n_manager = I18nManager::new_with_language_id("zh-CN");
        let menus = build_menus(&theme_manager, &i18n_manager, &[]);

        assert_eq!(
            submenu(&menus[FILE_IDX].items[3]).name.to_string(),
            i18n_manager.strings().menu_open_recent_file.as_str()
        );

        let menu_names = menus
            .iter()
            .map(|menu| menu.name.to_string())
            .collect::<Vec<_>>();
        assert_eq!(menu_names, vec!["Splitype", "文件", "视图", "帮助"]);

        assert_eq!(action_name(&menus[FILE_IDX].items[0]), "新建窗口");
        assert_eq!(action_name(&menus[0].items[0]), "关于 Splitype");
        assert_eq!(action_name(&menus[0].items[3]), "打开设置");
        assert_eq!(action_name(&menus[0].items[5]), "退出 Splitype");
        let export_items = &submenu(&menus[FILE_IDX].items[EXPORT_ITEM_IDX]).items;
        assert_eq!(action_name(&export_items[0]), "HTML");
        assert_eq!(action_name(&export_items[1]), "PDF");
        let language_items = &submenu(&menus[VIEW_IDX].items[LANGUAGE_SUBMENU_IDX]).items;
        assert_eq!(action_name(&language_items[0]), "简体中文");
    }

    #[test]
    fn export_menu_items_dispatch_export_actions() {
        let theme_manager = ThemeManager::default();
        let i18n_manager = I18nManager::default();
        let menus = build_menus(&theme_manager, &i18n_manager, &[]);
        let export_items = &submenu(&menus[FILE_IDX].items[EXPORT_ITEM_IDX]).items;

        match &export_items[0] {
            MenuItem::Action { action, .. } => {
                assert!(action.as_any().is::<ExportHtml>());
            }
            _ => panic!("expected export html action item"),
        }

        match &export_items[1] {
            MenuItem::Action { action, .. } => {
                assert!(action.as_any().is::<ExportPdf>());
            }
            _ => panic!("expected export pdf action item"),
        }
    }

    #[test]
    fn language_menu_items_dispatch_select_language_actions() {
        let theme_manager = ThemeManager::default();
        let i18n_manager = I18nManager::default();
        let menus = build_menus(&theme_manager, &i18n_manager, &[]);
        let language_items = &submenu(&menus[VIEW_IDX].items[LANGUAGE_SUBMENU_IDX]).items;

        match &language_items[0] {
            MenuItem::Action { action, .. } => {
                let action = action
                    .as_any()
                    .downcast_ref::<SelectLanguage>()
                    .expect("language item should dispatch SelectLanguage");
                assert_eq!(action.language_id, "zh-CN");
            }
            _ => panic!("expected language action item"),
        }
    }

    #[test]
    fn recent_files_submenu_uses_empty_state_when_history_is_empty() {
        let theme_manager = ThemeManager::default();
        let i18n_manager = I18nManager::default();
        let menus = build_menus(&theme_manager, &i18n_manager, &[]);

        // File menu is index 1 on every platform; Open Recent is item 3.
        let recent_menu = submenu(&menus[FILE_IDX].items[3]);

        assert_eq!(recent_menu.name.to_string(), "Open Recent File");
        assert_eq!(recent_menu.items.len(), 1);
        assert_eq!(action_name(&recent_menu.items[0]), "No Recent Files");
        match &recent_menu.items[0] {
            MenuItem::Action { action, .. } => {
                assert!(action.as_any().is::<NoRecentFiles>());
            }
            _ => panic!("expected empty recent-file action item"),
        }
    }

    #[test]
    fn recent_files_submenu_dispatches_path_actions() {
        let theme_manager = ThemeManager::default();
        let i18n_manager = I18nManager::default();
        let recent_files = vec![
            PathBuf::from(r"C:\docs\one.md"),
            PathBuf::from(r"D:\notes\two.markdown"),
        ];
        let menus = build_menus(&theme_manager, &i18n_manager, &recent_files);

        let recent_menu = submenu(&menus[FILE_IDX].items[3]);

        assert_eq!(recent_menu.items.len(), 2);
        assert_eq!(action_name(&recent_menu.items[0]), r"C:\docs\one.md");
        match &recent_menu.items[0] {
            MenuItem::Action { action, .. } => {
                let action = action
                    .as_any()
                    .downcast_ref::<OpenRecentFile>()
                    .expect("recent file should dispatch OpenRecentFile");
                assert_eq!(action.path, r"C:\docs\one.md");
            }
            _ => panic!("expected recent-file action item"),
        }
    }

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

    #[test]
    fn config_import_items_are_bottom_menu_actions() {
        let theme_manager = ThemeManager::default();
        let i18n_manager = I18nManager::default();
        let menus = build_menus(&theme_manager, &i18n_manager, &[]);

        let language_items = &submenu(&menus[VIEW_IDX].items[LANGUAGE_SUBMENU_IDX]).items;
        assert!(matches!(
            language_items[language_items.len() - 2],
            MenuItem::Separator
        ));
        assert_eq!(
            action_name(&language_items[language_items.len() - 1]),
            "Add Language Config"
        );
        match &language_items[language_items.len() - 1] {
            MenuItem::Action { action, .. } => {
                assert!(action.as_any().is::<AddLanguageConfig>());
            }
            _ => panic!("expected add language config action item"),
        }

        let theme_items = &submenu(&menus[VIEW_IDX].items[THEME_SUBMENU_IDX]).items;
        assert_eq!(action_name(&theme_items[0]), "Dark");
        assert_eq!(action_name(&theme_items[1]), "Light");
        assert!(matches!(
            theme_items[theme_items.len() - 2],
            MenuItem::Separator
        ));
        assert_eq!(
            action_name(&theme_items[theme_items.len() - 1]),
            "Add Theme Config"
        );
        match &theme_items[0] {
            MenuItem::Action { action, .. } => {
                assert!(action.as_any().is::<SelectTheme>());
            }
            _ => panic!("expected select theme action item"),
        }
        match &theme_items[theme_items.len() - 1] {
            MenuItem::Action { action, .. } => {
                assert!(action.as_any().is::<AddThemeConfig>());
            }
            _ => panic!("expected add theme config action item"),
        }
    }

    #[test]
    fn theme_menu_marks_selected_builtin_light_theme() {
        let mut theme_manager = ThemeManager::default();
        assert!(theme_manager.set_theme_by_id("splitype-light"));
        let i18n_manager = I18nManager::default();
        let menus = build_menus(&theme_manager, &i18n_manager, &[]);
        let theme_items = &submenu(&menus[VIEW_IDX].items[THEME_SUBMENU_IDX]).items;

        assert_eq!(action_name(&theme_items[0]), "Dark");
        assert_eq!(action_name(&theme_items[1]), "Light");
        match &theme_items[1] {
            MenuItem::Action { action, .. } => {
                let action = action
                    .as_any()
                    .downcast_ref::<SelectTheme>()
                    .expect("light theme item should dispatch SelectTheme");
                assert_eq!(action.theme_id, "splitype-light");
            }
            _ => panic!("expected light theme action item"),
        }
    }

    #[test]
    fn help_menu_first_item_opens_repository() {
        let theme_manager = ThemeManager::default();
        let i18n_manager = I18nManager::default();
        let menus = build_menus(&theme_manager, &i18n_manager, &[]);
        let help_items = &menus[HELP_IDX].items;

        assert_eq!(action_name(&help_items[0]), "Splitype Repository");
        match &help_items[0] {
            MenuItem::Action { action, .. } => {
                assert!(action.as_any().is::<OpenSplitypeRepository>());
            }
            _ => panic!("expected repository action item"),
        }
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn help_menu_contains_repository_links_only() {
        let theme_manager = ThemeManager::default();
        let i18n_manager = I18nManager::default();
        let menus = build_menus(&theme_manager, &i18n_manager, &[]);
        let help_items = &menus[HELP_IDX].items;

        assert_eq!(help_items.len(), 4);
        let cases: [(usize, &str, &dyn Action); 4] = [
            (0, "Splitype Repository", &OpenSplitypeRepository),
            (1, "File Bug Report...", &OpenBugReport),
            (2, "Request Feature...", &OpenFeatureRequest),
            (3, "Join the Discussion", &OpenDiscussions),
        ];
        for (index, label, expected_action) in cases {
            assert_eq!(action_name(&help_items[index]), label);
            let action = match &help_items[index] {
                MenuItem::Action { action, .. } => action,
                _ => panic!("expected action menu item"),
            };
            assert_eq!(
                action.as_any().type_id(),
                expected_action.as_any().type_id()
            );
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn help_menu_contains_cli_on_macos() {
        let theme_manager = ThemeManager::default();
        let i18n_manager = I18nManager::default();
        let menus = build_menus(&theme_manager, &i18n_manager, &[]);
        let help_items = &menus[HELP_IDX].items;

        // 4 GitHub links, separator, Install/Uninstall CLI
        assert_eq!(help_items.len(), 6);
        assert!(matches!(help_items[4], MenuItem::Separator));
        match &help_items[5] {
            MenuItem::Action { action, .. } => {
                assert!(
                    action.as_any().is::<InstallCliTool>()
                        || action.as_any().is::<UninstallCliTool>()
                );
            }
            _ => panic!("expected cli tool action item"),
        }
    }
}
