//! Context menu state, lifecycle, and action dispatchers for WYSIWYG and
//! Source Code editing modes.
//!
//! Submenu hover state is debounced with a timer to ensure fluid navigation.
//! Rendering lives in `context_menu_render.rs` and table axis actions live in
//! `context_menu_actions.rs`.

use std::time::Duration;

use gpui::*;

use crate::editor::engine::controller::{Editor, EditorPaneKind, TableAxisSelection};
use crate::editor::input::actions::DismissTransientUi;
use crate::editor::panes::document_pane::dialogs::TableInsertDialogState;
use crate::editor::document::block::Block;
use crate::model::block::table::TableData;
use crate::model::parse::BlockKind;

/// Active secondary submenu in the context menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ContextSubmenu {
    TextFormat,
    ParagraphSettings,
    Insert,
}

/// Target block position for inserting a native table.
#[derive(Clone, Copy)]
pub(crate) enum TableInsertTarget {
    /// Insert the table immediately after the referenced block.
    After(EntityId),
    /// Append the table to the end of the current root list.
    Append,
}

/// Tooltip state for a hovered footnote (reference or definition header):
/// the definition text plus the pointer position in window coordinates.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FootnoteTooltipState {
    pub(crate) content: SharedString,
    pub(crate) position: Point<Pixels>,
}

/// Context menu currently open in the editor (WYSIWYG or Source Code).
#[derive(Clone)]
pub(crate) enum ContextMenuState {
    /// General editor context menu with submenus for text format, paragraph settings, and insert.
    Edit {
        position: Point<Pixels>,
        target: TableInsertTarget,
        target_entity: Option<EntityId>,
        active_submenu: Option<ContextSubmenu>,
        submenu_hovered: bool,
        menu_hovered_submenu: Option<ContextSubmenu>,
    },
    /// Table row or column context menu for an existing native table.
    TableAxis {
        position: Point<Pixels>,
        selection: TableAxisSelection,
    },
}

impl Editor {
    pub(crate) fn root_ancestor_entity_id(&self, entity_id: EntityId) -> EntityId {
        let mut current = entity_id;
        while let Some(location) = self.doc().find_block_location(current) {
            let Some(parent) = location.parent else {
                break;
            };
            current = parent.entity_id();
        }
        current
    }

    /// Opens the general edit context menu at pointer location.
    pub(crate) fn open_edit_context_menu(
        &mut self,
        position: Point<Pixels>,
        target: TableInsertTarget,
        target_entity: Option<EntityId>,
        cx: &mut Context<Self>,
    ) {
        self.context_menu_submenu_close_task = None;
        self.context_menu = Some(ContextMenuState::Edit {
            position,
            target,
            target_entity,
            active_submenu: None,
            submenu_hovered: false,
            menu_hovered_submenu: None,
        });
        cx.notify();
    }


    pub(crate) fn open_table_axis_context_menu(
        &mut self,
        position: Point<Pixels>,
        selection: TableAxisSelection,
        cx: &mut Context<Self>,
    ) {
        if !self.is_wysiwyg() {
            return;
        }

        self.context_menu_submenu_close_task = None;
        self.context_menu = Some(ContextMenuState::TableAxis {
            position,
            selection,
        });
        cx.notify();
    }

    pub(crate) fn close_table_insert_dialog(&mut self, cx: &mut Context<Self>) {
        if self.table_insert_dialog.take().is_some() {
            cx.notify();
        }
    }

    pub(crate) fn close_context_menu(&mut self, cx: &mut Context<Self>) {
        let had_menu = self.context_menu.take().is_some();
        let had_submenu_close = self.context_menu_submenu_close_task.take().is_some();
        if had_menu || had_submenu_close {
            cx.notify();
        }
    }

    pub(crate) fn dismiss_contextual_overlays(&mut self, cx: &mut Context<Self>) {
        let had_menu = self.context_menu.take().is_some();
        let had_dialog = self.table_insert_dialog.take().is_some();
        let had_picker = self.table_size_picker.take().is_some();
        let had_submenu_close = self.context_menu_submenu_close_task.take().is_some();
        if had_menu || had_dialog || had_picker || had_submenu_close {
            cx.notify();
        }
    }

    /// Schedules smooth submenu closing with a short debounce window.
    pub(crate) fn schedule_context_menu_submenu_close(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.context_menu, Some(ContextMenuState::Edit { .. })) {
            return;
        }

