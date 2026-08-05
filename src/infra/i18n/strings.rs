//! All localisable UI strings and their defaults.
//!
//! `I18nStrings` is the complete string table; missing entries in a custom
//! language pack fall back to English defaults.

use serde::{Deserialize, Deserializer, Serialize};

/// All localisable UI strings for the editor.
#[derive(Debug, Clone, Serialize)]
pub struct I18nStrings {
    /// Marker prepended to the window title when the document is dirty.
    pub dirty_title_marker: String,
    /// Title of the unsaved-changes dialog.
    pub unsaved_changes_title: String,
    /// Body message of the unsaved-changes dialog.
    pub unsaved_changes_message: String,
    /// Label for the "save and close" button.
    pub unsaved_changes_save_and_close: String,
    /// Label for the "discard and close" button.
    pub unsaved_changes_discard_and_close: String,
    /// Label for the "keep editing" button.
    pub unsaved_changes_cancel: String,
    /// Title of the dropped-file replacement dialog.
    pub drop_replace_title: String,
    /// Body message of the dropped-file replacement dialog.
    pub drop_replace_message: String,
    /// Label for saving before replacing the current document.
    pub drop_replace_save_and_replace: String,
    /// Label for replacing the current document without saving.
    pub drop_replace_discard_and_replace: String,
    /// Label for cancelling a dropped-file replacement.
    pub drop_replace_cancel: String,
    /// Prompt detail shown when no supported Markdown file was dropped.
    pub drop_no_markdown_file_message: String,
    /// Label for dismissing simple informational dialogs.
    pub info_dialog_ok: String,
    /// Title of the placeholder update-check dialog.
    pub help_check_updates_title: String,
    /// Body text shown while an update check is running.
    pub help_check_updates_message: String,
    /// Title shown when a newer version is available.
    pub update_available_title: String,
    /// Message template for newer-version prompts. Supports `{current}` and `{latest}`.
    pub update_available_message_template: String,
    /// Title shown when the running app is already current.
    pub update_up_to_date_title: String,
    /// Message template for up-to-date prompts. Supports `{current}` and `{latest}`.
    pub update_up_to_date_message_template: String,
    /// Title shown when an update check fails.
    pub update_failed_title: String,
    /// Message template for update-check failures. Supports `{error}`.
    pub update_failed_message_template: String,
    /// Button label for opening the GitHub Releases page.
    pub update_open_release: String,
    /// Button label for dismissing an available-update prompt.
    pub update_later: String,
    /// Title of the About dialog.
    pub help_about_title: String,
    /// Supplemental About dialog text shown below the app name and version.
    pub help_about_message: String,
    /// Label for the project repository link in the About dialog.
    pub help_about_github_label: String,
    /// Star request shown in the About dialog.
    pub help_about_star_message: String,
    /// Top-level File menu label.
    pub menu_file: String,
    /// Top-level View menu label.
    pub menu_view: String,
    /// Top-level Export menu label.
    pub menu_export: String,
    /// Top-level Language menu label.
    pub menu_language: String,
    /// Top-level Theme menu label.
    pub menu_theme: String,
    /// Top-level ExplorerState menu label.
    pub menu_explorer: String,
    /// Top-level Help menu label.
    pub menu_help: String,
    /// Language menu item for importing a custom language pack.
    pub menu_add_language_config: String,
    /// Theme menu item for importing a custom theme pack.
    pub menu_add_theme_config: String,
    /// File menu item for opening a new window.
    pub menu_new_window: String,
    /// File menu item for closing the current window.
    pub menu_close_window: String,
    /// File menu item for opening Markdown files.
    pub menu_open_file: String,
    /// File menu item for opening a recent file submenu.
    pub menu_open_recent_file: String,
    /// File menu item for opening app settings.
    pub menu_settings: String,
    /// Placeholder item shown when no recent files are recorded.
    pub menu_no_recent_files: String,
    /// File menu item for saving the current document.
    pub menu_save: String,
    /// File menu item for saving the current document to a new path.
    pub menu_save_as: String,
    /// File menu item for quitting the app.
    pub menu_quit: String,
    /// Export menu item for writing an HTML document.
    pub menu_export_html: String,
    /// Export menu item for writing a PDF document.
    pub menu_export_pdf: String,
    /// Help menu item for checking updates.
    pub menu_check_updates: String,
    /// Help menu item for showing About information.
    pub menu_about: String,
    /// About panel label for the version line.
    pub about_version_label: String,
    /// About panel label for the website link.
    pub about_website_label: String,
    /// About panel label for the wiki link.
    pub about_wiki_label: String,
    /// Help menu item for installing the CLI tool (symlink to /usr/local/bin).
    pub menu_install_cli_tool: String,
    /// Help menu item for uninstalling the CLI tool.
    pub menu_uninstall_cli_tool: String,
    /// Help menu item for opening the splitype repository on GitHub.
    pub menu_repository: String,
    /// Help menu item for filing a bug report on GitHub.
    pub menu_bug_report: String,
    /// Help menu item for requesting a feature on GitHub.
    pub menu_feature_request: String,
    /// Help menu item for joining the GitHub discussions.
    pub menu_discussions: String,
    /// ExplorerState menu item for opening or closing the explorer drawer.
    pub menu_toggle_explorer: String,
    /// Native file-dialog prompt for opening Markdown files.
    pub open_markdown_files_prompt: String,
    /// Native file-dialog prompt for importing a language pack.
    pub add_language_config_prompt: String,
    /// Native file-dialog prompt for importing a theme pack.
    pub add_theme_config_prompt: String,
    /// Title of the open-file failure prompt.
    pub open_failed_title: String,
    /// Title shown when a recent file path no longer exists.
    pub recent_file_missing_title: String,
    /// Message template for missing recent files. Supports `{path}`.
    pub recent_file_missing_message_template: String,
    /// Title of the save failure prompt.
    pub save_failed_title: String,
    /// Title of the export failure prompt.
    pub export_failed_title: String,
    /// Title of the image-paste failure prompt.
    pub image_paste_failed_title: String,
    /// Title of the custom configuration import failure prompt.
    pub config_import_failed_title: String,
    /// settings window title.
    pub settings_window_title: String,
    /// File settings navigation label.
    pub settings_nav_file: String,
    /// Theme settings navigation label.
    pub settings_nav_theme: String,
    /// Image settings navigation label.
    pub settings_nav_image: String,
    /// Shortcut settings navigation label.
    pub settings_nav_shortcuts: String,
    /// Startup option field label.
    pub settings_startup_option: String,
    /// Startup option for creating a new Markdown document.
    pub settings_startup_new_file: String,
    /// Startup option for opening the last opened Markdown document.
    pub settings_startup_last_opened_file: String,
    /// Theme preference field label.
    pub settings_local_theme: String,
    /// Image paste behavior field label.
    pub settings_image_insert_behavior: String,
    pub settings_image_paste_none: String,
    pub settings_image_paste_copy_to_document_folder: String,
    pub settings_image_paste_copy_to_assets_folder: String,
    pub settings_image_paste_copy_to_named_assets_folder: String,
    /// Save button label in the settings window.
    pub settings_save: String,
    /// Cancel button label in the settings window.
    pub settings_cancel: String,
    /// Title shown when settings cannot be saved.
    pub settings_save_failed_title: String,
    pub settings_shortcuts_group_file: String,
    pub settings_shortcuts_group_edit: String,
    pub settings_shortcuts_group_navigation: String,
    pub settings_shortcuts_group_formatting: String,
    pub settings_shortcuts_group_block: String,
    pub settings_shortcuts_group_other: String,
    pub settings_shortcut_record: String,
    pub settings_shortcut_reset: String,
    pub settings_shortcut_recording: String,
    pub settings_shortcut_conflict_template: String,
    pub settings_shortcut_invalid_template: String,
    pub settings_shortcut_newline: String,
    pub settings_shortcut_delete_back: String,
    pub settings_shortcut_delete: String,
    pub settings_shortcut_word_delete_back: String,
    pub settings_shortcut_word_delete_forward: String,
    pub settings_shortcut_focus_prev: String,
    pub settings_shortcut_focus_next: String,
    pub settings_shortcut_move_left: String,
    pub settings_shortcut_move_right: String,
    pub settings_shortcut_word_move_left: String,
    pub settings_shortcut_word_move_right: String,
    pub settings_shortcut_home: String,
    pub settings_shortcut_end: String,
    pub settings_shortcut_block_up: String,
    pub settings_shortcut_block_down: String,
    pub settings_shortcut_page_up: String,
    pub settings_shortcut_page_down: String,
    pub settings_shortcut_jump_to_top: String,
    pub settings_shortcut_jump_to_bottom: String,
    pub settings_shortcut_select_left: String,
    pub settings_shortcut_select_right: String,
    pub settings_shortcut_word_select_left: String,
    pub settings_shortcut_word_select_right: String,
    pub settings_shortcut_select_home: String,
    pub settings_shortcut_select_end: String,
    pub settings_shortcut_select_all: String,
    pub settings_shortcut_copy: String,
    pub settings_shortcut_cut: String,
    pub settings_shortcut_paste: String,
    pub settings_shortcut_undo: String,
    pub settings_shortcut_redo: String,
    pub settings_shortcut_bold_selection: String,
    pub settings_shortcut_italic_selection: String,
    pub settings_shortcut_underline_selection: String,
    pub settings_shortcut_code_selection: String,
    pub settings_shortcut_indent_block: String,
    pub settings_shortcut_outdent_block: String,
    pub settings_shortcut_exit_code_block: String,
    pub settings_shortcut_save_document: String,
    pub settings_shortcut_save_document_as: String,
    pub settings_shortcut_new_window: String,
    pub settings_shortcut_open_file: String,
    pub settings_shortcut_quit_application: String,
    pub settings_shortcut_close_window: String,
    pub settings_shortcut_dismiss_transient_ui: String,
    pub settings_shortcut_toggle_view_mode: String,
    pub settings_shortcut_toggle_explorer: String,
    /// Explorer drawer Files tab.
    pub explorer_tab_files: String,
    /// Explorer drawer Outline tab.
    pub explorer_tab_outline: String,
    /// Title shown when no Markdown file path is available for explorer mode.
    pub explorer_no_file_title: String,
    /// Message shown when no Markdown file path is available for explorer mode.
    pub explorer_no_file_message: String,
    /// Message shown when a explorer directory has no visible Markdown files.
    pub explorer_empty_files: String,
    /// Message shown when the current document has no headings.
    pub explorer_empty_outline: String,
    /// Title shown when the explorer file tree cannot be scanned.
    pub explorer_scan_failed_title: String,
    /// Title of the link-opening confirmation prompt.
    pub open_link_title: String,
    /// Confirm button for the link-opening prompt.
    pub open_link_open: String,
    /// Cancel button for the link-opening prompt.
    pub open_link_cancel: String,
    /// Compact label shown when rendered mode can switch to source mode.
    pub view_mode_source: String,
    /// Hover label shown when rendered mode can switch to source mode.
    pub view_mode_switch_to_source: String,
    /// Compact label shown when source mode can switch to rendered mode.
    pub view_mode_rendered: String,
    /// Hover label shown when source mode can switch to rendered mode.
    pub view_mode_switch_to_rendered: String,
    /// Root context-menu insert label.
    pub context_menu_insert: String,
    /// Insert submenu item for tables.
    pub context_menu_table: String,
    /// Table-axis menu item for left-aligning a column.
    pub table_axis_align_column_left: String,
    /// Table-axis menu item for center-aligning a column.
    pub table_axis_align_column_center: String,
    /// Table-axis menu item for right-aligning a column.
    pub table_axis_align_column_right: String,
    /// Table-axis menu item for moving a column left.
    pub table_axis_move_column_left: String,
    /// Table-axis menu item for moving a column right.
    pub table_axis_move_column_right: String,
    /// Table-axis menu item for deleting a column.
    pub table_axis_delete_column: String,
    /// Table-axis menu item for moving a row up.
    pub table_axis_move_row_up: String,
    /// Table-axis menu item for moving a row down.
    pub table_axis_move_row_down: String,
    /// Table-axis menu item for deleting a row.
    pub table_axis_delete_row: String,
    /// Table header-row menu item that toggles header styling on the top row.
    pub table_header_row: String,
    /// Title of the table-insert dialog.
    pub table_insert_title: String,
    /// Body text of the table-insert dialog.
    pub table_insert_description: String,
    /// Label for table body rows in the table-insert dialog.
    pub table_insert_body_rows: String,
    /// Label for table columns in the table-insert dialog.
    pub table_insert_columns: String,
    /// Cancel button in the table-insert dialog.
    pub table_insert_cancel: String,
    /// Confirm button in the table-insert dialog.
    pub table_insert_confirm: String,
    /// Placeholder label for rendered images without alt text.
    pub image_placeholder: String,
    /// Loading label for rendered images without alt text.
    pub image_loading_without_alt: String,
    /// Loading label template for rendered images with alt text; `{alt}` is replaced.
    pub image_loading_with_alt_template: String,
    /// Placeholder shown in the code-block language button when no language is set.
    pub code_language_placeholder: String,
    /// Placeholder shown in the code-block language picker search field.
    pub code_language_search_placeholder: String,
    /// Label for the sidebar/files toggle button in the status bar.
    pub status_bar_files: String,
    /// Label for source mode in the status bar mode switch.
    pub status_bar_mode_source: String,
    /// Label for rendered mode in the status bar mode switch.
    pub status_bar_mode_rendered: String,
    /// Suffix shown after the word count number.
    pub status_bar_word_count_suffix: String,
    /// Nav label for the status bar settings tab.
    pub settings_nav_status_bar: String,
    /// Label for the status bar enabled toggle.
    pub settings_status_bar_enabled: String,
    /// Label for the word count toggle.
    pub settings_status_bar_show_word_count: String,
    /// Label for the cursor position toggle.
    pub settings_status_bar_show_cursor_position: String,
    /// Label for the sidebar toggle visibility.
    pub settings_status_bar_show_sidebar_toggle: String,
    /// Label for the mode switch visibility.
    pub settings_status_bar_show_mode_switch: String,
}

