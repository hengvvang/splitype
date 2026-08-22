//! Interaction events handler: selection sync, link click dispatch, and footnote popups.

use gpui::*;

use crate::editor::block_protocol::BlockEvent;
use crate::editor::controller::*;

impl Editor {
    pub(crate) fn update_footnote_tooltip(
        &mut self,
        id: &str,
        content: Option<SharedString>,
        position: Point<Pixels>,
        show: bool,
        cx: &mut Context<Self>,
    ) {
        let next = if !show {
            None
        } else if let Some(text) = content {
            let (_, pure_content) =
                crate::model::block::footnote::split_footnote_definition_text(&text);
            Some(FootnoteTooltipState {
                content: pure_content.to_string().into(),
                position,
            })
        } else {
            self.tab()
                .references
                .footnotes
                .binding(id)
                .and_then(|binding| self.focusable_entity_by_id(binding.definition_entity_id))
                .map(|entity| {
                    let plain = entity.read(cx).data.text.plain_text();
                    let (_, pure_content) =
                        crate::model::block::footnote::split_footnote_definition_text(&plain);
                    FootnoteTooltipState {
                        content: pure_content.to_string().into(),
                        position,
                    }
                })
        };
        if self.footnote_tooltip != next {
            self.footnote_tooltip = next;
            cx.notify();
        }
    }

    pub(crate) fn on_interaction_event(
        &mut self,
        block: &Entity<crate::editor::tree::block::Block>,
        event: &BlockEvent,
        current_entry_index: usize,
        entries_before: &[crate::editor::tree::document::BlockEntry],
        cx: &mut Context<Self>,
    ) {
        match event {
            BlockEvent::RequestOpenLink {
                prompt_target,
                open_target,
            } => {
                self.request_open_link_prompt(prompt_target.clone(), open_target.clone(), cx);
            }
            BlockEvent::RequestJumpToFootnoteDefinition { id, .. } => {
                let _ = self.jump_to_footnote_definition(id, cx);
                cx.notify();
            }
            BlockEvent::RequestJumpToFootnoteBackref { id } => {
                let _ = self.jump_to_footnote_backref(id, cx);
                cx.notify();
            }
            BlockEvent::RequestFootnoteTooltip {
                id,
                content,
                position,
                show,
            } => {
                self.update_footnote_tooltip(id, content.clone(), *position, *show, cx);
            }
            BlockEvent::RequestFocusPrevious { preferred_x } => {
                if current_entry_index == 0 {
                    return;
                }

                let target = entries_before[current_entry_index - 1].entity.clone();
                // Entering a table from below lands in a body cell instead of
                // the non-editable table container.
                if target.read(cx).kind() == BlockKind::Table
                    && self.focus_table_entry_cell(&target, false, cx)
                {
                    return;
                }
                let target_x = preferred_x.map(px);
                let offset = target
                    .read(cx)
                    .entry_offset_for_vertical_focus(true, target_x);
                self.focus_block(target.entity_id());
                target.update(cx, move |target, cx| {
                    target.move_to_with_preferred_x(offset, target_x, cx);
                });
                cx.notify();
            }
            BlockEvent::RequestFocusNext { preferred_x } => {
                if current_entry_index + 1 >= entries_before.len() {
                    // A trailing multi-line block (code, math, ...) has nowhere
                    // below to move to, so give it a paragraph to land on and
                    // focus that, matching how a trailing table behaves.
                    if block.read(cx).kind().is_multiline_text_block() {
                        self.ensure_trailing_paragraph_after_structural(block, cx);
                        let entries = self.doc().cloned_entries();
                        if let Some(landing) = entries
                            .iter()
                            .position(|v| v.entity.entity_id() == block.entity_id())
                            .and_then(|index| entries.get(index + 1))
                            .map(|v| v.entity.clone())
                        {
                            self.focus_block(landing.entity_id());
                            landing.update(cx, |landing, cx| landing.move_to(0, cx));
                            cx.notify();
                        }
                    }
                    return;
                }

                let target = entries_before[current_entry_index + 1].entity.clone();
                // Entering a table from above lands in a header cell instead of
                // the non-editable table container.
                if target.read(cx).kind() == BlockKind::Table
                    && self.focus_table_entry_cell(&target, true, cx)
                {
                    return;
                }
                let target_x = preferred_x.map(px);
                let offset = target
                    .read(cx)
                    .entry_offset_for_vertical_focus(false, target_x);
                self.focus_block(target.entity_id());
                target.update(cx, move |target, cx| {
                    target.move_to_with_preferred_x(offset, target_x, cx);
                });
                cx.notify();
            }
            BlockEvent::RequestBlockUp => {
                if current_entry_index == 0 {
                    return;
                }

                let target = entries_before[current_entry_index - 1].entity.clone();
                if target.read(cx).kind() == BlockKind::Table
                    && self.focus_table_entry_cell(&target, false, cx)
                {
                    return;
                }
                self.focus_block(target.entity_id());
                target.update(cx, |target, cx| target.move_to(0, cx));
                cx.notify();
            }
            BlockEvent::RequestBlockDown => {
                if current_entry_index + 1 >= entries_before.len() {
                    return;
                }

                let target = entries_before[current_entry_index + 1].entity.clone();
                if target.read(cx).kind() == BlockKind::Table
                    && self.focus_table_entry_cell(&target, true, cx)
                {
                    return;
                }
                self.focus_block(target.entity_id());
                target.update(cx, |target, cx| target.move_to(0, cx));
                cx.notify();
            }
            BlockEvent::RequestFocus => {
                self.clear_table_axis_preview(cx);
                self.clear_table_axis_selection(cx);
                self.focus_block(block.entity_id());
                for entry in self.doc().blocks() {
                    entry.entity.update(cx, |_, cx| cx.notify());
                }
                cx.notify();
            }
            _ => {}
        }
    }
}
