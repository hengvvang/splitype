//! Text-editing action handlers on a focused block: newline splitting,
//! deletion, indentation, and the tab-key routing in
//! [`on_block_key_down`](Block::on_block_key_down).
//!
//! Structural changes that cross block boundaries are emitted as
//! `BlockEvent`s for the parent editor to resolve.

use gpui::*;

use crate::document::protocol::{BlockEvent, UndoCaptureKind};
use crate::actions::{
    Delete, DeleteBackward, IndentBlock, Newline, OutdentBlock, WordDeleteBackward,
    WordDeleteForward,
};
use crate::document::block::{Block, CollapsedCaretAffinity};
use crate::markdown::inline::text::BlockText;
use crate::markdown::parse::BlockKind;
impl Block {
    pub fn is_leaf_quote(&self) -> bool {
        self.kind() == BlockKind::Blockquote
            && self.children.is_empty()
            && !self.display_text().contains('\n')
    }

    pub fn is_leaf_callout(&self) -> bool {
        matches!(self.kind(), BlockKind::Callout(_)) && self.children.is_empty()
    }

    pub fn is_empty_leaf_quote(&self) -> bool {
        self.is_leaf_quote() && self.selected_range.is_empty() && self.is_empty()
    }

    pub fn downgrade_leaf_callout_to_quote_at_start(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.is_leaf_callout() || !self.selected_range.is_empty() || self.cursor_offset() != 0 {
            return false;
        }

        let BlockKind::Callout(variant) = self.kind() else {
            return false;
        };
        let header_markdown = variant.header_markdown(&self.data.text.serialize_markdown());
        self.data.kind = BlockKind::Blockquote;
        self.data
            .set_text(BlockText::from_markdown(&header_markdown));
        self.sync_edit_mode_from_kind();
        self.sync_render_cache();
        self.assign_collapsed_selection_offset(0, CollapsedCaretAffinity::Default, None);
        self.marked_range = None;
        self.cursor_blink_epoch = std::time::Instant::now();
        cx.emit(BlockEvent::Changed);
        cx.notify();
        true
    }

    pub fn downgrade_empty_leaf_quote_to_paragraph(&mut self, cx: &mut Context<Self>) -> bool {
        if self.is_empty_leaf_quote() {
            self.convert_to_paragraph(cx);
            return true;
        }
        false
    }
    /// If the code block's last line is a bare fence (three or more backticks
    /// or tildes, no info string), returns the byte offset to cut from so the
    /// whole line is removed; otherwise `None`.
    pub fn trailing_code_fence_line_start(&self) -> Option<usize> {
        let text = self.display_text();
        let line_start = text.rfind('\n').map(|idx| idx + 1).unwrap_or(0);
        let is_bare_fence = BlockKind::parse_code_fence_opening(&text[line_start..])
            .is_some_and(|fence| fence.language.is_none());
        // Cut from the preceding newline too, unless the fence is the only line.
        is_bare_fence.then(|| line_start.saturating_sub(1))
    }

