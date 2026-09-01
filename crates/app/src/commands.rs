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
        // Keybinding-only commands: labels are only used by shortcut listings.
        ("splitype.core", "copy") => CommandBinding {
            label: |_| SharedString::from("Copy"),
            make_action: || Box::new(platform_contracts::actions::Copy),
        },
        ("splitype.core", "cut") => CommandBinding {
            label: |_| SharedString::from("Cut"),
            make_action: || Box::new(platform_contracts::actions::Cut),
        },
        ("splitype.core", "paste") => CommandBinding {
            label: |_| SharedString::from("Paste"),
            make_action: || Box::new(platform_contracts::actions::Paste),
        },
        ("splitype.core", "undo") => CommandBinding {
            label: |_| SharedString::from("Undo"),
            make_action: || Box::new(editor::actions::Undo),
        },
        ("splitype.core", "redo") => CommandBinding {
            label: |_| SharedString::from("Redo"),
            make_action: || Box::new(editor::actions::Redo),
        },
        ("splitype.core", "toggle-maximize-area") => CommandBinding {
            label: |_| SharedString::from("Toggle Maximize Area"),
            make_action: || Box::new(crate::actions::ToggleMaximizeArea),
        },
        ("splitype.core", "dismiss-transient-ui") => CommandBinding {
            label: |_| SharedString::from("Dismiss Transient UI"),
            make_action: || Box::new(platform_contracts::actions::DismissTransientUi),
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
        // Keybinding-only editor commands.
        ("splitype.editor", "page-up") => CommandBinding {
            label: |_| SharedString::from("Page Up"),
            make_action: || Box::new(editor::actions::PageUp),
        },
        ("splitype.editor", "page-down") => CommandBinding {
            label: |_| SharedString::from("Page Down"),
            make_action: || Box::new(editor::actions::PageDown),
        },
        ("splitype.editor", "jump-to-top") => CommandBinding {
            label: |_| SharedString::from("Jump to Top"),
            make_action: || Box::new(editor::actions::JumpToTop),
        },
        ("splitype.editor", "jump-to-bottom") => CommandBinding {
            label: |_| SharedString::from("Jump to Bottom"),
            make_action: || Box::new(editor::actions::JumpToBottom),
        },
        ("splitype.editor", "toggle-pane-kind") => CommandBinding {
            label: |_| SharedString::from("Toggle Pane Kind"),
            make_action: || Box::new(editor::actions::TogglePaneKind),
        },
        ("splitype.editor", "toggle-maximize-pane") => CommandBinding {
            label: |_| SharedString::from("Toggle Maximize Pane"),
            make_action: || Box::new(editor::actions::ToggleMaximizePane),
        },
        ("splitype.editor", "toggle-search") => CommandBinding {
            label: |_| SharedString::from("Toggle Search"),
            make_action: || Box::new(editor::actions::ToggleSearch),
        },
        ("splitype.editor", "toggle-replace") => CommandBinding {
            label: |_| SharedString::from("Toggle Replace"),
            make_action: || Box::new(editor::actions::ToggleReplace),
        },
        ("splitype.editor", "find-next") => CommandBinding {
            label: |_| SharedString::from("Find Next"),
            make_action: || Box::new(editor::actions::FindNext),
        },
        ("splitype.editor", "find-previous") => CommandBinding {
            label: |_| SharedString::from("Find Previous"),
            make_action: || Box::new(editor::actions::FindPrevious),
        },
        // Keybinding-only block-editing commands owned by the WYSIWYG pane.
        ("splitype.wysiwyg", "newline") => CommandBinding {
            label: |_| SharedString::from("Newline"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::Newline),
        },
        ("splitype.wysiwyg", "delete-backward") => CommandBinding {
            label: |_| SharedString::from("Delete Backward"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::DeleteBackward),
        },
        ("splitype.wysiwyg", "delete") => CommandBinding {
            label: |_| SharedString::from("Delete"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::Delete),
        },
        ("splitype.wysiwyg", "word-delete-backward") => CommandBinding {
            label: |_| SharedString::from("Word Delete Backward"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::WordDeleteBackward),
        },
        ("splitype.wysiwyg", "word-delete-forward") => CommandBinding {
            label: |_| SharedString::from("Word Delete Forward"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::WordDeleteForward),
        },
        ("splitype.wysiwyg", "focus-previous") => CommandBinding {
            label: |_| SharedString::from("Focus Previous"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::FocusPrevious),
        },
        ("splitype.wysiwyg", "focus-next") => CommandBinding {
            label: |_| SharedString::from("Focus Next"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::FocusNext),
        },
        ("splitype.wysiwyg", "move-left") => CommandBinding {
            label: |_| SharedString::from("Move Left"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::MoveLeft),
        },
        ("splitype.wysiwyg", "move-right") => CommandBinding {
            label: |_| SharedString::from("Move Right"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::MoveRight),
        },
        ("splitype.wysiwyg", "word-move-left") => CommandBinding {
            label: |_| SharedString::from("Word Move Left"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::WordMoveLeft),
        },
        ("splitype.wysiwyg", "word-move-right") => CommandBinding {
            label: |_| SharedString::from("Word Move Right"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::WordMoveRight),
        },
        ("splitype.wysiwyg", "home") => CommandBinding {
            label: |_| SharedString::from("Home"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::Home),
        },
        ("splitype.wysiwyg", "end") => CommandBinding {
            label: |_| SharedString::from("End"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::End),
        },
        ("splitype.wysiwyg", "block-up") => CommandBinding {
            label: |_| SharedString::from("Block Up"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::BlockUp),
        },
        ("splitype.wysiwyg", "block-down") => CommandBinding {
            label: |_| SharedString::from("Block Down"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::BlockDown),
        },
        ("splitype.wysiwyg", "select-left") => CommandBinding {
            label: |_| SharedString::from("Select Left"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::SelectLeft),
        },
        ("splitype.wysiwyg", "select-right") => CommandBinding {
            label: |_| SharedString::from("Select Right"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::SelectRight),
        },
        ("splitype.wysiwyg", "word-select-left") => CommandBinding {
            label: |_| SharedString::from("Word Select Left"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::WordSelectLeft),
        },
        ("splitype.wysiwyg", "word-select-right") => CommandBinding {
            label: |_| SharedString::from("Word Select Right"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::WordSelectRight),
        },
        ("splitype.wysiwyg", "select-home") => CommandBinding {
            label: |_| SharedString::from("Select Home"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::SelectHome),
        },
        ("splitype.wysiwyg", "select-end") => CommandBinding {
            label: |_| SharedString::from("Select End"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::SelectEnd),
        },
        ("splitype.wysiwyg", "select-all") => CommandBinding {
            label: |_| SharedString::from("Select All"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::SelectAll),
        },
        ("splitype.wysiwyg", "bold-selection") => CommandBinding {
            label: |_| SharedString::from("Bold Selection"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::BoldSelection),
        },
        ("splitype.wysiwyg", "italic-selection") => CommandBinding {
            label: |_| SharedString::from("Italic Selection"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::ItalicSelection),
        },
        ("splitype.wysiwyg", "underline-selection") => CommandBinding {
            label: |_| SharedString::from("Underline Selection"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::UnderlineSelection),
        },
        ("splitype.wysiwyg", "code-selection") => CommandBinding {
            label: |_| SharedString::from("Code Selection"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::CodeSelection),
        },
        ("splitype.wysiwyg", "strikethrough-selection") => CommandBinding {
            label: |_| SharedString::from("Strikethrough Selection"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::StrikethroughSelection),
        },
        ("splitype.wysiwyg", "indent-block") => CommandBinding {
            label: |_| SharedString::from("Indent Block"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::IndentBlock),
        },
        ("splitype.wysiwyg", "outdent-block") => CommandBinding {
            label: |_| SharedString::from("Outdent Block"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::OutdentBlock),
        },
        ("splitype.wysiwyg", "exit-code-block") => CommandBinding {
            label: |_| SharedString::from("Exit Code Block"),
            make_action: || Box::new(pane_wysiwyg::pane::actions::ExitCodeBlock),
        },
        _ => return None,
    })
}
