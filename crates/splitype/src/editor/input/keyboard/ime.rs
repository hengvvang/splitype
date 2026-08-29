//! GPUI [`EntityInputHandler`] implementation for Block.
//!
//! Bridges between GPUI's UTF-16-based IME subsystem and the block's
//! internal UTF-8 representation.  All range arguments from GPUI arrive
//! as UTF-16 offsets and are converted through `range_from_utf16` before
//! operating on the block's text.

use std::ops::Range;

use gpui::*;

use crate::editor::document::protocol::{BlockEvent, UndoCaptureKind};
use crate::editor::geometry::text_layout as element;
use crate::editor::document::block::Block;
use markdown::inline::offsets::ImeConverter;

impl EntityInputHandler for Block {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        if self.code_language_focus_handle.is_focused(_window) {
            let range = self.code_language_range_from_utf16(&range_utf16);
            *actual_range = Some(self.code_language_range_to_utf16(&range));
            let text = self.code_language_text();
            let start = range.start.min(text.len());
            let end = range.end.min(text.len());
            return Some(text[start..end].to_string());
        }

        let range = self.range_from_utf16(&range_utf16);
        *actual_range = Some(self.range_to_utf16(&range));
        let text = self.display_text();
        let start = range.start.min(text.len());
        let end = range.end.min(text.len());
        Some(text[start..end].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        if self.code_language_focus_handle.is_focused(_window) {
            let range = self.code_language_range_to_utf16(&self.code_toolbar.picker.selected_range);
            return Some(UTF16Selection {
                range,
                reversed: self.code_toolbar.picker.selection_reversed,
            });
        }

        let range = self.range_to_utf16(&self.selected_range);
        Some(UTF16Selection {
            range,
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        if self.code_language_focus_handle.is_focused(_window) {
            return self
                .code_toolbar
                .picker
                .marked_range
                .as_ref()
                .map(|range| self.code_language_range_to_utf16(range));
        }

        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.code_language_focus_handle.is_focused(_window) {
            self.code_toolbar.picker.marked_range = None;
            cx.notify();
            return;
        }

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
        if self.code_language_focus_handle.is_focused(_window) {
            let display_range = range_utf16
                .as_ref()
                .map(|range| self.code_language_range_from_utf16(range))
                .or(self.code_toolbar.picker.marked_range.clone())
                .unwrap_or(self.code_toolbar.picker.selected_range.clone());
            self.replace_code_language_text_in_range(display_range, new_text, None, false, cx);
            return;
        }

        if self.editor_selection_range.is_some() {
            cx.emit(BlockEvent::RequestReplaceCrossBlockSelection {
                text: new_text.to_string(),
                selected_range_relative: None,
                mark_inserted_text: false,
                undo_kind: UndoCaptureKind::CoalescibleText,
            });
            return;
        }

        self.prepare_undo_capture(UndoCaptureKind::CoalescibleText, cx);
        let display_range = range_utf16
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        let display_text = self.display_text();
        let is_plain_selection = self.marked_range.is_none()
            && display_range.start < display_range.end
            && display_range.end <= display_text.len();
        if is_plain_selection {
            let selected_slice = &display_text[display_range.clone()];
            if let Some((wrapped, inner_start, inner_end)) = wrap_selection_pair(selected_slice, new_text) {
                let relative_sel = Some(inner_start..inner_end);
                self.replace_text_in_display_range(display_range, &wrapped, relative_sel, false, cx);
                return;
            }
        }

        self.replace_text_in_display_range(display_range, new_text, None, false, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.code_language_focus_handle.is_focused(_window) {
            let display_range = range_utf16
                .as_ref()
                .map(|range| self.code_language_range_from_utf16(range))
                .or(self.code_toolbar.picker.marked_range.clone())
                .unwrap_or(self.code_toolbar.picker.selected_range.clone());
            let sanitized_new_text = new_text.replace("\r\n", " ").replace(['\r', '\n'], " ");
            let selected_range_relative = new_selected_range_utf16
                .as_ref()
                .map(|range_utf16| {
                    ImeConverter::utf16_range_to_utf8_in(&sanitized_new_text, range_utf16)
                })
                .map(|relative| relative.start..relative.end);

            self.replace_code_language_text_in_range(
                display_range,
                &sanitized_new_text,
                selected_range_relative,
                !sanitized_new_text.is_empty(),
                cx,
            );
            return;
        }

        if self.editor_selection_range.is_some() {
            let selected_range_relative = new_selected_range_utf16
                .as_ref()
                .map(|range_utf16| ImeConverter::utf16_range_to_utf8_in(new_text, range_utf16))
                .map(|relative| relative.start..relative.end);
            cx.emit(BlockEvent::RequestReplaceCrossBlockSelection {
                text: new_text.to_string(),
                selected_range_relative,
                mark_inserted_text: !new_text.is_empty(),
                undo_kind: UndoCaptureKind::CoalescibleText,
            });
            return;
        }

        self.prepare_undo_capture(UndoCaptureKind::CoalescibleText, cx);
        let display_range = range_utf16
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());
        let selected_range_relative = new_selected_range_utf16
            .as_ref()
            .map(|range_utf16| ImeConverter::utf16_range_to_utf8_in(new_text, range_utf16))
            .map(|relative| relative.start..relative.end);

        self.replace_text_in_display_range(
            display_range,
            new_text,
            selected_range_relative,
            !new_text.is_empty(),
            cx,
        );
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        if self.code_language_focus_handle.is_focused(_window) {
            let paint = self.code_language_paint()?;
            let range = self.code_language_range_from_utf16(&range_utf16);
            let start_x = paint.line.x_for_index(range.start);
            let end_x = paint.line.x_for_index(range.end);
            return Some(Bounds::from_corners(
                point(bounds.left() + start_x, bounds.top()),
                point(bounds.left() + end_x, bounds.bottom()),
            ));
        }

        let paint = self.last_paint()?;
        let range = self.range_from_utf16(&range_utf16);
        let line_height = paint.line_height;
        let text = self.display_text();
        element::range_bounds(
            &paint.layout,
            bounds,
            line_height,
            text,
            range,
            self.text_align(),
        )
    }

    fn character_index_for_point(
        &mut self,
        pt: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        if self.code_language_focus_handle.is_focused(_window) {
            let index = self.code_language_index_for_mouse_position(pt);
            return Some(ImeConverter::utf8_to_utf16_in(
                self.code_language_input_text(),
                index,
            ));
        }

        let paint = self.last_paint_at(pt)?;
        let bounds = paint.bounds;
        let lines = &paint.layout;
        let text = self.display_text();
        let ranges = element::hard_line_ranges(text);
        let relative = Point {
            x: pt.x - bounds.left(),
            y: pt.y - bounds.top(),
        };
        let (line_idx, y_in_line) =
            element::wrapped_line_for_y(lines, paint.line_height, relative.y)?;
        let layout = &lines[line_idx];
        let origin_x = element::aligned_line_left(layout, bounds, self.text_align());
        let utf8_offset_in_line = match layout
            .closest_index_for_position(point(pt.x - origin_x, y_in_line), paint.line_height)
        {
            Ok(idx) | Err(idx) => idx,
        };
        let utf8_index =
            ranges[line_idx.min(ranges.len().saturating_sub(1))].start + utf8_offset_in_line;
        Some(ImeConverter::utf8_to_utf16_in(
            self.display_text(),
            utf8_index,
        ))
    }
}

pub(crate) fn wrap_selection_pair(selected: &str, input: &str) -> Option<(String, usize, usize)> {
    match input {
        "*" => Some((format!("*{selected}*"), 1, 1 + selected.len())),
        "_" => Some((format!("_{selected}_"), 1, 1 + selected.len())),
        "`" => Some((format!("`{selected}`"), 1, 1 + selected.len())),
        "~" => Some((format!("~{selected}~"), 1, 1 + selected.len())),
        "$" => Some((format!("${selected}$"), 1, 1 + selected.len())),
        "(" => Some((format!("({selected})"), 1, 1 + selected.len())),
        "[" => Some((format!("[{selected}]()"), 1, 1 + selected.len())),
        "{" => Some((format!("{{{selected}}}"), 1, 1 + selected.len())),
        "\"" => Some((format!("\"{selected}\""), 1, 1 + selected.len())),
        "'" => Some((format!("'{selected}'"), 1, 1 + selected.len())),
        _ => None,
    }
}
