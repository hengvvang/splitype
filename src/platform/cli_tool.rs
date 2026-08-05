//! macOS CLI tool installation and uninstallation.
//!
//! Installs a `splitype` symlink into `/usr/local/bin` pointing at the
//! running `.app` bundle via AppleScript with administrator privileges.
//! Non-macOS targets provide stubs that report unavailability.

use gpui::*;

use crate::infra::i18n::I18nManager;

/// Returns `true` only if the symlink exists **and** resolves (directly or via
/// one level of canonicalization) to the currently running executable.
#[cfg(target_os = "macos")]
fn is_cli_symlink_current_app() -> bool {
    let link = std::path::Path::new("/usr/local/bin/splitype");
    let Ok(target) = std::fs::read_link(link) else {
        return false; // does not exist or not a symlink
    };
    let resolved = if target.is_absolute() {
        // Canonicalize the target itself (may fail if dangling).
        std::fs::canonicalize(&target).unwrap_or(target)
    } else {
        // Relative — resolve from symlink's parent directory.
        link.parent()
            .unwrap_or(std::path::Path::new("/"))
            .join(&target)
            .canonicalize()
            .unwrap_or(target)
    };
    match std::env::current_exe() {
        Ok(exe) => resolved == exe,
        Err(_) => false,
    }
}

#[cfg(any(target_os = "macos", test))]
fn applescript_string_literal(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            _ => escaped.push(ch),
        }
    }
    escaped.push('"');
    escaped
}

#[cfg(target_os = "macos")]
pub(crate) fn install_cli_tool(cx: &mut App) {
    use std::process::Command;

    let bin_link = "/usr/local/bin/splitype";
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
    let link_path = applescript_string_literal(bin_link);
    let script = format!(
        r#"set exePath to {exe_path}
set linkPath to {link_path}
do shell script "rm -f " & quoted form of linkPath & linefeed & "ln -s " & quoted form of exePath & space & quoted form of linkPath with administrator privileges"#
    );

    match Command::new("osascript").arg("-e").arg(&script).output() {
        Ok(output) => {
            if output.status.success() {
                let title = "CLI Command Installed";
                let detail = format!(
                    "Successfully installed! You can now use 'splitype' from the terminal:\n\n\
                     \x1b[1msplitype README.md\x1b[0m\n\
                     \x1b[1msplitype file1.md file2.md\x1b[0m\n\n\
                     Location: {bin_link}\n\n\
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
    use std::process::Command;

    let bin_link = "/usr/local/bin/splitype";
    let strings = cx.global::<I18nManager>().strings();

    if !is_cli_symlink_current_app() {
        show_install_cli_error(cx, "CLI command is not installed for this app.");
        return;
    }

    let link_path = applescript_string_literal(bin_link);
    let script = format!(
        r#"set linkPath to {link_path}
do shell script "rm -f " & quoted form of linkPath with administrator privileges"#
    );

    match Command::new("osascript").arg("-e").arg(&script).output() {
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