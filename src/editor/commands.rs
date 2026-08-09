//! Window commands — save/export/quit/close, CLI tooling, view-mode
//! switching, and dirty tracking.
//!
//! Action handlers wired in the window chrome (`src/windows/`) delegate to
//! these methods. File prompts and menu state live in
//! `crate::editor::menu_bar`.

use std::path::Path;

use futures::FutureExt;
use futures::channel::oneshot;

use crate::editor::actions::{
    CloseWindow, ExportHtml, ExportPdf, InstallCliTool, QuitApplication, SaveDocument,
    SaveDocumentAs, ToggleViewMode, UninstallCliTool,
};
use crate::editor::controller::*;
use crate::editor::editing::input::actions::{Redo, Undo};
use crate::infra::i18n::I18nManager;
use crate::infra::net::update_checker::{
    self as update_check, UpdateCheckResult, UpdateVersionInfo,
};

impl Editor {
    /// Builds the OS window title, including the dirty marker when the
    /// document has unsaved changes.
    pub(crate) fn window_title(
        file_path: Option<&Path>,
        is_dirty: bool,
        strings: &crate::infra::i18n::I18nStrings,
    ) -> String {
        let base_title = if let Some(path) = file_path {
            path.file_name().map_or_else(
                || path.to_string_lossy().to_string(),
                |name| name.to_string_lossy().to_string(),
            )
        } else {
            String::new()
        };

        if base_title.is_empty() {
            String::new()
        } else if is_dirty && !strings.dirty_title_marker.is_empty() {
            format!("{} {}", strings.dirty_title_marker, base_title)
        } else {
            base_title
        }
    }

    pub(crate) fn on_toggle_view_mode_action(
        &mut self,
        _: &ToggleViewMode,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_active_tab() {
            return;
        }
        self.toggle_view_mode_from_ui(cx);
    }

    pub(crate) fn toggle_view_mode_from_ui(&mut self, cx: &mut Context<Self>) {
        self.end_block_pointer_selection_sessions(cx);
        self.tab_mut().undo.last_selection_snapshot =
            self.capture_source_selection_snapshot_global(cx);
        self.toggle_view_mode(cx);
    }

    pub(crate) fn on_undo(&mut self, _: &Undo, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_active_tab() {
            return;
        }
        self.undo_document(cx);
    }

    pub(crate) fn on_redo(&mut self, _: &Redo, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_active_tab() {
            return;
        }
        self.redo_document(cx);
    }

    pub(crate) fn on_save_document(
        &mut self,
        _: &SaveDocument,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_active_tab() {
            return;
        }
        self.request_save_document(cx);
    }

    pub(crate) fn on_save_document_as(
        &mut self,
        _: &SaveDocumentAs,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_active_tab() {
            return;
        }
        self.request_save_document_as(cx);
    }

    pub(crate) fn on_export_html(
        &mut self,
        _: &ExportHtml,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_active_tab() {
            return;
        }
        self.export_document_via_prompt(
            crate::editor::render::export::ExportFormat::Html,
            window,
            cx,
        );
    }

    pub(crate) fn on_export_pdf(
        &mut self,
        _: &ExportPdf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_active_tab() {
            return;
        }
        self.export_document_via_prompt(
            crate::editor::render::export::ExportFormat::Pdf,
            window,
            cx,
        );
    }

    pub(crate) fn on_quit_application(
        &mut self,
        _: &QuitApplication,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        crate::app::menus::request_quit_application(cx);
    }

    pub(crate) fn on_close_window(
        &mut self,
        _: &CloseWindow,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.request_close_current_window(window, cx);
    }

    pub(crate) fn on_install_cli_tool(
        &mut self,
        _: &InstallCliTool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        crate::platform::cli_tool::install_cli_tool(cx);
    }

    pub(crate) fn on_uninstall_cli_tool(
        &mut self,
        _: &UninstallCliTool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        crate::platform::cli_tool::uninstall_cli_tool(cx);
    }

    #[allow(dead_code)]
    pub(crate) fn set_view_mode(&mut self, target_mode: EditorMode, cx: &mut Context<Self>) {
        if self.tab().mode != target_mode {
            self.toggle_view_mode(cx);
        }
    }

    pub(crate) fn toggle_view_mode(&mut self, cx: &mut Context<Self>) {
        self.end_block_pointer_selection_sessions(cx);
        let selection_snapshot = self.capture_source_selection_snapshot_global(cx);
        self.clear_cross_block_selection(cx);
        self.tab_mut().selection.select_all_cycle = None;
        match self.tab().mode {
            EditorMode::Wysiwyg => {
                let markdown = self.doc().to_markdown(cx);
                let block = Self::new_block(cx, BlockData::paragraph(markdown));
                block.update(cx, |block, _cx| block.set_source_document_mode());
                self.doc_mut().replace_blocks(vec![block], cx);
                self.tab_mut().mode = EditorMode::SourceCode;
                self.tab_mut().tables.cells.clear();
            }
            EditorMode::SourceCode => {
                let source = self.doc().to_raw_source(cx);
                let mut roots = Self::parse_document(cx, &source);
                if roots.is_empty() {
                    roots.push(Self::new_block(cx, BlockData::paragraph(String::new())));
                }
                self.doc_mut().replace_blocks(roots, cx);
                self.tab_mut().mode = EditorMode::Wysiwyg;
                self.rebuild_table_runtimes(cx);
                self.rebuild_image_runtimes(cx);
            }
        }

        self.apply_selection_snapshot_in_current_mode(&selection_snapshot, cx);
        self.tab_mut().focus.pending_scroll_active_block_into_view = true;
        self.tab_mut().focus.pending_scroll_recheck_after_layout = true;
        self.tab_mut().scroll.last_viewport_size = None;
        self.tab_mut().file.pending_window_title_refresh = true;
        self.tab_mut().file.close_dialog_restore_focus = None;
        self.tab_mut().tables.axis_preview = None;
        self.tab_mut().tables.axis_selection = None;
        self.dismiss_contextual_overlays(cx);
        self.sync_table_axis_visuals(cx);
        self.refresh_stable_document_snapshot(cx);
        cx.notify();
    }