/// Partial string set used by JSON language packs.
#[derive(Debug, Default, Deserialize)]
pub struct I18nStringsDe {
    dirty_title_marker: Option<String>,
    unsaved_changes_title: Option<String>,
    unsaved_changes_message: Option<String>,
    unsaved_changes_save_and_close: Option<String>,
    unsaved_changes_discard_and_close: Option<String>,
    unsaved_changes_cancel: Option<String>,
    drop_replace_title: Option<String>,
    drop_replace_message: Option<String>,
    drop_replace_save_and_replace: Option<String>,
    drop_replace_discard_and_replace: Option<String>,
    drop_replace_cancel: Option<String>,
    drop_no_markdown_file_message: Option<String>,
    info_dialog_ok: Option<String>,
    help_check_updates_title: Option<String>,
    help_check_updates_message: Option<String>,
    update_available_title: Option<String>,
    update_available_message_template: Option<String>,
    update_up_to_date_title: Option<String>,
    update_up_to_date_message_template: Option<String>,
    update_failed_title: Option<String>,
    update_failed_message_template: Option<String>,
    update_open_release: Option<String>,
    update_later: Option<String>,
    help_about_title: Option<String>,
    help_about_message: Option<String>,
    help_about_github_label: Option<String>,
    help_about_star_message: Option<String>,
    menu_file: Option<String>,
    menu_view: Option<String>,
    menu_export: Option<String>,
    menu_language: Option<String>,
    menu_theme: Option<String>,
    menu_explorer: Option<String>,
    menu_help: Option<String>,
    menu_add_language_config: Option<String>,
    menu_add_theme_config: Option<String>,
    menu_new_window: Option<String>,
    menu_close_window: Option<String>,
    menu_open_file: Option<String>,
    menu_open_recent_file: Option<String>,
    menu_settings: Option<String>,
    menu_no_recent_files: Option<String>,
    menu_save: Option<String>,
    menu_save_as: Option<String>,
    menu_quit: Option<String>,
    menu_export_html: Option<String>,
    menu_export_pdf: Option<String>,
    menu_check_updates: Option<String>,
    menu_about: Option<String>,
    about_version_label: Option<String>,
    about_website_label: Option<String>,
    about_wiki_label: Option<String>,
    menu_install_cli_tool: Option<String>,
    menu_uninstall_cli_tool: Option<String>,
    menu_repository: Option<String>,
    menu_bug_report: Option<String>,
    menu_feature_request: Option<String>,
    menu_discussions: Option<String>,
    menu_toggle_explorer: Option<String>,
    open_markdown_files_prompt: Option<String>,
    add_language_config_prompt: Option<String>,
    add_theme_config_prompt: Option<String>,
    open_failed_title: Option<String>,
    recent_file_missing_title: Option<String>,
    recent_file_missing_message_template: Option<String>,
    save_failed_title: Option<String>,
    export_failed_title: Option<String>,
    image_paste_failed_title: Option<String>,
    config_import_failed_title: Option<String>,
    settings_window_title: Option<String>,
    settings_nav_file: Option<String>,
    settings_nav_theme: Option<String>,
    settings_nav_image: Option<String>,
    settings_nav_shortcuts: Option<String>,
    settings_startup_option: Option<String>,
    settings_startup_new_file: Option<String>,
    settings_startup_last_opened_file: Option<String>,
    settings_local_theme: Option<String>,
    settings_image_insert_behavior: Option<String>,
    settings_image_paste_none: Option<String>,
    settings_image_paste_copy_to_document_folder: Option<String>,
    settings_image_paste_copy_to_assets_folder: Option<String>,
    settings_image_paste_copy_to_named_assets_folder: Option<String>,
    settings_save: Option<String>,
    settings_cancel: Option<String>,
    settings_save_failed_title: Option<String>,
    settings_shortcuts_group_file: Option<String>,
    settings_shortcuts_group_edit: Option<String>,
    settings_shortcuts_group_navigation: Option<String>,
    settings_shortcuts_group_formatting: Option<String>,
    settings_shortcuts_group_block: Option<String>,
    settings_shortcuts_group_other: Option<String>,
    settings_shortcut_record: Option<String>,
    settings_shortcut_reset: Option<String>,
    settings_shortcut_recording: Option<String>,
    settings_shortcut_conflict_template: Option<String>,
    settings_shortcut_invalid_template: Option<String>,
    settings_shortcut_newline: Option<String>,
    settings_shortcut_delete_back: Option<String>,
    settings_shortcut_delete: Option<String>,
    settings_shortcut_word_delete_back: Option<String>,
    settings_shortcut_word_delete_forward: Option<String>,
    settings_shortcut_focus_prev: Option<String>,
    settings_shortcut_focus_next: Option<String>,
    settings_shortcut_move_left: Option<String>,
    settings_shortcut_move_right: Option<String>,
    settings_shortcut_word_move_left: Option<String>,
    settings_shortcut_word_move_right: Option<String>,
    settings_shortcut_home: Option<String>,
    settings_shortcut_end: Option<String>,
    settings_shortcut_block_up: Option<String>,
    settings_shortcut_block_down: Option<String>,
    settings_shortcut_page_up: Option<String>,
    settings_shortcut_page_down: Option<String>,
    settings_shortcut_jump_to_top: Option<String>,
    settings_shortcut_jump_to_bottom: Option<String>,
    settings_shortcut_select_left: Option<String>,
    settings_shortcut_select_right: Option<String>,
    settings_shortcut_word_select_left: Option<String>,
    settings_shortcut_word_select_right: Option<String>,
    settings_shortcut_select_home: Option<String>,
    settings_shortcut_select_end: Option<String>,
    settings_shortcut_select_all: Option<String>,
    settings_shortcut_copy: Option<String>,
    settings_shortcut_cut: Option<String>,
    settings_shortcut_paste: Option<String>,
    settings_shortcut_undo: Option<String>,
    settings_shortcut_redo: Option<String>,
    settings_shortcut_bold_selection: Option<String>,
    settings_shortcut_italic_selection: Option<String>,
    settings_shortcut_underline_selection: Option<String>,
    settings_shortcut_code_selection: Option<String>,
    settings_shortcut_indent_block: Option<String>,
    settings_shortcut_outdent_block: Option<String>,
    settings_shortcut_exit_code_block: Option<String>,
    settings_shortcut_save_document: Option<String>,
    settings_shortcut_save_document_as: Option<String>,
    settings_shortcut_new_window: Option<String>,
    settings_shortcut_open_file: Option<String>,
    settings_shortcut_quit_application: Option<String>,
    settings_shortcut_close_window: Option<String>,
    settings_shortcut_dismiss_transient_ui: Option<String>,
    settings_shortcut_toggle_view_mode: Option<String>,
    settings_shortcut_toggle_explorer: Option<String>,
    explorer_tab_files: Option<String>,
    explorer_tab_outline: Option<String>,
    explorer_no_file_title: Option<String>,
    explorer_no_file_message: Option<String>,
    explorer_empty_files: Option<String>,
    explorer_empty_outline: Option<String>,
    explorer_scan_failed_title: Option<String>,
    open_link_title: Option<String>,
    open_link_open: Option<String>,
    open_link_cancel: Option<String>,
    view_mode_source: Option<String>,
    view_mode_switch_to_source: Option<String>,
    view_mode_rendered: Option<String>,
    view_mode_switch_to_rendered: Option<String>,
    context_menu_insert: Option<String>,
    context_menu_table: Option<String>,
    table_axis_align_column_left: Option<String>,
    table_axis_align_column_center: Option<String>,
    table_axis_align_column_right: Option<String>,
    table_axis_move_column_left: Option<String>,
    table_axis_move_column_right: Option<String>,
    table_axis_delete_column: Option<String>,
    table_axis_move_row_up: Option<String>,
    table_axis_move_row_down: Option<String>,
    table_axis_delete_row: Option<String>,
    table_header_row: Option<String>,
    table_insert_title: Option<String>,
    table_insert_description: Option<String>,
    table_insert_body_rows: Option<String>,
    table_insert_columns: Option<String>,
    table_insert_cancel: Option<String>,
    table_insert_confirm: Option<String>,
    image_placeholder: Option<String>,
    image_loading_without_alt: Option<String>,
    image_loading_with_alt_template: Option<String>,
    code_language_placeholder: Option<String>,
    code_language_search_placeholder: Option<String>,
    status_bar_files: Option<String>,
    status_bar_mode_source: Option<String>,
    status_bar_mode_rendered: Option<String>,
    status_bar_word_count_suffix: Option<String>,
    settings_nav_status_bar: Option<String>,
    settings_status_bar_enabled: Option<String>,
    settings_status_bar_show_word_count: Option<String>,
    settings_status_bar_show_cursor_position: Option<String>,
    settings_status_bar_show_sidebar_toggle: Option<String>,
    settings_status_bar_show_mode_switch: Option<String>,
}

