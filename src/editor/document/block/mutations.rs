//! Block AST kind conversions, formatting, and text edit mutations.

use std::ops::Range;
use std::time::Instant;

use gpui::*;

use super::Block;
use super::state::{CollapsedCaretAffinity, InlineFormat};
use crate::editor::document::protocol::{BlockEvent, UndoCaptureKind};
use crate::model::inline::text::BlockText;
use crate::model::parse::BlockKind;

impl Block {
    pub(crate) fn apply_source_space_text_edit(
        &mut self,
        display_range: Range<usize>,
        new_text: &str,
        selected_range_relative: Option<Range<usize>>,
        mark_inserted_text: bool,
        cx: &mut Context<Self>,
    ) {
        let old_plain_len = self.data.text.plain_text().len();
        let source_range = self.display_range_to_source_range(display_range.clone());
        let mut markdown = self.data.text.serialize_markdown();
        let start =
            crate::model::inline::serialize::clamp_to_char_boundary(&markdown, source_range.start);
        let end = crate::model::inline::serialize::clamp_to_char_boundary(
            &markdown,
            source_range.end.max(start),
        );
        let replaced_text = markdown[start..end].to_string();
        markdown.replace_range(start..end, new_text);

        let next_text = BlockText::from_markdown_with_link_references(
            &markdown,
            &self.link_reference_definitions,
        );
        let map = next_text.source_offset_map();
        let selected_source = selected_range_relative
            .as_ref()
            .map(|relative| start + relative.start..start + relative.end);
        let cursor_source = selected_source
            .as_ref()
            .map(|range| range.end)
            .unwrap_or(start + new_text.len());
        let marked_source = if mark_inserted_text && !new_text.is_empty() {
            Some(start..start + new_text.len())
        } else {
            None
        };
        let selected_plain = selected_source
            .as_ref()
            .map(|range| map.source_to_plain_range(range.clone()));
        let marked_plain = marked_source
            .as_ref()
            .map(|range| map.source_to_plain_range(range.clone()));
        let cursor_plain = map.source_to_plain_offset(cursor_source);

        let quote_structure_edit = self.quote_depth > 0
            && (new_text.contains('\n')
                || replaced_text.contains('\n')
                || (self.kind() == BlockKind::Blockquote
                    && Self::multiline_quote_edit_requires_reparse(&next_text.plain_text())));
        if quote_structure_edit {
            self.quote_reparse_requested = true;
        }

        // Typing a closing marker (for example the `)` that completes a link)
        // absorbs that markup into a span, so the plain text grows by less than
        // the inserted text. Flag it so the caret is placed just past the new
        // closing delimiter instead of landing inside the span.
        let caret_may_have_closed_span = !new_text.is_empty()
            && !mark_inserted_text
            && next_text.plain_text().len() < old_plain_len + new_text.len();

        self.apply_text_edit(
            next_text,
            cursor_plain,
            marked_plain,
            selected_plain.clone(),
            selected_plain
                .as_ref()
                .and_then(|range| (!range.is_empty()).then_some(false)),
            caret_may_have_closed_span,
            cx,
        );
    }

    pub(crate) fn mark_changed(&mut self, cx: &mut Context<Self>) {
        self.sync_edit_mode_from_kind();
        self.sync_render_cache();
        self.cursor_blink_epoch = Instant::now();
        self.clear_vertical_motion();
        cx.emit(BlockEvent::Changed);
        cx.notify();
    }

    pub(crate) fn convert_to_paragraph(&mut self, cx: &mut Context<Self>) {
        self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
        self.data.kind = BlockKind::Paragraph;
        self.data.raw_source = None;
        self.quote_reparse_requested = false;
        self.mark_changed(cx);
    }

    pub(crate) fn convert_to_separator(&mut self, cx: &mut Context<Self>) {
        self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
        self.make_separator();
        cx.emit(BlockEvent::Changed);
        cx.notify();
    }

    /// Turns this block into a separator in place without emitting events or
    /// capturing undo, so editor-level flows that already manage those can
    /// reuse the conversion.
    pub(crate) fn make_separator(&mut self) {
        let current_text = self.display_text().to_string();
        let source_text = if current_text.trim().is_empty() {
            "---".to_string()
        } else {
            current_text
        };
        let source_len = source_text.len();
        self.clear_inline_projection();
        self.data.kind = BlockKind::ThematicBreak;
        self.data.raw_source = Some(source_text.clone());
        self.data.set_text(BlockText::plain(source_text));
        self.quote_reparse_requested = false;
        self.sync_edit_mode_from_kind();
        self.sync_render_cache();
        self.assign_collapsed_selection_offset(source_len, CollapsedCaretAffinity::Default, None);
        self.marked_range = None;
        self.cursor_blink_epoch = Instant::now();
        self.clear_vertical_motion();
    }

    pub(crate) fn enter_code_block(&mut self, language: Option<String>, cx: &mut Context<Self>) {
        self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
        self.clear_inline_projection();
        self.data.kind = BlockKind::CodeBlock { language };
        self.data.raw_source = None;
        self.data.set_text(BlockText::plain(String::new()));
        self.quote_reparse_requested = false;
        self.sync_edit_mode_from_kind();
        self.sync_render_cache();
        self.assign_collapsed_selection_offset(0, CollapsedCaretAffinity::Default, None);
        self.marked_range = None;
        self.cursor_blink_epoch = Instant::now();
        self.clear_vertical_motion();
        cx.emit(BlockEvent::Changed);
        cx.notify();
    }

    /// Convert the current paragraph into a display-math block. `body` is
    /// stored as the formula source (the `$$` delimiters are rebuilt on
    /// serialization), and the caret lands at the start of the body.
    pub(crate) fn enter_math_block(&mut self, body: &str, cx: &mut Context<Self>) {
        self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
        self.clear_inline_projection();
        self.data.kind = BlockKind::MathBlock;
        self.data.set_text(BlockText::plain(body.to_string()));
        self.quote_reparse_requested = false;
        self.sync_edit_mode_from_kind();
        self.sync_render_cache();
        self.assign_collapsed_selection_offset(0, CollapsedCaretAffinity::Default, None);
        self.marked_range = None;
        self.cursor_blink_epoch = Instant::now();
        self.clear_vertical_motion();
        cx.emit(BlockEvent::Changed);
        cx.notify();
    }

    /// Toggle a style flag directly on the fragment tree without ever
    /// manipulating raw marker characters.  The selection range determines
    /// which fragments have their [`InlineStyle`] flag flipped.
    ///
    /// Serializers later translate these flags back to markers on export.
    pub(crate) fn toggle_inline_format(&mut self, format: InlineFormat, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() || self.edits_verbatim_text() {
            return;
        }

        let mut next_text = self.data.text.clone();
        let selection = self.selection_plain_range();
        let changed = match format {
            InlineFormat::Bold => next_text.toggle_bold(selection.clone()),
            InlineFormat::Italic => next_text.toggle_italic(selection.clone()),
            InlineFormat::Underline => next_text.toggle_underline(selection.clone()),
            InlineFormat::Code => next_text.toggle_code(selection.clone()),
            InlineFormat::Strikethrough => next_text.toggle_strikethrough(selection.clone()),
        };
        if !changed {
            return;
        }

        self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
        self.apply_text_edit(
            next_text,
            selection.end,
            None,
            Some(selection),
            Some(self.selection_reversed),
            false,
            cx,
        );
    }
}
