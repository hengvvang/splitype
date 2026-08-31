//! GPUI IME and text input bridge (EntityInputHandler) for Search & Replace inputs.

use std::ops::Range;
use gpui::*;

use crate::editor::Editor;
use core_contracts::search::{ceil_char_boundary, floor_char_boundary};

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

impl EntityInputHandler for Editor {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let is_search = self.search.search_focus_handle.is_focused(window);
        let is_replace = self.search.replace_focus_handle.is_focused(window);
        if !is_search && !is_replace {
            return None;
        }

        let input = if is_search {
            &self.search.search_input
        } else {
            &self.search.replace_input
        };

        let range = utf16_range_to_utf8(&input.text, &range_utf16);
        actual_range.replace(utf8_range_to_utf16(&input.text, &range));
        let start = floor_char_boundary(&input.text, range.start);
        let end = ceil_char_boundary(&input.text, range.end.max(start));
        Some(input.text[start..end].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let is_search = self.search.search_focus_handle.is_focused(window);
        let is_replace = self.search.replace_focus_handle.is_focused(window);
        if !is_search && !is_replace {
            return None;
        }

        let input = if is_search {
            &self.search.search_input
        } else {
            &self.search.replace_input
        };

        Some(UTF16Selection {
            range: utf8_range_to_utf16(&input.text, &input.selection_range()),
            reversed: input.reversed,
        })
    }

    fn marked_text_range(
        &self,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        let is_search = self.search.search_focus_handle.is_focused(window);
        let is_replace = self.search.replace_focus_handle.is_focused(window);
        if !is_search && !is_replace {
            return None;
        }

        let input = if is_search {
            &self.search.search_input
        } else {
            &self.search.replace_input
        };

        input
            .marked_range
            .as_ref()
            .map(|range| utf8_range_to_utf16(&input.text, range))
    }

    fn unmark_text(&mut self, window: &mut Window, _cx: &mut Context<Self>) {
        let is_search = self.search.search_focus_handle.is_focused(window);
        let is_replace = self.search.replace_focus_handle.is_focused(window);
        if is_search {
            self.search.search_input.marked_range = None;
        } else if is_replace {
            self.search.replace_input.marked_range = None;
        }
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let is_search = self.search.search_focus_handle.is_focused(window);
        let is_replace = self.search.replace_focus_handle.is_focused(window);
        if !is_search && !is_replace {
            return;
        }

        let current_range = if is_search {
            let t = &self.search.search_input.text;
            range_utf16
                .as_ref()
                .map(|r| utf16_range_to_utf8(t, r))
                .or_else(|| self.search.search_input.marked_range.clone())
                .unwrap_or_else(|| self.search.search_input.selection_range())
        } else {
            let t = &self.search.replace_input.text;
            range_utf16
                .as_ref()
                .map(|r| utf16_range_to_utf8(t, r))
                .or_else(|| self.search.replace_input.marked_range.clone())
                .unwrap_or_else(|| self.search.replace_input.selection_range())
        };

        if is_search {
            self.search.search_input.replace_range(current_range, new_text);
            self.execute_search(cx);
        } else {
            self.search.replace_input.replace_range(current_range, new_text);
            cx.notify();
        }
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let is_search = self.search.search_focus_handle.is_focused(window);
        let is_replace = self.search.replace_focus_handle.is_focused(window);
        if !is_search && !is_replace {
            return;
        }

        let input = if is_search {
            &mut self.search.search_input
        } else {
            &mut self.search.replace_input
        };

        let text = input.text.clone();
        let range = range_utf16
            .as_ref()
            .map(|r| utf16_range_to_utf8(&text, r))
            .or_else(|| input.marked_range.clone())
            .unwrap_or_else(|| input.selection_range());

        let sanitized = new_text.replace(['\r', '\n'], "");
        let start = floor_char_boundary(&input.text, range.start);
        let end = ceil_char_boundary(&input.text, range.end.max(start));
        input.text.replace_range(start..end, &sanitized);

        let marked = start..start + sanitized.len();
        let selection = new_selected_range_utf16
            .as_ref()
            .map(|r| utf16_range_to_utf8(&sanitized, r))
            .map(|relative| marked.start + relative.start..marked.start + relative.end)
            .unwrap_or_else(|| marked.clone());

        input.marked_range = Some(marked);
        input.selection = selection;
        input.reversed = false;

        if is_search {
            self.execute_search(cx);
        } else {
            cx.notify();
        }
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: Range<usize>,
        _bounds: Bounds<Pixels>,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let is_search = self.search.search_focus_handle.is_focused(window);
        let is_replace = self.search.replace_focus_handle.is_focused(window);
        if is_search {
            self.search.search_input.last_bounds
        } else if is_replace {
            self.search.replace_input.last_bounds
        } else {
            None
        }
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