pub(crate) const I18N_STRING_KEYS: &[&str] = &[
    "dirty_title_marker",
    "unsaved_changes_title",
    "unsaved_changes_message",
    "unsaved_changes_save_and_close",
    "unsaved_changes_discard_and_close",
    "unsaved_changes_cancel",
    "drop_replace_title",
    "drop_replace_message",
    "drop_replace_save_and_replace",
    "drop_replace_discard_and_replace",
    "drop_replace_cancel",
    "drop_no_markdown_file_message",
    "info_dialog_ok",
    "help_check_updates_title",
    "help_check_updates_message",
    "update_available_title",
    "update_available_message_template",
    "update_up_to_date_title",
    "update_up_to_date_message_template",
    "update_failed_title",
    "update_failed_message_template",
    "update_open_release",
    "update_later",
    "help_about_title",
    "help_about_message",
    "help_about_github_label",
    "help_about_star_message",
    "menu_file",
    "menu_view",
    "menu_export",
    "menu_language",
    "menu_theme",
    "menu_explorer",
    "menu_help",
    "menu_add_language_config",
    "menu_add_theme_config",
    "menu_new_window",
    "menu_close_window",
    "menu_open_file",
    "menu_open_recent_file",
    "menu_settings",
    "menu_no_recent_files",
    "menu_save",
    "menu_save_as",
    "menu_quit",
    "menu_export_html",
    "menu_export_pdf",
    "menu_check_updates",
    "menu_about",
    "about_version_label",
    "about_website_label",
    "about_wiki_label",
    "menu_install_cli_tool",
    "menu_uninstall_cli_tool",
    "menu_repository",
    "menu_bug_report",
    "menu_feature_request",
    "menu_discussions",
    "menu_toggle_explorer",
    "open_markdown_files_prompt",
    "add_language_config_prompt",
    "add_theme_config_prompt",
    "open_failed_title",
    "recent_file_missing_title",
    "recent_file_missing_message_template",
    "save_failed_title",
    "export_failed_title",
    "image_paste_failed_title",
    "config_import_failed_title",
    "settings_window_title",
    "settings_nav_file",
    "settings_nav_theme",
    "settings_nav_image",
    "settings_nav_shortcuts",
    "settings_startup_option",
    "settings_startup_new_file",
    "settings_startup_last_opened_file",
    "settings_local_theme",
    "settings_image_insert_behavior",
    "settings_image_paste_none",
    "settings_image_paste_copy_to_document_folder",
    "settings_image_paste_copy_to_assets_folder",
    "settings_image_paste_copy_to_named_assets_folder",
    "settings_save",
    "settings_cancel",
    "settings_save_failed_title",
    "settings_shortcuts_group_file",
    "settings_shortcuts_group_edit",
    "settings_shortcuts_group_navigation",
    "settings_shortcuts_group_formatting",
    "settings_shortcuts_group_block",
    "settings_shortcuts_group_other",
    "settings_shortcut_record",
    "settings_shortcut_reset",
    "settings_shortcut_recording",
    "settings_shortcut_conflict_template",
    "settings_shortcut_invalid_template",
    "settings_shortcut_newline",
    "settings_shortcut_delete_back",
    "settings_shortcut_delete",
    "settings_shortcut_word_delete_back",
    "settings_shortcut_word_delete_forward",
    "settings_shortcut_focus_prev",
    "settings_shortcut_focus_next",
    "settings_shortcut_move_left",
    "settings_shortcut_move_right",
    "settings_shortcut_word_move_left",
    "settings_shortcut_word_move_right",
    "settings_shortcut_home",
    "settings_shortcut_end",
    "settings_shortcut_block_up",
    "settings_shortcut_block_down",
    "settings_shortcut_page_up",
    "settings_shortcut_page_down",
    "settings_shortcut_jump_to_top",
    "settings_shortcut_jump_to_bottom",
    "settings_shortcut_select_left",
    "settings_shortcut_select_right",
    "settings_shortcut_word_select_left",
    "settings_shortcut_word_select_right",
    "settings_shortcut_select_home",
    "settings_shortcut_select_end",
    "settings_shortcut_select_all",
    "settings_shortcut_copy",
    "settings_shortcut_cut",
    "settings_shortcut_paste",
    "settings_shortcut_undo",
    "settings_shortcut_redo",
    "settings_shortcut_bold_selection",
    "settings_shortcut_italic_selection",
    "settings_shortcut_underline_selection",
    "settings_shortcut_code_selection",
    "settings_shortcut_indent_block",
    "settings_shortcut_outdent_block",
    "settings_shortcut_exit_code_block",
    "settings_shortcut_save_document",
    "settings_shortcut_save_document_as",
    "settings_shortcut_new_window",
    "settings_shortcut_open_file",
    "settings_shortcut_quit_application",
    "settings_shortcut_close_window",
    "settings_shortcut_dismiss_transient_ui",
    "settings_shortcut_toggle_view_mode",
    "settings_shortcut_toggle_explorer",
    "explorer_tab_files",
    "explorer_tab_outline",
    "explorer_no_file_title",
    "explorer_no_file_message",
    "explorer_empty_files",
    "explorer_empty_outline",
    "explorer_scan_failed_title",
    "open_link_title",
    "open_link_open",
    "open_link_cancel",
    "view_mode_source",
    "view_mode_switch_to_source",
    "view_mode_rendered",
    "view_mode_switch_to_rendered",
    "context_menu_insert",
    "context_menu_table",
    "table_axis_align_column_left",
    "table_axis_align_column_center",
    "table_axis_align_column_right",
    "table_axis_move_column_left",
    "table_axis_move_column_right",
    "table_axis_delete_column",
    "table_axis_move_row_up",
    "table_axis_move_row_down",
    "table_axis_delete_row",
    "table_header_row",
    "table_insert_title",
    "table_insert_description",
    "table_insert_body_rows",
    "table_insert_columns",
    "table_insert_cancel",
    "table_insert_confirm",
    "image_placeholder",
    "image_loading_without_alt",
    "image_loading_with_alt_template",
    "code_language_placeholder",
    "code_language_search_placeholder",
    "status_bar_files",
    "status_bar_mode_source",
    "status_bar_mode_rendered",
    "status_bar_word_count_suffix",
    "settings_nav_status_bar",
    "settings_status_bar_enabled",
    "settings_status_bar_show_word_count",
    "settings_status_bar_show_cursor_position",
    "settings_status_bar_show_sidebar_toggle",
    "settings_status_bar_show_mode_switch",
];

