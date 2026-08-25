//! All localisable UI strings and their defaults.
//!
//! `I18nStrings` is the complete string table; missing entries in a custom
//! language pack fall back to English defaults.

use serde::{Deserialize, Serialize};

/// All localisable UI strings for the editor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct I18nStrings {
    /// Marker prepended to the window title when the document is dirty.
    pub dirty_title_marker: String,
    /// Title of the unsaved-changes dialog.
    pub unsaved_changes_title: String,
    /// Body message of the unsaved-changes dialog.
    pub unsaved_changes_message: String,
    /// Title of the window-level unsaved-changes dialog.
    pub unsaved_changes_window_title: String,
    /// Message of the window-level unsaved-changes dialog.
    pub unsaved_changes_window_message: String,
    /// Title of the editor-panel-level unsaved-changes dialog.
    pub unsaved_changes_editor_title: String,
    /// Message of the editor-panel-level unsaved-changes dialog.
    pub unsaved_changes_editor_message: String,
    /// Title of the tab-level unsaved-changes dialog.
    pub unsaved_changes_tab_title: String,
    /// Message template of the tab-level unsaved-changes dialog. Supports `{name}`.
    pub unsaved_changes_tab_message_template: String,
    /// Label for the "save" button.
    pub unsaved_changes_save: String,
    /// Label for the "don't save" button.
    pub unsaved_changes_discard: String,
    /// Label for the "save and close" button.
    pub unsaved_changes_save_and_close: String,
    /// Label for the "discard and close" button.
    pub unsaved_changes_discard_and_close: String,
    /// Label for the "keep editing" / "cancel" button.
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
    /// About panel label for the website link.
    pub about_website_label: String,
    /// About panel label for the wiki link.
    pub about_wiki_label: String,
    /// About panel label for the releases link.
    pub about_releases_label: String,
    /// About panel tagline shown under the app name.
    pub about_tagline: String,
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
    /// Menu item for closing the folder shown in the explorer.
    pub menu_close_explorer_folder: String,
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
    pub settings_shortcut_toggle_pane_kind: String,
    pub settings_shortcut_toggle_explorer: String,
    /// Title of the recent-files list in the explorer empty state.
    pub explorer_recent_title: String,
    /// Message shown when the current document has no headings.
    pub explorer_empty_outline: String,
    /// Explorer context menu: create a new file in the current directory.
    pub explorer_new_file: String,
    /// Explorer context menu: create a new folder in the current directory.
    pub explorer_new_folder: String,
    /// Explorer context menu: reveal the entry in the OS file manager.
    pub explorer_reveal_in_file_manager: String,
    /// Explorer context menu: open the entry with the default OS application.
    pub explorer_open_in_default_app: String,
    /// Explorer context menu: move the entry to the OS trash.
    pub explorer_trash: String,
    /// Explorer context menu: cut the selected entries.
    pub explorer_cut: String,
    /// Explorer context menu: copy the selected entries.
    pub explorer_copy: String,
    /// Explorer context menu: duplicate the selected entries.
    pub explorer_duplicate: String,
    /// Explorer context menu: paste the clipboard into the target.
    pub explorer_paste: String,
    /// Explorer context menu: undo the last file operation.
    pub explorer_undo: String,
    /// Explorer context menu: redo the last undone file operation.
    pub explorer_redo: String,
    /// Explorer context menu: copy the absolute path of the entry.
    pub explorer_copy_path: String,
    /// Explorer context menu: copy the path relative to the explorer root.
    pub explorer_copy_relative_path: String,
    /// Explorer context menu: rename the entry.
    pub explorer_rename: String,
    /// Explorer context menu: permanently delete the entry.
    pub explorer_delete: String,
    /// Explorer context menu: expand a directory and all of its children.
    pub explorer_expand_all: String,
    /// Explorer context menu: collapse a directory and all of its children.
    pub explorer_collapse_all: String,
    /// Explorer context menu (worktree root): add another folder to the explorer.
    pub explorer_add_folder: String,
    /// Explorer context menu (worktree root): remove this folder from the explorer.
    pub explorer_remove_folder: String,
    /// Title of the link-opening confirmation prompt.
    pub open_link_title: String,
    /// Confirm button for the link-opening prompt.
    pub open_link_open: String,
    /// Cancel button for the link-opening prompt.
    pub open_link_cancel: String,
    /// Compact label shown when rendered mode can switch to source mode.
    pub pane_mode_source: String,
    /// Hover label shown when rendered mode can switch to source mode.
    pub pane_mode_switch_to_source: String,
    /// Compact label shown when source mode can switch to rendered mode.
    pub pane_mode_rendered: String,
    /// Hover label shown when source mode can switch to rendered mode.
    pub pane_mode_switch_to_rendered: String,
    /// Root context-menu insert label.
    pub context_menu_insert: String,
    /// Insert submenu item for tables.
    pub context_menu_table: String,
    /// Table-axis menu item for inserting a column to the left.
    pub table_axis_insert_column_left: String,
    /// Table-axis menu item for inserting a column to the right.
    pub table_axis_insert_column_right: String,
    /// Table-axis menu item for duplicating a column.
    pub table_axis_duplicate_column: String,
    /// Table-axis menu item for inserting a row above.
    pub table_axis_insert_row_above: String,
    /// Table-axis menu item for inserting a row below.
    pub table_axis_insert_row_below: String,
    /// Table-axis menu item for duplicating a row.
    pub table_axis_duplicate_row: String,
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
}


