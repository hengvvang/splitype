//! File lifecycle state and disk persistence flows.

use std::path::{Path, PathBuf};

use anyhow::Result;
use gpui::*;

use crate::editor::Editor;
use config::language::I18nManager;
use core_contracts::AutoscrollStrategy;

/// Link navigation request deferred until a `Window` is available.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingOpenLink {
    pub prompt_target: String,
    pub open_target: String,
}

/// File lifecycle: path, dirty tracking, save/close and drop-replace flows.
#[derive(Default)]
pub struct FileState {
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub pending_save: bool,
    pub pending_save_as: bool,
    pub pending_open_link: Option<PendingOpenLink>,
    pub pending_window_edited: bool,
    pub pending_window_title_refresh: bool,
    pub show_unsaved_changes_dialog: bool,
    pub pending_close_after_save: bool,
    pub close_dialog_restore_focus: Option<EntityId>,
    pub pending_drop_replace_path: Option<PathBuf>,
    pub show_drop_replace_dialog: bool,
    pub pending_drop_replace_after_save: bool,
    pub drop_replace_restore_focus: Option<EntityId>,
}

impl Editor {
    /// The active tab's current authoritative raw text.
    pub fn serialized_document_text(&self, cx: &App) -> String {
        if let Some(tab) = self.session.active_tab() {
            tab.serialized_text(cx)
        } else if let Some(pane_id) = self.focused_pane_id.or_else(|| {
            self.session
                .root
                .tree
                .first_leaf_id()
                .map(core_contracts::PaneId)
        }) {
            if let Some(state) = self.session.empty_panes.get(&pane_id) {
                state.pane.serialize_text(cx).unwrap_or_default()
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    }

    pub fn save_dialog_defaults(&self) -> (PathBuf, Option<String>) {
        if let Some(tab) = self.session.active_tab() {
            if let Some(path) = tab.file.path.as_ref() {
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

    pub fn apply_successful_save(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if let Some(tab) = self.session.active_tab_mut() {
            tab.file.path = Some(path.clone());
            tab.file.dirty = false;
            tab.file.pending_window_edited = false;
            tab.file.pending_window_title_refresh = true;
            tab.file.pending_close_after_save = false;
            tab.file.close_dialog_restore_focus = None;
        }
        if let Some(host) = self.host.clone() {
            host.record_recent_file(&path, cx);
            host.sync_explorer_after_document_path_change(cx);
        }
        cx.notify();
    }

    pub fn update_tab_path(&mut self, from: &Path, to: &Path) {
        for tab in self.session.tabs_mut() {
            if let Some(path) = &tab.file.path {
                if path == from {
                    tab.file.path = Some(to.to_path_buf());
                    tab.file.pending_window_title_refresh = true;
                } else if path.starts_with(from) {
                    if let Ok(rel) = path.strip_prefix(from) {
                        let new_path = to.join(rel);
                        tab.file.path = Some(new_path);
                        tab.file.pending_window_title_refresh = true;
                    }
                }
            }
        }
    }

    pub fn save_to_existing_path(
        &mut self,
        path: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let markdown = self.serialized_document_text(cx);
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
            if let Some(path) = tab.file.path.clone() {
                let should_close_after_save = tab.file.pending_close_after_save;
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
            .is_some_and(|t| t.file.pending_close_after_save);
        let markdown = self.serialized_document_text(cx);
        let (default_dir, suggested_name) = self.save_dialog_defaults();
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
        if !tab.file.dirty {
            return true;
        }
        if let Some(path) = tab.file.path.clone() {
            let text = tab.serialized_text(cx);
            if std::fs::write(&path, text).is_ok() {
                if let Some(tab) = self.session.tab_mut(index) {
                    tab.file.dirty = false;
                    tab.file.pending_window_edited = false;
                    tab.file.pending_window_title_refresh = true;
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
            if self.session.tab(i).is_some_and(|t| t.file.dirty) {
                self.save_tab_at(i, window, cx);
            }
        }
    }

    pub fn save_document_as(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.save_document_via_prompt(window, cx);
    }

    pub fn abort_pending_close_after_save(&mut self, cx: &mut App) {
        if let Some(tab) = self.session.active_tab_mut() {
            tab.file.pending_close_after_save = false;
        }
        cx.notify(self.entity_id);
    }

    pub fn cancel_close_dialog(&mut self, cx: &mut Context<Self>) {
        if let Some(tab) = self.session.active_tab_mut() {
            tab.file.show_unsaved_changes_dialog = false;
            tab.file.pending_close_after_save = false;
            tab.file.close_dialog_restore_focus = None;
        }
        cx.notify();
    }

    pub fn save_and_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.session.active_tab_mut() {
            tab.file.pending_close_after_save = true;
        }
        self.save_document(window, cx);
    }

    pub fn discard_and_close(&mut self, cx: &mut Context<Self>) {
        if let Some(tab) = self.session.active_tab_mut() {
            tab.file.dirty = false;
            tab.file.show_unsaved_changes_dialog = false;
        }
        if let Some(host) = self.host.clone() {
            host.request_close_panel(self.panel_id, cx);
        }
        cx.notify();
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
            if tab.file.dirty {
                tab.file.pending_drop_replace_path = Some(path);
                if !tab.file.show_drop_replace_dialog {
                    tab.file.show_drop_replace_dialog = true;
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
        let pane = self.active_pane_state();
        pane.scroll.pending_autoscroll = Some(AutoscrollStrategy::Fit { margin: px(20.0) });
        cx.notify();
    }

    pub fn discard_pending_drop_replace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = self
            .session
            .active_tab_mut()
            .and_then(|t| t.file.pending_drop_replace_path.take())
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
        if tab.file.pending_drop_replace_path.is_none() {
            self.clear_pending_drop_replace_state(cx);
            return;
        }

        tab.file.show_drop_replace_dialog = false;
        tab.file.pending_drop_replace_after_save = true;

        if let Some(path) = tab.file.path.clone() {
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
            .and_then(|t| t.file.pending_drop_replace_path.take())
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
            .and_then(|t| t.file.pending_drop_replace_path.clone())
        else {
            self.clear_pending_drop_replace_state(cx);
            return;
        };
        let markdown = self.serialized_document_text(cx);
        let (default_dir, suggested_name) = self.save_dialog_defaults();
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
                    tab.file.pending_drop_replace_path = Some(drop_path);
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
            .and_then(|t| t.file.pending_drop_replace_path.take())
        else {
            self.clear_pending_drop_replace_state(cx);
            return Ok(());
        };

        self.clear_pending_drop_replace_state(cx);
        self.replace_document_from_path(&drop_path, cx)
    }

    pub fn abort_pending_drop_replace_after_save(&mut self, cx: &mut Context<Self>) {
        if let Some(tab) = self.session.active_tab_mut() {
            tab.file.pending_drop_replace_after_save = false;
            tab.file.show_drop_replace_dialog = false;
            tab.file.pending_drop_replace_path = None;
        }
        let pane = self.active_pane_state();
        pane.scroll.pending_autoscroll = Some(AutoscrollStrategy::Fit { margin: px(20.0) });
        cx.notify();
    }

    pub fn clear_pending_drop_replace_state(&mut self, cx: &mut Context<Self>) {
        if let Some(tab) = self.session.active_tab_mut() {
            let had_path = tab.file.pending_drop_replace_path.take().is_some();
            let had_dialog = tab.file.show_drop_replace_dialog;
            let had_after_save = tab.file.pending_drop_replace_after_save;
            let had_restore_focus = tab.file.drop_replace_restore_focus.take().is_some();
            let had_state = had_path || had_dialog || had_after_save || had_restore_focus;
            tab.file.show_drop_replace_dialog = false;
            tab.file.pending_drop_replace_after_save = false;
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

    pub fn replace_document_from_path(
        &mut self,
        path: &Path,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        let content = std::fs::read_to_string(path)?;
        if let Some(tab) = self.session.active_tab_mut() {
            tab.text = content;
            tab.file.path = Some(path.to_path_buf());
            tab.file.dirty = false;
            tab.file.pending_window_edited = false;
            tab.file.pending_window_title_refresh = true;
            tab.document_revision = tab.document_revision.wrapping_add(1);
            tab.cached_word_count = None;
        }
        self.sync_panes_with_active_tab(cx);
        if let Some(host) = self.host.clone() {
            host.record_recent_file(path, cx);
            host.sync_explorer_after_document_path_change(cx);
        }
        cx.notify();
        Ok(())
    }
}
