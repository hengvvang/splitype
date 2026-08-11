//! Native menu bar construction — the platform menu tree for the active
//! theme, language pack, and recent-file history.
//!
//! Owns [`build_menus`] only; the action dispatch and lifecycle live in
//! `super::menus`, and the file/import prompts in `super::menu_prompts`.

use std::path::PathBuf;

use gpui::*;

use crate::editor::actions::{
    AddLanguageConfig, AddThemeConfig, CheckForUpdates, CloseExplorerFolder, CloseWindow,
    ExportHtml, ExportPdf, NewWindow, NoRecentFiles, OpenBugReport, OpenDiscussions,
    OpenFeatureRequest, OpenFile, OpenRecentFile, OpenSettings, OpenSplitypeRepository,
    QuitApplication, SaveDocument, SaveDocumentAs, SelectLanguage, SelectTheme, ShowAbout,
};
#[cfg(target_os = "macos")]
use crate::editor::actions::{InstallCliTool, UninstallCliTool};
use crate::infra::i18n::I18nManager;
use crate::infra::theme::ThemeManager;
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
                MenuItem::action(
                    strings.menu_close_explorer_folder.clone(),
                    CloseExplorerFolder,
                ),
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

#[cfg(test)]
mod tests {
    use super::build_menus;
    use crate::editor::actions::{
        AddLanguageConfig, AddThemeConfig, ExportHtml, ExportPdf, NoRecentFiles, OpenBugReport,
        OpenDiscussions, OpenFeatureRequest, OpenRecentFile, OpenSplitypeRepository,
        SelectLanguage, SelectTheme,
    };
    use crate::infra::i18n::I18nManager;
    use crate::infra::theme::ThemeManager;
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
