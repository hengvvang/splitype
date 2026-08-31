//! GPUI IME bridge (`EntityInputHandler`) for the inline explorer filename
//! editor.
//!
//! The filename input element registers this entity as its window input
//! handler; every read goes through `ExplorerState::global` and every
//! mutation through `ExplorerState::update` — the host entity itself is
//! stateless.

use std::ops::Range;

use gpui::*;

use crate::ExplorerState;
use crate::filename_editor::buffer::{utf16_range_to_utf8_in, utf8_range_to_utf16_in, utf8_to_utf16_in_single};
use crate::filename_editor::element::shape_filename_line;

/// Stateless IME host for the inline filename editor (one per edit).
#[derive(Clone, Debug)]
pub struct ExplorerFilenameImeHost;

impl EntityInputHandler for ExplorerFilenameImeHost {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<String> {
        let edit = ExplorerState::global(cx).edit.as_ref()?;
        if !edit.filename.focus_handle.as_ref()?.is_focused(window) {
            return None;
        }
        let range = utf16_range_to_utf8_in(&edit.filename.text, &range_utf16);
        actual_range.replace(utf8_range_to_utf16_in(&edit.filename.text, &range));
        Some(edit.filename.text[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let edit = ExplorerState::global(cx).edit.as_ref()?;
        if !edit.filename.focus_handle.as_ref()?.is_focused(window) {
            return None;
        }
        Some(UTF16Selection {
            range: utf8_range_to_utf16_in(&edit.filename.text, &edit.filename.selection_range()),
            reversed: edit.filename.reversed,
        })
    }

    fn marked_text_range(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        let edit = ExplorerState::global(cx).edit.as_ref()?;
        if !edit.filename.focus_handle.as_ref()?.is_focused(window) {
            return None;
        }
        edit.filename
            .marked_range
            .as_ref()
            .map(|range| utf8_range_to_utf16_in(&edit.filename.text, range))
    }

    fn unmark_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        ExplorerState::update(cx, |state, _cx| {
            if let Some(edit) = state.edit.as_mut() {
                if edit
                    .filename
                    .focus_handle
                    .as_ref()
                    .is_some_and(|handle| handle.is_focused(window))
                {
                    edit.filename.marked_range = None;
                }
            }
        });
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(edit) = ExplorerState::global(cx).edit.as_ref() else {
            return;
        };
        if !edit
            .filename
            .focus_handle
            .as_ref()
            .is_some_and(|handle| handle.is_focused(window))
        {
            return;
        }
        let text = edit.filename.text.clone();
        let range = range_utf16
            .as_ref()
            .map(|range| utf16_range_to_utf8_in(&text, range))
            .or_else(|| edit.filename.marked_range.clone())
            .unwrap_or_else(|| edit.filename.selection_range());
        ExplorerState::update(cx, |state, cx| {
            if let Some(edit) = state.edit.as_mut() {
                edit.filename.replace_range(range, new_text);
                state.populate_explorer_validation(cx);
            }
        });
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(edit) = ExplorerState::global(cx).edit.as_ref() else {
            return;
        };
        if !edit
            .filename
            .focus_handle
            .as_ref()
            .is_some_and(|handle| handle.is_focused(window))
        {
            return;
        }
        let text = edit.filename.text.clone();
        let range = range_utf16
            .as_ref()
            .map(|range| utf16_range_to_utf8_in(&text, range))
            .or_else(|| edit.filename.marked_range.clone())
            .unwrap_or_else(|| edit.filename.selection_range());
        let sanitized = new_text.replace(['\r', '\n'], "");
        let marked = range.start..range.start + sanitized.len();
        let selection = new_selected_range_utf16
            .as_ref()
            .map(|range| utf16_range_to_utf8_in(&sanitized, range))
            .map(|relative| marked.start + relative.start..marked.start + relative.end)
            .unwrap_or_else(|| marked.clone());
        ExplorerState::update(cx, |state, cx| {
            if let Some(edit) = state.edit.as_mut() {
                edit.filename.text.replace_range(range, &sanitized);
                edit.filename.marked_range = Some(marked);
                edit.filename.selection = selection;
                edit.filename.reversed = false;
                state.populate_explorer_validation(cx);
            }
        });
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let edit = ExplorerState::global(cx).edit.as_ref()?;
        if !edit.filename.focus_handle.as_ref()?.is_focused(window) {
            return None;
        }
        let line = shape_filename_line(window, &edit.filename.text);
        let range = utf16_range_to_utf8_in(&edit.filename.text, &range_utf16);
        Some(Bounds::from_corners(
            point(bounds.left() + line.x_for_index(range.start), bounds.top()),
            point(bounds.left() + line.x_for_index(range.end), bounds.bottom()),
        ))
    }

    fn character_index_for_point(
        &mut self,
        pt: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<usize> {
        let edit = ExplorerState::global(cx).edit.as_ref()?;
        if !edit.filename.focus_handle.as_ref()?.is_focused(window) {
            return None;
        }
        let bounds = edit.filename.last_bounds?;
        let line = shape_filename_line(window, &edit.filename.text);
        let x = pt.x - bounds.left();
        let index = line.closest_index_for_x(x);
        Some(utf8_to_utf16_in_single(&edit.filename.text, index))
    }
}
