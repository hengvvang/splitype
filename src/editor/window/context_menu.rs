//! Context menu state and lifecycle: opening, closing, hover tracking, and
//! the table insert dialog. Rendering lives in `context_menu_render.rs` and
//! the table manipulation actions in `context_menu_actions.rs`.

use std::time::Duration;

use gpui::*;

use crate::editor::controller::{Editor, EditorMode, TableAxisSelection};
use crate::editor::editing::input::actions::DismissTransientUi;
use crate::editor::window::dialogs::TableInsertDialogState;
use crate::model::syntax::table::TableData;

/// Target block position for inserting a native table.
#[derive(Clone, Copy)]
pub(crate) enum TableInsertTarget {
    /// Insert the table immediately after the referenced block.
    After(EntityId),
    /// Append the table to the end of the current root list.
    Append,
}

/// Rendered-mode context menu currently open in the editor.
#[derive(Clone)]
pub(crate) enum ContextMenuState {
    /// General block context menu with an insert submenu.
    Insert {
        position: Point<Pixels>,
        target: TableInsertTarget,
        insert_hovered: bool,
        submenu_hovered: bool,
        submenu_open: bool,
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

    pub(crate) fn open_insert_context_menu(
        &mut self,
        position: Point<Pixels>,
        target: TableInsertTarget,
        cx: &mut Context<Self>,
    ) {
        if self.tab().mode != EditorMode::Wysiwyg {
            return;
        }

        self.context_menu_submenu_close_task = None;
        self.context_menu = Some(ContextMenuState::Insert {
            position,
            target,
            insert_hovered: false,
            submenu_hovered: false,
            submenu_open: false,
        });
        cx.notify();
    }

    pub(crate) fn open_table_axis_context_menu(
        &mut self,
        position: Point<Pixels>,
        selection: TableAxisSelection,
        cx: &mut Context<Self>,
    ) {
        if self.tab().mode != EditorMode::Wysiwyg {
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
        let had_submenu_close = self.context_menu_submenu_close_task.take().is_some();
        if had_menu || had_dialog || had_submenu_close {
            cx.notify();
        }
    }

    pub(crate) fn schedule_context_menu_submenu_close(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.context_menu, Some(ContextMenuState::Insert { .. })) {
            return;
        }

        let weak_editor = cx.entity().downgrade();
        self.context_menu_submenu_close_task = Some(cx.spawn(
            async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
                cx.background_executor()
                    .timer(Duration::from_millis(120))
                    .await;
                let _ = weak_editor.update(cx, |editor, cx| {
                    editor.context_menu_submenu_close_task = None;
                    let Some(ContextMenuState::Insert {
                        insert_hovered,
                        submenu_hovered,
                        submenu_open,
                        ..
                    }) = editor.context_menu.as_mut()
                    else {
                        return;
                    };
                    if !*insert_hovered && !*submenu_hovered && *submenu_open {
                        *submenu_open = false;
                        cx.notify();
                    }
                });
            },
        ));
    }

