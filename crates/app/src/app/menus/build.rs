//! Native menu bar construction — the platform menu tree for the active
//! theme, language pack, and recent-file history.
//!
//! Owns [`build_menus`] only; the action dispatch and lifecycle live in
//! `super::menus`, and the file/import prompts in `super::menu_prompts`.

use std::path::PathBuf;

use gpui::*;

use crate::app::actions::{
    AddLanguageConfig, AddThemeConfig, CheckForUpdates, CloseExplorerFolder, CloseWindow,
    NewWindow, NoRecentFiles, OpenBugReport, OpenDiscussions, OpenFeatureRequest, OpenFile,
    OpenRecentFile, OpenSettings, OpenSplitypeRepository, QuitApplication, SelectLanguage,
    SelectTheme, ShowAbout,
};
#[cfg(target_os = "macos")]
use crate::app::actions::{InstallCliTool, UninstallCliTool};
use editor_scheduler::actions::{ExportHtml, ExportPdf, SaveDocument, SaveDocumentAs};
use i18n::I18nManager;
use theme::ThemeManager;
#[cfg(target_os = "macos")]
use crate::platform::cli_tool::is_cli_symlink_current_app;

pub(super) fn build_menus(
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
            disabled: false,
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
                    disabled: false,
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
                    disabled: false,
                }),
                MenuItem::separator(),
                MenuItem::action(
                    strings.menu_close_explorer_folder.clone(),
                    CloseExplorerFolder,
                ),
            ],
            disabled: false,
        },
        Menu {
            name: strings.menu_view.into(),
            items: vec![
                MenuItem::submenu(Menu {
                    name: strings.menu_theme.clone().into(),
                    items: theme_items,
                    disabled: false,
                }),
                MenuItem::separator(),
                MenuItem::submenu(Menu {
                    name: strings.menu_language.clone().into(),
                    items: language_items,
                    disabled: false,
                }),
            ],
            disabled: false,
        },
        Menu {
            name: strings.menu_help.into(),
            items: help_items,
            disabled: false,
        },
    ]
}

