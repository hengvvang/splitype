//! Context menu state, lifecycle, and action dispatchers for WYSIWYG and
//! Source Code editing modes.
//!
//! Submenu hover state is debounced with a timer to ensure fluid navigation.
//! Rendering lives in `context_menu_render.rs` and table axis actions live in
//! `context_menu_actions.rs`.

use std::time::Duration;

use gpui::*;

use crate::editor_scheduler::commands::edit_command::{
    BlockStructureKind, DocumentEditCommand, InlineFormatKind, InsertBlockKind,
};
use crate::editor_scheduler::engine::controller::{
    Editor, TableAxisSelection};
use workspace::actions::DismissTransientUi;
use crate::editor_scheduler::document::dialogs::TableInsertDialogState;
use editor_wysiwyg::document::block::Block;

/// Active secondary submenu in the context menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ContextSubmenu {
    TextFormat,
    ParagraphSettings,
    Insert,
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
    /// Opens the general edit context menu at pointer location.
    pub(crate) fn open_edit_context_menu(
        &mut self,
        position: Point<Pixels>,
        target_entity: Option<EntityId>,
        cx: &mut Context<Self>,
    ) {
        self.context_menu_submenu_close_task = None;
        self.context_menu = Some(ContextMenuState::Edit {
            position,
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

    pub(crate) fn dismiss_contextual_overlays(&mut self, cx: &mut App) {
        let had_menu = self.context_menu.take().is_some();
        let had_dialog = self.table_insert_dialog.take().is_some();
        let had_picker = self.table_size_picker.take().is_some();
        let had_submenu_close = self.context_menu_submenu_close_task.take().is_some();
        if had_menu || had_dialog || had_picker || had_submenu_close {
            cx.notify(self.entity_id);
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
            self.open_edit_context_menu(event.position, Some(endpoint.entity_id), cx);
        } else {
            let active_id = self.active_pane_state().as_wysiwyg().and_then(|w| w.focus.active_entity);
            self.open_edit_context_menu(event.position, active_id, cx);
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
        self.sync_source_pane(pane_id, cx);
        self.open_edit_context_menu(event.position, None, cx);
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
        self.open_edit_context_menu(event.position, Some(entity_id), cx);
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
        let (target_entity, position) = match self.context_menu.take() {
            Some(ContextMenuState::Edit { target_entity, position, .. }) => (target_entity, Some(position)),
            _ => (None, None),
        };
        self.context_menu_submenu_close_task = None;
        self.table_insert_dialog = Some(TableInsertDialogState::new(target_entity, 4, 3, position));
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

    pub(crate) fn insert_table_from_dialog(
        &mut self,
        rows: usize,
        cols: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(dialog) = self.table_insert_dialog.take() else {
            return;
        };
        self.execute_edit_command(
            DocumentEditCommand::InsertTable {
                rows,
                columns: cols,
            },
            dialog.target_entity,
            cx,
        );
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

    pub(crate) fn confirm_table_insert_action(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = self.table_insert_dialog.take() else {
            return;
        };
        self.execute_edit_command(
            DocumentEditCommand::InsertTable {
                rows: dialog.rows,
                columns: dialog.columns,
            },
            dialog.target_entity,
            cx,
        );
    }

    /// Determines if there is an active text selection in WYSIWYG or Source mode.
    pub(crate) fn context_menu_has_selection(&self, cx: &App) -> bool {
        let pane_id = self.active_pane_id();
        let pane_state = self.pane_state_ref(pane_id);
        if self.is_source_code() {
            pane_state
                .and_then(|p| p.as_source_code())
                .map(|s| s.selection.is_some())
                .unwrap_or(false)
        } else {
            if self.active_pane_selection().cross_block.is_some() {
                return true;
            }
            if let Some(active_id) = pane_state.and_then(|p| p.as_wysiwyg()).and_then(|w| w.focus.active_entity) {
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
            None
        } else {
            target_entity
                .and_then(|id| self.focusable_entity_by_id(id))
                .or_else(|| {
                    pane_state
                        .and_then(|p| p.as_wysiwyg())
                        .and_then(|w| w.focus.active_entity)
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
        self.execute_edit_command(DocumentEditCommand::Cut, None, cx);
    }

    pub(crate) fn on_context_menu_copy(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_context_menu(cx);
        self.execute_edit_command(DocumentEditCommand::Copy, None, cx);
    }

    pub(crate) fn on_context_menu_paste(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        self.execute_edit_command(DocumentEditCommand::Paste, target, cx);
    }

    pub(crate) fn on_context_menu_paste_plain(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        self.execute_edit_command(DocumentEditCommand::PastePlain, target, cx);
    }

    pub(crate) fn on_context_menu_select_all(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_context_menu(cx);
        self.execute_edit_command(DocumentEditCommand::SelectAll, None, cx);
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
        self.execute_edit_command(DocumentEditCommand::Format(InlineFormatKind::Bold), target, cx);
    }

    pub(crate) fn on_format_italic(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        self.execute_edit_command(DocumentEditCommand::Format(InlineFormatKind::Italic), target, cx);
    }

    pub(crate) fn on_format_strikethrough(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        self.execute_edit_command(DocumentEditCommand::Format(InlineFormatKind::Strikethrough), target, cx);
    }

    pub(crate) fn on_format_highlight(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        self.execute_edit_command(DocumentEditCommand::Format(InlineFormatKind::Highlight), target, cx);
    }

    pub(crate) fn on_format_inline_code(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        self.execute_edit_command(DocumentEditCommand::Format(InlineFormatKind::InlineCode), target, cx);
    }

    pub(crate) fn on_format_inline_math(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        self.execute_edit_command(DocumentEditCommand::Format(InlineFormatKind::InlineMath), target, cx);
    }

    pub(crate) fn on_format_comment(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        self.execute_edit_command(DocumentEditCommand::Format(InlineFormatKind::Comment), target, cx);
    }

    pub(crate) fn on_format_clear(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        self.execute_edit_command(DocumentEditCommand::Format(InlineFormatKind::ClearFormat), target, cx);
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
        self.execute_edit_command(DocumentEditCommand::Structure(BlockStructureKind::Heading(1)), target, cx);
    }

    pub(crate) fn on_set_heading_2(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        self.execute_edit_command(DocumentEditCommand::Structure(BlockStructureKind::Heading(2)), target, cx);
    }

    pub(crate) fn on_set_heading_3(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        self.execute_edit_command(DocumentEditCommand::Structure(BlockStructureKind::Heading(3)), target, cx);
    }

    pub(crate) fn on_set_heading_4(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        self.execute_edit_command(DocumentEditCommand::Structure(BlockStructureKind::Heading(4)), target, cx);
    }

    pub(crate) fn on_set_heading_5(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        self.execute_edit_command(DocumentEditCommand::Structure(BlockStructureKind::Heading(5)), target, cx);
    }

    pub(crate) fn on_set_heading_6(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        self.execute_edit_command(DocumentEditCommand::Structure(BlockStructureKind::Heading(6)), target, cx);
    }

    pub(crate) fn on_set_paragraph(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        self.execute_edit_command(DocumentEditCommand::Structure(BlockStructureKind::Paragraph), target, cx);
    }

    pub(crate) fn on_set_bullet_list(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        self.execute_edit_command(DocumentEditCommand::Structure(BlockStructureKind::BulletList), target, cx);
    }

    pub(crate) fn on_set_numbered_list(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        self.execute_edit_command(DocumentEditCommand::Structure(BlockStructureKind::NumberedList), target, cx);
    }

    pub(crate) fn on_set_task_list(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        self.execute_edit_command(DocumentEditCommand::Structure(BlockStructureKind::TaskList), target, cx);
    }

    pub(crate) fn on_set_quote(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        self.execute_edit_command(DocumentEditCommand::Structure(BlockStructureKind::Blockquote), target, cx);
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
        self.execute_edit_command(DocumentEditCommand::Insert(InsertBlockKind::Footnote), target, cx);
    }

    pub(crate) fn on_insert_callout(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        self.execute_edit_command(DocumentEditCommand::Insert(InsertBlockKind::Callout), target, cx);
    }

    pub(crate) fn on_insert_thematic_break(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        self.execute_edit_command(DocumentEditCommand::Insert(InsertBlockKind::ThematicBreak), target, cx);
    }

    pub(crate) fn on_insert_code_block(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        self.execute_edit_command(DocumentEditCommand::Insert(InsertBlockKind::CodeBlock), target, cx);
    }

    pub(crate) fn on_insert_math_block(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        self.execute_edit_command(DocumentEditCommand::Insert(InsertBlockKind::MathBlock), target, cx);
    }

    pub(crate) fn on_insert_mermaid(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.context_menu_target_entity_from_state();
        self.close_context_menu(cx);
        self.execute_edit_command(DocumentEditCommand::Insert(InsertBlockKind::Mermaid), target, cx);
    }

    fn context_menu_target_entity_from_state(&self) -> Option<EntityId> {
        match self.context_menu.as_ref() {
            Some(ContextMenuState::Edit { target_entity, .. }) => *target_entity,
            _ => None,
        }
    }
}





