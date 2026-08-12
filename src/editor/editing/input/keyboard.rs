//! Keyboard event handling for the Editor.
//!
//! This module owns the full [`Editor`] keyboard event handler: tab-key
//! routing between blocks (indent / outdent) and the focused-block query it
//! uses. Focus management lives in [`super::focus`], rendered-quote metadata
//! refresh in [`super::quote_metadata`], and block-event classification in
//! [`super::block_events`]; the editor's keyboard tests live here because
//! they exercise the whole input pipeline end to end.
//!
//! # Event dispatch flow
//!
//! ```text
//! Block (key event) → BlockAction (emitted)
//!   → Editor::on_block_event
//!     ├─ Early returns: PrepareUndo / ReplaceCrossBlockSelection /
//!     │                  RenderedSelectAll / PasteImage
//!     ├─ Table-cell routing → Editor::on_table_cell_event
//!     └─ Main match: classify_block_action maps each variant → handler
//! ```

use gpui::*;

use super::actions::{IndentBlock, OutdentBlock};
use crate::editor::controller::Editor;
use crate::model::parse::BlockKind;

impl Editor {
    pub(crate) fn focused_block_for_tab_key(
        &self,
        window: &mut Window,
        cx: &App,
    ) -> Option<Entity<crate::editor::tree::block::Block>> {
        let is_focused = |block: &Entity<crate::editor::tree::block::Block>| {
            let block = block.read(cx);
            block.focus_handle.is_focused(window)
                || block.code_language_focus_handle.is_focused(window)
        };

        if let Some(block) = self
            .pane_state_ref(self.active_pane_id())
            .and_then(|state| state.focus.active_entity)
            .and_then(|entity_id| self.focusable_entity_by_id(entity_id))
            .filter(is_focused)
        {
            return Some(block);
        }

        for binding in self.tab().tables.cells.values() {
            if is_focused(&binding.cell) {
                return Some(binding.cell.clone());
            }
        }

        self.doc()
            .blocks()
            .iter()
            .find_map(|entry| is_focused(&entry.entity).then(|| entry.entity.clone()))
    }

    pub(crate) fn on_editor_key_down_capture(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.keystroke.key != "tab" {
            return;
        }
        if !self.has_active_tab() {
            return;
        }

        let modifiers = event.keystroke.modifiers;
        if modifiers.control || modifiers.platform || modifiers.alt || modifiers.function {
            return;
        }

        let Some(target) = self.focused_block_for_tab_key(window, cx) else {
            return;
        };

        let handles_tab = {
            let block = target.read(cx);
            if block.code_language_focus_handle.is_focused(window) {
                cx.stop_propagation();
                return;
            }
            block.is_table_cell()
                || block.kind().is_list_item()
                || block.kind() == BlockKind::Paragraph
                || block.kind().is_code_block()
        };

        if !handles_tab {
            return;
        }

        if modifiers.shift {
            target.update(cx, |block, block_cx| {
                block.on_outdent_block(&OutdentBlock, window, block_cx);
            });
        } else {
            target.update(cx, |block, block_cx| {
                block.on_indent_block(&IndentBlock, window, block_cx);
            });
        }
        cx.stop_propagation();
    }
}
