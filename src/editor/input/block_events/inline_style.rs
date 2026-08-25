//! Inline style toggle actions on a focused block: bold, italic,
//! underline, code, and exiting a code block.

use gpui::*;

use crate::editor::document::protocol::BlockEvent;
use crate::editor::input::actions::{
    BoldSelection, CodeSelection, ExitCodeBlock, ItalicSelection, StrikethroughSelection,
    UnderlineSelection,
};
use crate::editor::document::block::{Block, InlineFormat};
use crate::model::inline::text::BlockText;
impl Block {
    pub(crate) fn on_bold_selection(
        &mut self,
        _: &BoldSelection,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_inline_format(InlineFormat::Bold, cx);
    }

    pub(crate) fn on_italic_selection(
        &mut self,
        _: &ItalicSelection,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_inline_format(InlineFormat::Italic, cx);
    }

    pub(crate) fn on_underline_selection(
        &mut self,
        _: &UnderlineSelection,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_inline_format(InlineFormat::Underline, cx);
    }

    pub(crate) fn on_code_selection(
        &mut self,
        _: &CodeSelection,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_inline_format(InlineFormat::Code, cx);
    }

    pub(crate) fn on_strikethrough_selection(
        &mut self,
        _: &StrikethroughSelection,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_inline_format(InlineFormat::Strikethrough, cx);
    }

    pub(crate) fn on_exit_code_block(
        &mut self,
        _: &ExitCodeBlock,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let exits_multiline_block = self.is_table_cell() || self.kind().is_multiline_text_block();

        if exits_multiline_block {
            cx.emit(BlockEvent::RequestNewline {
                trailing: BlockText::plain(String::new()),
                source_already_mutated: false,
            });
        } else if self.callout_depth > 0 {
            cx.emit(BlockEvent::RequestCalloutBreak);
        } else if self.quote_depth > 0 {
            cx.emit(BlockEvent::RequestQuoteBreak);
        }
    }
}
