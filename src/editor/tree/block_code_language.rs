//! Block code-language input and highlight state.

use std::ops::Range;
use std::time::Instant;

use gpui::*;
use unicode_segmentation::*;

use crate::editor::block_protocol::{BlockEvent, UndoCaptureKind};
use crate::editor::render::code_highlight::highlight::{CodeHighlightResult, highlight_code_block};
use crate::editor::tree::block::Block;
use crate::editor::tree::block::normalize_code_language_input;
use crate::model::inline::offsets::ImeConverter;
use crate::model::parse::BlockKind;

impl Block {
    pub(crate) fn code_highlight_result(&self) -> Option<&CodeHighlightResult> {
        self.code_highlight.as_ref()
    }

    pub(crate) fn sync_code_highlight(&mut self) {
        self.code_highlight = match &self.data.kind {
            BlockKind::CodeBlock { language } => {
                highlight_code_block(language.as_deref(), self.render_cache.text())
            }
            BlockKind::MathBlock => highlight_code_block(Some("math"), self.render_cache.text()),
            BlockKind::MermaidBlock => {
                highlight_code_block(Some("mermaid"), self.render_cache.text())
            }
            _ => None,
        };
    }

    pub(crate) fn code_language_text(&self) -> &str {
        match &self.data.kind {
            BlockKind::CodeBlock {
                language: Some(language),
            } => language.as_ref(),
            BlockKind::MathBlock => "math",
            BlockKind::MermaidBlock => "mermaid",
            _ => "",
        }
    }

    pub(crate) fn code_language_input_text(&self) -> &str {
        if self.code_toolbar.picker.is_open {
            &self.code_toolbar.picker.query
        } else {
            self.code_language_text()
        }
    }

