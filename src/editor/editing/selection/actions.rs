//! Keyboard shortcuts, clipboard commands, and select-all state machine.

use gpui::*;

use crate::editor::controller::{CrossBlockSelection, CrossBlockSelectionEndpoint, Editor};
use crate::editor::editing::input::actions::{Copy, Cut, Delete, DeleteBackward};
use crate::editor::tree::block::Block;

impl Editor {
    pub(crate) fn on_copy_capture(
        &mut self,
        _: &Copy,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_active_tab() {
            cx.propagate();
            return;
        }
        let Some(markdown) = self.cross_block_selected_markdown(cx) else {
            cx.propagate();
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(markdown));
        cx.stop_propagation();
    }

    pub(crate) fn on_cut_capture(&mut self, _: &Cut, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_active_tab() {
            cx.propagate();
            return;
        }
        let Some(markdown) = self.cross_block_selected_markdown(cx) else {
            cx.propagate();
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(markdown));
        self.delete_cross_block_selection(cx);
        cx.stop_propagation();
    }

    pub(crate) fn on_delete_capture(
        &mut self,
        _: &Delete,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_active_tab() {
            cx.propagate();
            return;
        }
        if !self.delete_cross_block_selection(cx) {
            cx.propagate();
            return;
        }
        cx.stop_propagation();
    }

    pub(crate) fn on_delete_backward_capture(
        &mut self,
        _: &DeleteBackward,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.has_active_tab() {
            cx.propagate();
            return;
        }
        if !self.delete_cross_block_selection(cx) {
            cx.propagate();
            return;
        }
        cx.stop_propagation();
    }

    fn is_wysiwyg_document_fully_selected(&self, cx: &App) -> bool {
        let entries = self.doc().blocks();
        let Some(first) = entries.first() else {
            return false;
        };
        let Some(last) = entries.last() else {
            return false;
        };
        let Some(selection) = self.active_pane_selection().cross_block else {
            return false;
        };
        let last_len = last.entity.read(cx).display_len();
        selection.anchor
            == CrossBlockSelectionEndpoint {
                entity_id: first.entity.entity_id(),
                offset: 0,
            }
            && selection.focus
                == CrossBlockSelectionEndpoint {
                    entity_id: last.entity.entity_id(),
                    offset: last_len,
                }
    }

    fn select_focused_block_text_for_wysiwyg_select_all(
        &mut self,
        block: Entity<Block>,
        cx: &mut Context<Self>,
    ) {
        self.clear_cross_block_selection(cx);
        self.end_block_pointer_selection_sessions(cx);
        self.clear_table_axis_preview(cx);
        self.clear_table_axis_selection(cx);
        block.update(cx, |block, cx| {
            let len = block.display_len();
            block.selected_range = 0..len;
            block.selection_reversed = false;
            block.marked_range = None;
            block.vertical_motion_x = None;
            block.cursor_blink_epoch = std::time::Instant::now();
            cx.notify();
        });
        let pane = self.active_pane_state();
        pane.focus.active_entity = Some(block.entity_id());
        cx.notify();
    }

    fn select_all_wysiwyg_document(&mut self, cx: &mut Context<Self>) {
        if self.is_wysiwyg_document_fully_selected(cx) {
            return;
        }

        self.end_block_pointer_selection_sessions(cx);
        self.dismiss_contextual_overlays(cx);
        self.clear_table_axis_preview(cx);
        self.clear_table_axis_selection(cx);

        let entries = self.doc().blocks();
        let Some(first) = entries.first() else {
            return;
        };
        let Some(last) = entries.last() else {
            return;
        };
        let first_id = first.entity.entity_id();
        let last_id = last.entity.entity_id();
        let last_len = last.entity.read(cx).display_len();

        for entries in entries {
            entries.entity.update(cx, |block, cx| {
                let cursor = block.cursor_offset();
                let collapsed = cursor..cursor;
                if block.selected_range != collapsed {
                    block.selected_range = collapsed;
                    cx.notify();
                }
            });
        }

        {
            let selection = &mut self.active_pane_state().selection;
            selection.cross_block_drag = None;
            selection.cross_block = Some(CrossBlockSelection {
                anchor: CrossBlockSelectionEndpoint {
                    entity_id: first_id,
                    offset: 0,
                },
                focus: CrossBlockSelectionEndpoint {
                    entity_id: last_id,
                    offset: last_len,
                },
            });
        }
        self.sync_cross_block_selection_visuals(cx);
        cx.notify();
    }

    pub(crate) fn on_wysiwyg_select_all_press(
        &mut self,
        block: Entity<Block>,
        cx: &mut Context<Self>,
    ) {
        if !self.is_wysiwyg() {
            let state = self.active_pane_state();
            state.selection.select_all_cycle = None;
            return;
        }

        let now = std::time::Instant::now();
        let block_id = block.entity_id();
        let count = match self.active_pane_selection().select_all_cycle {
            Some(cycle)
                if cycle.entity_id == block_id
                    && now.duration_since(cycle.last_pressed_at)
                        <= Self::WYSIWYG_SELECT_ALL_CYCLE_WINDOW =>
            {
                cycle.count.saturating_add(1)
            }
            _ => 1,
        }
        .min(3);

        let state = self.active_pane_state();
        state.selection.select_all_cycle = Some(crate::editor::controller::WysiwygSelectAllCycle {
            entity_id: block_id,
            count,
            last_pressed_at: now,
        });

        if count == 1 {
            self.select_focused_block_text_for_wysiwyg_select_all(block, cx);
        } else {
            self.select_all_wysiwyg_document(cx);
        }
    }
}
