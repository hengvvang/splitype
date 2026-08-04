//! Navigation: scrolling, page keys, scrollbar drag, table-cell focus
//! movement, and quote/callout focus routing.

use std::time::Duration;

use gpui::*;

use super::shortcuts::{JumpToBottom, JumpToTop, PageDown, PageUp};
use crate::editor::actions::BlockAction;
use crate::editor::tree::block::CollapsedCaretAffinity;
use crate::editor::controller::*;
use crate::model::block::BlockKind;
use crate::model::syntax::table::TableCellPosition;


impl Editor {
    pub(crate) fn bump_scrollbar_visibility(&mut self, cx: &mut Context<Self>) {
        let duration = Duration::from_millis(900);
        self.scroll.scrollbar_visible_until = Instant::now() + duration;

        let weak_editor = cx.entity().downgrade();
        self.scroll.scrollbar_fade_task = Some(cx.spawn(
            async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
                cx.background_executor()
                    .timer(duration + Duration::from_millis(50))
                    .await;
                let _ = weak_editor.update(cx, |this, cx| {
                    this.scroll.scrollbar_fade_task = None;
                    cx.notify();
                });
            },
        ));

        cx.notify();
    }


    pub(crate) fn on_editor_hover(
        &mut self,
        hovered: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.scroll.scrollbar_hovered = *hovered;
        if *hovered {
            self.bump_scrollbar_visibility(cx);
        } else {
            cx.notify();
        }
    }


    pub(crate) fn toggle_menu_bar_expanded(&mut self, cx: &mut Context<Self>) {
        self.chrome.menu_bar_expanded = !self.chrome.menu_bar_expanded;
        if !self.chrome.menu_bar_expanded {
            self.chrome.menu_bar_open = None;
            self.chrome.menu_submenu_open = None;
        }
        cx.notify();
    }

    #[allow(dead_code)]

    pub(crate) fn on_menu_bar_hover(
        &mut self,
        hovered: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_menu_bar_hovered(*hovered, cx);
    }


    pub(crate) fn on_menu_panel_hover(
        &mut self,
        hovered: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_menu_panel_hovered(*hovered, cx);
    }


    pub(crate) fn on_menu_submenu_panel_hover(
        &mut self,
        hovered: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_menu_submenu_panel_hovered(*hovered, cx);
    }


    pub(crate) fn on_menu_submenu_bridge_hover(
        &mut self,
        hovered: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_menu_submenu_bridge_hovered(*hovered, cx);
    }


    pub(crate) fn on_editor_mouse_down(
        &mut self,
        _event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dismiss_menu_bar_from_body(cx);
        self.clear_table_axis_preview(cx);
        self.clear_table_axis_selection(cx);
    }


    pub(crate) fn on_editor_scroll_wheel(
        &mut self,
        _event: &ScrollWheelEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.bump_scrollbar_visibility(cx);
    }


    pub(crate) fn on_page_up(&mut self, _: &PageUp, _window: &mut Window, cx: &mut Context<Self>) {
        let page = self.scroll.handle.bounds().size.height;
        self.scroll_viewport_by(page, cx);
    }


    pub(crate) fn on_page_down(
        &mut self,
        _: &PageDown,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let page = self.scroll.handle.bounds().size.height;
        self.scroll_viewport_by(-page, cx);
    }


    pub(crate) fn on_jump_to_top(
        &mut self,
        _: &JumpToTop,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_vertical_scroll_offset(px(0.0), cx);
    }


    pub(crate) fn on_jump_to_bottom(
        &mut self,
        _: &JumpToBottom,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let max_offset_y = self.scroll.handle.max_offset().height.max(px(0.0));
        self.set_vertical_scroll_offset(-max_offset_y, cx);
    }

    /// Scrolls the viewport vertically by `delta`. A positive `delta` moves
    /// toward the start of the document; a negative one moves toward the end.
    /// One page is the current viewport height, so the step tracks window size.

    pub(crate) fn scroll_viewport_by(&mut self, delta: Pixels, cx: &mut Context<Self>) {
        let target = self.scroll.handle.offset().y + delta;
        self.set_vertical_scroll_offset(target, cx);
    }

    /// Applies an absolute vertical scroll offset, clamped to the scrollable
    /// range. Offsets run from 0 at the top to `-max_offset` at the bottom.

    pub(crate) fn set_vertical_scroll_offset(&mut self, target_y: Pixels, cx: &mut Context<Self>) {
        let max_offset_y = self.scroll.handle.max_offset().height.max(px(0.0));
        let mut offset = self.scroll.handle.offset();
        offset.y = target_y.min(px(0.0)).max(-max_offset_y);
        self.scroll.handle.set_offset(offset);
        // A direct viewport scroll should stick, so cancel any queued pass that
        // would otherwise re-center the active block on the next frame.
        self.focus.pending_scroll_active_block_into_view = false;
        self.focus.pending_scroll_recheck_after_layout = false;
        self.bump_scrollbar_visibility(cx);
        cx.notify();
    }


    pub(crate) fn start_scrollbar_drag(
        &mut self,
        pointer_offset_y: f32,
        track_height: f32,
        thumb_height: f32,
        max_scroll_y: f32,
        cx: &mut Context<Self>,
    ) {
        self.scroll.scrollbar_drag = Some(crate::editor::controller::ScrollbarDragSession {
            pointer_offset_y: pointer_offset_y.clamp(0.0, thumb_height.max(0.0)),
            track_height,
            thumb_height,
            max_scroll_y,
        });
        self.focus.pending_scroll_active_block_into_view = false;
        self.focus.pending_scroll_recheck_after_layout = false;
        self.bump_scrollbar_visibility(cx);
        cx.notify();
    }


    pub(crate) fn update_scrollbar_drag(
        &mut self,
        pointer_y_in_track: f32,
        cx: &mut Context<Self>,
    ) {
        let Some(drag) = self.scroll.scrollbar_drag else {
            return;
        };

        let travel = (drag.track_height - drag.thumb_height).max(0.0);
        let thumb_top = (pointer_y_in_track - drag.pointer_offset_y).clamp(0.0, travel);
        let scroll_y = Self::scroll_offset_for_thumb_top(
            thumb_top,
            drag.track_height,
            drag.thumb_height,
            drag.max_scroll_y,
        );

        let mut offset = self.scroll.handle.offset();
        offset.y = -px(scroll_y);
        self.scroll.handle.set_offset(offset);
        self.bump_scrollbar_visibility(cx);
        cx.notify();
    }


    pub(crate) fn end_scrollbar_drag(&mut self, cx: &mut Context<Self>) {
        if self.scroll.scrollbar_drag.take().is_some() {
            self.bump_scrollbar_visibility(cx);
            cx.notify();
        }
    }


    pub(crate) fn focus_table_cell_position(
        &mut self,
        table_block: &Entity<crate::editor::tree::block::Block>,
        position: TableCellPosition,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(cell) = table_block
            .read(cx)
            .table_runtime
            .as_ref()
            .and_then(|runtime| runtime.cell(position))
        else {
            return false;
        };
        self.focus_block(cell.entity_id());
        cx.notify();
        true
    }

    /// Focus a cell when keyboard navigation enters a table from an adjacent
    /// block. Entering from above lands on the first header cell; entering from
    /// below lands on the first cell of the last body row, falling back to the
    /// header when the table has no body rows.

    pub(crate) fn focus_table_entry_cell(
        &mut self,
        table_block: &Entity<crate::editor::tree::block::Block>,
        from_top: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(runtime) = table_block.read(cx).table_runtime.clone() else {
            return false;
        };
        let cell = if from_top {
            runtime.header.first().cloned()
        } else {
            runtime
                .rows
                .last()
                .and_then(|row| row.first())
                .cloned()
                .or_else(|| runtime.header.first().cloned())
        };
        let Some(cell) = cell else {
            return false;
        };
        self.focus_block(cell.entity_id());
        cx.notify();
        true
    }

    /// Move focus from a table edge to the block immediately above (delta < 0)
    /// or below (delta > 0) it, mirroring how plain blocks transfer focus when
    /// the caret leaves their first or last line. When the neighbor is itself a
    /// table, drop into one of its cells so the caret stays editable instead of
    /// landing on the table container. `to_block_start` lands the caret at the
    /// neighbor's start (Block Up/Down semantics) rather than the nearest edge
    /// (Move Up/Down semantics).

    pub(crate) fn focus_block_adjacent_to_table(
        &mut self,
        table_block: &Entity<crate::editor::tree::block::Block>,
        delta: i32,
        to_block_start: bool,
        cx: &mut Context<Self>,
    ) {
        let visible = self.document.flatten_visible_blocks();
        let Some(index) = visible
            .iter()
            .position(|visible| visible.entity.entity_id() == table_block.entity_id())
        else {
            return;
        };
        let target_index = if delta < 0 {
            index.checked_sub(1)
        } else {
            Some(index + 1)
        };
        let Some(target) = target_index
            .and_then(|target_index| visible.get(target_index))
            .map(|visible| visible.entity.clone())
        else {
            return;
        };
        if target.read(cx).kind() == BlockKind::Table
            && self.focus_table_entry_cell(&target, delta > 0, cx)
        {
            return;
        }
        self.focus_block(target.entity_id());
        if to_block_start {
            target.update(cx, |target, cx| target.move_to(0, cx));
        } else {
            let prefer_last_line = delta < 0;
            let offset = target
                .read(cx)
                .entry_offset_for_vertical_focus(prefer_last_line, None);
            target.update(cx, move |target, cx| {
                target.move_to_with_preferred_x(offset, None, cx);
            });
        }
        cx.notify();
    }


    pub(crate) fn focus_table_cell_horizontal_neighbor(
        &mut self,
        table_block: &Entity<crate::editor::tree::block::Block>,
        position: TableCellPosition,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let Some(runtime) = table_block.read(cx).table_runtime.clone() else {
            return;
        };
        let columns = runtime.header.len();
        let total_rows = 1 + runtime.rows.len();
        if columns == 0 || total_rows == 0 {
            return;
        }

        let linear = position.row * columns + position.column;
        let next = if delta < 0 {
            linear.checked_sub(delta.unsigned_abs() as usize)
        } else {
            linear.checked_add(delta as usize)
        };
        let Some(next) = next else {
            return;
        };
        if next >= total_rows * columns {
            return;
        }

        let next_position = TableCellPosition {
            row: next / columns,
            column: next % columns,
        };
        let _ = self.focus_table_cell_position(table_block, next_position, cx);
    }


    pub(crate) fn focus_table_cell_vertical_neighbor(
        &mut self,
        table_block: &Entity<crate::editor::tree::block::Block>,
        position: TableCellPosition,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let Some(runtime) = table_block.read(cx).table_runtime.clone() else {
            return;
        };
        let max_row = runtime.rows.len();
        let next_row = if delta < 0 {
            position.row.checked_sub(delta.unsigned_abs() as usize)
        } else {
            position.row.checked_add(delta as usize)
        };
        // Moving past the first/last row leaves the table for the adjacent
        // block rather than stopping at the edge.
        let Some(next_row) = next_row.filter(|row| *row <= max_row) else {
            self.focus_block_adjacent_to_table(table_block, delta, false, cx);
            return;
        };

        let next_position = TableCellPosition {
            row: next_row,
            column: position.column.min(runtime.header.len().saturating_sub(1)),
        };
        let _ = self.focus_table_cell_position(table_block, next_position, cx);
    }


    pub(crate) fn on_table_cell_event(
        &mut self,
        binding: crate::editor::controller::TableCellBinding,
        event: &BlockAction,
        cx: &mut Context<Self>,
    ) {
        if Self::block_event_clears_cross_block_selection(event) {
            self.selection.select_all_cycle = None;
            self.clear_cross_block_selection(cx);
        }

        match event {
            BlockAction::Changed => {
                self.sync_table_record_from_runtime(&binding.table_block, cx);
                self.rebuild_image_runtimes(cx);
                self.mark_dirty(cx);
                self.request_active_block_scroll_into_view(cx);
                self.finalize_pending_undo_capture(cx);
            }
            BlockAction::RequestOpenLink {
                prompt_target,
                open_target,
            } => {
                self.request_open_link_prompt(prompt_target.clone(), open_target.clone(), cx);
            }
            BlockAction::RequestJumpToFootnoteDefinition { id, .. } => {
                let _ = self.jump_to_footnote_definition(id, cx);
            }
            BlockAction::RequestJumpToFootnoteBackref { id } => {
                let _ = self.jump_to_footnote_backref(id, cx);
            }
            BlockAction::RequestTableCellMoveHorizontal { delta } => {
                self.focus_table_cell_horizontal_neighbor(
                    &binding.table_block,
                    binding.position,
                    *delta,
                    cx,
                );
            }
            BlockAction::RequestTableCellMoveVertical { delta } => {
                self.focus_table_cell_vertical_neighbor(
                    &binding.table_block,
                    binding.position,
                    *delta,
                    cx,
                );
            }
            BlockAction::RequestNewline { .. } => {
                let Some(location) = self
                    .document
                    .find_block_location(binding.table_block.entity_id())
                else {
                    return;
                };
                self.clear_table_axis_preview(cx);
                self.clear_table_axis_selection(cx);
                self.sync_table_record_from_runtime(&binding.table_block, cx);
                self.prepare_undo_capture(
                    crate::editor::actions::UndoCaptureKind::NonCoalescible,
                    cx,
                );
                let new_block = Self::new_block(cx, BlockData::paragraph(String::new()));
                self.document.insert_blocks_at(
                    location.parent,
                    location.index + 1,
                    vec![new_block.clone()],
                    cx,
                );
                self.rebuild_image_runtimes(cx);
                self.focus_block(new_block.entity_id());
                self.mark_dirty(cx);
                self.request_active_block_scroll_into_view(cx);
                self.finalize_pending_undo_capture(cx);
                cx.notify();
            }
            BlockAction::RequestFocus => {
                self.close_menu_bar(cx);
                self.clear_table_axis_preview(cx);
                self.clear_table_axis_selection(cx);
                self.focus_block(binding.cell.entity_id());
                cx.notify();
            }
            BlockAction::RequestFocusPrev { .. } => {
                self.focus_table_cell_vertical_neighbor(
                    &binding.table_block,
                    binding.position,
                    -1,
                    cx,
                );
            }
            BlockAction::RequestFocusNext { .. } => {
                self.focus_table_cell_vertical_neighbor(
                    &binding.table_block,
                    binding.position,
                    1,
                    cx,
                );
            }
            // Block Up/Down treat the table as a single block: leave it
            // entirely for the block above/below rather than stepping by cell.
            BlockAction::RequestBlockUp => {
                self.focus_block_adjacent_to_table(&binding.table_block, -1, true, cx);
            }
            BlockAction::RequestBlockDown => {
                self.focus_block_adjacent_to_table(&binding.table_block, 1, true, cx);
            }
            _ => {}
        }
    }


    pub(crate) fn nearest_quote_ancestor(
        &self,
        entity_id: EntityId,
        cx: &App,
    ) -> Option<Entity<crate::editor::tree::block::Block>> {
        let mut current = self.focusable_entity_by_id(entity_id)?;
        loop {
            if current.read(cx).kind().is_quote_container() {
                return Some(current);
            }
            let location = self.document.find_block_location(current.entity_id())?;
            current = location.parent?;
        }
    }


    pub(crate) fn topmost_quote_ancestor(
        &self,
        entity_id: EntityId,
        cx: &App,
    ) -> Option<Entity<crate::editor::tree::block::Block>> {
        let mut current = self.nearest_quote_ancestor(entity_id, cx)?;
        loop {
            let Some(location) = self.document.find_block_location(current.entity_id()) else {
                break;
            };
            let Some(parent) = location.parent.clone() else {
                break;
            };
            if !parent.read(cx).kind().is_quote_container() {
                break;
            }
            current = parent;
        }
        Some(current)
    }


    pub(crate) fn quote_break_insertion_target(
        &self,
        entity_id: EntityId,
        cx: &App,
    ) -> Option<(Option<Entity<crate::editor::tree::block::Block>>, usize)> {
        let quote_block = self.nearest_quote_ancestor(entity_id, cx)?;
        let location = self.document.find_block_location(quote_block.entity_id())?;
        Some((location.parent.clone(), location.index + 1))
    }


    pub(crate) fn callout_break_insertion_target(
        &self,
        entity_id: EntityId,
        cx: &App,
    ) -> Option<(Option<Entity<crate::editor::tree::block::Block>>, usize)> {
        let callout_root = self.topmost_quote_ancestor(entity_id, cx)?;
        let location = self
            .document
            .find_block_location(callout_root.entity_id())?;
        Some((location.parent.clone(), location.index + 1))
    }


    pub(crate) fn ensure_callout_body_entry(
        &mut self,
        callout: &Entity<crate::editor::tree::block::Block>,
        cx: &mut Context<Self>,
    ) -> Option<Entity<crate::editor::tree::block::Block>> {
        if !matches!(callout.read(cx).kind(), BlockKind::Callout(_)) {
            return None;
        }

        if let Some(first_child) = callout.read(cx).children.first().cloned() {
            return Some(first_child);
        }

        let body = Self::new_block(cx, BlockData::paragraph(String::new()));
        self.document
            .insert_blocks_at(Some(callout.clone()), 0, vec![body.clone()], cx);
        Some(body)
    }


    pub(crate) fn materialize_empty_callout_shortcut(
        &mut self,
        block: &Entity<crate::editor::tree::block::Block>,
        cx: &mut Context<Self>,
    ) -> Option<EntityId> {
        if self.mode != crate::editor::controller::EditorMode::Wysiwyg {
            return None;
        }

        let (kind, text_markdown, has_children) = block.read_with(cx, |block, _cx| {
            (
                block.kind(),
                block.record.text.serialize_markdown(),
                !block.children.is_empty(),
            )
        });
        if kind != BlockKind::Blockquote || has_children {
            return None;
        }

        let Some((variant, title)) =
            crate::model::block::CalloutKind::parse_header_line(&text_markdown)
        else {
            return None;
        };

        block.update(cx, |block, cx| {
            block.record.kind = BlockKind::Callout(variant);
            block.record.set_text(RichText::from_markdown(&title));
            block.sync_edit_mode_from_kind();
            block.sync_render_cache();
            block.cursor_blink_epoch = Instant::now();
            cx.notify();
        });
        let body = self.ensure_callout_body_entry(block, cx)?;
        Some(body.entity_id())
    }


    pub(crate) fn downgrade_empty_callout_body_to_quote(
        &mut self,
        block: &Entity<crate::editor::tree::block::Block>,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(location) = self.document.find_block_location(block.entity_id()) else {
            return false;
        };
        let Some(parent) = location.parent.clone() else {
            return false;
        };

        let (header_markdown, only_child, block_is_empty_leaf) = {
            let parent_ref = parent.read(cx);
            let Some(variant) = parent_ref.kind().callout_kind() else {
                return false;
            };
            let block_ref = block.read(cx);
            (
                variant.header_markdown(&parent_ref.record.text.serialize_markdown()),
                parent_ref.children.len() == 1,
                block_ref.kind() == BlockKind::Paragraph
                    && block_ref.display_text().is_empty()
                    && block_ref.children.is_empty(),
            )
        };
        if !only_child || !block_is_empty_leaf {
            return false;
        }

        self.prepare_undo_capture(
            crate::editor::actions::UndoCaptureKind::NonCoalescible,
            cx,
        );
        self.document.with_structure_mutation(cx, |document, cx| {
            let _ = document.remove_block_by_id_raw(block.entity_id(), cx);
            parent.update(cx, |parent, cx| {
                parent.record.kind = BlockKind::Blockquote;
                parent
                    .record
                    .set_text(RichText::from_markdown(&header_markdown));
                parent.sync_edit_mode_from_kind();
                parent.sync_render_cache();
                parent.assign_collapsed_selection_offset(0, CollapsedCaretAffinity::Default, None);
                parent.marked_range = None;
                parent.cursor_blink_epoch = Instant::now();
                cx.notify();
            });
        });
        self.focus_block(parent.entity_id());
        self.rebuild_image_runtimes(cx);
        self.mark_dirty(cx);
        self.finalize_pending_undo_capture(cx);
        cx.notify();
        true
    }

}