impl I18nStringsDe {
    pub fn into_strings(self, defaults: I18nStrings) -> I18nStrings {
        I18nStrings {
            dirty_title_marker: self
                .dirty_title_marker
                .unwrap_or(defaults.dirty_title_marker),
            unsaved_changes_title: self
                .unsaved_changes_title
                .unwrap_or(defaults.unsaved_changes_title),
            unsaved_changes_message: self
                .unsaved_changes_message
                .unwrap_or(defaults.unsaved_changes_message),
            unsaved_changes_save_and_close: self
                .unsaved_changes_save_and_close
                .unwrap_or(defaults.unsaved_changes_save_and_close),
            unsaved_changes_discard_and_close: self
                .unsaved_changes_discard_and_close
                .unwrap_or(defaults.unsaved_changes_discard_and_close),
            unsaved_changes_cancel: self
                .unsaved_changes_cancel
                .unwrap_or(defaults.unsaved_changes_cancel),
            drop_replace_title: self
                .drop_replace_title
                .unwrap_or(defaults.drop_replace_title),
            drop_replace_message: self
                .drop_replace_message
                .unwrap_or(defaults.drop_replace_message),
            drop_replace_save_and_replace: self
                .drop_replace_save_and_replace
                .unwrap_or(defaults.drop_replace_save_and_replace),
            drop_replace_discard_and_replace: self
                .drop_replace_discard_and_replace
                .unwrap_or(defaults.drop_replace_discard_and_replace),
            drop_replace_cancel: self
                .drop_replace_cancel
                .unwrap_or(defaults.drop_replace_cancel),
            drop_no_markdown_file_message: self
                .drop_no_markdown_file_message
                .unwrap_or(defaults.drop_no_markdown_file_message),
            info_dialog_ok: self.info_dialog_ok.unwrap_or(defaults.info_dialog_ok),
            help_check_updates_title: self
                .help_check_updates_title
                .unwrap_or(defaults.help_check_updates_title),
            help_check_updates_message: self
                .help_check_updates_message
                .unwrap_or(defaults.help_check_updates_message),
            update_available_title: self
                .update_available_title
                .unwrap_or(defaults.update_available_title),
            update_available_message_template: self
                .update_available_message_template
                .unwrap_or(defaults.update_available_message_template),
            update_up_to_date_title: self
                .update_up_to_date_title
                .unwrap_or(defaults.update_up_to_date_title),
            update_up_to_date_message_template: self
                .update_up_to_date_message_template
                .unwrap_or(defaults.update_up_to_date_message_template),
            update_failed_title: self
                .update_failed_title
                .unwrap_or(defaults.update_failed_title),
            update_failed_message_template: self
                .update_failed_message_template
                .unwrap_or(defaults.update_failed_message_template),
            update_open_release: self
                .update_open_release
                .unwrap_or(defaults.update_open_release),
            update_later: self.update_later.unwrap_or(defaults.update_later),
            help_about_title: self.help_about_title.unwrap_or(defaults.help_about_title),
            help_about_message: self
                .help_about_message
                .unwrap_or(defaults.help_about_message),
            help_about_github_label: self
                .help_about_github_label
                .unwrap_or(defaults.help_about_github_label),
            help_about_star_message: self
                .help_about_star_message
                .unwrap_or(defaults.help_about_star_message),
            menu_file: self.menu_file.unwrap_or(defaults.menu_file),
            menu_view: self.menu_view.unwrap_or(defaults.menu_view),
            menu_export: self.menu_export.unwrap_or(defaults.menu_export),
            menu_language: self.menu_language.unwrap_or(defaults.menu_language),
            menu_theme: self.menu_theme.unwrap_or(defaults.menu_theme),
            menu_explorer: self.menu_explorer.unwrap_or(defaults.menu_explorer),
            menu_help: self.menu_help.unwrap_or(defaults.menu_help),
            menu_add_language_config: self
                .menu_add_language_config
                .unwrap_or(defaults.menu_add_language_config),
            menu_add_theme_config: self
                .menu_add_theme_config
                .unwrap_or(defaults.menu_add_theme_config),
            menu_new_window: self.menu_new_window.unwrap_or(defaults.menu_new_window),
            menu_close_window: self.menu_close_window.unwrap_or(defaults.menu_close_window),
            menu_open_file: self.menu_open_file.unwrap_or(defaults.menu_open_file),
            menu_open_recent_file: self
                .menu_open_recent_file
                .unwrap_or(defaults.menu_open_recent_file),
            menu_settings: self.menu_settings.unwrap_or(defaults.menu_settings),
            menu_no_recent_files: self
                .menu_no_recent_files
                .unwrap_or(defaults.menu_no_recent_files),
            menu_save: self.menu_save.unwrap_or(defaults.menu_save),
            menu_save_as: self.menu_save_as.unwrap_or(defaults.menu_save_as),
            menu_quit: self.menu_quit.unwrap_or(defaults.menu_quit),
            menu_export_html: self.menu_export_html.unwrap_or(defaults.menu_export_html),
            menu_export_pdf: self.menu_export_pdf.unwrap_or(defaults.menu_export_pdf),
            menu_check_updates: self
                .menu_check_updates
                .unwrap_or(defaults.menu_check_updates),
            menu_about: self.menu_about.unwrap_or(defaults.menu_about),
            about_version_label: self
                .about_version_label
                .unwrap_or(defaults.about_version_label),
            about_website_label: self
                .about_website_label
                .unwrap_or(defaults.about_website_label),
            about_wiki_label: self.about_wiki_label.unwrap_or(defaults.about_wiki_label),
            menu_install_cli_tool: self
                .menu_install_cli_tool
                .unwrap_or(defaults.menu_install_cli_tool),
            menu_uninstall_cli_tool: self
                .menu_uninstall_cli_tool
                .unwrap_or(defaults.menu_uninstall_cli_tool),
            menu_repository: self.menu_repository.unwrap_or(defaults.menu_repository),
            menu_bug_report: self.menu_bug_report.unwrap_or(defaults.menu_bug_report),
            menu_feature_request: self
                .menu_feature_request
                .unwrap_or(defaults.menu_feature_request),
            menu_discussions: self.menu_discussions.unwrap_or(defaults.menu_discussions),
            menu_toggle_explorer: self
                .menu_toggle_explorer
                .unwrap_or(defaults.menu_toggle_explorer),
            open_markdown_files_prompt: self
                .open_markdown_files_prompt
                .unwrap_or(defaults.open_markdown_files_prompt),
            add_language_config_prompt: self
                .add_language_config_prompt
                .unwrap_or(defaults.add_language_config_prompt),
            add_theme_config_prompt: self
                .add_theme_config_prompt
                .unwrap_or(defaults.add_theme_config_prompt),
            open_failed_title: self.open_failed_title.unwrap_or(defaults.open_failed_title),
            recent_file_missing_title: self
                .recent_file_missing_title
                .unwrap_or(defaults.recent_file_missing_title),
            recent_file_missing_message_template: self
                .recent_file_missing_message_template
                .unwrap_or(defaults.recent_file_missing_message_template),
            save_failed_title: self.save_failed_title.unwrap_or(defaults.save_failed_title),
            export_failed_title: self
                .export_failed_title
                .unwrap_or(defaults.export_failed_title),
            image_paste_failed_title: self
                .image_paste_failed_title
                .unwrap_or(defaults.image_paste_failed_title),
            config_import_failed_title: self
                .config_import_failed_title
                .unwrap_or(defaults.config_import_failed_title),
            settings_window_title: self
                .settings_window_title
                .unwrap_or(defaults.settings_window_title),
            settings_nav_file: self.settings_nav_file.unwrap_or(defaults.settings_nav_file),
            settings_nav_theme: self
                .settings_nav_theme
                .unwrap_or(defaults.settings_nav_theme),
            settings_nav_image: self
                .settings_nav_image
                .unwrap_or(defaults.settings_nav_image),
            settings_nav_shortcuts: self
                .settings_nav_shortcuts
                .unwrap_or(defaults.settings_nav_shortcuts),
            settings_startup_option: self
                .settings_startup_option
                .unwrap_or(defaults.settings_startup_option),
            settings_startup_new_file: self
                .settings_startup_new_file
                .unwrap_or(defaults.settings_startup_new_file),
            settings_startup_last_opened_file: self
                .settings_startup_last_opened_file
                .unwrap_or(defaults.settings_startup_last_opened_file),
            settings_local_theme: self
                .settings_local_theme
                .unwrap_or(defaults.settings_local_theme),
            settings_image_insert_behavior: self
                .settings_image_insert_behavior
                .unwrap_or(defaults.settings_image_insert_behavior),
            settings_image_paste_none: self
                .settings_image_paste_none
                .unwrap_or(defaults.settings_image_paste_none),
            settings_image_paste_copy_to_document_folder: self
                .settings_image_paste_copy_to_document_folder
                .unwrap_or(defaults.settings_image_paste_copy_to_document_folder),
            settings_image_paste_copy_to_assets_folder: self
                .settings_image_paste_copy_to_assets_folder
                .unwrap_or(defaults.settings_image_paste_copy_to_assets_folder),
            settings_image_paste_copy_to_named_assets_folder: self
                .settings_image_paste_copy_to_named_assets_folder
                .unwrap_or(defaults.settings_image_paste_copy_to_named_assets_folder),
            settings_save: self.settings_save.unwrap_or(defaults.settings_save),
            settings_cancel: self.settings_cancel.unwrap_or(defaults.settings_cancel),
            settings_save_failed_title: self
                .settings_save_failed_title
                .unwrap_or(defaults.settings_save_failed_title),
            settings_shortcuts_group_file: self
                .settings_shortcuts_group_file
                .unwrap_or(defaults.settings_shortcuts_group_file),
            settings_shortcuts_group_edit: self
                .settings_shortcuts_group_edit
                .unwrap_or(defaults.settings_shortcuts_group_edit),
            settings_shortcuts_group_navigation: self
                .settings_shortcuts_group_navigation
                .unwrap_or(defaults.settings_shortcuts_group_navigation),
            settings_shortcuts_group_formatting: self
                .settings_shortcuts_group_formatting
                .unwrap_or(defaults.settings_shortcuts_group_formatting),
            settings_shortcuts_group_block: self
                .settings_shortcuts_group_block
                .unwrap_or(defaults.settings_shortcuts_group_block),
            settings_shortcuts_group_other: self
                .settings_shortcuts_group_other
                .unwrap_or(defaults.settings_shortcuts_group_other),
            settings_shortcut_record: self
                .settings_shortcut_record
                .unwrap_or(defaults.settings_shortcut_record),
            settings_shortcut_reset: self
                .settings_shortcut_reset
                .unwrap_or(defaults.settings_shortcut_reset),
            settings_shortcut_recording: self
                .settings_shortcut_recording
                .unwrap_or(defaults.settings_shortcut_recording),
            settings_shortcut_conflict_template: self
                .settings_shortcut_conflict_template
                .unwrap_or(defaults.settings_shortcut_conflict_template),
            settings_shortcut_invalid_template: self
                .settings_shortcut_invalid_template
                .unwrap_or(defaults.settings_shortcut_invalid_template),
            settings_shortcut_newline: self
                .settings_shortcut_newline
                .unwrap_or(defaults.settings_shortcut_newline),
            settings_shortcut_delete_back: self
                .settings_shortcut_delete_back
                .unwrap_or(defaults.settings_shortcut_delete_back),
            settings_shortcut_delete: self
                .settings_shortcut_delete
                .unwrap_or(defaults.settings_shortcut_delete),
            settings_shortcut_word_delete_back: self
                .settings_shortcut_word_delete_back
                .unwrap_or(defaults.settings_shortcut_word_delete_back),
            settings_shortcut_word_delete_forward: self
                .settings_shortcut_word_delete_forward
                .unwrap_or(defaults.settings_shortcut_word_delete_forward),
            settings_shortcut_focus_prev: self
                .settings_shortcut_focus_prev
                .unwrap_or(defaults.settings_shortcut_focus_prev),
            settings_shortcut_focus_next: self
                .settings_shortcut_focus_next
                .unwrap_or(defaults.settings_shortcut_focus_next),
            settings_shortcut_move_left: self
                .settings_shortcut_move_left
                .unwrap_or(defaults.settings_shortcut_move_left),
            settings_shortcut_move_right: self
                .settings_shortcut_move_right
                .unwrap_or(defaults.settings_shortcut_move_right),
            settings_shortcut_word_move_left: self
                .settings_shortcut_word_move_left
                .unwrap_or(defaults.settings_shortcut_word_move_left),
            settings_shortcut_word_move_right: self
                .settings_shortcut_word_move_right
                .unwrap_or(defaults.settings_shortcut_word_move_right),
            settings_shortcut_home: self
                .settings_shortcut_home
                .unwrap_or(defaults.settings_shortcut_home),
            settings_shortcut_end: self
                .settings_shortcut_end
                .unwrap_or(defaults.settings_shortcut_end),
            settings_shortcut_block_up: self
                .settings_shortcut_block_up
                .unwrap_or(defaults.settings_shortcut_block_up),
            settings_shortcut_block_down: self
                .settings_shortcut_block_down
                .unwrap_or(defaults.settings_shortcut_block_down),
            settings_shortcut_page_up: self
                .settings_shortcut_page_up
                .unwrap_or(defaults.settings_shortcut_page_up),
            settings_shortcut_page_down: self
                .settings_shortcut_page_down
                .unwrap_or(defaults.settings_shortcut_page_down),
            settings_shortcut_jump_to_top: self
                .settings_shortcut_jump_to_top
                .unwrap_or(defaults.settings_shortcut_jump_to_top),
            settings_shortcut_jump_to_bottom: self
                .settings_shortcut_jump_to_bottom
                .unwrap_or(defaults.settings_shortcut_jump_to_bottom),
            settings_shortcut_select_left: self
                .settings_shortcut_select_left
                .unwrap_or(defaults.settings_shortcut_select_left),
            settings_shortcut_select_right: self
                .settings_shortcut_select_right
                .unwrap_or(defaults.settings_shortcut_select_right),
            settings_shortcut_word_select_left: self
                .settings_shortcut_word_select_left
                .unwrap_or(defaults.settings_shortcut_word_select_left),
            settings_shortcut_word_select_right: self
                .settings_shortcut_word_select_right
                .unwrap_or(defaults.settings_shortcut_word_select_right),
            settings_shortcut_select_home: self
                .settings_shortcut_select_home
                .unwrap_or(defaults.settings_shortcut_select_home),
            settings_shortcut_select_end: self
                .settings_shortcut_select_end
                .unwrap_or(defaults.settings_shortcut_select_end),
            settings_shortcut_select_all: self
                .settings_shortcut_select_all
                .unwrap_or(defaults.settings_shortcut_select_all),
            settings_shortcut_copy: self
                .settings_shortcut_copy
                .unwrap_or(defaults.settings_shortcut_copy),
            settings_shortcut_cut: self
                .settings_shortcut_cut
                .unwrap_or(defaults.settings_shortcut_cut),
            settings_shortcut_paste: self
                .settings_shortcut_paste
                .unwrap_or(defaults.settings_shortcut_paste),
            settings_shortcut_undo: self
                .settings_shortcut_undo
                .unwrap_or(defaults.settings_shortcut_undo),
            settings_shortcut_redo: self
                .settings_shortcut_redo
                .unwrap_or(defaults.settings_shortcut_redo),
            settings_shortcut_bold_selection: self
                .settings_shortcut_bold_selection
                .unwrap_or(defaults.settings_shortcut_bold_selection),
            settings_shortcut_italic_selection: self
                .settings_shortcut_italic_selection
                .unwrap_or(defaults.settings_shortcut_italic_selection),
            settings_shortcut_underline_selection: self
                .settings_shortcut_underline_selection
                .unwrap_or(defaults.settings_shortcut_underline_selection),
            settings_shortcut_code_selection: self
                .settings_shortcut_code_selection
                .unwrap_or(defaults.settings_shortcut_code_selection),
            settings_shortcut_indent_block: self
                .settings_shortcut_indent_block
                .unwrap_or(defaults.settings_shortcut_indent_block),
            settings_shortcut_outdent_block: self
                .settings_shortcut_outdent_block
                .unwrap_or(defaults.settings_shortcut_outdent_block),
            settings_shortcut_exit_code_block: self
                .settings_shortcut_exit_code_block
                .unwrap_or(defaults.settings_shortcut_exit_code_block),
            settings_shortcut_save_document: self
                .settings_shortcut_save_document
                .unwrap_or(defaults.settings_shortcut_save_document),
            settings_shortcut_save_document_as: self
                .settings_shortcut_save_document_as
                .unwrap_or(defaults.settings_shortcut_save_document_as),
            settings_shortcut_new_window: self
                .settings_shortcut_new_window
                .unwrap_or(defaults.settings_shortcut_new_window),
            settings_shortcut_open_file: self
                .settings_shortcut_open_file
                .unwrap_or(defaults.settings_shortcut_open_file),
            settings_shortcut_quit_application: self
                .settings_shortcut_quit_application
                .unwrap_or(defaults.settings_shortcut_quit_application),
            settings_shortcut_close_window: self
                .settings_shortcut_close_window
                .unwrap_or(defaults.settings_shortcut_close_window),
            settings_shortcut_dismiss_transient_ui: self
                .settings_shortcut_dismiss_transient_ui
                .unwrap_or(defaults.settings_shortcut_dismiss_transient_ui),
            settings_shortcut_toggle_view_mode: self
                .settings_shortcut_toggle_view_mode
                .unwrap_or(defaults.settings_shortcut_toggle_view_mode),
            settings_shortcut_toggle_explorer: self
                .settings_shortcut_toggle_explorer
                .unwrap_or(defaults.settings_shortcut_toggle_explorer),
            explorer_tab_files: self
                .explorer_tab_files
                .unwrap_or(defaults.explorer_tab_files),
            explorer_tab_outline: self
                .explorer_tab_outline
                .unwrap_or(defaults.explorer_tab_outline),
            explorer_no_file_title: self
                .explorer_no_file_title
                .unwrap_or(defaults.explorer_no_file_title),
            explorer_no_file_message: self
                .explorer_no_file_message
                .unwrap_or(defaults.explorer_no_file_message),
            explorer_empty_files: self
                .explorer_empty_files
                .unwrap_or(defaults.explorer_empty_files),
            explorer_empty_outline: self
                .explorer_empty_outline
                .unwrap_or(defaults.explorer_empty_outline),
            explorer_scan_failed_title: self
                .explorer_scan_failed_title
                .unwrap_or(defaults.explorer_scan_failed_title),
            open_link_title: self.open_link_title.unwrap_or(defaults.open_link_title),
            open_link_open: self.open_link_open.unwrap_or(defaults.open_link_open),
            open_link_cancel: self.open_link_cancel.unwrap_or(defaults.open_link_cancel),
            view_mode_source: self.view_mode_source.unwrap_or(defaults.view_mode_source),
            view_mode_switch_to_source: self
                .view_mode_switch_to_source
                .unwrap_or(defaults.view_mode_switch_to_source),
            view_mode_rendered: self
                .view_mode_rendered
                .unwrap_or(defaults.view_mode_rendered),
            view_mode_switch_to_rendered: self
                .view_mode_switch_to_rendered
                .unwrap_or(defaults.view_mode_switch_to_rendered),
            context_menu_insert: self
                .context_menu_insert
                .unwrap_or(defaults.context_menu_insert),
            context_menu_table: self
                .context_menu_table
                .unwrap_or(defaults.context_menu_table),
            table_axis_align_column_left: self
                .table_axis_align_column_left
                .unwrap_or(defaults.table_axis_align_column_left),
            table_axis_align_column_center: self
                .table_axis_align_column_center
                .unwrap_or(defaults.table_axis_align_column_center),
            table_axis_align_column_right: self
                .table_axis_align_column_right
                .unwrap_or(defaults.table_axis_align_column_right),
            table_axis_move_column_left: self
                .table_axis_move_column_left
                .unwrap_or(defaults.table_axis_move_column_left),
            table_axis_move_column_right: self
                .table_axis_move_column_right
                .unwrap_or(defaults.table_axis_move_column_right),
            table_axis_delete_column: self
                .table_axis_delete_column
                .unwrap_or(defaults.table_axis_delete_column),
            table_axis_move_row_up: self
                .table_axis_move_row_up
                .unwrap_or(defaults.table_axis_move_row_up),
            table_axis_move_row_down: self
                .table_axis_move_row_down
                .unwrap_or(defaults.table_axis_move_row_down),
            table_axis_delete_row: self
                .table_axis_delete_row
                .unwrap_or(defaults.table_axis_delete_row),
            table_header_row: self.table_header_row.unwrap_or(defaults.table_header_row),
            table_insert_title: self
                .table_insert_title
                .unwrap_or(defaults.table_insert_title),
            table_insert_description: self
                .table_insert_description
                .unwrap_or(defaults.table_insert_description),
            table_insert_body_rows: self
                .table_insert_body_rows
                .unwrap_or(defaults.table_insert_body_rows),
            table_insert_columns: self
                .table_insert_columns
                .unwrap_or(defaults.table_insert_columns),
            table_insert_cancel: self
                .table_insert_cancel
                .unwrap_or(defaults.table_insert_cancel),
            table_insert_confirm: self
                .table_insert_confirm
                .unwrap_or(defaults.table_insert_confirm),
            image_placeholder: self.image_placeholder.unwrap_or(defaults.image_placeholder),
            image_loading_without_alt: self
                .image_loading_without_alt
                .unwrap_or(defaults.image_loading_without_alt),
            image_loading_with_alt_template: self
                .image_loading_with_alt_template
                .unwrap_or(defaults.image_loading_with_alt_template),
            code_language_placeholder: self
                .code_language_placeholder
                .unwrap_or(defaults.code_language_placeholder),
            code_language_search_placeholder: self
                .code_language_search_placeholder
                .unwrap_or(defaults.code_language_search_placeholder),
            status_bar_files: self.status_bar_files.unwrap_or(defaults.status_bar_files),
            status_bar_mode_source: self
                .status_bar_mode_source
                .unwrap_or(defaults.status_bar_mode_source),
            status_bar_mode_rendered: self
                .status_bar_mode_rendered
                .unwrap_or(defaults.status_bar_mode_rendered),
            status_bar_word_count_suffix: self
                .status_bar_word_count_suffix
                .unwrap_or(defaults.status_bar_word_count_suffix),
            settings_nav_status_bar: self
                .settings_nav_status_bar
                .unwrap_or(defaults.settings_nav_status_bar),
            settings_status_bar_enabled: self
                .settings_status_bar_enabled
                .unwrap_or(defaults.settings_status_bar_enabled),
            settings_status_bar_show_word_count: self
                .settings_status_bar_show_word_count
                .unwrap_or(defaults.settings_status_bar_show_word_count),
            settings_status_bar_show_cursor_position: self
                .settings_status_bar_show_cursor_position
                .unwrap_or(defaults.settings_status_bar_show_cursor_position),
            settings_status_bar_show_sidebar_toggle: self
                .settings_status_bar_show_sidebar_toggle
                .unwrap_or(defaults.settings_status_bar_show_sidebar_toggle),
            settings_status_bar_show_mode_switch: self
                .settings_status_bar_show_mode_switch
                .unwrap_or(defaults.settings_status_bar_show_mode_switch),
        }
    }
}

