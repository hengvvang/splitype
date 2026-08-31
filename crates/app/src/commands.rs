//! Command binding table — the composition-root link between manifest-
//! declared command ids and their localized labels plus concrete actions.
//!
//! This module is the ONLY place allowed to map a command id to a concrete
//! action type. Menu assembly and dispatch route through the command
//! registry and this table, so adding a command is one table row plus a
//! manifest declaration.

use gpui::*;

use config::language::I18nStrings;

/// One command's composition-root binding.
pub(crate) struct CommandBinding {
    /// Resolves the localized menu label for this command.
    pub label: fn(&I18nStrings) -> SharedString,
    /// Constructs the action instance carried by the menu item.
    pub make_action: fn() -> Box<dyn Action>,
}

/// Resolves a manifest-declared command to its binding.
pub(crate) fn binding_for(plugin: &str, id: &str) -> Option<CommandBinding> {
    Some(match (plugin, id) {
        ("splitype.core", "about") => CommandBinding {
            label: |s| SharedString::from(s.menu_about.as_str()),
            make_action: || Box::new(crate::actions::ShowAbout),
        },
        ("splitype.core", "check-updates") => CommandBinding {
            label: |s| SharedString::from(s.menu_check_updates.as_str()),
            make_action: || Box::new(crate::actions::CheckForUpdates),
        },
        ("splitype.core", "open-settings") => CommandBinding {
            label: |s| SharedString::from(s.menu_settings.as_str()),
            make_action: || Box::new(crate::actions::OpenSettings),
        },
        ("splitype.core", "quit") => CommandBinding {
            label: |s| SharedString::from(s.menu_quit.as_str()),
            make_action: || Box::new(crate::actions::QuitApplication),
        },
        ("splitype.core", "new-window") => CommandBinding {
            label: |s| SharedString::from(s.menu_new_window.as_str()),
            make_action: || Box::new(crate::actions::NewWindow),
        },
        ("splitype.core", "close-window") => CommandBinding {
            label: |s| SharedString::from(s.menu_close_window.as_str()),
            make_action: || Box::new(crate::actions::CloseWindow),
        },
        ("splitype.core", "open-file") => CommandBinding {
            label: |s| SharedString::from(s.menu_open_file.as_str()),
            make_action: || Box::new(crate::actions::OpenFile),
        },
        ("splitype.core", "close-explorer-folder") => CommandBinding {
            label: |s| SharedString::from(s.menu_close_explorer_folder.as_str()),
            make_action: || Box::new(crate::actions::CloseExplorerFolder),
        },
        // Keybinding-only command: label is only used by shortcut listings.
        ("splitype.core", "toggle-explorer") => CommandBinding {
            label: |_| SharedString::from("Toggle Explorer"),
            make_action: || Box::new(crate::actions::ToggleExplorer),
        },
        ("splitype.core", "repository") => CommandBinding {
            label: |s| SharedString::from(s.menu_repository.as_str()),
            make_action: || Box::new(crate::actions::OpenSplitypeRepository),
        },
        ("splitype.core", "bug-report") => CommandBinding {
            label: |s| SharedString::from(s.menu_bug_report.as_str()),
            make_action: || Box::new(crate::actions::OpenBugReport),
        },
        ("splitype.core", "feature-request") => CommandBinding {
            label: |s| SharedString::from(s.menu_feature_request.as_str()),
            make_action: || Box::new(crate::actions::OpenFeatureRequest),
        },
        ("splitype.core", "discussions") => CommandBinding {
            label: |s| SharedString::from(s.menu_discussions.as_str()),
            make_action: || Box::new(crate::actions::OpenDiscussions),
        },
        ("splitype.core", "install-cli") => CommandBinding {
            label: |s| SharedString::from(s.menu_install_cli_tool.as_str()),
            make_action: || Box::new(crate::actions::InstallCliTool),
        },
        ("splitype.core", "uninstall-cli") => CommandBinding {
            label: |s| SharedString::from(s.menu_uninstall_cli_tool.as_str()),
            make_action: || Box::new(crate::actions::UninstallCliTool),
        },
        ("splitype.editor", "save") => CommandBinding {
            label: |s| SharedString::from(s.menu_save.as_str()),
            make_action: || Box::new(editor::actions::SaveDocument),
        },
        ("splitype.editor", "save-as") => CommandBinding {
            label: |s| SharedString::from(s.menu_save_as.as_str()),
            make_action: || Box::new(editor::actions::SaveDocumentAs),
        },
        ("splitype.editor", "export-html") => CommandBinding {
            label: |s| SharedString::from(s.menu_export_html.as_str()),
            make_action: || Box::new(editor::actions::ExportHtml),
        },
        ("splitype.editor", "export-pdf") => CommandBinding {
            label: |s| SharedString::from(s.menu_export_pdf.as_str()),
            make_action: || Box::new(editor::actions::ExportPdf),
        },
        _ => return None,
    })
}
