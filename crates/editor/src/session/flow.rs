//! Document lifecycle flows: save, save-as, close confirmations, and
//! drop-replacement. All content mutations go through the shared buffer.

use std::path::{Path, PathBuf};

use anyhow::Result;
use gpui::*;

use crate::document::{DocumentBuffer, DocumentStore};
use crate::editor::Editor;
use config::language::I18nManager;

impl Editor {
    /// Queues a save request for the active tab, consumed on the next
    /// render frame by the view-sync layer.
    pub fn request_save_document(&mut self, cx: &mut Context<Self>) {
        if let Some(tab) = self.session.active_tab_mut() {
            if !tab.pending.pending_save {
                tab.pending.pending_save = true;
                cx.notify();
            }
        }
    }

    /// Queues a save-as request for the active tab, consumed on the next
    /// render frame by the view-sync layer.
    pub fn request_save_document_as(&mut self, cx: &mut Context<Self>) {
        if let Some(tab) = self.session.active_tab_mut() {
            if !tab.pending.pending_save_as {
                tab.pending.pending_save_as = true;
                cx.notify();
            }
        }
    }

    /// The active tab's current raw document text; empty when no tab is open.
    pub fn active_document_text(&self, cx: &App) -> String {
        self.session
            .active_tab()
            .map(|tab| tab.buffer.read(cx).text.clone())
            .unwrap_or_default()
    }