impl<'de> Deserialize<'de> for I18nStrings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = I18nStringsDe::deserialize(deserializer)?;
        Ok(raw.into_strings(I18nStrings::en_us()))
    }
}

impl I18nStrings {
    /// Built-in Simplified Chinese UI strings.
    pub fn zh_cn() -> Self {
        let mut strings = Self {
            dirty_title_marker: "\u{00B7}".into(),
            unsaved_changes_title: "不保存并关闭？".into(),
            unsaved_changes_message: "此文档有未保存的更改。关闭前保存可避免丢失最新编辑。".into(),
            unsaved_changes_save_and_close: "保存并关闭".into(),
            unsaved_changes_discard_and_close: "放弃并关闭".into(),
            unsaved_changes_cancel: "继续编辑".into(),
            drop_replace_title: "替换当前文档？".into(),
            drop_replace_message: "当前文档有未保存的更改。替换前保存可避免丢失最新编辑。".into(),
            drop_replace_save_and_replace: "保存并替换".into(),
            drop_replace_discard_and_replace: "直接替换".into(),
            drop_replace_cancel: "取消".into(),
            drop_no_markdown_file_message:
                "请拖入 Markdown 文件（.md 或 .markdown）以在当前窗口打开。".into(),
            info_dialog_ok: "确定".into(),
            help_check_updates_title: "检查更新".into(),
            help_check_updates_message: "正在检查 Splitype 的最新版本...".into(),
            update_available_title: "发现新版本".into(),
            update_available_message_template:
                "当前版本：{current}\n最新版本：{latest}\n是否前往 GitHub Releases 下载？".into(),
            update_up_to_date_title: "已是最新版本".into(),
            update_up_to_date_message_template: "当前版本：{current}\n远程版本：{latest}".into(),
            update_failed_title: "检查更新失败".into(),
            update_failed_message_template: "无法完成在线更新检查：{error}".into(),
            update_open_release: "前往下载".into(),
            update_later: "稍后".into(),
            help_about_title: "关于 Splitype".into(),
            help_about_message: "作者：hengvvang".into(),
            help_about_github_label: "GitHub".into(),
            help_about_star_message: "如果本项目对您有帮助，那不妨给本项目一颗 Star⭐，十分感谢！"
                .into(),
            menu_file: "文件".into(),
            menu_view: "视图".into(),
            menu_export: "导出".into(),
            menu_language: "语言".into(),
            menu_theme: "主题".into(),
            menu_explorer: "资源管理器".into(),
            menu_help: "帮助".into(),
            menu_add_language_config: "添加语言配置".into(),
            menu_add_theme_config: "添加主题配置".into(),
            menu_new_window: "新建窗口".into(),
            menu_close_window: "关闭窗口".into(),
            menu_open_file: "打开文件".into(),
            menu_open_recent_file: "打开最近文件".into(),
            menu_settings: "打开设置".into(),
            menu_no_recent_files: "无最近文件".into(),
            menu_save: "保存".into(),
            menu_save_as: "另存为".into(),
            menu_quit: "退出 Splitype".into(),
            menu_export_html: "HTML".into(),
            menu_export_pdf: "PDF".into(),
            menu_check_updates: "检查更新".into(),
            menu_about: "关于 Splitype".into(),
            about_version_label: "版本".into(),
            about_website_label: "网站".into(),
            about_wiki_label: "文档".into(),
            menu_install_cli_tool: "安装CLI命令".into(),
            menu_uninstall_cli_tool: "卸载CLI命令".into(),
            menu_repository: "Splitype 仓库".into(),
            menu_bug_report: "报告 Bug...".into(),
            menu_feature_request: "请求功能...".into(),
            menu_discussions: "加入讨论".into(),
            menu_toggle_explorer: "切换资源管理器".into(),
            open_markdown_files_prompt: "打开 Markdown 文件".into(),
            add_language_config_prompt: "选择语言配置文件".into(),
            add_theme_config_prompt: "选择主题配置文件".into(),
            open_failed_title: "打开失败".into(),
            recent_file_missing_title: "最近文件不存在".into(),
            recent_file_missing_message_template: "此最近文件已经不存在，已从记录中移除：\n{path}"
                .into(),
            save_failed_title: "保存失败".into(),
            export_failed_title: "导出失败".into(),
            config_import_failed_title: "配置导入失败".into(),
            settings_window_title: "设置".into(),
            settings_nav_file: "文件".into(),
            settings_nav_theme: "主题".into(),
            settings_nav_shortcuts: "快捷键".into(),
            settings_startup_option: "启动选项".into(),
            settings_startup_new_file: "新 md 文件".into(),
            settings_startup_last_opened_file: "上一次打开的 md 文件".into(),
            settings_local_theme: "本地主题".into(),
            settings_save: "保存".into(),
            settings_cancel: "取消".into(),
            settings_save_failed_title: "保存偏好设置失败".into(),
            settings_shortcuts_group_file: "文件".into(),
            settings_shortcuts_group_edit: "编辑".into(),
            settings_shortcuts_group_navigation: "移动与选择".into(),
            settings_shortcuts_group_formatting: "格式化".into(),
            settings_shortcuts_group_block: "块操作".into(),
            settings_shortcuts_group_other: "其他".into(),
            settings_shortcut_record: "录制".into(),
            settings_shortcut_reset: "重置".into(),
            settings_shortcut_recording: "按下快捷键...".into(),
            settings_shortcut_conflict_template: "该快捷键已被“{command}”使用".into(),
            settings_shortcut_invalid_template: "无法使用快捷键“{shortcut}”".into(),
            settings_shortcut_newline: "换行".into(),
            settings_shortcut_delete_back: "向前删除".into(),
            settings_shortcut_delete: "向后删除".into(),
            settings_shortcut_word_delete_back: "向前删除单词".into(),
            settings_shortcut_word_delete_forward: "向后删除单词".into(),
            settings_shortcut_focus_prev: "上移".into(),
            settings_shortcut_focus_next: "下移".into(),
            settings_shortcut_move_left: "光标左移".into(),
            settings_shortcut_move_right: "光标右移".into(),
            settings_shortcut_word_move_left: "按词左移".into(),
            settings_shortcut_word_move_right: "按词右移".into(),
            settings_shortcut_home: "行首".into(),
            settings_shortcut_end: "行尾".into(),
            settings_shortcut_block_up: "上一块开头".into(),
            settings_shortcut_block_down: "下一块开头".into(),
            settings_shortcut_page_up: "上翻一页".into(),
            settings_shortcut_page_down: "下翻一页".into(),
            settings_shortcut_jump_to_top: "跳至开头".into(),
            settings_shortcut_jump_to_bottom: "跳至末尾".into(),
            settings_shortcut_select_left: "向左选择".into(),
            settings_shortcut_select_right: "向右选择".into(),
            settings_shortcut_word_select_left: "向左选择单词".into(),
            settings_shortcut_word_select_right: "向右选择单词".into(),
            settings_shortcut_select_home: "选择到行首".into(),
            settings_shortcut_select_end: "选择到行尾".into(),
            settings_shortcut_select_all: "全选".into(),
            settings_shortcut_copy: "复制".into(),
            settings_shortcut_cut: "剪切".into(),
            settings_shortcut_paste: "粘贴".into(),
            settings_shortcut_undo: "撤销".into(),
            settings_shortcut_redo: "重做".into(),
            settings_shortcut_bold_selection: "加粗".into(),
            settings_shortcut_italic_selection: "斜体".into(),
            settings_shortcut_underline_selection: "下划线".into(),
            settings_shortcut_code_selection: "行内代码".into(),
            settings_shortcut_indent_block: "缩进块".into(),
            settings_shortcut_outdent_block: "取消缩进块".into(),
            settings_shortcut_exit_code_block: "退出代码块".into(),
            settings_shortcut_save_document: "保存文档".into(),
            settings_shortcut_save_document_as: "另存为".into(),
            settings_shortcut_new_window: "新建窗口".into(),
            settings_shortcut_open_file: "打开文件".into(),
            settings_shortcut_quit_application: "退出应用".into(),
            settings_shortcut_close_window: "关闭窗口".into(),
            settings_shortcut_dismiss_transient_ui: "关闭临时界面".into(),
            settings_shortcut_toggle_view_mode: "切换视图模式".into(),
            settings_shortcut_toggle_explorer: "切换资源管理器".into(),
            explorer_tab_files: "文件".into(),
            explorer_tab_outline: "大纲".into(),
            explorer_no_file_title: "未打开 Markdown 文件".into(),
            explorer_no_file_message: "打开或保存一个 .md 文件后，工作区会使用该文件所在目录。"
                .into(),
            explorer_empty_files: "没有可显示的 Markdown 文件".into(),
            explorer_empty_outline: "当前文档没有标题".into(),
            explorer_scan_failed_title: "无法读取工作区".into(),
            open_link_title: "打开链接？".into(),
            open_link_open: "打开".into(),
            open_link_cancel: "取消".into(),
            view_mode_source: "源码".into(),
            view_mode_switch_to_source: "切换到源码".into(),
            view_mode_rendered: "渲染".into(),
            view_mode_switch_to_rendered: "切换到渲染".into(),
            context_menu_insert: "插入".into(),
            context_menu_table: "表格".into(),
            table_axis_align_column_left: "左对齐此列".into(),
            table_axis_align_column_center: "居中此列".into(),
            table_axis_align_column_right: "右对齐此列".into(),
            table_axis_move_column_left: "向左移动此列".into(),
            table_axis_move_column_right: "向右移动此列".into(),
            table_axis_delete_column: "删除此列".into(),
            table_axis_move_row_up: "向上移动此行".into(),
            table_axis_move_row_down: "向下移动此行".into(),
            table_axis_delete_row: "删除此行".into(),
            table_header_row: "标题行".into(),
            table_insert_title: "插入表格".into(),
            table_insert_description: "创建 1 个表头行，并配置正文行数与列数。".into(),
            table_insert_body_rows: "正文行数".into(),
            table_insert_columns: "列数".into(),
            table_insert_cancel: "取消".into(),
            table_insert_confirm: "插入".into(),
            image_placeholder: "图片".into(),
            image_loading_without_alt: "正在加载图片...".into(),
            image_loading_with_alt_template: "正在加载 {alt}".into(),
            code_language_placeholder: "语言".into(),
            code_language_search_placeholder: "搜索语言...".into(),
            status_bar_files: "侧边栏".into(),
            status_bar_mode_source: "源码".into(),
            status_bar_mode_rendered: "渲染".into(),
            status_bar_word_count_suffix: "字".into(),
            ..Self::en_us()
        };
        strings.image_paste_failed_title = "图片粘贴失败".into();
        strings.settings_nav_image = "图像".into();
        strings.settings_nav_status_bar = "状态栏".into();
        strings.settings_status_bar_enabled = "显示状态栏".into();
        strings.settings_status_bar_show_word_count = "字数统计".into();
        strings.settings_status_bar_show_cursor_position = "光标位置".into();
        strings.settings_status_bar_show_sidebar_toggle = "侧边栏".into();
        strings.settings_status_bar_show_mode_switch = "模式切换".into();
        strings.settings_image_insert_behavior = "插入图片时...".into();
        strings.settings_image_paste_none = "无特殊操作".into();
        strings.settings_image_paste_copy_to_document_folder = "复制图片到 ./ 文件夹".into();
        strings.settings_image_paste_copy_to_assets_folder = "复制图片到 ./assets 文件夹".into();
        strings.settings_image_paste_copy_to_named_assets_folder =
            "复制图片到 ./${filename}.assets 文件夹".into();
        strings
    }

