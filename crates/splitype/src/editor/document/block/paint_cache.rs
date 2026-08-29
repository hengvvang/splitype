//! Block painting cache and hitbox bounds hit-testing.

use gpui::*;

use super::Block;
use super::state::{BlockLastPaint, CodeLanguageLastPaint, MAX_LAST_PAINTS};
use crate::editor::geometry::text_layout as element;

impl Block {
    pub(crate) fn push_last_paint(
        &mut self,
        bounds: Bounds<Pixels>,
        layout: Vec<WrappedLine>,
        line_height: Pixels,
    ) {
        if let Some(entry) = self
            .last_paints
            .iter_mut()
            .find(|entry| entry.bounds == bounds)
        {
            *entry = BlockLastPaint {
                bounds,
                layout,
                line_height,
            };
            return;
        }
        self.last_paints.push(BlockLastPaint {
            bounds,
            layout,
            line_height,
        });
        if self.last_paints.len() > MAX_LAST_PAINTS {
            self.last_paints.remove(0);
        }
    }

    /// The paint entry whose bounds contain `position` — the pane the
    /// pointer is inside. Falls back to the newest entry.
    pub(crate) fn last_paint_at(&self, position: Point<Pixels>) -> Option<&BlockLastPaint> {
        self.last_paints
            .iter()
            .rev()
            .find(|entry| entry.bounds.contains(&position))
            .or_else(|| self.last_paints.last())
    }

    /// The newest paint entry (any pane). Used where no pointer position is
    /// available — keyboard navigation, IME popup placement, row strides.
    pub(crate) fn last_paint(&self) -> Option<&BlockLastPaint> {
        self.last_paints.last()
    }

    /// Record one pane's paint of the code-language input, mirroring
    /// [`Self::push_last_paint`].
    pub(crate) fn push_code_language_paint(&mut self, bounds: Bounds<Pixels>, line: ShapedLine) {
        self.code_toolbar.picker.push_paint(bounds, line);
    }

    /// The code-language input paint whose bounds contain `position`.
    pub(crate) fn code_language_paint_at(
        &self,
        position: Point<Pixels>,
    ) -> Option<&CodeLanguageLastPaint> {
        self.code_toolbar.picker.paint_at(position)
    }

    /// The newest code-language input paint (any pane).
    pub(crate) fn code_language_paint(&self) -> Option<&CodeLanguageLastPaint> {
        self.code_toolbar.picker.paints.last()
    }

    pub(crate) fn active_range_or_cursor_bounds(&self) -> Option<Bounds<Pixels>> {
        let paint = self.last_paint()?;
        let bounds = paint.bounds;
        let lines = &paint.layout;
        let line_height = paint.line_height;
        let text = self.display_text();
        let active_range = self
            .marked_range
            .clone()
            .unwrap_or_else(|| self.selected_range.clone());

        if active_range.is_empty() {
            return element::cursor_bounds_for_offset(
                lines,
                bounds,
                line_height,
                text,
                self.cursor_offset(),
                self.text_align(),
                px(1.0),
            );
        }

        element::range_bounds(
            lines,
            bounds,
            line_height,
            text,
            active_range,
            self.text_align(),
        )
    }
}