    pub(crate) fn set_context_menu_hover_state(
        &mut self,
        hovered: bool,
        submenu: bool,
        cx: &mut Context<Self>,
    ) {
        let mut changed = false;
        let mut should_clear_close = false;
        let mut should_schedule_close = false;

        if let Some(ContextMenuState::Insert {
            insert_hovered,
            submenu_hovered,
            submenu_open,
            ..
        }) = self.context_menu.as_mut()
        {
            if submenu {
                if *submenu_hovered != hovered {
                    *submenu_hovered = hovered;
                    changed = true;
                }
            } else if *insert_hovered != hovered {
                *insert_hovered = hovered;
                changed = true;
            }

            if hovered {
                should_clear_close = true;
                if !*submenu_open {
                    *submenu_open = true;
                    changed = true;
                }
            } else {
                let insert_still_hovered = *insert_hovered;
                let submenu_still_hovered = *submenu_hovered;
                if !insert_still_hovered && !submenu_still_hovered {
                    should_schedule_close = true;
                }
            }
        }

        if should_clear_close {
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
        if self.tab().mode != EditorMode::Wysiwyg {
            return;
        }
        cx.stop_propagation();
        self.open_insert_context_menu(event.position, TableInsertTarget::Append, cx);
    }

    pub(crate) fn on_block_context_menu_mouse_down(
        &mut self,
        entity_id: EntityId,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.tab().mode != EditorMode::Wysiwyg {
            return;
        }
        cx.stop_propagation();
        // Right-clicking inside a table cell, or any block where inserting a
        // table makes no sense (code, math, etc.), offers no insert menu.
        if self.table_cell_binding(entity_id).is_some() {
            return;
        }
        let allows_insert = self
            .focusable_entity_by_id(entity_id)
            .is_none_or(|block| block.read(cx).kind().allows_context_table_insert());
        if !allows_insert {
            return;
        }
        let target = TableInsertTarget::After(self.root_ancestor_entity_id(entity_id));
        self.open_insert_context_menu(event.position, target, cx);
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

    pub(crate) fn on_context_menu_insert_hover(
        &mut self,
        hovered: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_context_menu_hover_state(*hovered, false, cx);
    }

    pub(crate) fn on_context_menu_submenu_hover(
        &mut self,
        hovered: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_context_menu_hover_state(*hovered, true, cx);
    }

    pub(crate) fn on_open_table_insert_dialog(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(ContextMenuState::Insert { target, .. }) = self.context_menu.take() else {
            return;
        };
        self.context_menu_submenu_close_task = None;
        self.table_insert_dialog = Some(TableInsertDialogState {
            target,
            body_rows: 2,
            columns: 2,
        });
        cx.notify();
    }

    pub(crate) fn on_table_rows_decrement(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(dialog) = self.table_insert_dialog.as_mut() {
            dialog.body_rows = dialog.body_rows.saturating_sub(1).max(1);
            cx.notify();
        }
    }

    pub(crate) fn on_table_rows_increment(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(dialog) = self.table_insert_dialog.as_mut() {
            dialog.body_rows += 1;
            cx.notify();
        }
    }

    pub(crate) fn on_table_columns_decrement(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(dialog) = self.table_insert_dialog.as_mut() {
            dialog.columns = dialog.columns.saturating_sub(1).max(1);
            cx.notify();
        }
    }

    pub(crate) fn on_table_columns_increment(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(dialog) = self.table_insert_dialog.as_mut() {
            dialog.columns += 1;
            cx.notify();
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
        let Some(dialog) = self.table_insert_dialog.take() else {
            return;
        };

        let table = TableData::new_empty(dialog.body_rows, dialog.columns);
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

        // A table inserted as the last block in its container leaves no line
        // below it, so in rendered mode the caret cannot move past the table.
        // Add a trailing empty paragraph to land on when nothing follows it.
        self.ensure_trailing_paragraph_after_structural(&new_block, cx);

        self.rebuild_table_runtimes(cx);
        if let Some(first_cell) = new_block
            .read(cx)
            .table_runtime
            .as_ref()
            .and_then(|runtime| runtime.header.first())
        {
            self.focus_block(first_cell.entity_id());
        }
        self.mark_dirty(cx);
        self.request_active_block_scroll_into_view(cx);
        cx.notify();
    }
}
#[cfg(test)]
mod tests {
    use super::{ContextMenuState, Editor, TableInsertTarget};
    use gpui::{AppContext, Point, TestAppContext, px};

    #[gpui::test]
    async fn context_submenu_stays_open_while_crossing_hover_gap(cx: &mut TestAppContext) {
        let editor = cx.new(|cx| Editor::from_markdown(cx, "alpha".to_string(), None));

        editor.update(cx, |editor, cx| {
            editor.open_insert_context_menu(
                Point {
                    x: px(24.0),
                    y: px(24.0),
                },
                TableInsertTarget::Append,
                cx,
            );

            editor.set_context_menu_hover_state(true, false, cx);
            let Some(ContextMenuState::Insert { submenu_open, .. }) = editor.context_menu.as_ref()
            else {
                panic!("expected insert context menu");
            };
            assert!(*submenu_open);
            assert!(editor.context_menu_submenu_close_task.is_none());

            editor.set_context_menu_hover_state(false, false, cx);
            let Some(ContextMenuState::Insert { submenu_open, .. }) = editor.context_menu.as_ref()
            else {
                panic!("expected insert context menu");
            };
            assert!(*submenu_open);
            assert!(editor.context_menu_submenu_close_task.is_some());

            editor.set_context_menu_hover_state(true, true, cx);
            let Some(ContextMenuState::Insert { submenu_open, .. }) = editor.context_menu.as_ref()
            else {
                panic!("expected insert context menu");
            };
            assert!(*submenu_open);
            assert!(editor.context_menu_submenu_close_task.is_none());
        });
    }
}