        self.context_menu_submenu_close_token = self.context_menu_submenu_close_token.wrapping_add(1);
        let token = self.context_menu_submenu_close_token;
        let weak_editor = cx.entity().downgrade();
        self.context_menu_submenu_close_task = Some(cx.spawn(
            async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
                cx.background_executor()
                    .timer(Duration::from_millis(250))
                    .await;
                let _ = weak_editor.update(cx, |editor, cx| {
                    if editor.context_menu_submenu_close_token != token {
                        return;
                    }
                    editor.context_menu_submenu_close_task = None;
                    let Some(ContextMenuState::Edit {
                        active_submenu,
                        submenu_hovered,
                        menu_hovered_submenu,
                        ..
                    }) = editor.context_menu.as_mut()
                    else {
                        return;
                    };
                    if menu_hovered_submenu.is_none() && !*submenu_hovered && active_submenu.is_some() {
                        *active_submenu = None;
                        cx.notify();
                    }
                });
            },
        ));
    }

    /// Updates hover tracking across the parent menu row and the submenu panel.
    pub(crate) fn set_context_menu_submenu_hover(
        &mut self,
        submenu: Option<ContextSubmenu>,
        is_submenu_body: bool,
        cx: &mut Context<Self>,
    ) {
        let mut changed = false;
        let mut should_clear_close = false;
        let mut should_schedule_close = false;

        if let Some(ContextMenuState::Edit {
            active_submenu,
            submenu_hovered,
            menu_hovered_submenu,
            ..
        }) = self.context_menu.as_mut()
        {
            if is_submenu_body {
                let entering_body = submenu.is_some();
                if *submenu_hovered != entering_body {
                    *submenu_hovered = entering_body;
                    changed = true;
                }
                if entering_body {
                    should_clear_close = true;
                } else if menu_hovered_submenu.is_none() {
                    should_schedule_close = true;
                }
            } else if let Some(target_submenu) = submenu {
                *menu_hovered_submenu = Some(target_submenu);
                if *active_submenu != Some(target_submenu) {
                    *active_submenu = Some(target_submenu);
                    changed = true;
                }
                should_clear_close = true;
            } else {
                *menu_hovered_submenu = None;
                if !*submenu_hovered {
                    should_schedule_close = true;
                }
            }
        }

        if should_clear_close {
            self.context_menu_submenu_close_token = self.context_menu_submenu_close_token.wrapping_add(1);
            self.context_menu_submenu_close_task = None;
        }
        if should_schedule_close {
            self.schedule_context_menu_submenu_close(cx);
        }
        if changed {
            cx.notify();
        }
    }

    pub(crate) fn on_editor_context_menu_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        if let Some(endpoint) = self.cross_block_endpoint_for_point(event.position, cx) {
            self.focus_block(endpoint.entity_id);
            if let Some(block) = self.focusable_entity_by_id(endpoint.entity_id) {
                block.update(cx, |block, cx| {
                    if block.selected_range.is_empty() {
                        block.move_to(endpoint.offset, cx);
                    }
                });
            }
            let target = TableInsertTarget::After(self.root_ancestor_entity_id(endpoint.entity_id));
            self.open_edit_context_menu(event.position, target, Some(endpoint.entity_id), cx);
        } else {
            let active_id = self.active_pane_state().focus.active_entity;
            self.open_edit_context_menu(event.position, TableInsertTarget::Append, active_id, cx);
        }
    }

    pub(crate) fn on_source_context_menu_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        let pane_id = self.active_pane_id();
        let source_block = if let Some(b) = self.pane_state_ref(pane_id).and_then(|p| p.source_block.clone()) {
            b
        } else {
            self.sync_source_pane(pane_id, cx);
            let Some(b) = self.pane_state_ref(pane_id).and_then(|p| p.source_block.clone()) else {
                return;
            };
            b
        };
        self.focus_block(source_block.entity_id());
        source_block.update(cx, |block, cx| {
            if block.selected_range.is_empty() {
                let offset = block.index_for_mouse_position(event.position);
                block.move_to(offset, cx);
            }
        });
        self.open_edit_context_menu(event.position, TableInsertTarget::Append, Some(source_block.entity_id()), cx);
    }

    pub(crate) fn on_block_context_menu_mouse_down(
        &mut self,
        entity_id: EntityId,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        if self.table_cell_binding(entity_id).is_some() {
            return;
        }
        self.focus_block(entity_id);
        if let Some(block) = self.focusable_entity_by_id(entity_id) {
            block.update(cx, |block, cx| {
                if block.selected_range.is_empty() {
                    let offset = block.index_for_mouse_position(event.position);
                    block.move_to(offset, cx);
                }
            });
        }
        let target = TableInsertTarget::After(self.root_ancestor_entity_id(entity_id));
        self.open_edit_context_menu(event.position, target, Some(entity_id), cx);
    }

    pub(crate) fn on_dismiss_context_menu_overlay(
        &mut self,
        _event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dismiss_contextual_overlays(cx);
    }

    pub(crate) fn on_dismiss_transient_ui(
        &mut self,
        _: &DismissTransientUi,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dismiss_contextual_overlays(cx);
    }

    pub(crate) fn on_open_table_insert_dialog(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (target, position) = match self.context_menu.take() {
            Some(ContextMenuState::Edit { target, position, .. }) => (target, Some(position)),
            _ => (TableInsertTarget::Append, None),
        };
        self.context_menu_submenu_close_task = None;
        self.table_insert_dialog = Some(TableInsertDialogState::new(target, 4, 3, position));
        cx.notify();
    }

    pub(crate) fn set_table_insert_hover(
        &mut self,
        rows: Option<usize>,
        cols: Option<usize>,
        cx: &mut Context<Self>,
    ) {
        if let Some(dialog) = self.table_insert_dialog.as_mut() {
            dialog.hovered_rows = rows;
            dialog.hovered_cols = cols;
            if let (Some(r), Some(c)) = (rows, cols) {
                dialog.rows = r.clamp(1, 8);
                dialog.columns = c.clamp(1, 8);
            }
            cx.notify();
        }
    }

    pub(crate) fn set_table_insert_size(
        &mut self,
        rows: usize,
        cols: usize,
        cx: &mut Context<Self>,
    ) {
        if let Some(dialog) = self.table_insert_dialog.as_mut() {
            dialog.rows = rows.clamp(1, 8);
            dialog.columns = cols.clamp(1, 8);
            cx.notify();
        }
    }

    pub(crate) fn handle_table_insert_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        let key = event.keystroke.key.as_str();
        match key {
            "escape" => {
                self.close_table_insert_dialog(cx);
            }
            "enter" => {
                // Confirm insert on Enter key
                self.confirm_table_insert_action(cx);
            }
            _ => {}
        }
    }


    pub(crate) fn on_cancel_table_insert_dialog(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_table_insert_dialog(cx);
    }

    pub(crate) fn on_confirm_table_insert_dialog(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.confirm_table_insert_action(cx);
    }

    pub(crate) fn confirm_table_insert_action(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = self.table_insert_dialog.take() else {
            return;
        };

        let body_rows = dialog.rows.saturating_sub(1).max(1);
        let columns = dialog.columns.max(1);
        let table = TableData::new_empty(body_rows, columns);

        if self.is_source_code() {
            let pane_id = self.active_pane_id();
            if let Some(block) = self.pane_state_ref(pane_id).and_then(|p| p.source_block.clone()) {
                block.update(cx, |block, cx| {
                    let cursor = block.cursor_offset();
                    let table_md = format!("\n{}\n", table.serialize_markdown());
                    let insert_len = table_md.len();
                    block.replace_text_in_display_range(
                        cursor..cursor,
                        &table_md,
                        Some(insert_len..insert_len),
                        false,
                        cx,
                    );
                });
            }
            self.mark_dirty(cx);
            cx.notify();
            return;
        }

        let new_block = Self::new_table_block(cx, table);

        match dialog.target {
            TableInsertTarget::After(entity_id) => {
                if let Some(location) = self.doc().find_block_location(entity_id) {
                    self.doc_mut().insert_blocks_at(
                        location.parent,
                        location.index + 1,
                        vec![new_block.clone()],
                        cx,
                    );
                } else {
                    let root_count = self.doc().root_count();
                    self.doc_mut()
                        .insert_blocks_at(None, root_count, vec![new_block.clone()], cx);
                }
            }
            TableInsertTarget::Append => {
                let root_count = self.doc().root_count();
                self.doc_mut()
                    .insert_blocks_at(None, root_count, vec![new_block.clone()], cx);
            }
        }

        self.ensure_trailing_paragraph_after_structural(&new_block, cx);
        self.rebuild_table_grids(cx);
        if let Some(first_cell) = new_block
            .read(cx)
            .table_grid
            .as_ref()
            .and_then(|runtime| runtime.header.first())
        {
            self.focus_block(first_cell.entity_id());
        }
        self.mark_dirty(cx);
        self.request_active_block_scroll_into_view(self.active_pane_id(), cx);
        cx.notify();
    }

    /// Determines if there is an active text selection in WYSIWYG or Source mode.
    pub(crate) fn context_menu_has_selection(&self, cx: &App) -> bool {
        let pane_id = self.active_pane_id();
        let pane_state = self.pane_state_ref(pane_id);
        if self.is_source_code() {
            if let Some(block_entity) = pane_state.and_then(|p| p.source_block.as_ref()) {
                return !block_entity.read(cx).selected_range.is_empty();
            }
            false
        } else {
            if self.active_pane_selection().cross_block.is_some() {
                return true;
            }
            if let Some(active_id) = pane_state.and_then(|p| p.focus.active_entity) {
                if let Some(block) = self.focusable_entity_by_id(active_id) {
                    return !block.read(cx).selected_range.is_empty();
                }
            }
            false
        }
    }

    /// Resolves the target block entity for formatting and block operations.
    pub(crate) fn context_menu_target_block(
        &self,
        target_entity: Option<EntityId>,
    ) -> Option<Entity<Block>> {
        let pane_id = self.active_pane_id();
        let pane_state = self.pane_state_ref(pane_id);
        if self.is_source_code() {
            pane_state.and_then(|p| p.source_block.clone())
        } else {
            target_entity
                .and_then(|id| self.focusable_entity_by_id(id))
                .or_else(|| {
                    pane_state
                        .and_then(|p| p.focus.active_entity)
                        .and_then(|id| self.focusable_entity_by_id(id))
                })
                .or_else(|| self.doc().blocks().first().map(|b| b.entity.clone()))
        }
    }

    // ─── Clipboard Actions ───

    pub(crate) fn on_context_menu_cut(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_context_menu(cx);
        if self.is_source_code() {
            if let Some(block) = self.active_pane_state().source_block.clone() {
                block.update(cx, |block, cx| {
                    if !block.selected_range.is_empty() {
                        let text = block.selected_text();
                        cx.write_to_clipboard(ClipboardItem::new_string(text));
                        block.replace_text_in_display_range(
                            block.selected_range.clone(),
                            "",
                            Some(0..0),
                            false,
                            cx,
                        );
                    }
                });
            }
        } else if let Some(markdown) = self.cross_block_selected_markdown(cx) {
            cx.write_to_clipboard(ClipboardItem::new_string(markdown));
            self.delete_cross_block_selection(cx);
        } else if let Some(active_id) = self.active_pane_state().focus.active_entity {
            if let Some(block) = self.focusable_entity_by_id(active_id) {
                block.update(cx, |block, cx| {
                    if !block.selected_range.is_empty() {
                        let text = block.selected_text();
                        cx.write_to_clipboard(ClipboardItem::new_string(text));
                        block.replace_text_in_display_range(
                            block.selected_range.clone(),
                            "",
                            Some(0..0),
                            false,
                            cx,
                        );
                    }
                });
            }
        }
        self.mark_dirty(cx);
        cx.notify();
    }

    pub(crate) fn on_context_menu_copy(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_context_menu(cx);
        if self.is_source_code() {
            let pane_id = self.active_pane_id();
            if let Some(block) = self.pane_state_ref(pane_id).and_then(|p| p.source_block.as_ref()) {
                let text = block.read(cx).selected_text();
                if !text.is_empty() {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
            }
        } else if let Some(markdown) = self.cross_block_selected_markdown(cx) {
            cx.write_to_clipboard(ClipboardItem::new_string(markdown));
        } else {
            let pane_id = self.active_pane_id();
            if let Some(active_id) = self.pane_state_ref(pane_id).and_then(|p| p.focus.active_entity) {
                if let Some(block) = self.focusable_entity_by_id(active_id) {
                    let text = block.read(cx).selected_text();
                    if !text.is_empty() {
                        cx.write_to_clipboard(ClipboardItem::new_string(text));
                    }
                }
            }
        }
    }

    pub(crate) fn on_context_menu_paste(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        let Some(clipboard_item) = cx.read_from_clipboard() else {
            return;
        };
        let Some(text) = clipboard_item.text() else {
            return;
        };
        if let Some(block) = self.context_menu_target_block(target) {
            self.focus_block(block.entity_id());
            block.update(cx, |block, cx| {
                let range = block.selected_range.clone();
                let text_len = text.len();
                block.replace_text_in_display_range(
                    range,
                    &text,
                    Some(text_len..text_len),
                    false,
                    cx,
                );
            });
        }
        self.mark_dirty(cx);
        cx.notify();
    }

    pub(crate) fn on_context_menu_paste_plain(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        let Some(clipboard_item) = cx.read_from_clipboard() else {
            return;
        };
        let Some(text) = clipboard_item.text() else {
            return;
        };
        if let Some(block) = self.context_menu_target_block(target) {
            self.focus_block(block.entity_id());
            block.update(cx, |block, cx| {
                let range = block.selected_range.clone();
                let text_len = text.len();
                block.replace_text_in_display_range(
                    range,
                    &text,
                    Some(text_len..text_len),
                    false,
                    cx,
                );
            });
        }
        self.mark_dirty(cx);
        cx.notify();
    }

    pub(crate) fn on_context_menu_select_all(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_context_menu(cx);
        if self.is_source_code() {
            if let Some(block) = self.active_pane_state().source_block.clone() {
                block.update(cx, |block, cx| {
                    let len = block.display_len();
                    block.selected_range = 0..len;
                    block.selection_reversed = false;
                    cx.notify();
                });
            }
        } else {
            self.select_all_wysiwyg_document(cx);
        }
        cx.notify();
    }

    // ─── Inline Formatting Operations ───

    // ─── Pure Source Code Mode Helpers ───

    pub(crate) fn source_editor_block(&self) -> Option<Entity<Block>> {
        let pane_id = self.active_pane_id();
        self.pane_state_ref(pane_id).and_then(|p| p.source_block.clone())
    }

    pub(crate) fn apply_source_inline_markup(
        &mut self,
        template: &str,
        caret_offset_inside_template: usize,
        wrap_prefix: &str,
        wrap_suffix: &str,
        cx: &mut Context<Self>,
    ) {
        let Some(block) = self.source_editor_block() else {
            return;
        };
        self.focus_block(block.entity_id());
        block.update(cx, |block, cx| {
            let cursor = block.cursor_offset();
            let range = block.selected_range.clone();
            if range.is_empty() {
                block.replace_text_in_display_range(
                    cursor..cursor,
                    template,
                    Some(caret_offset_inside_template..caret_offset_inside_template),
                    false,
                    cx,
                );
            } else {
                let text = block.selected_text();
                let inner_len = text.len();
                let wrapped = format!("{wrap_prefix}{text}{wrap_suffix}");
                let prefix_len = wrap_prefix.len();
                block.replace_text_in_display_range(
                    range,
                    &wrapped,
                    Some(prefix_len..prefix_len + inner_len),
                    false,
                    cx,
                );
            }
        });
        self.mark_dirty(cx);
        cx.notify();
    }

    pub(crate) fn apply_source_prefix(&mut self, prefix: &str, cx: &mut Context<Self>) {
        let Some(block) = self.source_editor_block() else {
            return;
        };
        self.focus_block(block.entity_id());
        block.update(cx, |block, cx| {
            let cursor = block.cursor_offset();
            let range = block.selected_range.clone();
            let prefix_len = prefix.len();
            if range.is_empty() {
                block.replace_text_in_display_range(
                    cursor..cursor,
                    prefix,
                    Some(prefix_len..prefix_len),
                    false,
                    cx,
                );
            } else {
                let text = block.selected_text();
                let inner_len = text.len();
                let wrapped = format!("{prefix}{text}");
                block.replace_text_in_display_range(
                    range,
                    &wrapped,
                    Some(prefix_len..prefix_len + inner_len),
                    false,
                    cx,
                );
            }
        });
        self.mark_dirty(cx);
        cx.notify();
    }

    pub(crate) fn apply_source_snippet(
        &mut self,
        snippet: &str,
        caret_offset: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(block) = self.source_editor_block() else {
            return;
        };
        self.focus_block(block.entity_id());
        block.update(cx, |block, cx| {
            let cursor = block.cursor_offset();
            let len = snippet.len();
            let offset = caret_offset.min(len);
            block.replace_text_in_display_range(
                cursor..cursor,
                snippet,
                Some(offset..offset),
                false,
                cx,
            );
        });
        self.mark_dirty(cx);
        cx.notify();
    }

    // ─── Pure WYSIWYG Mode Helpers ───

    pub(crate) fn apply_wysiwyg_inline_markup(
        &mut self,
        target_entity: Option<EntityId>,
        empty_markup: &str,
        cursor_offset_in_empty: usize,
        wrap_prefix: &str,
        wrap_suffix: &str,
        cx: &mut Context<Self>,
    ) {
        let Some(block) = self.context_menu_target_block(target_entity) else {
            return;
        };
        self.focus_block(block.entity_id());
        block.update(cx, |block, cx| {
            let range = block.selected_range.clone();
            if range.is_empty() {
                let cursor = block.cursor_offset();
                block.replace_text_in_display_range(
                    cursor..cursor,
                    empty_markup,
                    Some(cursor_offset_in_empty..cursor_offset_in_empty),
                    false,
                    cx,
                );
            } else {
                let text = block.selected_text();
                let inner_len = text.len();
                let wrapped = format!("{wrap_prefix}{text}{wrap_suffix}");
                let prefix_len = wrap_prefix.len();
                block.replace_text_in_display_range(
                    range,
                    &wrapped,
                    Some(prefix_len..prefix_len + inner_len),
                    false,
                    cx,
                );
            }
        });
        self.mark_dirty(cx);
        cx.notify();
    }

    pub(crate) fn apply_wysiwyg_block_kind(
        &mut self,
        target_entity: Option<EntityId>,
        kind: BlockKind,
        cx: &mut Context<Self>,
    ) {
        let Some(block) = self.context_menu_target_block(target_entity) else {
            return;
        };
        self.focus_block(block.entity_id());
        let text = block.read(cx).data.text.clone();
        let cursor = block.read(cx).cursor_offset();
        Self::set_block_text_and_kind(&block, kind, text, cursor, cx);
        self.mark_dirty(cx);
        cx.notify();
    }

    // ─── Inline Formatting Operations ───

    pub(crate) fn on_format_bold(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        match self.tab().mode {
            EditorPaneKind::SourceCode => {
                self.apply_source_inline_markup("****", 2, "**", "**", cx);
            }
            EditorPaneKind::Wysiwyg | EditorPaneKind::Preview | EditorPaneKind::Outline => {
                self.apply_wysiwyg_inline_markup(target, "****", 2, "**", "**", cx);
            }
        }
    }

    pub(crate) fn on_format_italic(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        match self.tab().mode {
            EditorPaneKind::SourceCode => {
                self.apply_source_inline_markup("**", 1, "*", "*", cx);
            }
            EditorPaneKind::Wysiwyg | EditorPaneKind::Preview | EditorPaneKind::Outline => {
                self.apply_wysiwyg_inline_markup(target, "**", 1, "*", "*", cx);
            }
        }
    }

    pub(crate) fn on_format_strikethrough(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        match self.tab().mode {
            EditorPaneKind::SourceCode => {
                self.apply_source_inline_markup("~~~~", 2, "~~", "~~", cx);
            }
            EditorPaneKind::Wysiwyg | EditorPaneKind::Preview | EditorPaneKind::Outline => {
                self.apply_wysiwyg_inline_markup(target, "~~~~", 2, "~~", "~~", cx);
            }
        }
    }

    pub(crate) fn on_format_highlight(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        match self.tab().mode {
            EditorPaneKind::SourceCode => {
                self.apply_source_inline_markup("====", 2, "==", "==", cx);
            }
            EditorPaneKind::Wysiwyg | EditorPaneKind::Preview | EditorPaneKind::Outline => {
                self.apply_wysiwyg_inline_markup(target, "====", 2, "==", "==", cx);
            }
        }
    }

    pub(crate) fn on_format_inline_code(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        match self.tab().mode {
            EditorPaneKind::SourceCode => {
                self.apply_source_inline_markup("``", 1, "`", "`", cx);
            }
            EditorPaneKind::Wysiwyg | EditorPaneKind::Preview | EditorPaneKind::Outline => {
                self.apply_wysiwyg_inline_markup(target, "``", 1, "`", "`", cx);
            }
        }
    }

    pub(crate) fn on_format_inline_math(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        match self.tab().mode {
            EditorPaneKind::SourceCode => {
                self.apply_source_inline_markup("$$", 1, "$", "$", cx);
            }
            EditorPaneKind::Wysiwyg | EditorPaneKind::Preview | EditorPaneKind::Outline => {
                self.apply_wysiwyg_inline_markup(target, "$$", 1, "$", "$", cx);
            }
        }
    }

    pub(crate) fn on_format_comment(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        match self.tab().mode {
            EditorPaneKind::SourceCode => {
                self.apply_source_inline_markup("<!--  -->", 5, "<!-- ", " -->", cx);
            }
            EditorPaneKind::Wysiwyg | EditorPaneKind::Preview | EditorPaneKind::Outline => {
                self.apply_wysiwyg_inline_markup(target, "<!--  -->", 5, "<!-- ", " -->", cx);
            }
        }
    }

    pub(crate) fn on_format_clear(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        match self.tab().mode {
            EditorPaneKind::SourceCode => {}
            EditorPaneKind::Wysiwyg | EditorPaneKind::Preview | EditorPaneKind::Outline => {
                let Some(block) = self.context_menu_target_block(target) else {
                    return;
                };
                self.focus_block(block.entity_id());
                block.update(cx, |block, cx| {
                    let range = block.selected_range.clone();
                    if !range.is_empty() {
                        let plain = block.selected_text();
                        let inner_len = plain.len();
                        block.replace_text_in_display_range(range, &plain, Some(0..inner_len), false, cx);
                    }
                });
                self.mark_dirty(cx);
                cx.notify();
            }
        }
    }

    // ─── Paragraph Settings Operations ───

    pub(crate) fn on_set_heading_1(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        match self.tab().mode {
            EditorPaneKind::SourceCode => {
                self.apply_source_prefix("# ", cx);
            }
            EditorPaneKind::Wysiwyg | EditorPaneKind::Preview | EditorPaneKind::Outline => {
                self.apply_wysiwyg_block_kind(target, BlockKind::Heading { level: 1 }, cx);
            }
        }
    }

    pub(crate) fn on_set_heading_2(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        match self.tab().mode {
            EditorPaneKind::SourceCode => {
                self.apply_source_prefix("## ", cx);
            }
            EditorPaneKind::Wysiwyg | EditorPaneKind::Preview | EditorPaneKind::Outline => {
                self.apply_wysiwyg_block_kind(target, BlockKind::Heading { level: 2 }, cx);
            }
        }
    }

    pub(crate) fn on_set_heading_3(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        match self.tab().mode {
            EditorPaneKind::SourceCode => {
                self.apply_source_prefix("### ", cx);
            }
            EditorPaneKind::Wysiwyg | EditorPaneKind::Preview | EditorPaneKind::Outline => {
                self.apply_wysiwyg_block_kind(target, BlockKind::Heading { level: 3 }, cx);
            }
        }
    }

    pub(crate) fn on_set_heading_4(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        match self.tab().mode {
            EditorPaneKind::SourceCode => {
                self.apply_source_prefix("#### ", cx);
            }
            EditorPaneKind::Wysiwyg | EditorPaneKind::Preview | EditorPaneKind::Outline => {
                self.apply_wysiwyg_block_kind(target, BlockKind::Heading { level: 4 }, cx);
            }
        }
    }

    pub(crate) fn on_set_heading_5(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        match self.tab().mode {
            EditorPaneKind::SourceCode => {
                self.apply_source_prefix("##### ", cx);
            }
            EditorPaneKind::Wysiwyg | EditorPaneKind::Preview | EditorPaneKind::Outline => {
                self.apply_wysiwyg_block_kind(target, BlockKind::Heading { level: 5 }, cx);
            }
        }
    }

    pub(crate) fn on_set_heading_6(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        match self.tab().mode {
            EditorPaneKind::SourceCode => {
                self.apply_source_prefix("###### ", cx);
            }
            EditorPaneKind::Wysiwyg | EditorPaneKind::Preview | EditorPaneKind::Outline => {
                self.apply_wysiwyg_block_kind(target, BlockKind::Heading { level: 6 }, cx);
            }
        }
    }

    pub(crate) fn on_set_paragraph(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        match self.tab().mode {
            EditorPaneKind::SourceCode => {}
            EditorPaneKind::Wysiwyg | EditorPaneKind::Preview | EditorPaneKind::Outline => {
                let Some(block) = self.context_menu_target_block(target) else {
                    return;
                };
                self.focus_block(block.entity_id());
                block.update(cx, |b, cx| b.convert_to_paragraph(cx));
                self.mark_dirty(cx);
                cx.notify();
            }
        }
    }

    pub(crate) fn on_set_bullet_list(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        match self.tab().mode {
            EditorPaneKind::SourceCode => {
                self.apply_source_prefix("- ", cx);
            }
            EditorPaneKind::Wysiwyg | EditorPaneKind::Preview | EditorPaneKind::Outline => {
                self.apply_wysiwyg_block_kind(target, BlockKind::BulletListItem, cx);
            }
        }
    }

    pub(crate) fn on_set_numbered_list(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        match self.tab().mode {
            EditorPaneKind::SourceCode => {
                self.apply_source_prefix("1. ", cx);
            }
            EditorPaneKind::Wysiwyg | EditorPaneKind::Preview | EditorPaneKind::Outline => {
                self.apply_wysiwyg_block_kind(target, BlockKind::NumberedListItem, cx);
            }
        }
    }

    pub(crate) fn on_set_task_list(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        match self.tab().mode {
            EditorPaneKind::SourceCode => {
                self.apply_source_prefix("- [ ] ", cx);
            }
            EditorPaneKind::Wysiwyg | EditorPaneKind::Preview | EditorPaneKind::Outline => {
                self.apply_wysiwyg_block_kind(
                    target,
                    BlockKind::TaskListItem { checked: false },
                    cx,
                );
            }
        }
    }

    pub(crate) fn on_set_quote(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        match self.tab().mode {
            EditorPaneKind::SourceCode => {
                self.apply_source_prefix("> ", cx);
            }
            EditorPaneKind::Wysiwyg | EditorPaneKind::Preview | EditorPaneKind::Outline => {
                self.apply_wysiwyg_block_kind(target, BlockKind::Blockquote, cx);
            }
        }
    }

    // ─── Insert Operations ───

    pub(crate) fn on_insert_footnote(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        match self.tab().mode {
            EditorPaneKind::SourceCode => {
                self.apply_source_inline_markup("[^1]", 3, "[^", "]", cx);
            }
            EditorPaneKind::Wysiwyg | EditorPaneKind::Preview | EditorPaneKind::Outline => {
                self.apply_wysiwyg_inline_markup(target, "[^1]", 3, "[^", "]", cx);
            }
        }
    }

    pub(crate) fn on_insert_callout(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        match self.tab().mode {
            EditorPaneKind::SourceCode => {
                self.apply_source_inline_markup("> [!]", 4, "> [!", "]", cx);
            }
            EditorPaneKind::Wysiwyg | EditorPaneKind::Preview | EditorPaneKind::Outline => {
                self.apply_wysiwyg_inline_markup(target, "> [!]", 4, "> [!", "]", cx);
            }
        }
    }

    pub(crate) fn on_insert_thematic_break(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        match self.tab().mode {
            EditorPaneKind::SourceCode => {
                self.apply_source_snippet("\n---\n", 5, cx);
            }
            EditorPaneKind::Wysiwyg | EditorPaneKind::Preview | EditorPaneKind::Outline => {
                let Some(block) = self.context_menu_target_block(target) else {
                    return;
                };
                self.focus_block(block.entity_id());
                block.update(cx, |b, cx| b.convert_to_separator(cx));
                self.mark_dirty(cx);
                cx.notify();
            }
        }
    }

    pub(crate) fn on_insert_code_block(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        match self.tab().mode {
            EditorPaneKind::SourceCode => {
                self.apply_source_snippet("\n```\n\n```\n", 5, cx);
            }
            EditorPaneKind::Wysiwyg | EditorPaneKind::Preview | EditorPaneKind::Outline => {
                let Some(block) = self.context_menu_target_block(target) else {
                    return;
                };
                self.focus_block(block.entity_id());
                block.update(cx, |b, cx| b.enter_code_block(None, cx));
                self.mark_dirty(cx);
                cx.notify();
            }
        }
    }

    pub(crate) fn on_insert_math_block(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        match self.tab().mode {
            EditorPaneKind::SourceCode => {
                self.apply_source_snippet("\n$$\nf(x) = x^2\n$$\n", 14, cx);
            }
            EditorPaneKind::Wysiwyg | EditorPaneKind::Preview | EditorPaneKind::Outline => {
                let Some(block) = self.context_menu_target_block(target) else {
                    return;
                };
                self.focus_block(block.entity_id());
                block.update(cx, |b, cx| {
                    let default_math = "f(x) = x^2";
                    b.enter_math_block(default_math, cx);
                    b.assign_collapsed_selection_offset(
                        default_math.len(),
                        crate::editor::document::block::CollapsedCaretAffinity::Default,
                        None,
                    );
                });
                self.mark_dirty(cx);
                cx.notify();
            }
        }
    }

    pub(crate) fn on_insert_mermaid(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        match self.tab().mode {
            EditorPaneKind::SourceCode => {
                self.apply_source_snippet("\n```mermaid\ngraph LR\n    A --> B\n```\n", 32, cx);
            }
            EditorPaneKind::Wysiwyg | EditorPaneKind::Preview | EditorPaneKind::Outline => {
                let Some(block) = self.context_menu_target_block(target) else {
                    return;
                };
                self.focus_block(block.entity_id());
                block.update(cx, |b, cx| {
                    let default_mermaid = "graph LR\n    A --> B";
                    b.enter_mermaid_block(default_mermaid, cx);
                    b.assign_collapsed_selection_offset(
                        default_mermaid.len(),
                        crate::editor::document::block::CollapsedCaretAffinity::Default,
                        None,
                    );
                });
                self.mark_dirty(cx);
                cx.notify();
            }
        }
    }

    fn context_menu_target_entity_from_state(&self) -> Option<EntityId> {
        match self.context_menu.as_ref() {
            Some(ContextMenuState::Edit { target_entity, .. }) => *target_entity,
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ContextMenuState, ContextSubmenu, Editor, TableInsertTarget};
    use gpui::{AppContext, Point, TestAppContext, px};

    #[gpui::test]
    async fn context_submenu_stays_open_while_crossing_hover_gap(cx: &mut TestAppContext) {
        let editor = cx.new(|cx| Editor::from_markdown(cx, "alpha".to_string(), None));

        editor.update(cx, |editor, cx| {
            editor.open_edit_context_menu(
                Point {
                    x: px(24.0),
                    y: px(24.0),
                },
                TableInsertTarget::Append,
                None,
                cx,
            );

            editor.set_context_menu_submenu_hover(Some(ContextSubmenu::TextFormat), false, cx);
            let Some(ContextMenuState::Edit { active_submenu, .. }) = editor.context_menu.as_ref()
            else {
                panic!("expected edit context menu");
            };
            assert_eq!(active_submenu, &Some(ContextSubmenu::TextFormat));
            assert!(editor.context_menu_submenu_close_task.is_none());

            editor.set_context_menu_submenu_hover(None, false, cx);
            let Some(ContextMenuState::Edit { active_submenu, .. }) = editor.context_menu.as_ref()
            else {
                panic!("expected edit context menu");
            };
            assert_eq!(active_submenu, &Some(ContextSubmenu::TextFormat));
            assert!(editor.context_menu_submenu_close_task.is_some());

            editor.set_context_menu_submenu_hover(Some(ContextSubmenu::TextFormat), true, cx);
            let Some(ContextMenuState::Edit { active_submenu, .. }) = editor.context_menu.as_ref()
            else {
                panic!("expected edit context menu");
            };
            assert_eq!(active_submenu, &Some(ContextSubmenu::TextFormat));
            assert!(editor.context_menu_submenu_close_task.is_none());
        });
    }

    #[gpui::test]
    async fn context_menu_format_empty_places_caret_in_middle(cx: &mut TestAppContext) {
        let editor = cx.new(|cx| Editor::from_markdown(cx, "".to_string(), None));

        editor.update(cx, |editor, cx| {
            let block = editor.doc().blocks().first().unwrap().entity.clone();
            editor.apply_wysiwyg_inline_markup(Some(block.entity_id()), "****", 2, "**", "**", cx);

            let block_ref = block.read(cx);
            assert_eq!(block_ref.display_text(), "****");
            assert_eq!(block_ref.cursor_offset(), 2);
        });
    }

    #[gpui::test]
    async fn context_menu_format_wraps_active_selection(cx: &mut TestAppContext) {
        let editor = cx.new(|cx| Editor::from_markdown(cx, "sample text".to_string(), None));

        editor.update(cx, |editor, cx| {
            let block = editor.doc().blocks().first().unwrap().entity.clone();
            block.update(cx, |b, _cx| {
                b.selected_range = 0..6; // "sample"
            });

            editor.apply_wysiwyg_inline_markup(Some(block.entity_id()), "****", 2, "**", "**", cx);

            let block_ref = block.read(cx);
            assert_eq!(block_ref.display_text(), "**sample** text");
        });
    }

    #[gpui::test]
    async fn context_menu_paragraph_and_quote_kind_switches(cx: &mut TestAppContext) {
        use crate::model::parse::BlockKind;
        let editor = cx.new(|cx| Editor::from_markdown(cx, "my heading".to_string(), None));

        editor.update(cx, |editor, cx| {
            let block = editor.doc().blocks().first().unwrap().entity.clone();
            editor.apply_wysiwyg_block_kind(
                Some(block.entity_id()),
                BlockKind::Heading { level: 2 },
                cx,
            );

            assert_eq!(block.read(cx).kind(), BlockKind::Heading { level: 2 });
            assert_eq!(block.read(cx).display_text(), "my heading");

            editor.apply_wysiwyg_block_kind(
                Some(block.entity_id()),
                BlockKind::Blockquote,
                cx,
            );
            assert_eq!(block.read(cx).kind(), BlockKind::Blockquote);
        });
    }

    #[gpui::test]
    async fn test_bold_insert_and_type_multiple_chars(cx: &mut TestAppContext) {
        let editor = cx.new(|cx| Editor::from_markdown(cx, "".to_string(), None));

        editor.update(cx, |editor, cx| {
            let block = editor.doc().blocks().first().unwrap().entity.clone();
            editor.apply_wysiwyg_inline_markup(Some(block.entity_id()), "****", 2, "**", "**", cx);

            let block_ref = block.read(cx);
            assert_eq!(block_ref.display_text(), "****");
            let cursor1 = block_ref.cursor_offset();
            assert_eq!(cursor1, 2);

            // Type first char 'a'
            block.update(cx, |b, cx| {
                b.replace_text_in_display_range(cursor1..cursor1, "a", None, false, cx);
            });

            let cursor2 = block.read(cx).cursor_offset();
            // Type second char 'b'
            block.update(cx, |b, cx| {
                b.replace_text_in_display_range(cursor2..cursor2, "b", None, false, cx);
            });

            let cursor3 = block.read(cx).cursor_offset();
            // Type third char 'c'
            block.update(cx, |b, cx| {
                b.replace_text_in_display_range(cursor3..cursor3, "c", None, false, cx);
            });

            let block_ref = block.read(cx);
            assert_eq!(block_ref.data.text.serialize_markdown(), "**abc**");
        });
    }

    #[gpui::test]
    async fn test_italic_strikethrough_and_code_insert_and_type(cx: &mut TestAppContext) {
        let editor = cx.new(|cx| Editor::from_markdown(cx, "".to_string(), None));

        editor.update(cx, |editor, cx| {
            let block = editor.doc().blocks().first().unwrap().entity.clone();

            // Test italic
            editor.apply_wysiwyg_inline_markup(Some(block.entity_id()), "**", 1, "*", "*", cx);
            let c1 = block.read(cx).cursor_offset();
            block.update(cx, |b, cx| b.replace_text_in_display_range(c1..c1, "x", None, false, cx));
            let c2 = block.read(cx).cursor_offset();
            block.update(cx, |b, cx| b.replace_text_in_display_range(c2..c2, "y", None, false, cx));
            assert_eq!(block.read(cx).data.text.serialize_markdown(), "*xy*");

            // Reset and test strikethrough
            block.update(cx, |b, cx| b.replace_text_in_display_range(0..b.display_len(), "", None, false, cx));
            editor.apply_wysiwyg_inline_markup(Some(block.entity_id()), "~~~~", 2, "~~", "~~", cx);
            let c1 = block.read(cx).cursor_offset();
            block.update(cx, |b, cx| b.replace_text_in_display_range(c1..c1, "h", None, false, cx));
            let c2 = block.read(cx).cursor_offset();
            block.update(cx, |b, cx| b.replace_text_in_display_range(c2..c2, "i", None, false, cx));
            assert_eq!(block.read(cx).data.text.serialize_markdown(), "~~hi~~");

            // Reset and test inline code
            block.update(cx, |b, cx| b.replace_text_in_display_range(0..b.display_len(), "", None, false, cx));
            editor.apply_wysiwyg_inline_markup(Some(block.entity_id()), "``", 1, "`", "`", cx);
            let c1 = block.read(cx).cursor_offset();
            block.update(cx, |b, cx| b.replace_text_in_display_range(c1..c1, "f", None, false, cx));
            let c2 = block.read(cx).cursor_offset();
            block.update(cx, |b, cx| b.replace_text_in_display_range(c2..c2, "n", None, false, cx));
            assert_eq!(block.read(cx).data.text.serialize_markdown(), "`fn`");
        });
    }

    #[gpui::test]
    async fn test_inline_markup_targets_specific_block(cx: &mut TestAppContext) {
        let editor = cx.new(|cx| {
            Editor::from_markdown(
                cx,
                "first line\n\nsecond line\n\nthird line".to_string(),
                None,
            )
        });

        editor.update(cx, |editor, cx| {
            let b0 = editor.doc().blocks()[0].entity.clone();
            let b2 = editor.doc().blocks()[2].entity.clone();
            let b4 = editor.doc().blocks()[4].entity.clone();
            let target_id = b2.entity_id();

            // Set cursor in second line block at offset 7 ("second ")
            b2.update(cx, |b, cx| b.move_to(7, cx));

            // Apply bold markup to second line block
            editor.apply_wysiwyg_inline_markup(Some(target_id), "****", 2, "**", "**", cx);

            assert_eq!(b0.read(cx).display_text(), "first line");
            assert_eq!(b2.read(cx).display_text(), "second ****line");
            assert_eq!(b2.read(cx).cursor_offset(), 9);
            assert_eq!(b4.read(cx).display_text(), "third line");

            // Type 'hi' into bold inside second line block
            b2.update(cx, |b, cx| {
                b.replace_text_in_display_range(9..9, "h", None, false, cx);
            });
            let c = b2.read(cx).cursor_offset();
            b2.update(cx, |b, cx| {
                b.replace_text_in_display_range(c..c, "i", None, false, cx);
            });

            assert_eq!(
                b2.read(cx).data.text.serialize_markdown(),
                "second **hi**line"
            );
        });
    }

    #[gpui::test]
    async fn test_source_code_bold_insertion(cx: &mut TestAppContext) {
        use crate::editor::engine::controller::EditorPaneKind;
        let editor = cx.new(|cx| {
            Editor::from_markdown(
                cx,
                "line one\nline two\nline three".to_string(),
                None,
            )
        });

        editor.update(cx, |editor, cx| {
            editor.toggle_pane_kind(cx);
            assert!(matches!(editor.tab().mode, EditorPaneKind::SourceCode));

            let pane_id = editor.active_pane_id();
            editor.sync_source_pane(pane_id, cx);
            let source_block = editor.pane_state_ref(pane_id).and_then(|p| p.source_block.clone()).unwrap();

            // Move cursor to "line two" (offset 14: "line one\nline ")
            source_block.update(cx, |b, cx| b.move_to(14, cx));

            // Apply bold markup in source mode
            editor.apply_source_inline_markup("****", 2, "**", "**", cx);

            assert_eq!(source_block.read(cx).display_text(), "line one\nline ****two\nline three");
            assert_eq!(source_block.read(cx).cursor_offset(), 16);

            // Type 'hi' into bold in source mode
            source_block.update(cx, |b, cx| {
                b.replace_text_in_display_range(16..16, "h", None, false, cx);
            });
            let c = source_block.read(cx).cursor_offset();
            source_block.update(cx, |b, cx| {
                b.replace_text_in_display_range(c..c, "i", None, false, cx);
            });

            assert_eq!(
                source_block.read(cx).display_text(),
                "line one\nline **hi**two\nline three"
            );
        });
    }

    #[gpui::test]
    async fn test_submenu_hover_and_debounce(cx: &mut TestAppContext) {
        let editor = cx.new(|cx| {
            Editor::from_markdown(cx, "hello world".to_string(), None)
        });

        editor.update(cx, |editor, cx| {
            editor.open_edit_context_menu(gpui::point(px(100.0), px(100.0)), TableInsertTarget::Append, None, cx);
            assert!(editor.context_menu.is_some());

            // Hover over ParagraphSettings
            editor.set_context_menu_submenu_hover(Some(ContextSubmenu::ParagraphSettings), false, cx);
            match &editor.context_menu {
                Some(ContextMenuState::Edit { active_submenu, .. }) => {
                    assert_eq!(*active_submenu, Some(ContextSubmenu::ParagraphSettings));
                }
                _ => panic!("Expected Edit context menu"),
            }

            // Quickly hover over Insert
            editor.set_context_menu_submenu_hover(None, false, cx);
            editor.set_context_menu_submenu_hover(Some(ContextSubmenu::Insert), false, cx);
            match &editor.context_menu {
                Some(ContextMenuState::Edit { active_submenu, .. }) => {
                    assert_eq!(*active_submenu, Some(ContextSubmenu::Insert));
                }
                _ => panic!("Expected Edit context menu"),
            }

            // Hover over submenu body
            editor.set_context_menu_submenu_hover(Some(ContextSubmenu::Insert), true, cx);
            match &editor.context_menu {
                Some(ContextMenuState::Edit { active_submenu, submenu_hovered, .. }) => {
                    assert_eq!(*active_submenu, Some(ContextSubmenu::Insert));
                    assert!(*submenu_hovered);
                }
                _ => panic!("Expected Edit context menu"),
            }
        });
    }

    #[gpui::test]
    async fn test_source_code_table_insertion(cx: &mut TestAppContext) {
        use crate::editor::engine::controller::EditorPaneKind;
        cx.update(|cx| {
            crate::infra::i18n::I18nManager::init(cx);
            crate::infra::theme::ThemeManager::init(cx);
        });
        let (editor, cx) = cx.add_window_view(|_window, cx| {
            Editor::from_markdown(cx, "above\n\nbelow".to_string(), None)
        });

        editor.update(cx, |editor, cx| {
            editor.toggle_pane_kind(cx);
            assert!(matches!(editor.tab().mode, EditorPaneKind::SourceCode));

            let pane_id = editor.active_pane_id();
            editor.sync_source_pane(pane_id, cx);
            let source_block = editor.pane_state_ref(pane_id).and_then(|p| p.source_block.clone()).unwrap();

            // Set cursor between lines
            source_block.update(cx, |b, cx| b.move_to(6, cx));

            // Set table insert dialog state and confirm
            editor.table_insert_dialog = Some(crate::editor::panes::document_pane::dialogs::TableInsertDialogState::new(
                TableInsertTarget::Append,
                3,
                2,
                None,
            ));
        });

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let fake_click = gpui::ClickEvent::default();
                editor.on_confirm_table_insert_dialog(&fake_click, window, cx);
            });
        });

        editor.update(cx, |editor, cx| {
            let pane_id = editor.active_pane_id();
            let source_block = editor.pane_state_ref(pane_id).and_then(|p| p.source_block.clone()).unwrap();
            let text = source_block.read(cx).display_text();
            assert!(text.contains("| --- | --- |"));
            assert!(text.contains("|  |  |"));
        });
    }

    #[gpui::test]
    async fn test_wysiwyg_quote_and_guides(cx: &mut TestAppContext) {
        use crate::editor::panes::wysiwyg::render::visible_quote_guides;
        use crate::model::parse::BlockKind;
        cx.update(|cx| {
            crate::infra::i18n::I18nManager::init(cx);
            crate::infra::theme::ThemeManager::init(cx);
        });
        let (editor, cx) = cx.add_window_view(|_window, cx| {
            Editor::from_markdown(cx, "Quote text".to_string(), None)
        });

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let block = editor.doc().root_blocks().first().cloned().unwrap();
                let fake_click = gpui::ClickEvent::default();
                editor.open_edit_context_menu(Point::default(), TableInsertTarget::Append, Some(block.entity_id()), cx);
                editor.on_set_quote(&fake_click, window, cx);
                assert_eq!(block.read(cx).kind(), BlockKind::Blockquote);
                assert_eq!(visible_quote_guides(block.read(cx)), 1);
            });
        });
    }

    #[gpui::test]
    async fn test_wysiwyg_math_and_mermaid_insert(cx: &mut TestAppContext) {
        use crate::model::parse::BlockKind;
        cx.update(|cx| {
            crate::infra::i18n::I18nManager::init(cx);
            crate::infra::theme::ThemeManager::init(cx);
        });
        let (editor, cx) = cx.add_window_view(|_window, cx| {
            Editor::from_markdown(cx, "Initial text".to_string(), None)
        });

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let block = editor.doc().root_blocks().first().cloned().unwrap();
                let fake_click = gpui::ClickEvent::default();

                // Test Math block insertion
                editor.open_edit_context_menu(Point::default(), TableInsertTarget::Append, Some(block.entity_id()), cx);
                editor.on_insert_math_block(&fake_click, window, cx);
                assert_eq!(block.read(cx).kind(), BlockKind::MathBlock);
                assert_eq!(block.read(cx).display_text(), "f(x) = x^2");
                assert_eq!(block.read(cx).cursor_offset(), "f(x) = x^2".len());

                // Test Mermaid block insertion
                editor.open_edit_context_menu(Point::default(), TableInsertTarget::Append, Some(block.entity_id()), cx);
                editor.on_insert_mermaid(&fake_click, window, cx);
                assert_eq!(block.read(cx).kind(), BlockKind::MermaidBlock);
                let expected_mermaid = "graph LR\n    A --> B";
                assert_eq!(block.read(cx).display_text(), expected_mermaid);
                assert_eq!(block.read(cx).cursor_offset(), expected_mermaid.len());
            });
        });
    }

    #[gpui::test]
    async fn test_source_code_paragraph_and_prefix_transforms(cx: &mut TestAppContext) {
        use crate::editor::engine::controller::EditorPaneKind;
        cx.update(|cx| {
            crate::infra::i18n::I18nManager::init(cx);
            crate::infra::theme::ThemeManager::init(cx);
        });
        let (editor, cx) = cx.add_window_view(|_window, cx| {
            Editor::from_markdown(cx, "Heading line".to_string(), None)
        });

        editor.update(cx, |editor, cx| {
            editor.toggle_pane_kind(cx);
            assert!(matches!(editor.tab().mode, EditorPaneKind::SourceCode));

            let pane_id = editor.active_pane_id();
            editor.sync_source_pane(pane_id, cx);
        });

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let pane_id = editor.active_pane_id();
                let source_block = editor.pane_state_ref(pane_id).and_then(|p| p.source_block.clone()).unwrap();

                // 1. Insert Heading 1 at cursor
                source_block.update(cx, |b, cx| b.move_to(0, cx));
                editor.open_edit_context_menu(Point::default(), TableInsertTarget::Append, Some(source_block.entity_id()), cx);
                let fake_click = gpui::ClickEvent::default();
                editor.on_set_heading_1(&fake_click, window, cx);

                let text = source_block.read(cx).display_text();
                assert!(text.starts_with("# Heading line"));
            });
        });
    }

    #[gpui::test]
    async fn test_wysiwyg_and_source_callout_insert(cx: &mut TestAppContext) {
        use crate::editor::engine::controller::EditorPaneKind;

        cx.update(|cx| {
            crate::infra::i18n::I18nManager::init(cx);
            crate::infra::theme::ThemeManager::init(cx);
        });

        // 1. WYSIWYG mode test
        let (editor, cx) = cx.add_window_view(|_window, cx| {
            Editor::from_markdown(cx, "Note title".to_string(), None)
        });

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let block = editor.doc().root_blocks().first().cloned().unwrap();
                editor.open_edit_context_menu(Point::default(), TableInsertTarget::Append, Some(block.entity_id()), cx);
                let fake_click = gpui::ClickEvent::default();
                editor.on_insert_callout(&fake_click, window, cx);

                let text = block.read(cx).display_text();
                assert!(text.contains("[!]"));
            });
        });

        // 2. Source Code mode test
        editor.update(cx, |editor, cx| {
            editor.toggle_pane_kind(cx);
            assert!(matches!(editor.tab().mode, EditorPaneKind::SourceCode));
            let pane_id = editor.active_pane_id();
            editor.sync_source_pane(pane_id, cx);
        });

        cx.update(|window, cx| {
            editor.update(cx, |editor, cx| {
                let pane_id = editor.active_pane_id();
                let source_block = editor.pane_state_ref(pane_id).and_then(|p| p.source_block.clone()).unwrap();
                let fake_click = gpui::ClickEvent::default();
                editor.open_edit_context_menu(Point::default(), TableInsertTarget::Append, Some(source_block.entity_id()), cx);
                editor.on_insert_callout(&fake_click, window, cx);

                let text = source_block.read(cx).display_text();
                assert!(text.contains("> [!]"));
            });
        });
    }
}

