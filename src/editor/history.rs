//! Undo history management, selection snapshot restoration, and update-check flow.
//!
//! Types (`UndoSelectionSnapshot`, `HistoryEntry`, `PendingUndoCapture`) are
//! defined in `super::editor`; constants live on `Editor`.

use std::time::Instant;

use gpui::*;

use super::controller::{
    BlockData, Editor, EditorMode, HistoryEntry, InfoDialogKind, PendingUndoCapture,
    UndoCaptureKind, UndoSelectionSnapshot,
};

impl Editor {
    pub(crate) fn empty_selection_snapshot() -> UndoSelectionSnapshot {
        UndoSelectionSnapshot {
            range: 0..0,
            reversed: false,
        }
    }

    pub(crate) fn capture_source_selection_snapshot(&self, cx: &App) -> UndoSelectionSnapshot {
        if let Some(snapshot) = self.cross_block_source_selection_snapshot(cx) {
            return snapshot;
        }

        if self.mode == EditorMode::Source {
            return self
                .document
                .first_root()
                .map(|block| {
                    let block_ref = block.read(cx);
                    UndoSelectionSnapshot {
                        range: block_ref.selected_range.clone(),
                        reversed: block_ref.selection_reversed,
                    }
                })
                .unwrap_or_else(Self::empty_selection_snapshot);
        }

        let Some(target) = self.current_edit_target_from_state(cx) else {
            return self.undo.last_selection_snapshot.clone();
        };
        let Some(mapping) = self
            .build_source_target_mappings(cx)
            .into_iter()
            .find(|mapping| mapping.entity.entity_id() == target.entity_id())
        else {
            return self.undo.last_selection_snapshot.clone();
        };

        let selected_range = target.read(cx).selected_range.clone();
        let content_range = target
            .read(cx)
            .current_range_to_markdown_range(selected_range);
        let max_offset = mapping.content_to_source.len().saturating_sub(1);
        let start = mapping.full_source_range.start
            + mapping.content_to_source[content_range.start.min(max_offset)];
        let end = mapping.full_source_range.start
            + mapping.content_to_source[content_range.end.min(max_offset)];

        UndoSelectionSnapshot {
            range: start..end,
            reversed: target.read(cx).selection_reversed,
        }
    }

    pub(crate) fn capture_history_entry(&self, kind: UndoCaptureKind, cx: &App) -> HistoryEntry {
        HistoryEntry {
            source_text: self.current_document_source(cx),
            selection: self.capture_source_selection_snapshot(cx),
            timestamp: Instant::now(),
            kind,
        }
    }

    pub(crate) fn capture_stable_history_entry(&self, kind: UndoCaptureKind) -> HistoryEntry {
        HistoryEntry {
            source_text: self.undo.last_stable_source_text.clone(),
            selection: self.undo.last_selection_snapshot.clone(),
            timestamp: Instant::now(),
            kind,
        }
    }

    pub(crate) fn prepare_undo_capture(&mut self, kind: UndoCaptureKind, cx: &mut Context<Self>) {
        if self.undo.restore_in_progress || self.undo.pending_capture.is_some() {
            return;
        }
        self.undo.pending_capture = Some(PendingUndoCapture {
            snapshot: self.capture_history_entry(kind, cx),
        });
    }

    pub(crate) fn prepare_undo_capture_from_stable_snapshot(&mut self, kind: UndoCaptureKind) {
        if self.undo.restore_in_progress || self.undo.pending_capture.is_some() {
            return;
        }
        self.undo.pending_capture = Some(PendingUndoCapture {
            snapshot: self.capture_stable_history_entry(kind),
        });
    }

    pub(crate) fn refresh_stable_document_snapshot(&mut self, cx: &App) {
        self.undo.last_selection_snapshot = self.capture_source_selection_snapshot(cx);
        self.undo.last_stable_source_text = self.current_document_source(cx);
    }

