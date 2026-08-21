//! Block event router and dispatcher — coordinates mutations and undo tracking.

pub(crate) mod interactions;
pub(crate) mod structure_ops;
pub(crate) mod table_events;
pub(crate) mod text_edits;

use gpui::*;

use crate::editor::block_protocol::BlockEvent;
use crate::editor::controller::*;

impl Editor {
    pub(crate) fn on_block_event(
        &mut self,
        block: Entity<crate::editor::tree::block::Block>,
        event: &BlockEvent,
        cx: &mut Context<Self>,
    ) {
        if let BlockEvent::PrepareUndo { kind } = event {
            self.prepare_undo_capture_from_stable_snapshot(*kind);
            return;
        }

        if let BlockEvent::RequestReplaceCrossBlockSelection {
            text,
            selected_range_relative,
            mark_inserted_text,
            undo_kind,
        } = event
            && self.replace_cross_block_selection_with_text(
                text,
                selected_range_relative.clone(),
                *mark_inserted_text,
                *undo_kind,
                cx,
            )
        {
            return;
        }

        if matches!(event, BlockEvent::RequestRenderedSelectAll) {
            self.on_wysiwyg_select_all_press(block, cx);
            return;
        }

        if let BlockEvent::RequestPasteImage {
            leading,
            source,
            trailing,
        } = event
        {
            self.on_paste_image_request(block, leading, source, trailing, cx);
            return;
        }

        if let Some(binding) = self.table_cell_binding(block.entity_id()) {
            self.on_table_cell_event(binding, event, cx);
            return;
        }

        if event.clears_cross_block_selection() {
            let state = self.active_pane_state();
            state.selection.select_all_cycle = None;
            self.clear_cross_block_selection(cx);
        }

        let entries_before = self.doc().flatten_entries();
        let current_entry_index = entries_before
            .iter()
            .position(|entry| entry.entity.entity_id() == block.entity_id())
            .unwrap_or(0);

        match event.category() {
            crate::editor::block_protocol::BlockEventCategory::ContentChange => {
                self.on_content_change_event(&block, cx);
            }
            crate::editor::block_protocol::BlockEventCategory::TextEdit => {
                self.on_text_edit_event(&block, event, current_entry_index, &entries_before, cx);
            }
            crate::editor::block_protocol::BlockEventCategory::Structure => {
                self.on_structure_event(&block, event, current_entry_index, &entries_before, cx);
            }
            crate::editor::block_protocol::BlockEventCategory::Table => {
                self.on_table_event(&block, event, cx);
            }
            crate::editor::block_protocol::BlockEventCategory::Interaction => {
                self.on_interaction_event(&block, event, current_entry_index, &entries_before, cx);
            }
            crate::editor::block_protocol::BlockEventCategory::Lifecycle => {}
        }
    }

    fn on_content_change_event(
        &mut self,
        block: &Entity<crate::editor::tree::block::Block>,
        cx: &mut Context<Self>,
    ) {
        let should_restart_numbered_list = block.update(cx, |block, _cx| {
            block.take_numbered_list_restart_requested()
        });
        if should_restart_numbered_list {
            self.insert_list_group_separator_before(block.entity_id(), cx);
        }

        let callout_focus_target = self.materialize_empty_callout_shortcut(block, cx);

        let should_normalize_quote = block.update(cx, |block, _cx| {
            let requested = block.take_quote_reparse_requested();
            requested && block.marked_range.is_none()
        }) || Self::rendered_quote_text_requires_reparse(block, cx);

        self.refresh_rendered_quote_metadata_if_needed(block, cx);
        if should_normalize_quote {
            self.normalize_rendered_quote_structure(cx);
        } else {
            self.sync_references_after_block_change(block, cx);
        }
        if let Some(focus_id) = callout_focus_target {
            self.focus_block(focus_id);
        }
        self.mark_dirty(cx);
        self.request_active_block_scroll_into_view(self.active_pane_id(), cx);
        self.finalize_pending_undo_capture(cx);
    }
}
