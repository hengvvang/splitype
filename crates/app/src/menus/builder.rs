//! Native menu bar construction — assembles the platform menu tree from the
//! command registry, with dynamic sections (recent files, themes, languages,
//! CLI tool) provided by the composition root.

use gpui::*;
use std::path::PathBuf;

use crate::actions::{
    AddLanguageConfig, AddThemeConfig, NoRecentFiles, OpenRecentFile, SelectLanguage, SelectTheme,
};
#[cfg(target_os = "macos")]
use crate::actions::{InstallCliTool, UninstallCliTool};
use crate::commands::binding_for;
use config::language::I18nManager;
#[cfg(target_os = "macos")]
use splitype_installer::cli_tool::is_cli_symlink_current_app;
use theme::ThemeManager;

/// Static skeleton of the application menu. Entries are command ids from
/// the command registry; `__…__` markers are composition-root dynamic
/// sections. Order is declaration order.
const APP_MENU_ITEMS: &[&str] = &[
    "splitype.core.about",
    "splitype.core.check-updates",
    "__separator__",
    "splitype.core.open-settings",
    "__separator__",
    "splitype.core.quit",
];

const FILE_MENU_ITEMS: &[&str] = &[
    "splitype.core.new-window",
    "splitype.core.close-window",
    "splitype.core.open-file",
    "__recent__",
    "__separator__",
    "splitype.editor.save",
    "splitype.editor.save-as",
    "__separator__",
    "__export__",
    "__separator__",
    "splitype.core.close-explorer-folder",
];

const VIEW_MENU_ITEMS: &[&str] = &["__theme__", "__separator__", "__language__"];

const HELP_MENU_ITEMS: &[&str] = &[
    "splitype.core.repository",
    "splitype.core.bug-report",
    "splitype.core.feature-request",
    "splitype.core.discussions",
    "__cli__",
];

/// Resolves one skeleton entry to its menu items.
///
/// Returns an empty vector for entries that do not apply on this platform.
fn skeleton_items(
    entry: &str,
    strings: &config::language::I18nStrings,
    theme_manager: &ThemeManager,
    i18n_manager: &I18nManager,
    recent_files: &[PathBuf],
) -> Vec<MenuItem> {
    match entry {
        "__separator__" => vec![MenuItem::separator()],
        "__recent__" => vec![MenuItem::submenu(Menu {
            name: strings.menu_open_recent_file.clone().into(),
            items: recent_file_items(strings, recent_files),
            disabled: false,
        })],
        "__export__" => vec![MenuItem::submenu(Menu {
            name: strings.menu_export.clone().into(),
            items: registry_items("file.export", strings),
            disabled: false,
        })],
        "__theme__" => {
            let mut items = theme_manager
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
            items.push(MenuItem::separator());
            items.push(MenuItem::action(
                strings.menu_add_theme_config.clone(),
                AddThemeConfig,
            ));
            vec![MenuItem::submenu(Menu {
                name: strings.menu_theme.clone().into(),
                items,
                disabled: false,
            })]
        }
        "__language__" => {
            let mut items = i18n_manager
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
            items.push(MenuItem::separator());
            items.push(MenuItem::action(
                strings.menu_add_language_config.clone(),
                AddLanguageConfig,
            ));
            vec![MenuItem::submenu(Menu {
                name: strings.menu_language.clone().into(),
                items,
                disabled: false,
            })]
        }
        "__cli__" => cli_tool_item(strings),
        command_id => vec![registry_item(command_id, strings)],
    }
}

/// The menu items contributed by commands located at `menu`, in registry order.
fn registry_items(menu: &str, strings: &config::language::I18nStrings) -> Vec<MenuItem> {
    editor_contracts::CommandRegistry::registered_commands()
        .unwrap_or_default()
        .into_iter()
        .filter(|command| command.menu.as_deref() == Some(menu))
        .map(|command| {
            let binding = binding_for_plugin_command(&command.id)
                .expect("registered commands must have bindings");
            menu_item_for(binding, strings)
        })
        .collect()
}