    /// Built-in English UI strings.
    pub fn en_us() -> Self {
        Self {
            dirty_title_marker: "\u{00B7}".into(),
            unsaved_changes_title: "Close without saving?".into(),
            unsaved_changes_message:
                "This document has unsaved changes. Save before closing to avoid losing your latest edits."
                    .into(),
            unsaved_changes_save_and_close: "Save and Close".into(),
            unsaved_changes_discard_and_close: "Discard and Close".into(),
            unsaved_changes_cancel: "Keep Editing".into(),
            drop_replace_title: "Replace current document?".into(),
            drop_replace_message:
                "This document has unsaved changes. Save before replacing it with the dropped file to avoid losing edits."
                    .into(),
            drop_replace_save_and_replace: "Save and Replace".into(),
            drop_replace_discard_and_replace: "Replace Without Saving".into(),
            drop_replace_cancel: "Cancel".into(),
            drop_no_markdown_file_message:
                "Drop a Markdown file (.md or .markdown) to open it in this window.".into(),
            info_dialog_ok: "OK".into(),
            help_check_updates_title: "Check for Updates".into(),
            help_check_updates_message: "Checking the latest Splitype version...".into(),
            update_available_title: "Update Available".into(),
            update_available_message_template:
                "Current version: {current}\nLatest version: {latest}\nOpen GitHub Releases to download it?"
                    .into(),
            update_up_to_date_title: "You're Up to Date".into(),
            update_up_to_date_message_template:
                "Current version: {current}\nRemote version: {latest}".into(),
            update_failed_title: "Update Check Failed".into(),
            update_failed_message_template: "Unable to complete the online update check: {error}"
                .into(),
            update_open_release: "Open Releases".into(),
            update_later: "Later".into(),
            help_about_title: "About Splitype".into(),
            help_about_message: "Author: hengvvang".into(),
            help_about_github_label: "GitHub".into(),
            help_about_star_message:
                "If this project helps you, consider giving it a Star⭐. Thank you!".into(),
            menu_file: "File".into(),
            menu_view: "View".into(),
            menu_export: "Export".into(),
            menu_language: "Language".into(),
            menu_theme: "Theme".into(),
            menu_explorer: "Explorer".into(),
            menu_help: "Help".into(),
            menu_add_language_config: "Add Language Config".into(),
            menu_add_theme_config: "Add Theme Config".into(),
            menu_new_window: "New Window".into(),
            menu_close_window: "Close Window".into(),
            menu_open_file: "Open File".into(),
            menu_open_recent_file: "Open Recent File".into(),
            menu_settings: "Open Settings".into(),
            menu_no_recent_files: "No Recent Files".into(),
            menu_save: "Save".into(),
            menu_save_as: "Save As".into(),
            menu_quit: "Quit Splitype".into(),
            menu_export_html: "HTML".into(),
            menu_export_pdf: "PDF".into(),
            menu_check_updates: "Check for Updates".into(),
            menu_about: "About Splitype".into(),
            about_version_label: "Version".into(),
            about_website_label: "Website".into(),
            about_wiki_label: "Wiki".into(),
            menu_install_cli_tool: "Install CLI Command".into(),
            menu_uninstall_cli_tool: "Uninstall CLI Command".into(),
            menu_repository: "Splitype Repository".into(),
            menu_bug_report: "File Bug Report...".into(),
            menu_feature_request: "Request Feature...".into(),
            menu_discussions: "Join the Discussion".into(),
            menu_toggle_explorer: "Toggle Explorer".into(),
            open_markdown_files_prompt: "Open Markdown Files".into(),
            add_language_config_prompt: "Choose Language Config".into(),
            add_theme_config_prompt: "Choose Theme Config".into(),
            open_failed_title: "Open Failed".into(),
            recent_file_missing_title: "Recent File Missing".into(),
            recent_file_missing_message_template:
                "This recent file no longer exists and has been removed:\n{path}".into(),
            save_failed_title: "Save Failed".into(),
            export_failed_title: "Export Failed".into(),
            image_paste_failed_title: "Image Paste Failed".into(),
            config_import_failed_title: "Config Import Failed".into(),
            settings_window_title: "Settings".into(),
            settings_nav_file: "File".into(),
            settings_nav_theme: "Theme".into(),
            settings_nav_image: "Image".into(),
            settings_nav_shortcuts: "Shortcuts".into(),
            settings_startup_option: "Startup Option".into(),
            settings_startup_new_file: "New Markdown File".into(),
            settings_startup_last_opened_file: "Last Opened Markdown File".into(),
            settings_local_theme: "Local Theme".into(),
            settings_image_insert_behavior: "When inserting images...".into(),
            settings_image_paste_none: "No special action".into(),
            settings_image_paste_copy_to_document_folder:
                "Copy image to ./ folder".into(),
            settings_image_paste_copy_to_assets_folder:
                "Copy image to ./assets folder".into(),
            settings_image_paste_copy_to_named_assets_folder:
                "Copy image to ./${filename}.assets folder".into(),
            settings_save: "Save".into(),
            settings_cancel: "Cancel".into(),
            settings_save_failed_title: "Save settings Failed".into(),
            settings_shortcuts_group_file: "File".into(),
            settings_shortcuts_group_edit: "Edit".into(),
            settings_shortcuts_group_navigation: "Move and Select".into(),
            settings_shortcuts_group_formatting: "Formatting".into(),
            settings_shortcuts_group_block: "Block Operations".into(),
            settings_shortcuts_group_other: "Other".into(),
            settings_shortcut_record: "Record".into(),
            settings_shortcut_reset: "Reset".into(),
            settings_shortcut_recording: "Press shortcut...".into(),
            settings_shortcut_conflict_template: "This shortcut is already used by {command}"
                .into(),
            settings_shortcut_invalid_template: "Cannot use shortcut {shortcut}".into(),
            settings_shortcut_newline: "Newline".into(),
            settings_shortcut_delete_back: "Delete Backward".into(),
            settings_shortcut_delete: "Delete Forward".into(),
            settings_shortcut_word_delete_back: "Word Delete Backward".into(),
            settings_shortcut_word_delete_forward: "Word Delete Forward".into(),
            settings_shortcut_focus_prev: "Move Up".into(),
            settings_shortcut_focus_next: "Move Down".into(),
            settings_shortcut_move_left: "Move Left".into(),
            settings_shortcut_move_right: "Move Right".into(),
            settings_shortcut_word_move_left: "Word Move Left".into(),
            settings_shortcut_word_move_right: "Word Move Right".into(),
            settings_shortcut_home: "Line Start".into(),
            settings_shortcut_end: "Line End".into(),
            settings_shortcut_block_up: "Block Up".into(),
            settings_shortcut_block_down: "Block Down".into(),
            settings_shortcut_page_up: "Page Up".into(),
            settings_shortcut_page_down: "Page Down".into(),
            settings_shortcut_jump_to_top: "Jump to Top".into(),
            settings_shortcut_jump_to_bottom: "Jump to Bottom".into(),
            settings_shortcut_select_left: "Select Left".into(),
            settings_shortcut_select_right: "Select Right".into(),
            settings_shortcut_word_select_left: "Word Select Left".into(),
            settings_shortcut_word_select_right: "Word Select Right".into(),
            settings_shortcut_select_home: "Select to Line Start".into(),
            settings_shortcut_select_end: "Select to Line End".into(),
            settings_shortcut_select_all: "Select All".into(),
            settings_shortcut_copy: "Copy".into(),
            settings_shortcut_cut: "Cut".into(),
            settings_shortcut_paste: "Paste".into(),
            settings_shortcut_undo: "Undo".into(),
            settings_shortcut_redo: "Redo".into(),
            settings_shortcut_bold_selection: "Bold".into(),
            settings_shortcut_italic_selection: "Italic".into(),
            settings_shortcut_underline_selection: "Underline".into(),
            settings_shortcut_code_selection: "Inline Code".into(),
            settings_shortcut_indent_block: "Indent Block".into(),
            settings_shortcut_outdent_block: "Outdent Block".into(),
            settings_shortcut_exit_code_block: "Exit Code Block".into(),
            settings_shortcut_save_document: "Save Document".into(),
            settings_shortcut_save_document_as: "Save Document As".into(),
            settings_shortcut_new_window: "New Window".into(),
            settings_shortcut_open_file: "Open File".into(),
            settings_shortcut_quit_application: "Quit Application".into(),
            settings_shortcut_close_window: "Close Window".into(),
            settings_shortcut_dismiss_transient_ui: "Dismiss Temporary UI".into(),
            settings_shortcut_toggle_view_mode: "Toggle View Mode".into(),
            settings_shortcut_toggle_explorer: "Toggle Explorer".into(),
            explorer_tab_files: "Files".into(),
            explorer_tab_outline: "Outline".into(),
            explorer_no_file_title: "No Markdown File Open".into(),
            explorer_no_file_message:
                "Open or save a .md file to use its folder as the explorer.".into(),
            explorer_empty_files: "No Markdown files to show".into(),
            explorer_empty_outline: "This document has no headings".into(),
            explorer_scan_failed_title: "Unable to Read ExplorerState".into(),
            open_link_title: "Open link?".into(),
            open_link_open: "Open".into(),
            open_link_cancel: "Cancel".into(),
            view_mode_source: "Source".into(),
            view_mode_switch_to_source: "Switch to Source".into(),
            view_mode_rendered: "Rendered".into(),
            view_mode_switch_to_rendered: "Switch to Rendered".into(),
            context_menu_insert: "Insert".into(),
            context_menu_table: "Table".into(),
            table_axis_align_column_left: "Align Column Left".into(),
            table_axis_align_column_center: "Align Column Center".into(),
            table_axis_align_column_right: "Align Column Right".into(),
            table_axis_move_column_left: "Move Column Left".into(),
            table_axis_move_column_right: "Move Column Right".into(),
            table_axis_delete_column: "Delete Column".into(),
            table_axis_move_row_up: "Move Row Up".into(),
            table_axis_move_row_down: "Move Row Down".into(),
            table_axis_delete_row: "Delete Row".into(),
            table_header_row: "Header Row".into(),
            table_insert_title: "Insert Table".into(),
            table_insert_description:
                "Create one header row and configure body rows and columns.".into(),
            table_insert_body_rows: "Body Rows".into(),
            table_insert_columns: "Columns".into(),
            table_insert_cancel: "Cancel".into(),
            table_insert_confirm: "Insert".into(),
            image_placeholder: "Image".into(),
            image_loading_without_alt: "Loading image...".into(),
            image_loading_with_alt_template: "Loading {alt}".into(),
            code_language_placeholder: "Language".into(),
            code_language_search_placeholder: "Search languages...".into(),
            status_bar_files: "Sidebar".into(),
            status_bar_mode_source: "Source".into(),
            status_bar_mode_rendered: "Rendered".into(),
            status_bar_word_count_suffix: "words".into(),
            settings_nav_status_bar: "Status Bar".into(),
            settings_status_bar_enabled: "Show Status Bar".into(),
            settings_status_bar_show_word_count: "Word Count".into(),
            settings_status_bar_show_cursor_position: "Cursor Position".into(),
            settings_status_bar_show_sidebar_toggle: "Sidebar Toggle".into(),
            settings_status_bar_show_mode_switch: "Mode Switch".into(),
        }
    }

    /// Returns a built-in string set for a supported language id.
    pub fn for_language_id(language_id: &str) -> Option<Self> {
        match language_id {
            "zh-CN" => Some(Self::zh_cn()),
            "en-US" => Some(Self::en_us()),
            _ => None,
        }
    }
}