    pub(crate) fn finalize_pending_undo_capture(&mut self, cx: &mut Context<Self>) {
        if self.undo.restore_in_progress {
            self.undo.pending_capture = None;
            return;
        }

        let Some(pending) = self.undo.pending_capture.take() else {
            self.refresh_stable_document_snapshot(cx);
            return;
        };

        let current_source = self.current_document_source(cx);
        if current_source == pending.snapshot.source_text {
            self.refresh_stable_document_snapshot(cx);
            return;
        }

        // A fresh edit invalidates any forward history available for redo.
        self.undo.redo_entries.clear();

        let should_merge = matches!(pending.snapshot.kind, UndoCaptureKind::CoalescibleText)
            && self.undo.undo_entries.last().is_some_and(|entry| {
                matches!(entry.kind, UndoCaptureKind::CoalescibleText)
                    && pending
                        .snapshot
                        .timestamp
                        .saturating_duration_since(entry.timestamp)
                        <= Self::HISTORY_COALESCE_WINDOW
            });
        if !should_merge {
            self.undo.undo_entries.push(pending.snapshot);
            if self.undo.undo_entries.len() > Self::HISTORY_LIMIT {
                let overflow = self.undo.undo_entries.len() - Self::HISTORY_LIMIT;
                self.undo.undo_entries.drain(0..overflow);
            }
        }
        self.refresh_stable_document_snapshot(cx);
    }

    pub(crate) fn apply_selection_snapshot_in_current_mode(
        &mut self,
        snapshot: &UndoSelectionSnapshot,
        cx: &mut Context<Self>,
    ) {
        match self.mode {
            EditorMode::Source => {
                let Some(block) = self.document.first_root().cloned() else {
                    return;
                };
                let len = block.read(cx).visible_len();
                let selected_range = snapshot.range.start.min(len)..snapshot.range.end.min(len);
                block.update(cx, move |block, cx| {
                    block.selected_range = selected_range.clone();
                    block.selection_reversed = snapshot.reversed;
                    block.marked_range = None;
                    block.vertical_motion_x = None;
                    block.cursor_blink_epoch = Instant::now();
                    cx.notify();
                });
                self.focus.pending = Some(block.entity_id());
                self.focus.active_entity = Some(block.entity_id());
            }
            EditorMode::Wysiwyg => {
                if self.apply_cross_block_selection_snapshot_if_possible(snapshot, cx) {
                    return;
                }

                let mappings = self.build_source_target_mappings(cx);
                let exact_mapping = mappings.iter().find(|mapping| {
                    let contains_start = Self::source_range_contains(
                        &mapping.full_source_range,
                        snapshot.range.start,
                    );
                    let contains_end =
                        Self::source_range_contains(&mapping.full_source_range, snapshot.range.end);
                    if !contains_start || !contains_end {
                        return false;
                    }
                    let local_start = snapshot
                        .range
                        .start
                        .saturating_sub(mapping.full_source_range.start);
                    let local_end = snapshot
                        .range
                        .end
                        .saturating_sub(mapping.full_source_range.start);
                    let content_start = mapping.source_to_content
                        [local_start.min(mapping.source_to_content.len().saturating_sub(1))];
                    let content_end = mapping.source_to_content
                        [local_end.min(mapping.source_to_content.len().saturating_sub(1))];
                    let max_content = mapping.content_to_source.len().saturating_sub(1);
                    mapping.content_to_source[content_start.min(max_content)] == local_start
                        && mapping.content_to_source[content_end.min(max_content)] == local_end
                });

                if let Some(mapping) = exact_mapping {
                    let local_start = snapshot.range.start - mapping.full_source_range.start;
                    let local_end = snapshot.range.end - mapping.full_source_range.start;
                    let content_start = mapping.source_to_content[local_start];
                    let content_end = mapping.source_to_content[local_end];
                    let selected_range = mapping
                        .entity
                        .read(cx)
                        .markdown_range_to_current_range(content_start..content_end);
                    mapping.entity.update(cx, move |block, cx| {
                        block.selected_range = selected_range.clone();
                        block.selection_reversed = snapshot.reversed;
                        block.marked_range = None;
                        block.vertical_motion_x = None;
                        block.cursor_blink_epoch = Instant::now();
                        cx.notify();
                    });
                    self.focus.pending = Some(mapping.entity.entity_id());
                    self.focus.active_entity = Some(mapping.entity.entity_id());
                    return;
                }

                let caret_offset = snapshot.range.end;
                let best = mappings.iter().min_by_key(|mapping| {
                    Self::source_offset_distance(&mapping.full_source_range, caret_offset)
                });
                let Some(mapping) = best else {
                    self.focus.pending = self.first_focusable_entity_id(cx);
                    self.focus.active_entity = self.focus.pending;
                    return;
                };
                let local_source = if caret_offset <= mapping.full_source_range.start {
                    0
                } else if caret_offset >= mapping.full_source_range.end {
                    mapping.full_source_range.len()
                } else {
                    caret_offset - mapping.full_source_range.start
                };
                let content_offset = mapping.source_to_content
                    [local_source.min(mapping.source_to_content.len().saturating_sub(1))];
                let current_offset = mapping
                    .entity
                    .read(cx)
                    .markdown_offset_to_current_offset(content_offset);
                mapping.entity.update(cx, move |block, cx| {
                    block.assign_collapsed_selection_offset(
                        current_offset,
                        crate::editor::block::CollapsedCaretAffinity::Default,
                        None,
                    );
                    block.marked_range = None;
                    block.cursor_blink_epoch = Instant::now();
                    cx.notify();
                });
                self.focus.pending = Some(mapping.entity.entity_id());
                self.focus.active_entity = Some(mapping.entity.entity_id());
            }
        }
    }

