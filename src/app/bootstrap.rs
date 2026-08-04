//! Application bootstrap: install globals, open the startup window, and
//! route macOS file-open URLs.

#[cfg(target_os = "macos")]
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[cfg(target_os = "macos")]
use futures::{StreamExt, channel::mpsc};
use gpui::*;

use crate::app::cli::Args;
use crate::app::menus::{init as init_app_menu, install_menus};
use crate::app::windows::open_editor_window;
#[cfg(target_os = "macos")]
use crate::app::windows::open_file_in_new_window;
#[cfg(target_os = "macos")]
use crate::platform::file_url::parse_file_url;
use crate::infra::config::settings::{
    EditorSettings, StartupOpenSetting, first_existing_recent_markdown_file,
    load_or_create_app_settings,
};
use crate::infra::i18n::I18nManager;
use crate::infra::net::http_client::install_http_client;
use crate::ui::input::shortcuts::init_with_keybindings as init_editor;
use crate::ui::theme::ThemeManager;

/// On macOS, re-launch the process detached from the terminal.
#[cfg(target_os = "macos")]
fn relaunch_detached() {
    use std::process::Command;

    let args: Vec<String> = std::env::args().collect();
    let exe_path = std::env::current_exe().expect("Failed to get executable path");
    let non_detach_args: Vec<String> = args
        .iter()
        .filter(|arg| *arg != "--detach" && *arg != "-d")
        .cloned()
        .collect();

    Command::new(exe_path)
        .args(&non_detach_args[1..])
        .spawn()
        .expect("Failed to detach process");
}

fn open_startup_window(cx: &mut App, startup_open: StartupOpenSetting) {
    if startup_open == StartupOpenSetting::LastOpenedFile
        && let Some(path) = first_existing_recent_markdown_file()
    {
        match std::fs::read_to_string(&path) {
            Ok(markdown) => {
                open_editor_window(cx, markdown, Some(path));
                return;
            }
            Err(err) => {
                eprintln!(
                    "failed to read last opened file '{}': {err}",
                    path.display()
                );
            }
        }
    }

    open_editor_window(cx, String::new(), None);
}

/// Runs the application with the given CLI arguments.
pub fn run(args: Args) {
    #[cfg(not(target_os = "macos"))]
    let _ = args.detach;

    #[cfg(target_os = "macos")]
    if args.detach {
        relaunch_detached();
        return;
    }

    #[cfg(target_os = "macos")]
    let (open_file_tx, mut open_file_rx) = mpsc::unbounded::<PathBuf>();
    #[cfg(target_os = "macos")]
    let open_file_requested = Arc::new(AtomicBool::new(false));

    let app = Application::new().with_assets(crate::app::assets::VelotypeAssets);

    #[cfg(target_os = "macos")]
    {
        let open_file_requested_for_callback = open_file_requested.clone();
        app.on_open_urls(move |urls| {
            for url in urls {
                let Some(path) = parse_file_url(&url) else {
                    continue;
                };
                open_file_requested_for_callback.store(true, Ordering::SeqCst);
                let _ = open_file_tx.unbounded_send(path);
            }
        });
    }

    app.run(move |cx: &mut App| {
        let settings = load_or_create_app_settings().unwrap_or_else(|err| {
            eprintln!("failed to initialize app settings: {err}");
            Default::default()
        });
        I18nManager::init_with_language_id(cx, &settings.default_language_id);
        ThemeManager::init_with_theme_id(cx, &settings.default_theme_id);
        EditorSettings::init(cx, settings.show_table_headers);
        install_http_client(cx);
        init_editor(cx, &settings.keybindings);
        init_app_menu(cx);

        #[cfg(target_os = "macos")]
        cx.spawn(async move |cx| {
            while let Some(path) = open_file_rx.next().await {
                let _ = cx.update(move |cx| {
                    if let Err(err) = open_file_in_new_window(cx, &path) {
                        eprintln!("failed to open '{}': {err}", path.display());
                    }
                });
            }
        })
        .detach();

        if args.input_paths.is_empty() {
            #[cfg(target_os = "macos")]
            {
                let startup_open = settings.startup_open;
                let open_file_requested = open_file_requested.clone();
                cx.spawn(async move |cx| {
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(150))
                        .await;
                    if !open_file_requested.load(Ordering::SeqCst) {
                        let _ = cx.update(move |cx| open_startup_window(cx, startup_open));
                    }
                })
                .detach();
            }

            #[cfg(not(target_os = "macos"))]
            open_startup_window(cx, settings.startup_open);

            return;
        }

        for path in &args.input_paths {
            let absolute_path = if path.is_absolute() {
                path.clone()
            } else {
                match std::env::current_dir() {
                    Ok(cwd) => cwd.join(path),
                    Err(_) => path.clone(),
                }
            };

            let markdown = match std::fs::read_to_string(&absolute_path) {
                Ok(content) => {
                    if let Err(err) =
                        crate::infra::config::recent::record_recent_file(&absolute_path)
                    {
                        eprintln!("failed to update recent file history: {err}");
                    }
                    content
                }
                Err(err) => {
                    eprintln!(
                        "failed to read '{}': {err}. opened as empty document.",
                        absolute_path.display()
                    );
                    String::new()
                }
            };
            open_editor_window(cx, markdown, Some(absolute_path));
        }
        install_menus(cx);
        cx.refresh_windows();
    });
}