    pub fn save_dialog_defaults(&self, cx: &App) -> (PathBuf, Option<String>) {
        if let Some(tab) = self.session.active_tab() {
            if let Some(path) = tab.buffer.read(cx).path.as_ref() {
                let directory = path.parent().map(Path::to_path_buf).unwrap_or_else(|| {
                    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                });
                let suggested_name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string());
                return (directory, suggested_name);
            }
        }
        (
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            Some("untitled.md".to_string()),
        )
    }

    /// Marks the active buffer saved at `path` and refreshes every observer.
    pub fn apply_successful_save(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if let Some(tab) = self.session.active_tab_mut() {
            tab.pending.pending_close_after_save = false;
            tab.pending.close_dialog_restore_focus = None;
        }
        if let Some(buffer) = self.session.active_tab().map(|tab| tab.buffer.clone()) {
            let (id, old_path) = {
                let buffer = buffer.read(cx);
                (buffer.id, buffer.path.clone())
            };
            buffer.update(cx, |buffer, cx| buffer.mark_saved(path.clone(), cx));
            cx.global_mut::<DocumentStore>().update_path_index(
                id,
                old_path.as_deref(),
                Some(path.clone()),
            );
        }
        if let Some(host) = self.host.clone() {
            host.record_recent_file(&path, cx);
            host.on_document_path_changed(cx);
        }
        cx.notify();
    }

    /// Repoints every open buffer after a filesystem rename.
    pub fn on_fs_path_renamed(&mut self, from: &Path, to: &Path, cx: &mut App) {
        DocumentStore::rename_paths(from, to, cx);
    }

    pub fn save_to_existing_path(
        &mut self,
        path: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let markdown = self.active_document_text(cx);
        match std::fs::write(path, markdown) {
            Ok(_) => {
                self.apply_successful_save(path.to_path_buf(), cx);
                window.set_window_edited(false);
                true
            }
            Err(err) => {
                let detail = err.to_string();
                let strings = cx.global::<I18nManager>().strings().clone();
                let buttons = [strings.info_dialog_ok.as_str()];
                let _ = window.prompt(
                    PromptLevel::Critical,
                    &strings.save_failed_title,
                    Some(&detail),
                    &buttons,
                    cx,
                );
                false
            }
        }
    }

    pub fn save_document(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.session.active_tab() {
            if let Some(path) = tab.buffer.read(cx).path.clone() {
                let should_close_after_save = tab.pending.pending_close_after_save;
                if self.save_to_existing_path(&path, window, cx) {
                    if should_close_after_save {
                        window.remove_window();
                    }
                } else if should_close_after_save {
                    self.abort_pending_close_after_save(cx);
                }
                return;
            }
        }

        self.save_document_via_prompt(window, cx);
    }

    pub fn save_document_via_prompt(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let should_close_after_save = self
            .session
            .active_tab()
            .is_some_and(|t| t.pending.pending_close_after_save);
        let markdown = self.active_document_text(cx);
        let (default_dir, suggested_name) = self.save_dialog_defaults(cx);
        let prompt = cx.prompt_for_new_path(&default_dir, suggested_name.as_deref());
        let weak_editor = cx.entity().downgrade();
        let weak_editor_for_cancel = weak_editor.clone();
        let weak_editor_for_error = weak_editor.clone();
        let weak_editor_for_write_error = weak_editor.clone();
        let window_handle = window.window_handle();

        cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut path = match prompt.await {
                Ok(Ok(Some(path))) => path,
                Ok(Ok(None)) | Err(_) => {
                    if should_close_after_save {
                        let _ = weak_editor_for_cancel
                            .update(cx, |this, cx| this.abort_pending_close_after_save(cx));
                    }
                    return;
                }
                Ok(Err(err)) => {
                    if should_close_after_save {
                        let _ = weak_editor_for_error
                            .update(cx, |this, cx| this.abort_pending_close_after_save(cx));
                    }
                    let detail = err.to_string();
                    let _ = cx.update_window(
                        window_handle,
                        move |_view: AnyView, window: &mut Window, cx: &mut App| {
                            let strings = cx.global::<I18nManager>().strings().clone();
                            let buttons = [strings.info_dialog_ok.as_str()];
                            let _ = window.prompt(
                                PromptLevel::Critical,
                                &strings.save_failed_title,
                                Some(&detail),
                                &buttons,
                                cx,
                            );
                        },
                    );
                    return;
                }
            };

            if path.extension().is_none() {
                path.set_extension("md");
            }

            let file_content = markdown.clone();
            let write_path = path.clone();
            let write_result = std::fs::write(&write_path, file_content);

            if let Err(err) = write_result {
                if should_close_after_save {
                    let _ = weak_editor_for_write_error
                        .update(cx, |this, cx| this.abort_pending_close_after_save(cx));
                }
                let detail = err.to_string();
                let _ = cx.update_window(
                    window_handle,
                    move |_view: AnyView, window: &mut Window, cx: &mut App| {
                        let strings = cx.global::<I18nManager>().strings().clone();
                        let buttons = [strings.info_dialog_ok.as_str()];
                        let _ = window.prompt(
                            PromptLevel::Critical,
                            &strings.save_failed_title,
                            Some(&detail),
                            &buttons,
                            cx,
                        );
                    },
                );
                return;
            }

            let _ = weak_editor.update(cx, |this, cx| {
                this.apply_successful_save(path, cx);
                if should_close_after_save {
                    window_handle
                        .update(cx, |_view, window, _cx| {
                            window.remove_window();
                        })
                        .ok();
                }
            });
        })
        .detach();
    }

    pub fn save_tab_at(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(tab) = self.session.tab(index) else {
            return false;
        };
        let buffer = tab.buffer.clone();
        let (dirty, path) = {
            let buffer = buffer.read(cx);
            (buffer.dirty, buffer.path.clone())
        };
        if !dirty {
            return true;
        }
        if let Some(path) = path {
            let text = buffer.read(cx).text.clone();
            if std::fs::write(&path, text).is_ok() {
                buffer.update(cx, |buffer, cx| buffer.mark_saved(path, cx));
                if let Some(tab) = self.session.tab_mut(index) {
                    tab.pending.window_title_refresh = true;
                }
                cx.notify();
                return true;
            }
        } else {
            self.activate_tab(index, cx);
            self.save_document_via_prompt(window, cx);
        }
        false
    }

    pub fn save_all_dirty_tabs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for i in 0..self.session.tab_count() {
            let Some(tab) = self.session.tab(i) else {
                continue;
            };
            if tab.buffer.read(cx).dirty {
                self.save_tab_at(i, window, cx);
            }
        }
    }

    pub fn save_document_as(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.save_document_via_prompt(window, cx);
    }

    pub fn abort_pending_close_after_save(&mut self, cx: &mut App) {
        if let Some(tab) = self.session.active_tab_mut() {
            tab.pending.pending_close_after_save = false;
        }
        cx.notify(self.entity_id);
    }

    pub fn cancel_close_dialog(&mut self, cx: &mut Context<Self>) {
        if let Some(tab) = self.session.active_tab_mut() {
            tab.pending.show_unsaved_changes_dialog = false;
            tab.pending.pending_close_after_save = false;
            tab.pending.close_dialog_restore_focus = None;
        }
        cx.notify();
    }

    pub fn save_and_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.session.active_tab_mut() {
            tab.pending.pending_close_after_save = true;
        }
        self.save_document(window, cx);
    }

    pub fn discard_and_close(&mut self, cx: &mut Context<Self>) {
        self.discard_changes(cx);
        if let Some(host) = self.host.clone() {
            host.request_close_panel(self.panel_id, cx);
        }
        cx.notify();
    }

    /// Discards every tab of this panel: buffers owned entirely by this
    /// panel are destroyed; shared buffers only lose this panel's view.
    pub fn discard_changes(&mut self, cx: &mut App) {
        for index in (0..self.session.tab_count()).rev() {
            let Some(tab) = self.session.tab(index) else {
                continue;
            };
            let buffer = tab.buffer.clone();
            let (id, dirty) = {
                let buffer = buffer.read(cx);
                (buffer.id, buffer.dirty)
            };
            if dirty && cx.global::<DocumentStore>().view_count(id) == 1 {
                buffer.update(cx, |buffer, cx| buffer.mark_discarded(cx));
                cx.global_mut::<DocumentStore>().discard(id);
            } else {
                cx.global_mut::<DocumentStore>().release(id, false);
            }
        }
        self.session.clear_tabs();
        self.documents_released = true;
        cx.notify(self.entity_id);
    }

    /// Releases every tab view without touching content, ahead of panel or
    /// window teardown.
    pub fn release_documents(&mut self, cx: &mut App) {
        if self.documents_released {
            return;
        }
        self.documents_released = true;
        for tab in self.session.tabs() {
            let buffer = tab.buffer.clone();
            let id = buffer.read(cx).id;
            cx.global_mut::<DocumentStore>().release(id, false);
        }
        self.session.clear_tabs();
    }

    pub fn on_external_paths_drop(
        &mut self,
        paths: &ExternalPaths,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(path) = crate::input::first_dropped_markdown_path(paths.paths()) else {
            let strings = cx.global::<I18nManager>().strings().clone();
            self.show_drop_open_failed_prompt(strings.drop_no_markdown_file_message, window, cx);
            return;
        };

        self.request_dropped_markdown_replace(path, window, cx);
    }

    pub fn request_dropped_markdown_replace(
        &mut self,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(tab) = self.session.active_tab_mut() {
            if tab.buffer.read(cx).dirty {
                tab.pending.drop_replace_path = Some(path);
                if !tab.pending.show_drop_replace_dialog {
                    tab.pending.show_drop_replace_dialog = true;
                    window.blur();
                }
                cx.notify();
                return;
            }
        }

        match self.replace_document_from_path(&path, cx) {
            Ok(()) => window.set_window_edited(false),
            Err(err) => self.show_drop_open_failed_prompt(err.to_string(), window, cx),
        }
    }

    pub fn cancel_drop_replace_dialog(&mut self, cx: &mut Context<Self>) {
        self.clear_pending_drop_replace_state(cx);
        cx.notify();
    }

    pub fn discard_pending_drop_replace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = self
            .session
            .active_tab_mut()
            .and_then(|t| t.pending.drop_replace_path.take())
        else {
            self.clear_pending_drop_replace_state(cx);
            return;
        };

        self.clear_pending_drop_replace_state(cx);
        match self.replace_document_from_path(&path, cx) {
            Ok(()) => window.set_window_edited(false),
            Err(err) => self.show_drop_open_failed_prompt(err.to_string(), window, cx),
        }
    }

    pub fn save_and_replace_pending_drop(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.session.active_tab_mut() else {
            self.clear_pending_drop_replace_state(cx);
            return;
        };
        if tab.pending.drop_replace_path.is_none() {
            self.clear_pending_drop_replace_state(cx);
            return;
        }

        tab.pending.show_drop_replace_dialog = false;
        tab.pending.drop_replace_after_save = true;

        let path = tab.buffer.read(cx).path.clone();
        if let Some(path) = path {
            if self.save_to_existing_path(&path, window, cx) {
                self.replace_after_successful_save(window, cx);
            } else {
                self.abort_pending_drop_replace_after_save(cx);
            }
            return;
        }

        self.save_via_prompt_then_replace_drop(window, cx);
        cx.notify();
    }

    pub fn replace_after_successful_save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(drop_path) = self
            .session
            .active_tab_mut()
            .and_then(|t| t.pending.drop_replace_path.take())
        else {
            self.clear_pending_drop_replace_state(cx);
            return;
        };

        self.clear_pending_drop_replace_state(cx);
        match self.replace_document_from_path(&drop_path, cx) {
            Ok(()) => window.set_window_edited(false),
            Err(err) => self.show_drop_open_failed_prompt(err.to_string(), window, cx),
        }
    }

    pub fn save_via_prompt_then_replace_drop(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(drop_path) = self
            .session
            .active_tab()
            .and_then(|t| t.pending.drop_replace_path.clone())
        else {
            self.clear_pending_drop_replace_state(cx);
            return;
        };
        let markdown = self.active_document_text(cx);
        let (default_dir, suggested_name) = self.save_dialog_defaults(cx);
        let prompt = cx.prompt_for_new_path(&default_dir, suggested_name.as_deref());
        let weak_editor = cx.entity().downgrade();
        let weak_editor_for_cancel = weak_editor.clone();
        let weak_editor_for_error = weak_editor.clone();
        let weak_editor_for_write_error = weak_editor.clone();
        let window_handle = window.window_handle();

        cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut save_path = match prompt.await {
                Ok(Ok(Some(path))) => path,
                Ok(Ok(None)) | Err(_) => {
                    let _ = weak_editor_for_cancel.update(cx, |this, cx| {
                        this.abort_pending_drop_replace_after_save(cx)
                    });
                    return;
                }
                Ok(Err(err)) => {
                    let _ = weak_editor_for_error.update(cx, |this, cx| {
                        this.abort_pending_drop_replace_after_save(cx)
                    });
                    let detail = err.to_string();
                    let _ = cx.update_window(
                        window_handle,
                        move |_view: AnyView, window: &mut Window, cx: &mut App| {
                            let strings = cx.global::<I18nManager>().strings().clone();
                            let buttons = [strings.info_dialog_ok.as_str()];
                            let _ = window.prompt(
                                PromptLevel::Critical,
                                &strings.save_failed_title,
                                Some(&detail),
                                &buttons,
                                cx,
                            );
                        },
                    );
                    return;
                }
            };

            if save_path.extension().is_none() {
                save_path.set_extension("md");
            }

            if let Err(err) = std::fs::write(&save_path, &markdown) {
                let _ = weak_editor_for_write_error.update(cx, |this, cx| {
                    this.abort_pending_drop_replace_after_save(cx)
                });
                let detail = err.to_string();
                let _ = cx.update_window(
                    window_handle,
                    move |_view: AnyView, window: &mut Window, cx: &mut App| {
                        let strings = cx.global::<I18nManager>().strings().clone();
                        let buttons = [strings.info_dialog_ok.as_str()];
                        let _ = window.prompt(
                            PromptLevel::Critical,
                            &strings.save_failed_title,
                            Some(&detail),
                            &buttons,
                            cx,
                        );
                    },
                );
                return;
            }

            let saved_path = save_path.clone();
            let replace_result = weak_editor.update(cx, move |this, cx| {
                this.apply_successful_save(saved_path, cx);
                if let Some(tab) = this.session.active_tab_mut() {
                    tab.pending.drop_replace_path = Some(drop_path);
                }
                this.replace_after_successful_save_async(cx)
            });
            let _ = cx.update_window(
                window_handle,
                move |_view: AnyView, window: &mut Window, cx: &mut App| match replace_result {
                    Ok(Ok(())) => window.set_window_edited(false),
                    Ok(Err(err)) => {
                        let strings = cx.global::<I18nManager>().strings().clone();
                        let buttons = [strings.info_dialog_ok.as_str()];
                        let _ = window.prompt(
                            PromptLevel::Critical,
                            &strings.open_failed_title,
                            Some(&err.to_string()),
                            &buttons,
                            cx,
                        );
                    }
                    Err(_) => {}
                },
            );
        })
        .detach();
    }

    pub fn replace_after_successful_save_async(&mut self, cx: &mut Context<Self>) -> Result<()> {
        let Some(drop_path) = self
            .session
            .active_tab_mut()
            .and_then(|t| t.pending.drop_replace_path.take())
        else {
            self.clear_pending_drop_replace_state(cx);
            return Ok(());
        };

        self.clear_pending_drop_replace_state(cx);
        self.replace_document_from_path(&drop_path, cx)
    }

    pub fn abort_pending_drop_replace_after_save(&mut self, cx: &mut Context<Self>) {
        if let Some(tab) = self.session.active_tab_mut() {
            tab.pending.drop_replace_after_save = false;
            tab.pending.show_drop_replace_dialog = false;
            tab.pending.drop_replace_path = None;
        }
        cx.notify();
    }

    pub fn clear_pending_drop_replace_state(&mut self, cx: &mut Context<Self>) {
        if let Some(tab) = self.session.active_tab_mut() {
            let had_path = tab.pending.drop_replace_path.take().is_some();
            let had_dialog = tab.pending.show_drop_replace_dialog;
            let had_after_save = tab.pending.drop_replace_after_save;
            let had_restore_focus = tab.pending.drop_replace_restore_focus.take().is_some();
            let had_state = had_path || had_dialog || had_after_save || had_restore_focus;
            tab.pending.show_drop_replace_dialog = false;
            tab.pending.drop_replace_after_save = false;
            if had_state {
                cx.notify();
            }
        }
    }

    pub fn show_drop_open_failed_prompt(
        &self,
        detail: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let strings = cx.global::<I18nManager>().strings().clone();
        let buttons = [strings.info_dialog_ok.as_str()];
        let _ = window.prompt(
            PromptLevel::Critical,
            &strings.open_failed_title,
            Some(&detail),
            &buttons,
            cx,
        );
    }

    /// Switches the active tab to the document at `path`, reusing the shared
    /// buffer when the document is already open elsewhere.
    pub fn replace_document_from_path(
        &mut self,
        path: &Path,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        let buffer = DocumentStore::open(path, cx).map_err(anyhow::Error::new)?;
        self.switch_active_tab_buffer(buffer, cx);
        if let Some(host) = self.host.clone() {
            host.record_recent_file(path, cx);
            host.on_document_path_changed(cx);
        }
        cx.notify();
        Ok(())
    }

    /// Re-points the active tab at another buffer: releases the old view,
    /// acquires the new one, and re-subscribes observers.
    fn switch_active_tab_buffer(&mut self, buffer: Entity<DocumentBuffer>, cx: &mut Context<Self>) {
        let old = {
            let Some(tab) = self.session.active_tab_mut() else {
                return;
            };
            if tab.buffer == buffer {
                return;
            }
            let old = std::mem::replace(&mut tab.buffer, buffer.clone());
            tab.pending.window_title_refresh = true;
            old
        };

        let (old_id, keep) = {
            let old_buffer = old.read(cx);
            (old_buffer.id, old_buffer.dirty)
        };
        cx.global_mut::<DocumentStore>().release(old_id, keep);
        let new_id = buffer.read(cx).id;
        cx.global_mut::<DocumentStore>().acquire(new_id);

        self.observe_buffer(buffer, cx);
        if !self.session.tabs().any(|tab| tab.buffer == old) {
            self.buffer_subscriptions.remove(&old_id);
        }
        self.sync_panes_with_active_tab(cx);
    }
}