    pub(crate) fn source_range_contains(range: &std::ops::Range<usize>, offset: usize) -> bool {
        if range.start == range.end {
            offset == range.start
        } else {
            offset >= range.start && offset <= range.end
        }
    }

    pub(crate) fn source_offset_distance(range: &std::ops::Range<usize>, offset: usize) -> usize {
        if Self::source_range_contains(range, offset) {
            0
        } else if offset < range.start {
            range.start - offset
        } else {
            offset.saturating_sub(range.end)
        }
    }

    pub(crate) fn restore_history_entry(&mut self, entry: &HistoryEntry, cx: &mut Context<Self>) {
        match self.mode {
            EditorMode::Wysiwyg => {
                let mut roots = Self::parse_document(cx, &entry.source_text);
                if roots.is_empty() {
                    roots.push(Self::new_block(cx, BlockData::paragraph(String::new())));
                }
                self.document.replace_blocks(roots, cx);
                self.rebuild_table_runtimes(cx);
                self.rebuild_image_runtimes(cx);
            }
            EditorMode::Source => {
                let block = Self::new_block(cx, BlockData::paragraph(entry.source_text.clone()));
                block.update(cx, |block, _cx| block.set_source_document_mode());
                self.document.replace_blocks(vec![block], cx);
                self.tables.cells.clear();
            }
        }

        self.apply_selection_snapshot_in_current_mode(&entry.selection, cx);
        self.focus.pending_scroll_active_block_into_view = true;
        self.focus.pending_scroll_recheck_after_layout = true;
        self.scroll.last_viewport_size = None;
        self.refresh_stable_document_snapshot(cx);
    }

    pub(crate) fn undo_document(&mut self, cx: &mut Context<Self>) {
        let Some(entry) = self.undo.undo_entries.pop() else {
            return;
        };

        // Snapshot the current document so redo can step forward to it.
        let current = self.capture_history_entry(UndoCaptureKind::NonCoalescible, cx);
        self.undo.pending_capture = None;
        self.undo.restore_in_progress = true;
        self.clear_cross_block_selection(cx);
        self.restore_history_entry(&entry, cx);
        self.undo.restore_in_progress = false;
        self.undo.redo_entries.push(current);
        self.mark_dirty(cx);
        self.sync_table_axis_visuals(cx);
        self.dismiss_contextual_overlays(cx);
        cx.notify();
    }

    pub(crate) fn redo_document(&mut self, cx: &mut Context<Self>) {
        let Some(entry) = self.undo.redo_entries.pop() else {
            return;
        };

        // Snapshot the current document so undo can step back to it again.
        let current = self.capture_history_entry(UndoCaptureKind::NonCoalescible, cx);
        self.undo.pending_capture = None;
        self.undo.restore_in_progress = true;
        self.clear_cross_block_selection(cx);
        self.restore_history_entry(&entry, cx);
        self.undo.restore_in_progress = false;
        self.undo.undo_entries.push(current);
        self.mark_dirty(cx);
        self.sync_table_axis_visuals(cx);
        self.dismiss_contextual_overlays(cx);
        cx.notify();
    }
}

// ── Update-check flow ────────────────────────────────────────────────────

use futures::FutureExt;
use futures::channel::oneshot;

use crate::infra::i18n::I18nManager;
use crate::infra::net::update_checker::{
    self as update_check, UpdateCheckResult, UpdateVersionInfo,
};

impl Editor {
    pub(crate) fn request_check_updates(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.file.show_unsaved_changes_dialog {
            return;
        }
        if self.chrome.update_check_in_progress {
            self.show_info_dialog(InfoDialogKind::CheckForUpdates, cx);
            return;
        }

        self.chrome.update_check_in_progress = true;
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
                editor.chrome.update_check_in_progress = false;
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
