//! File-open and config-import prompts for app-menu actions.
//!
//! All functions here prompt for a path (open files, add theme/language
//! config) or resolve a recent-file entry, then route into the active
//! editor or a fresh window. The dispatch logic lives in `super::menus`;
//! this module owns only the prompt flows and their error windows.

use std::path::{Path, PathBuf};

use gpui::*;

use super::{install_menus, show_window_prompt, with_active_window};
use crate::app::window::{open_file_in_new_window, record_recent_file_and_refresh};
use crate::infra::config::recent::{read_recent_files, remove_recent_file};
use crate::infra::config::settings::{
    import_language_config_and_select, import_theme_config_and_select,
};
use crate::infra::i18n::I18nManager;

pub(super) fn open_recent_file(cx: &mut App, path: PathBuf) {
    let error_window = cx.active_window();
    open_recent_file_with_error_window(cx, path, error_window);
}

pub(super) fn open_recent_file_with_error_window(
    cx: &mut App,
    path: PathBuf,
    error_window: Option<AnyWindowHandle>,
) {
    if !path.is_file() {
        if let Err(err) = remove_recent_file(&path) {
            tracing::warn!(path = %path.display(), error = %err, "failed to remove missing recent file");
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

pub(super) fn prompt_and_open_files(cx: &mut App) {
    let error_window = cx.active_window();
    prompt_and_open_files_with_error_window(cx, error_window);
}

pub(super) fn prompt_and_open_files_with_error_window(
    cx: &mut App,
    error_window: Option<AnyWindowHandle>,
) {
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
            cx.update(move |cx| {
                for path in paths {
                    open_file_in_editor_or_new_window(cx, &path);
                }
            });
        }
        Ok(Err(err)) => {
            let detail = err.to_string();
            cx.update(move |cx| {
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
pub(super) fn open_file_in_editor_or_new_window(cx: &mut App, path: &Path) {
    let opened_in_editor = with_active_window(cx, |editor, window, cx| {
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

pub(super) fn prompt_and_import_language_config(cx: &mut App) {
    let error_window = cx.active_window();
    prompt_and_import_language_config_with_error_window(cx, error_window);
}

pub(super) fn prompt_and_import_language_config_with_error_window(
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
            cx.update(move |cx| {
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
            cx.update(move |cx| {
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

pub(super) fn prompt_and_import_theme_config(cx: &mut App) {
    let error_window = cx.active_window();
    prompt_and_import_theme_config_with_error_window(cx, error_window);
}

pub(super) fn prompt_and_import_theme_config_with_error_window(
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
            cx.update(move |cx| {
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
            cx.update(move |cx| {
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

/// Re-reads the recent-file list for menu construction; callers pass it to
/// [`build_menus`](super::build::build_menus).
pub(super) fn recent_files_for_menu() -> Vec<PathBuf> {
    match read_recent_files() {
        Ok(paths) => paths,
        Err(err) => {
            tracing::warn!(error = %err, "failed to read recent file history");
            Vec::new()
        }
    }
}
