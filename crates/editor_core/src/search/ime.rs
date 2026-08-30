//! GPUI IME and text input bridge (EntityInputHandler) for Search & Replace inputs.

use std::ops::Range;
use gpui::*;

use crate::engine::controller::Editor;
use editor_search::state::{ceil_char_boundary, floor_char_boundary};
use editor_wysiwyg::markdown::inline::offsets::ImeConverter;

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

        let range = ImeConverter::utf16_range_to_utf8_in(&input.text, &range_utf16);
        actual_range.replace(ImeConverter::utf8_range_to_utf16_in(&input.text, &range));
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
            range: ImeConverter::utf8_range_to_utf16_in(&input.text, &input.selection_range()),
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
            .map(|range| ImeConverter::utf8_range_to_utf16_in(&input.text, range))
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
                .map(|r| ImeConverter::utf16_range_to_utf8_in(t, r))
                .or_else(|| self.search.search_input.marked_range.clone())
                .unwrap_or_else(|| self.search.search_input.selection_range())
        } else {
            let t = &self.search.replace_input.text;
            range_utf16
                .as_ref()
                .map(|r| ImeConverter::utf16_range_to_utf8_in(t, r))
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
            .map(|r| ImeConverter::utf16_range_to_utf8_in(&text, r))
            .or_else(|| input.marked_range.clone())
            .unwrap_or_else(|| input.selection_range());

        let sanitized = new_text.replace(['\r', '\n'], "");
        let start = floor_char_boundary(&input.text, range.start);
        let end = ceil_char_boundary(&input.text, range.end.max(start));
        input.text.replace_range(start..end, &sanitized);

        let marked = start..start + sanitized.len();
        let selection = new_selected_range_utf16
            .as_ref()
            .map(|r| ImeConverter::utf16_range_to_utf8_in(&sanitized, r))
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
