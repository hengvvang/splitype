//! GPUI IME and text input bridge (EntityInputHandler) for Search & Replace inputs and Source Code editor.

use std::ops::Range;
use gpui::*;

use crate::editor::engine::controller::Editor;
use crate::editor::search::state::{ceil_char_boundary, floor_char_boundary};
use crate::model::inline::offsets::ImeConverter;

impl Editor {
    /// Returns the PaneId of the currently focused Source Code pane, if any.
    pub(crate) fn focused_source_pane_id(&self, window: &Window) -> Option<crate::editor::engine::controller::PaneId> {
        let active_pane_id = self.active_pane_id();
        let state = self.pane_state_ref(active_pane_id)?;
        let source = state.as_source_code()?;
        if let Some(ref handle) = source.focus_handle {
            if handle.is_focused(window) {
                return Some(active_pane_id);
            }
        }
        None
    }
}

impl EntityInputHandler for Editor {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        if let Some(pane_id) = self.focused_source_pane_id(window) {
            let state = self.pane_state_ref(pane_id)?;
            let source = state.as_source_code()?;
            let range = ImeConverter::utf16_range_to_utf8_in(&source.text, &range_utf16);
            *actual_range = Some(ImeConverter::utf8_range_to_utf16_in(&source.text, &range));
            let start = range.start.min(source.text.len());
            let end = range.end.min(source.text.len());
            return Some(source.text[start..end].to_string());
        }

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
        if let Some(pane_id) = self.focused_source_pane_id(window) {
            let state = self.pane_state_ref(pane_id)?;
            let source = state.as_source_code()?;
            let utf8_range = source.selection.clone().unwrap_or(source.cursor..source.cursor);
            let utf16_range = ImeConverter::utf8_range_to_utf16_in(&source.text, &utf8_range);
            return Some(UTF16Selection {
                range: utf16_range,
                reversed: false,
            });
        }

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
        if let Some(pane_id) = self.focused_source_pane_id(window) {
            let state = self.pane_state_ref(pane_id)?;
            let source = state.as_source_code()?;
            return source
                .marked_range
                .as_ref()
                .map(|range| ImeConverter::utf8_range_to_utf16_in(&source.text, range));
        }

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

    fn unmark_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(pane_id) = self.focused_source_pane_id(window) {
            if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
                source.marked_range = None;
            }
            cx.notify();
            return;
        }

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
        if let Some(pane_id) = self.focused_source_pane_id(window) {
            let current_range = {
                let state = self.pane_state_ref(pane_id).unwrap();
                let source = state.as_source_code().unwrap();
                range_utf16
                    .as_ref()
                    .map(|r| ImeConverter::utf16_range_to_utf8_in(&source.text, r))
                    .or_else(|| source.marked_range.clone())
                    .or_else(|| source.selection.clone())
                    .unwrap_or(source.cursor..source.cursor)
            };
            if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
                let start = current_range.start.min(source.text.len());
                let end = current_range.end.min(source.text.len());
                source.text.replace_range(start..end, new_text);
                source.cursor = start + new_text.len();
                source.selection = None;
                source.marked_range = None;
                source.rebuild_lines();
                source.refresh_highlight();
            }
            self.sync_source_edit_to_document(pane_id, cx);
            return;
        }

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
        if let Some(pane_id) = self.focused_source_pane_id(window) {
            let range = {
                let state = self.pane_state_ref(pane_id).unwrap();
                let source = state.as_source_code().unwrap();
                range_utf16
                    .as_ref()
                    .map(|r| ImeConverter::utf16_range_to_utf8_in(&source.text, r))
                    .or_else(|| source.marked_range.clone())
                    .or_else(|| source.selection.clone())
                    .unwrap_or(source.cursor..source.cursor)
            };
            if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
                let start = range.start.min(source.text.len());
                let end = range.end.min(source.text.len());
                source.text.replace_range(start..end, new_text);
                let marked = start..start + new_text.len();
                let selection = new_selected_range_utf16
                    .as_ref()
                    .map(|r| ImeConverter::utf16_range_to_utf8_in(new_text, r))
                    .map(|relative| marked.start + relative.start..marked.start + relative.end)
                    .unwrap_or_else(|| marked.clone());
                source.marked_range = Some(marked);
                source.selection = if selection.start == selection.end { None } else { Some(selection.clone()) };
                source.cursor = selection.end;
                source.refresh_highlight();
            }
            self.sync_source_edit_to_document(pane_id, cx);
            return;
        }

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
        if let Some(pane_id) = self.focused_source_pane_id(window) {
            let state = self.pane_state_ref(pane_id)?;
            let source = state.as_source_code()?;
            return source.last_bounds;
        }

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