pub const I18N_STRING_KEYS: &[&str] = &[
    "dirty_title_marker",
    "unsaved_changes_title",
    "unsaved_changes_message",
    "unsaved_changes_window_title",
    "unsaved_changes_window_message",
    "unsaved_changes_editor_title",
    "unsaved_changes_editor_message",
    "unsaved_changes_tab_title",
    "unsaved_changes_tab_message_template",
    "unsaved_changes_save",
    "unsaved_changes_discard",
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
    "about_website_label",
    "about_wiki_label",
    "about_releases_label",
    "about_tagline",
    "menu_install_cli_tool",
    "menu_uninstall_cli_tool",
    "menu_repository",
    "menu_bug_report",
    "menu_feature_request",
    "menu_discussions",
    "menu_close_explorer_folder",
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
    "settings_shortcut_toggle_pane_kind",
    "settings_shortcut_toggle_explorer",
    "explorer_recent_title",
    "explorer_empty_outline",
    "explorer_new_file",
    "explorer_new_folder",
    "explorer_reveal_in_file_manager",
    "explorer_open_in_default_app",
    "explorer_trash",
    "explorer_cut",
    "explorer_copy",
    "explorer_duplicate",
    "explorer_paste",
    "explorer_undo",
    "explorer_redo",
    "explorer_copy_path",
    "explorer_copy_relative_path",
    "explorer_rename",
    "explorer_delete",
    "explorer_expand_all",
    "explorer_collapse_all",
    "explorer_add_folder",
    "explorer_remove_folder",
    "open_link_title",
    "open_link_open",
    "open_link_cancel",
    "pane_mode_source",
    "pane_mode_switch_to_source",
    "pane_mode_rendered",
    "pane_mode_switch_to_rendered",
    "context_menu_insert",
    "context_menu_table",
    "table_axis_insert_column_left",
    "table_axis_insert_column_right",
    "table_axis_duplicate_column",
    "table_axis_insert_row_above",
    "table_axis_insert_row_below",
    "table_axis_duplicate_row",
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
    "status_bar_word_count_suffix",
    "settings_nav_status_bar",
    "settings_status_bar_enabled",
    "settings_status_bar_show_word_count",
    "settings_status_bar_show_cursor_position",
];