    pub fn on_newline(&mut self, _: &Newline, window: &mut Window, cx: &mut Context<Self>) {
        // Enter is ordered from special editors to rich-text splitting:
        // table/source/code/quote-like blocks keep local newline semantics,
        // while normal rendered blocks emit an editor-level split request.
        if self.is_table_cell() {
            cx.emit(BlockEvent::RequestTableCellMoveVertical { delta: 1 });
            return;
        }

        if self.editor_selection_range.is_some() {
            cx.emit(BlockEvent::RequestReplaceCrossBlockSelection {
                text: "\n".to_string(),
                selected_range_relative: None,
                mark_inserted_text: false,
                undo_kind: UndoCaptureKind::NonCoalescible,
            });
            return;
        }

        if self.is_verbatim_mode() {
            if !self.selected_range.is_empty() {
                self.replace_text_in_range(None, "", window, cx);
            }
            self.replace_text_in_range(None, "\n", window, cx);
            return;
        }

        if self.kind() == BlockKind::Paragraph
            && self.selected_range.is_empty()
            && self.cursor_offset() == self.display_len()
            && BlockKind::parse_thematic_break_line(self.display_text())
            // A dash run is also a setext underline; defer it to the editor so a
            // preceding paragraph can become a heading (the editor falls back to
            // a separator when there is no heading target).
            && BlockKind::parse_setext_underline(self.display_text()).is_none()
        {
            self.convert_to_separator(cx);
            cx.emit(BlockEvent::RequestNewline {
                trailing: BlockText::plain(String::new()),
                source_already_mutated: true,
            });
            return;
        }

        // `$$` then Enter opens a display-math block. Keying off the caret sitting
        // right after a leading `$$` (rather than the line being exactly `$$`)
        // means it also fires after pressing Home on an existing line and typing
        // the fence in front of a formula: the rest of the line becomes the math
        // body instead of being split off into a new paragraph.
        if self.kind() == BlockKind::Paragraph
            && self.selected_range.is_empty()
            && self.cursor_offset() == "$$".len()
            && self.display_text().starts_with("$$")
        {
            let body = self.display_text()["$$".len()..].to_string();
            self.enter_math_block(&body, cx);
            return;
        }

        if self.kind() == BlockKind::Paragraph
            && self.selected_range.is_empty()
            && self.cursor_offset() == self.display_len()
            && let Some(fence) = BlockKind::parse_code_fence_opening(self.display_text())
        {
            self.enter_code_block(fence.language, cx);
            return;
        }

        if self.kind().is_thematic_break() {
            cx.emit(BlockEvent::RequestNewline {
                trailing: BlockText::plain(String::new()),
                source_already_mutated: false,
            });
            return;
        }

        if self.kind().is_list_item() && self.selected_range.is_empty() && self.is_empty() {
            cx.emit(BlockEvent::RequestOutdent);
            return;
        }

        if self.kind() == BlockKind::Blockquote {
            if !self.selected_range.is_empty() {
                self.replace_text_in_range(None, "", window, cx);
            }
            self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
            self.replace_text_in_range(None, "\n", window, cx);
            return;
        }

        if matches!(self.kind(), BlockKind::Callout(_)) {
            cx.emit(BlockEvent::RequestEnterCalloutBody);
            return;
        }

        // In a code block, Enter inserts a newline into the block content
        // rather than splitting the block.  Pressing Enter on an empty
        // code block exits back to a paragraph.
        if self.kind().is_code_block() {
            if self.selected_range.is_empty() && self.is_empty() {
                self.convert_to_paragraph(cx);
                return;
            }
            // Typing a bare closing fence on the last line and pressing Enter
            // leaves the block, matching source mode.
            if self.selected_range.is_empty()
                && self.cursor_offset() == self.display_len()
                && let Some(fence_start) = self.trailing_code_fence_line_start()
            {
                let fence_end = self.display_len();
                self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
                self.replace_text_in_display_range(fence_start..fence_end, "", None, false, cx);
                cx.emit(BlockEvent::RequestNewline {
                    trailing: BlockText::plain(String::new()),
                    source_already_mutated: true,
                });
                return;
            }
            if !self.selected_range.is_empty() {
                self.replace_text_in_range(None, "", window, cx);
            }
            self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
            self.replace_text_in_range(None, "\n", window, cx);
            return;
        }

        if self.collapsed_caret_inherits_inline_code_style() {
            self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
            self.replace_text_in_range(None, "\n", window, cx);
            return;
        }

        if !self.selected_range.is_empty() {
            self.replace_text_in_range(None, "", window, cx);
        }

        let cursor = self.cursor_offset();
        if self.selected_range.is_empty() && cursor == 0 {
            cx.emit(BlockEvent::RequestNewlineAbove);
            return;
        }

        let (leading, trailing) = self.split_text(cursor);
        self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
        self.data.set_text(leading);
        self.mark_changed(cx);
        let cursor = self.display_len();
        self.assign_collapsed_selection_offset(cursor, CollapsedCaretAffinity::Default, None);
        self.marked_range = None;
        cx.emit(BlockEvent::RequestNewline {
            trailing,
            source_already_mutated: true,
        });
    }

