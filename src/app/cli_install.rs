//! CLI tool installation UI — user prompts and localization around the
//! platform operations in `crate::platform::cli_tool`.
//!
//! Lives in `app` (not `platform`) because it orchestrates windows, prompts,
//! and i18n strings; the platform module stays a pure operation layer.

use gpui::*;

use crate::infra::i18n::I18nManager;
#[cfg(target_os = "macos")]
use crate::platform::cli_tool::applescript_string_literal;
#[cfg(target_os = "macos")]
use crate::platform::cli_tool::{is_cli_symlink_current_app, run_osascript};

#[cfg(target_os = "macos")]
const CLI_BIN_LINK: &str = "/usr/local/bin/splitype";

#[cfg(target_os = "macos")]
pub(crate) fn install_cli_tool(cx: &mut App) {
    let strings = cx.global::<I18nManager>().strings();

    let current_exe = match std::env::current_exe() {
        Ok(path) => path,
        Err(err) => {
            show_install_cli_error(cx, &format!("Failed to get executable path: {err}"));
            return;
        }
    };

    // Only allow from a portable .app bundle (e.g. drag-installed to /Applications)
    if !current_exe
        .to_string_lossy()
        .contains(".app/Contents/MacOS/")
    {
        show_install_cli_error(
            cx,
            "Command-line tool installation requires running from an .app bundle.\n\n\
             If the app was installed via the `.pkg` installer,\n\
             the CLI command is configured automatically.",
        );
        return;
    }

    let exe_path = applescript_string_literal(&current_exe.to_string_lossy());
    let link_path = applescript_string_literal(CLI_BIN_LINK);
    let script = format!(
        r#"set exePath to {exe_path}
set linkPath to {link_path}
do shell script "rm -f " & quoted form of linkPath & linefeed & "ln -s " & quoted form of exePath & space & quoted form of linkPath with administrator privileges"#
    );

    match run_osascript(&script) {
        Ok(output) => {
            if output.status.success() {
                let title = "CLI Command Installed";
                let detail = format!(
                    "Successfully installed! You can now use 'splitype' from the terminal:\n\n\
                     \x1b[1msplitype README.md\x1b[0m\n\
                     \x1b[1msplitype file1.md file2.md\x1b[0m\n\n\
                     Location: {CLI_BIN_LINK}\n\n\
                     Note: If you move or delete splitype.app,\n\
                     the 'splitype' command will stop working\n\
                     automatically (no cleanup needed)."
                );
                if let Some(window) = cx.active_window() {
                    let ok = strings.info_dialog_ok.clone();
                    let _ = window.update(cx, |_view, window, cx| {
                        let _ = window.prompt(
                            PromptLevel::Info,
                            &title,
                            Some(&detail),
                            &[ok.as_str()],
                            cx,
                        );
                    });
                }
            } else {
                // User pressed Cancel on the admin password dialog
                // or the link creation failed for another reason.
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let detail = if stderr.contains("User canceled") || stderr.contains("(-128)") {
                    "Installation cancelled.".to_string()
                } else {
                    format!("Installation failed: {stderr}")
                };
                show_install_cli_error(cx, &detail);
            }
        }
        Err(err) => {
            show_install_cli_error(cx, &format!("Failed to run installer: {err}"));
        }
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn uninstall_cli_tool(cx: &mut App) {
    let strings = cx.global::<I18nManager>().strings();

    if !is_cli_symlink_current_app() {
        show_install_cli_error(cx, "CLI command is not installed for this app.");
        return;
    }

    let link_path = applescript_string_literal(CLI_BIN_LINK);
    let script = format!(
        r#"set linkPath to {link_path}
do shell script "rm -f " & quoted form of linkPath with administrator privileges"#
    );

    match run_osascript(&script) {
        Ok(output) => {
            if output.status.success() {
                let title = "CLI Command Uninstalled";
                let detail = "CLI command has been removed successfully.".to_string();
                if let Some(window) = cx.active_window() {
                    let ok = strings.info_dialog_ok.clone();
                    let _ = window.update(cx, |_view, window, cx| {
                        let _ = window.prompt(
                            PromptLevel::Info,
                            &title,
                            Some(&detail),
                            &[ok.as_str()],
                            cx,
                        );
                    });
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let detail = if stderr.contains("User canceled") || stderr.contains("(-128)") {
                    "Uninstall cancelled.".to_string()
                } else {
                    format!("Uninstall failed: {stderr}")
                };
                show_install_cli_error(cx, &detail);
            }
        }
        Err(err) => {
            show_install_cli_error(cx, &format!("Failed to run uninstaller: {err}"));
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn install_cli_tool(cx: &mut App) {
    show_install_cli_error(
        cx,
        "Command-line tool installation is only available on macOS.",
    );
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn uninstall_cli_tool(cx: &mut App) {
    show_install_cli_error(
        cx,
        "Command-line tool uninstallation is only available on macOS.",
    );
}

fn show_install_cli_error(cx: &mut App, detail: &str) {
    let strings = cx.global::<I18nManager>().strings();
    let title = "Install Command-Line Tool Failed";

    if let Some(window) = cx.active_window() {
        let ok = strings.info_dialog_ok.clone();
        let _ = window.update(cx, |_view, window, cx| {
            let _ = window.prompt(
                PromptLevel::Critical,
                title,
                Some(detail),
                &[ok.as_str()],
                cx,
            );
        });
    } else {
        eprintln!("{title}: {detail}");
    }
}