fn registry_item(command_id: &str, strings: &config::language::I18nStrings) -> MenuItem {
    let binding = binding_for_plugin_command_id(command_id)
        .unwrap_or_else(|| panic!("no binding for command '{command_id}'"));
    menu_item_for(binding, strings)
}

fn menu_item_for(
    binding: crate::commands::CommandBinding,
    strings: &config::language::I18nStrings,
) -> MenuItem {
    MenuItem::Action {
        name: (binding.label)(strings),
        action: (binding.make_action)(),
        os_action: None,
        checked: false,
        disabled: false,
    }
}

/// Splits a full command id into `(plugin, id)` and looks up its binding.
fn binding_for_plugin_command_id(command_id: &str) -> Option<crate::commands::CommandBinding> {
    let (plugin, id) = command_id.rsplit_once('.')?;
    binding_for(plugin, id)
}

fn binding_for_plugin_command(
    command_id: &editor_contracts::CommandId,
) -> Option<crate::commands::CommandBinding> {
    binding_for_plugin_command_id(command_id.as_str())
}

fn recent_file_items(
    strings: &config::language::I18nStrings,
    recent_files: &[PathBuf],
) -> Vec<MenuItem> {
    if recent_files.is_empty() {
        vec![MenuItem::action(
            strings.menu_no_recent_files.clone(),
            NoRecentFiles,
        )]
    } else {
        recent_files
            .iter()
            .map(|path| {
                let label = path.to_string_lossy().into_owned();
                MenuItem::action(label.clone(), OpenRecentFile { path: label })
            })
            .collect()
    }
}

#[cfg(target_os = "macos")]
fn cli_tool_item(strings: &config::language::I18nStrings) -> Vec<MenuItem> {
    let mut items = vec![MenuItem::separator()];
    if is_cli_symlink_current_app() {
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
}

#[cfg(not(target_os = "macos"))]
fn cli_tool_item(_strings: &config::language::I18nStrings) -> Vec<MenuItem> {
    Vec::new()
}

pub(super) fn build_menus(
    theme_manager: &ThemeManager,
    i18n_manager: &I18nManager,
    recent_files: &[PathBuf],
) -> Vec<Menu> {
    let strings = i18n_manager.strings().clone();
    let file_label: SharedString = strings.menu_file.clone().into();
    let view_label: SharedString = strings.menu_view.clone().into();
    let help_label: SharedString = strings.menu_help.clone().into();

    let assemble = |items: &[&str]| -> Vec<MenuItem> {
        items
            .iter()
            .flat_map(|entry| {
                skeleton_items(entry, &strings, theme_manager, i18n_manager, recent_files)
            })
            .collect()
    };

    vec![
        Menu {
            name: "Splitype".into(),
            items: assemble(APP_MENU_ITEMS),
            disabled: false,
        },
        Menu {
            name: file_label,
            items: assemble(FILE_MENU_ITEMS),
            disabled: false,
        },
        Menu {
            name: view_label,
            items: assemble(VIEW_MENU_ITEMS),
            disabled: false,
        },
        Menu {
            name: help_label,
            items: assemble(HELP_MENU_ITEMS),
            disabled: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[core::prelude::v1::test]
    fn every_menu_skeleton_command_has_a_binding() {
        let all_skeletons = [
            APP_MENU_ITEMS,
            FILE_MENU_ITEMS,
            VIEW_MENU_ITEMS,
            HELP_MENU_ITEMS,
        ];
        for items in all_skeletons {
            for item in items {
                if item.starts_with("__") && item.ends_with("__") {
                    continue;
                }
                assert!(
                    binding_for_plugin_command_id(item).is_some(),
                    "menu command '{item}' has no composition-root binding"
                );
            }
        }
    }
}

