//! Background online update check and prompt dialogs.

use futures::FutureExt;
use futures::channel::oneshot;
use gpui::*;

use super::InfoDialogKind;
use crate::shell::Shell;
use config::language::I18nManager;
use splitype_installer::update_checker::{
    self as update_check, UpdateCheckResult, UpdateVersionInfo,
};

impl Shell {
    pub(crate) fn request_check_updates(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_unsaved_dialog_open(cx) {
            return;
        }
        if self.update_check_in_progress {
            self.show_info_dialog(InfoDialogKind::CheckForUpdates, cx);
            return;
        }

        self.update_check_in_progress = true;
        self.show_info_dialog(InfoDialogKind::CheckForUpdates, cx);

        let weak_shell = cx.entity().downgrade();
        let window_handle = window.window_handle();
        let (tx, rx) = oneshot::channel();
        std::thread::spawn(move || {
            let result = update_check::check_latest_version(env!("CARGO_PKG_VERSION"));
            let _ = tx.send(result);
        });

        cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let result = rx
                .map(|result| {
                    result.unwrap_or_else(|_| {
                        Err(update_check::UpdateCheckError::ParseVersion(
                            "update check worker ended before returning a result".to_string(),
                        ))
                    })
                })
                .await;

            let _ = weak_shell.update(cx, |shell, cx| {
                shell.update_check_in_progress = false;
                shell.hide_info_dialog(cx);
            });

            let _ = cx.update_window(
                window_handle,
                move |_view: AnyView, window: &mut Window, cx: &mut App| match result {
                    Ok(UpdateCheckResult::UpdateAvailable(info)) => {
                        show_update_available_prompt(window, cx, &info);
                    }
                    Ok(UpdateCheckResult::UpToDate(info)) => {
                        show_up_to_date_prompt(window, cx, &info);
                    }
                    Err(error) => {
                        show_update_failed_prompt(window, cx, &error.to_string());
                    }
                },
            );
        })
        .detach();
    }
}

fn show_update_available_prompt(window: &mut Window, cx: &mut App, info: &UpdateVersionInfo) {
    let strings = cx.global::<I18nManager>().strings().clone();
    let detail = format_update_message(
        &strings.update_available_message_template,
        &info.current_version,
        &info.latest_version,
    );
    let buttons = [
        strings.update_open_release.as_str(),
        strings.update_later.as_str(),
    ];
    let prompt = window.prompt(
        PromptLevel::Info,
        &strings.update_available_title,
        Some(&detail),
        &buttons,
        cx,
    );
    let window_handle = window.window_handle();
    cx.spawn(async move |cx| {
        let Ok(choice) = prompt.await else {
            return;
        };
        if choice == 0 {
            let _ = cx.update_window(window_handle, |_view: AnyView, _window, cx| {
                cx.open_url(update_check::RELEASES_URL);
            });
        }
    })
    .detach();
}

fn show_up_to_date_prompt(window: &mut Window, cx: &mut App, info: &UpdateVersionInfo) {
    let strings = cx.global::<I18nManager>().strings().clone();
    let detail = format_update_message(
        &strings.update_up_to_date_message_template,
        &info.current_version,
        &info.latest_version,
    );
    let buttons = [strings.info_dialog_ok.as_str()];
    let _ = window.prompt(
        PromptLevel::Info,
        &strings.update_up_to_date_title,
        Some(&detail),
        &buttons,
        cx,
    );
}

fn show_update_failed_prompt(window: &mut Window, cx: &mut App, detail: &str) {
    let strings = cx.global::<I18nManager>().strings().clone();
    let message = strings
        .update_failed_message_template
        .replace("{error}", detail);
    let buttons = [strings.info_dialog_ok.as_str()];
    let _ = window.prompt(
        PromptLevel::Critical,
        &strings.update_failed_title,
        Some(&message),
        &buttons,
        cx,
    );
}

fn format_update_message(template: &str, current_version: &str, latest_version: &str) -> String {
    template
        .replace("{current}", current_version)
        .replace("{latest}", latest_version)
}