    /// Marks the document dirty and schedules window-title and edited-state
    /// refresh for the next render frame.
    pub(crate) fn mark_dirty(&mut self, cx: &mut Context<Self>) {
        if !self.tab().file.dirty {
            self.tab_mut().file.dirty = true;
            self.tab_mut().file.pending_window_edited = true;
            self.tab_mut().file.pending_window_title_refresh = true;
            cx.notify();
        }
    }
}

// ── Update-check flow ────────────────────────────────────────────────────

impl Editor {
    pub(crate) fn request_check_updates(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .active_editor_tab()
            .is_some_and(|tab| tab.file.show_unsaved_changes_dialog)
        {
            return;
        }
        if self.update_check_in_progress {
            self.show_info_dialog(InfoDialogKind::CheckForUpdates, cx);
            return;
        }

        self.update_check_in_progress = true;
        self.show_info_dialog(InfoDialogKind::CheckForUpdates, cx);

        let weak_editor = cx.entity().downgrade();
        let window_handle = window.window_handle();
        let (tx, rx) = oneshot::channel();
        std::thread::spawn(move || {
            let result = update_check::check_latest_version(env!("CARGO_PKG_VERSION"));
            let _ = tx.send(result);
        });

        cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let result = rx
                .map(|result| {
                    result.unwrap_or_else(|_| {
                        Err(update_check::UpdateCheckError::ParseVersion(
                            "update check worker ended before returning a result".to_string(),
                        ))
                    })
                })
                .await;

            let _ = weak_editor.update(cx, |editor, cx| {
                editor.update_check_in_progress = false;
                editor.hide_info_dialog(cx);
            });

            let _ = cx.update_window(
                window_handle,
                move |_view: AnyView, window: &mut Window, cx: &mut App| match result {
                    Ok(UpdateCheckResult::UpdateAvailable(info)) => {
                        show_update_available_prompt(window, cx, &info);
                    }
                    Ok(UpdateCheckResult::UpToDate(info)) => {
                        show_up_to_date_prompt(window, cx, &info);
                    }
                    Err(error) => {
                        show_update_failed_prompt(window, cx, &error.to_string());
                    }
                },
            );
        })
        .detach();
    }
}

fn show_update_available_prompt(window: &mut Window, cx: &mut App, info: &UpdateVersionInfo) {
    let strings = cx.global::<I18nManager>().strings().clone();
    let detail = format_update_message(
        &strings.update_available_message_template,
        &info.current_version,
        &info.latest_version,
    );
    let buttons = [
        strings.update_open_release.as_str(),
        strings.update_later.as_str(),
    ];
    let prompt = window.prompt(
        PromptLevel::Info,
        &strings.update_available_title,
        Some(&detail),
        &buttons,
        cx,
    );
    let window_handle = window.window_handle();
    cx.spawn(async move |cx| {
        let Ok(choice) = prompt.await else {
            return;
        };
        if choice == 0 {
            let _ = cx.update_window(window_handle, |_view: AnyView, _window, cx| {
                cx.open_url(update_check::RELEASES_URL);
            });
        }
    })
    .detach();
}

fn show_up_to_date_prompt(window: &mut Window, cx: &mut App, info: &UpdateVersionInfo) {
    let strings = cx.global::<I18nManager>().strings().clone();
    let detail = format_update_message(
        &strings.update_up_to_date_message_template,
        &info.current_version,
        &info.latest_version,
    );
    let buttons = [strings.info_dialog_ok.as_str()];
    let _ = window.prompt(
        PromptLevel::Info,
        &strings.update_up_to_date_title,
        Some(&detail),
        &buttons,
        cx,
    );
}

fn show_update_failed_prompt(window: &mut Window, cx: &mut App, detail: &str) {
    let strings = cx.global::<I18nManager>().strings().clone();
    let message = strings
        .update_failed_message_template
        .replace("{error}", detail);
    let buttons = [strings.info_dialog_ok.as_str()];
    let _ = window.prompt(
        PromptLevel::Critical,
        &strings.update_failed_title,
        Some(&message),
        &buttons,
        cx,
    );
}

fn format_update_message(template: &str, current_version: &str, latest_version: &str) -> String {
    template
        .replace("{current}", current_version)
        .replace("{latest}", latest_version)
}

#[cfg(test)]
mod tests {
    use super::format_update_message;

    #[test]
    pub(crate) fn update_message_templates_replace_versions() {
        assert_eq!(
            format_update_message("Current {current}, latest {latest}.", "0.2.1", "0.2.2"),
            "Current 0.2.1, latest 0.2.2."
        );
    }
}
