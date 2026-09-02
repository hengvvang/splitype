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

use crate::assets::SplitypeAssets;
use crate::keybindings::init_with_keybindings as init_editor;
use crate::menus::{init as init_app_menu, install_menus};
#[cfg(target_os = "macos")]
use crate::platform::file_url::parse_file_url;
use crate::window::open_editor_window;
#[cfg(target_os = "macos")]
use crate::window::open_file_in_new_window;
use config::language::I18nManager;
use config::settings::{
    SettingsStore, StartupOpenSetting, first_existing_recent_markdown_file,
    load_or_create_app_settings,
};
use splitype_cli::Args;
use theme::ThemeManager;
use wysiwyg::net::install_http_client;

/// On macOS, re-launch the process detached from the terminal.
#[cfg(target_os = "macos")]
fn relaunch_detached() {
    use std::process::Command;

    let args: Vec<String> = std::env::args().collect();
    let exe_path = std::env::current_exe().expect("Failed to get executable path");
    let non_detach_args: Vec<String> = args
        .iter()
        .skip(1)
        .filter(|arg| *arg != "--relaunch-detached")
        .cloned()
        .collect();

    let mut command = Command::new(exe_path);
    command.args(non_detach_args);
    if let Ok(mut child) = command.spawn() {
        std::mem::forget(child);
        std::process::exit(0);
    }
}

fn open_startup_window(cx: &mut App, core: config::settings::CoreSettings) {
    if core.startup.restore_window_state {
        match crate::window_state::load_window_state() {
            Ok(Some(state)) => {
                crate::window::open_restored_window(cx, state);
                return;
            }
            Ok(None) => {}
            Err(err) => {
                tracing::warn!(error = %err, "failed to restore window state");
            }
        }
    }

    let startup_open = core.startup.open;
    if startup_open == StartupOpenSetting::LastOpenedFile
        && let Some(path) = first_existing_recent_markdown_file()
    {
        match std::fs::read_to_string(&path) {
            Ok(markdown) => {
                open_editor_window(cx, markdown, Some(path));
                return;
            }
            Err(err) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %err,
                    "failed to read last opened file"
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

    let app = gpui_platform::application().with_assets(SplitypeAssets);

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
        if let Err(err) = SplitypeAssets::load_fonts(cx) {
            tracing::warn!(error = %err, "failed to load embedded Lexend fonts");
        }

        let settings = load_or_create_app_settings().unwrap_or_else(|err| {
            tracing::warn!(error = %err, "failed to initialize app settings, falling back to default");
            Default::default()
        });
        let core = settings.plugin_settings::<config::settings::CoreSettings>();
        SettingsStore::init(cx, settings.clone());
        I18nManager::init(cx);
        I18nManager::register_settings_sync_hook();
        ThemeManager::init(cx);
        ThemeManager::register_settings_sync_hook();
        theme::TypographyStore::init(cx, core.typography.clone());
        install_http_client(cx);
        crate::plugins::init_plugins();
        crate::plugins::discover_user_plugins();
        crate::plugins::register_plugin_theme_contributions(cx);
        init_editor(cx, &core.keybindings);
        init_app_menu(cx);

        // Prewarm CPU-intensive resources in background thread
        std::thread::Builder::new()
            .name("splitype-prewarm".to_string())
            .spawn(|| {
                syntax_highlighter::highlight::prewarm_code_highlight_registry();
                let _ = theme::TypographyStore::default_font(
                    theme::TypographyScope::Prose,
                );
            })
            .ok();

        #[cfg(target_os = "macos")]
        cx.spawn(async move |cx| {
            while let Some(path) = open_file_rx.next().await {
                let _ = cx.update(move |cx| {
                    if let Err(err) = open_file_in_new_window(cx, &path) {
                        tracing::error!(path = %path.display(), error = %err, "failed to open file");
                    }
                });
            }
        })
        .detach();

        if args.input_paths.is_empty() {
            #[cfg(target_os = "macos")]
            {
                let startup_settings = core.clone();
                let open_file_requested = open_file_requested.clone();
                cx.spawn(async move |cx| {
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(150))
                        .await;
                    if !open_file_requested.load(Ordering::SeqCst) {
                        let _ = cx.update(move |cx| open_startup_window(cx, startup_settings));
                    }
                })
                .detach();
            }

            #[cfg(not(target_os = "macos"))]
            open_startup_window(cx, core.clone());

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
                        config::recent::record_recent_file(&absolute_path)
                    {
                        tracing::warn!(path = %absolute_path.display(), error = %err, "failed to update recent file history");
                    }
                    content
                }
                Err(err) => {
                    tracing::warn!(
                        path = %absolute_path.display(),
                        error = %err,
                        "failed to read file, opened as empty document"
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