    pub(crate) fn code_language_range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        ImeConverter::utf8_range_to_utf16_in(self.code_language_input_text(), range)
    }

    pub(crate) fn code_language_range_from_utf16(
        &self,
        range_utf16: &Range<usize>,
    ) -> Range<usize> {
        ImeConverter::utf16_range_to_utf8_in(self.code_language_input_text(), range_utf16)
    }

    pub(crate) fn previous_code_language_boundary(&self, offset: usize) -> usize {
        self.code_language_input_text()
            .grapheme_indices(true)
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    pub(crate) fn next_code_language_boundary(&self, offset: usize) -> usize {
        self.code_language_input_text()
            .grapheme_indices(true)
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(self.code_language_input_text().len())
    }

    pub(crate) fn move_code_language_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let clamped = offset.min(self.code_language_input_text().len());
        self.code_toolbar.picker.selected_range = clamped..clamped;
        self.code_toolbar.picker.selection_reversed = false;
        self.code_toolbar.picker.marked_range = None;
        self.cursor_blink_epoch = Instant::now();
        cx.notify();
    }

    pub(crate) fn select_code_language_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let clamped = offset.min(self.code_language_input_text().len());
        if self.code_toolbar.picker.selection_reversed {
            self.code_toolbar.picker.selected_range.start = clamped;
        } else {
            self.code_toolbar.picker.selected_range.end = clamped;
        }
        if self.code_toolbar.picker.selected_range.end
            < self.code_toolbar.picker.selected_range.start
        {
            self.code_toolbar.picker.selection_reversed =
                !self.code_toolbar.picker.selection_reversed;
            self.code_toolbar.picker.selected_range = self.code_toolbar.picker.selected_range.end
                ..self.code_toolbar.picker.selected_range.start;
        }
        self.cursor_blink_epoch = Instant::now();
        cx.notify();
    }

    pub(crate) fn replace_code_language_text_in_range(
        &mut self,
        range: Range<usize>,
        new_text: &str,
        selected_range_relative: Option<Range<usize>>,
        mark_inserted_text: bool,
        cx: &mut Context<Self>,
    ) {
        if !self.kind().is_code_block() {
            return;
        }

        if self.code_toolbar.picker.is_open {
            let current = self.code_toolbar.picker.query.clone();
            let range = range.start.min(current.len())..range.end.min(current.len());
            let inserted = new_text.replace("\r\n", " ").replace(['\r', '\n'], " ");
            self.code_toolbar
                .picker
                .query
                .replace_range(range.clone(), &inserted);
            let next_len = self.code_toolbar.picker.query.len();
            let next_cursor = selected_range_relative
                .as_ref()
                .map(|relative| range.start + relative.end)
                .unwrap_or(range.start + inserted.len())
                .min(next_len);
            self.code_toolbar.picker.selected_range = selected_range_relative
                .as_ref()
                .map(|relative| {
                    let start = (range.start + relative.start).min(next_len);
                    let end = (range.start + relative.end).min(next_len);
                    start.min(end)..start.max(end)
                })
                .unwrap_or(next_cursor..next_cursor);
            self.code_toolbar.picker.selection_reversed = selected_range_relative
                .as_ref()
                .is_some_and(|relative| relative.end < relative.start);
            self.code_toolbar.picker.marked_range = if mark_inserted_text && !inserted.is_empty() {
                Some(range.start..(range.start + inserted.len()).min(next_len))
            } else {
                None
            };
            self.cursor_blink_epoch = Instant::now();
            cx.notify();
            return;
        }

        self.prepare_undo_capture(UndoCaptureKind::CoalescibleText, cx);

        let current = self.code_language_text().to_string();
        let range = range.start.min(current.len())..range.end.min(current.len());
        let inserted = new_text.replace("\r\n", " ").replace(['\r', '\n'], " ");
        let mut raw_next = String::new();
        raw_next.push_str(&current[..range.start]);
        raw_next.push_str(&inserted);
        raw_next.push_str(&current[range.end..]);

        let trimmed_start = raw_next.len() - raw_next.trim_start().len();
        let normalized = normalize_code_language_input(&raw_next);
        let normalized_len = normalized.len();
        let raw_inserted_end = range.start + inserted.len();
        let next_cursor = selected_range_relative
            .as_ref()
            .map(|relative| range.start + relative.end)
            .unwrap_or(raw_inserted_end)
            .saturating_sub(trimmed_start)
            .min(normalized_len);
        let next_selection = selected_range_relative
            .as_ref()
            .map(|relative| {
                let start = (range.start + relative.start)
                    .saturating_sub(trimmed_start)
                    .min(normalized_len);
                let end = (range.start + relative.end)
                    .saturating_sub(trimmed_start)
                    .min(normalized_len);
                start.min(end)..start.max(end)
            })
            .unwrap_or_else(|| next_cursor..next_cursor);
        let next_marked = if mark_inserted_text && !inserted.is_empty() {
            let start = range
                .start
                .saturating_sub(trimmed_start)
                .min(normalized_len);
            let end = raw_inserted_end
                .saturating_sub(trimmed_start)
                .min(normalized_len);
            (start < end).then_some(start..end)
        } else {
            None
        };

        let old_language = match &self.data.kind {
            BlockKind::CodeBlock { language } => language.clone(),
            _ => None,
        };
        self.data.kind = BlockKind::CodeBlock {
            language: (!normalized.is_empty()).then_some(normalized),
        };
        self.code_toolbar.picker.selected_range = next_selection;
        self.code_toolbar.picker.selection_reversed = selected_range_relative
            .as_ref()
            .is_some_and(|relative| relative.end < relative.start);
        self.code_toolbar.picker.marked_range = next_marked;
        self.cursor_blink_epoch = Instant::now();
        self.sync_code_highlight();

        let next_language = match &self.data.kind {
            BlockKind::CodeBlock { language } => language.clone(),
            _ => None,
        };
        if old_language != next_language {
            cx.emit(BlockEvent::Changed);
        }
        cx.notify();
    }

    pub(crate) fn choose_code_language(&mut self, value: &str, cx: &mut Context<Self>) {
        if !matches!(
            self.kind(),
            BlockKind::CodeBlock { .. } | BlockKind::MathBlock | BlockKind::MermaidBlock
        ) {
            return;
        }

        let old_language = self.code_language_text().to_string();
        if old_language != value {
            self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
            if value.eq_ignore_ascii_case("math") || value.eq_ignore_ascii_case("latex") {
                self.data.kind = BlockKind::MathBlock;
            } else if value.eq_ignore_ascii_case("mermaid") {
                self.data.kind = BlockKind::MermaidBlock;
            } else {
                self.data.kind = BlockKind::CodeBlock {
                    language: (!value.is_empty()).then(|| value.to_string()),
                };
            }
            self.sync_code_highlight();
            cx.emit(BlockEvent::Changed);
        }
        self.code_toolbar.picker.close();
        cx.notify();
    }

    pub(crate) fn code_language_index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        let text = self.code_language_input_text();
        if text.is_empty() {
            return 0;
        }

        let Some(paint) = self.code_language_paint_at(position) else {
            return 0;
        };
        if position.x <= paint.bounds.left() {
            return 0;
        }
        if position.x >= paint.bounds.right() {
            return text.len();
        }
        paint
            .line
            .closest_index_for_x(position.x - paint.bounds.left())
    }

    pub(crate) fn reset_code_language_input_layout(&mut self) {
        self.code_toolbar.picker.paints.clear();
        self.code_toolbar.picker.is_selecting = false;
    }

    pub(crate) fn on_code_line_numbers_toggle(
        &mut self,
        _event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.show_code_line_numbers = !self.show_code_line_numbers;
        cx.notify();
    }
}