    pub fn on_delete_backward(
        &mut self,
        _: &DeleteBackward,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_table_cell() {
            if self.selected_range.is_empty() {
                let previous = self.previous_boundary(self.cursor_offset());
                if previous == self.cursor_offset() {
                    return;
                }
                self.select_to(previous, cx);
            }
            self.replace_text_in_range(None, "", window, cx);
            return;
        }

        if self.is_verbatim_mode() {
            if self.selected_range.is_empty() {
                self.select_to(self.previous_boundary(self.cursor_offset()), cx);
            }
            self.replace_text_in_range(None, "", window, cx);
            return;
        }

        if self.selected_range.is_empty() && self.cursor_offset() == 0 {
            if self.kind() == BlockKind::Paragraph && self.is_direct_list_child() && self.is_empty()
            {
                cx.emit(BlockEvent::RequestOutdent);
                return;
            }
            if self.is_nested_list_item() {
                cx.emit(BlockEvent::RequestDowngradeNestedListItemToChildParagraph);
                return;
            }
            match self.kind() {
                BlockKind::BulletListItem
                | BlockKind::TaskListItem { .. }
                | BlockKind::NumberedListItem => {
                    cx.emit(BlockEvent::RequestOutdent);
                    return;
                }
                BlockKind::Heading { .. } => {
                    self.convert_to_paragraph(cx);
                    return;
                }
                BlockKind::Blockquote => {
                    if self.is_leaf_quote() {
                        self.convert_to_paragraph(cx);
                    }
                    return;
                }
                BlockKind::Callout(_) => {
                    if self.downgrade_leaf_callout_to_quote_at_start(cx) {
                        return;
                    }
                    return;
                }
                BlockKind::ThematicBreak => {
                    self.convert_to_paragraph(cx);
                    return;
                }
                BlockKind::CodeBlock { .. } => {
                    self.convert_to_paragraph(cx);
                    return;
                }
                _ => {}
            }
        }

        if self.downgrade_leaf_callout_to_quote_at_start(cx)
            || self.downgrade_empty_leaf_quote_to_paragraph(cx)
        {
            return;
        }

        if self.selected_range.is_empty() && self.display_text().is_empty() {
            cx.emit(BlockEvent::RequestDelete);
            return;
        }

        if self.selected_range.is_empty() && self.cursor_offset() == 0 {
            cx.emit(BlockEvent::RequestMergeIntoPrevious {
                content: self.data.text.clone(),
            });
            return;
        }

        if self.selected_range.is_empty() {
            self.select_to(self.previous_boundary(self.cursor_offset()), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    pub fn on_delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_table_cell() {
            if self.selected_range.is_empty() {
                let next = self.next_boundary(self.cursor_offset());
                if next == self.cursor_offset() {
                    return;
                }
                self.select_to(next, cx);
            }
            self.replace_text_in_range(None, "", window, cx);
            return;
        }

        if self.is_verbatim_mode() {
            if self.selected_range.is_empty() {
                self.select_to(self.next_boundary(self.cursor_offset()), cx);
            }
            self.replace_text_in_range(None, "", window, cx);
            return;
        }

        if self.downgrade_leaf_callout_to_quote_at_start(cx)
            || self.downgrade_empty_leaf_quote_to_paragraph(cx)
        {
            return;
        }

        if self.kind().is_thematic_break() {
            self.convert_to_paragraph(cx);
            return;
        }

        if self.selected_range.is_empty() {
            self.select_to(self.next_boundary(self.cursor_offset()), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    pub fn on_word_delete_backward(
        &mut self,
        _: &WordDeleteBackward,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected_range.is_empty() {
            if self.cursor_offset() == 0 {
                // Nothing to the left in this block; defer to grapheme
                // backspace, which handles block merge and downgrades.
                self.on_delete_backward(&DeleteBackward, window, cx);
                return;
            }
            self.select_to(self.previous_word_start(self.cursor_offset()), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    pub fn on_word_delete_forward(
        &mut self,
        _: &WordDeleteForward,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected_range.is_empty() {
            if self.cursor_offset() == self.display_len() {
                // Nothing to the right in this block; defer to grapheme
                // delete, which handles block merge and separator removal.
                self.on_delete(&Delete, window, cx);
                return;
            }
            self.select_to(self.next_word_start(self.cursor_offset()), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    pub fn on_indent_block(
        &mut self,
        _: &IndentBlock,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_table_cell() {
            cx.emit(BlockEvent::RequestTableCellMoveHorizontal { delta: 1 });
            return;
        }
        if self.can_adjust_list_nesting() {
            cx.emit(BlockEvent::RequestIndent);
            return;
        }
        if self.kind() == BlockKind::Paragraph || self.kind().is_code_block() {
            self.replace_text_in_range(None, "    ", window, cx);
        }
    }

    pub fn on_outdent_block(
        &mut self,
        _: &OutdentBlock,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_table_cell() {
            cx.emit(BlockEvent::RequestTableCellMoveHorizontal { delta: -1 });
            return;
        }
        if self.can_outdent_list_nesting() {
            cx.emit(BlockEvent::RequestOutdent);
        }
    }
    pub fn on_block_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.keystroke.key != "tab" {
            return;
        }

        let modifiers = event.keystroke.modifiers;
        if modifiers.control || modifiers.platform || modifiers.alt || modifiers.function {
            return;
        }

        if self.code_language_focus_handle.is_focused(window) {
            return;
        }

        if modifiers.shift {
            self.on_outdent_block(&OutdentBlock, window, cx);
        } else {
            self.on_indent_block(&IndentBlock, window, cx);
        }
        cx.stop_propagation();
    }
}