impl I18nStrings {
    /// Built-in Simplified Chinese UI strings.
    pub fn zh_cn() -> Self {
        let mut strings = Self {
            dirty_title_marker: "\u{00B7}".into(),
            unsaved_changes_title: "不保存并关闭？".into(),
            unsaved_changes_message: "此文档有未保存的更改。关闭前保存可避免丢失最新编辑。".into(),
            unsaved_changes_window_title: "关闭窗口？".into(),
            unsaved_changes_window_message: "窗口中有未保存的更改。关闭前是否保存？".into(),
            unsaved_changes_editor_title: "关闭编辑器？".into(),
            unsaved_changes_editor_message: "当前编辑器中有未保存的更改。关闭前是否保存？".into(),
            unsaved_changes_tab_title: "保存更改？".into(),
            unsaved_changes_tab_message_template: "关闭前是否保存对“{name}”的更改？".into(),
            unsaved_changes_save: "保存".into(),
            unsaved_changes_discard: "不保存".into(),
            unsaved_changes_save_and_close: "保存并关闭".into(),
            unsaved_changes_discard_and_close: "放弃并关闭".into(),
            unsaved_changes_cancel: "取消".into(),
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
            about_website_label: "网站".into(),
            about_wiki_label: "文档".into(),
            about_releases_label: "版本发布".into(),
            about_tagline: "拆分窗口，键入代码".into(),
            menu_install_cli_tool: "安装CLI命令".into(),
            menu_uninstall_cli_tool: "卸载CLI命令".into(),
            menu_repository: "Splitype 仓库".into(),
            menu_bug_report: "报告 Bug...".into(),
            menu_feature_request: "请求功能...".into(),
            menu_discussions: "加入讨论".into(),
            menu_close_explorer_folder: "关闭资源管理器文件夹".into(),
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
            settings_shortcut_toggle_pane_kind: "切换窗格模式".into(),
            settings_shortcut_toggle_explorer: "切换资源管理器".into(),
            explorer_recent_title: "最近打开".into(),
            explorer_empty_outline: "当前文档没有标题".into(),
            explorer_new_file: "新建文件".into(),
            explorer_new_folder: "新建文件夹".into(),
            explorer_reveal_in_file_manager: "在文件管理器中显示".into(),
            explorer_open_in_default_app: "用默认应用打开".into(),
            explorer_trash: "移到回收站".into(),
            explorer_cut: "剪切".into(),
            explorer_copy: "复制".into(),
            explorer_duplicate: "复制副本".into(),
            explorer_paste: "粘贴".into(),
            explorer_undo: "撤销".into(),
            explorer_redo: "重做".into(),
            explorer_copy_path: "复制路径".into(),
            explorer_copy_relative_path: "复制相对路径".into(),
            explorer_rename: "重命名".into(),
            explorer_delete: "删除".into(),
            explorer_expand_all: "全部展开".into(),
            explorer_collapse_all: "全部折叠".into(),
            explorer_add_folder: "添加文件夹到资源管理器".into(),
            explorer_remove_folder: "从资源管理器移除".into(),
            open_link_title: "打开链接？".into(),
            open_link_open: "打开".into(),
            open_link_cancel: "取消".into(),
            pane_mode_source: "源码".into(),
            pane_mode_switch_to_source: "切换到源码".into(),
            pane_mode_rendered: "渲染".into(),
            pane_mode_switch_to_rendered: "切换到渲染".into(),
            context_menu_insert: "插入".into(),
            context_menu_table: "表格".into(),
            table_axis_insert_column_left: "在左侧插入列".into(),
            table_axis_insert_column_right: "在右侧插入列".into(),
            table_axis_duplicate_column: "复制列".into(),
            table_axis_insert_row_above: "在上方插入行".into(),
            table_axis_insert_row_below: "在下方插入行".into(),
            table_axis_duplicate_row: "复制行".into(),
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
            status_bar_word_count_suffix: "字".into(),
            ..Self::en_us()
        };
        strings.image_paste_failed_title = "图片粘贴失败".into();
        strings.settings_nav_image = "图像".into();
        strings.settings_nav_status_bar = "状态栏".into();
        strings.settings_status_bar_enabled = "显示状态栏".into();
        strings.settings_status_bar_show_word_count = "字数统计".into();
        strings.settings_status_bar_show_cursor_position = "光标位置".into();
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
            unsaved_changes_window_title: "Close Window?".into(),
            unsaved_changes_window_message:
                "There are unsaved changes in this window. Save before closing?".into(),
            unsaved_changes_editor_title: "Close Editor?".into(),
            unsaved_changes_editor_message:
                "This editor has unsaved changes. Save before closing?".into(),
            unsaved_changes_tab_title: "Save Changes?".into(),
            unsaved_changes_tab_message_template:
                "Save changes to \"{name}\" before closing?".into(),
            unsaved_changes_save: "Save".into(),
            unsaved_changes_discard: "Don't Save".into(),
            unsaved_changes_save_and_close: "Save and Close".into(),
            unsaved_changes_discard_and_close: "Discard and Close".into(),
            unsaved_changes_cancel: "Cancel".into(),
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
            about_website_label: "Website".into(),
            about_wiki_label: "Wiki".into(),
            about_releases_label: "Releases".into(),
            about_tagline: "Split the window, type your code".into(),
            menu_install_cli_tool: "Install CLI Command".into(),
            menu_uninstall_cli_tool: "Uninstall CLI Command".into(),
            menu_repository: "Splitype Repository".into(),
            menu_bug_report: "File Bug Report...".into(),
            menu_feature_request: "Request Feature...".into(),
            menu_discussions: "Join the Discussion".into(),
            menu_close_explorer_folder: "Close Explorer Folder".into(),
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
            settings_shortcut_toggle_pane_kind: "Toggle Pane Mode".into(),
            settings_shortcut_toggle_explorer: "Toggle Explorer".into(),
            explorer_recent_title: "Recent".into(),
            explorer_empty_outline: "This document has no headings".into(),
            explorer_new_file: "New File".into(),
            explorer_new_folder: "New Folder".into(),
            explorer_reveal_in_file_manager: "Reveal in File Manager".into(),
            explorer_open_in_default_app: "Open in Default App".into(),
            explorer_trash: "Trash".into(),
            explorer_cut: "Cut".into(),
            explorer_copy: "Copy".into(),
            explorer_duplicate: "Duplicate".into(),
            explorer_paste: "Paste".into(),
            explorer_undo: "Undo".into(),
            explorer_redo: "Redo".into(),
            explorer_copy_path: "Copy Path".into(),
            explorer_copy_relative_path: "Copy Relative Path".into(),
            explorer_rename: "Rename".into(),
            explorer_delete: "Delete".into(),
            explorer_expand_all: "Expand All".into(),
            explorer_collapse_all: "Collapse All".into(),
            explorer_add_folder: "Add Folder to Explorer…".into(),
            explorer_remove_folder: "Remove from Explorer".into(),
            open_link_title: "Open link?".into(),
            open_link_open: "Open".into(),
            open_link_cancel: "Cancel".into(),
            pane_mode_source: "Source".into(),
            pane_mode_switch_to_source: "Switch to Source".into(),
            pane_mode_rendered: "Rendered".into(),
            pane_mode_switch_to_rendered: "Switch to Rendered".into(),
            context_menu_insert: "Insert".into(),
            context_menu_table: "Table".into(),
            table_axis_insert_column_left: "Insert Column Left".into(),
            table_axis_insert_column_right: "Insert Column Right".into(),
            table_axis_duplicate_column: "Duplicate Column".into(),
            table_axis_insert_row_above: "Insert Row Above".into(),
            table_axis_insert_row_below: "Insert Row Below".into(),
            table_axis_duplicate_row: "Duplicate Row".into(),
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
            status_bar_word_count_suffix: "words".into(),
            settings_nav_status_bar: "Status Bar".into(),
            settings_status_bar_enabled: "Show Status Bar".into(),
            settings_status_bar_show_word_count: "Word Count".into(),
            settings_status_bar_show_cursor_position: "Cursor Position".into(),
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
