//! GPUI IME and text input bridge for the source code editor.
//!
//! The platform drives composition through [`EntityInputHandler`]; the
//! editor applies composition updates as marked text (rendered with an
//! underline) and commits them through the pane host merged into the
//! current typing run, so one composition is one undo step.

use std::ops::Range;

use editor_contracts::search::{ceil_char_boundary, floor_char_boundary};
use gpui::{Bounds, Context, EntityInputHandler, Pixels, Point, UTF16Selection, Window};

use crate::editor::SourceCodeEditor;

#[inline]
fn utf16_offset_to_utf8(text: &str, utf16_offset: usize) -> usize {
    let mut utf16_count = 0;
    for (byte_offset, ch) in text.char_indices() {
        if utf16_count >= utf16_offset {
            return byte_offset;
        }
        utf16_count += ch.len_utf16();
    }
    text.len()
}

#[inline]
fn utf8_offset_to_utf16(text: &str, utf8_offset: usize) -> usize {
    let mut utf16_count = 0;
    for (byte_offset, ch) in text.char_indices() {
        if byte_offset >= utf8_offset {
            return utf16_count;
        }
        utf16_count += ch.len_utf16();
    }
    utf16_count
}

#[inline]
fn utf16_range_to_utf8(text: &str, range: &Range<usize>) -> Range<usize> {
    let start = utf16_offset_to_utf8(text, range.start);
    let end = utf16_offset_to_utf8(text, range.end).max(start);
    start..end
}

#[inline]
fn utf8_range_to_utf16(text: &str, range: &Range<usize>) -> Range<usize> {
    let start = utf8_offset_to_utf16(text, range.start);
    let end = utf8_offset_to_utf16(text, range.end).max(start);
    start..end
}

impl EntityInputHandler for SourceCodeEditor {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = utf16_range_to_utf8(&self.text, &range_utf16);
        actual_range.replace(utf8_range_to_utf16(&self.text, &range));
        let start = floor_char_boundary(&self.text, range.start);
        let end = ceil_char_boundary(&self.text, range.end.max(start));
        Some(self.text[start..end].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let range = self
            .selections
            .primary_range()
            .unwrap_or_else(|| self.cursor()..self.cursor());
        Some(UTF16Selection {
            range: utf8_range_to_utf16(&self.text, &range),
            reversed: self.selections.primary().is_reversed(),
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| utf8_range_to_utf16(&self.text, range))
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|r| utf16_range_to_utf8(&self.text, r))
            .or_else(|| self.marked_range.clone())
            .unwrap_or_else(|| {
                self.selections
                    .primary_range()
                    .unwrap_or_else(|| self.cursor()..self.cursor())
            });
        let sanitized = new_text.replace(['\r', '\n'], "");
        let start = floor_char_boundary(&self.text, range.start);
        let end = ceil_char_boundary(&self.text, range.end.max(start));

        let cursor_before = self.cursor_hint();
        let merge = self.marked_range.is_some();
        self.text.replace_range(start..end, &sanitized);
        self.marked_range = None;
        self.record_edit_run(merge, cursor_before, Some(start + sanitized.len()));
        self.rebuild_derived();
        self.selections.set_single_point(start + sanitized.len());
        self.commit_local_edit(merge, cursor_before, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = self.text.clone();
        let range = range_utf16
            .as_ref()
            .map(|r| utf16_range_to_utf8(&text, r))
            .or_else(|| self.marked_range.clone())
            .unwrap_or_else(|| {
                self.selections
                    .primary_range()
                    .unwrap_or_else(|| self.cursor()..self.cursor())
            });

        let sanitized = new_text.replace(['\r', '\n'], "");
        let start = floor_char_boundary(&self.text, range.start);
        let end = ceil_char_boundary(&self.text, range.end.max(start));

        let cursor_before = self.cursor_hint();
        let merge = self.marked_range.is_some() || self.typing_run.is_some();
        self.text.replace_range(start..end, &sanitized);

        let marked = start..start + sanitized.len();
        let selection = new_selected_range_utf16
            .as_ref()
            .map(|r| utf16_range_to_utf8(&sanitized, r))
            .map(|relative| marked.start + relative.start..marked.start + relative.end)
            .unwrap_or_else(|| marked.clone());

        self.marked_range = Some(marked);
        self.selections
            .set_single_range(selection.start, selection.end);
        self.record_edit_run(merge, cursor_before, Some(selection.end));
        self.rebuild_derived();
        self.commit_local_edit(merge, cursor_before, cx);
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: Range<usize>,
        _bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        Some(self.last_bounds())
    }

    fn character_index_for_point(
        &mut self,
        _point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        None
    }
}
