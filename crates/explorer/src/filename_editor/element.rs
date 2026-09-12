//! Inline filename editor GPUI Element: shaping, layout, selection, and cursor paint.

use gpui::*;

use crate::filename_editor::ExplorerFilenameImeHost;
use crate::state::{EXPLORER_NODE_HEIGHT, ExplorerState};
use theme::ThemeManager;

pub fn shape_filename_line(window: &mut Window, text: &str) -> ShapedLine {
    let display_text: SharedString = text.to_string().into();
    let style = window.text_style();
    let font_size = style.font_size.to_pixels(window.rem_size());
    let run = TextRun {
        len: display_text.len(),
        font: style.font(),
        color: style.color,
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    window
        .text_system()
        .shape_line(display_text, font_size, &[run], None)
}

// ── Input element ───────────────────────────────────────────────────────

pub struct ExplorerFilenamePrepaintState {
    line: Option<ShapedLine>,
    selection: Option<PaintQuad>,
    cursor: Option<PaintQuad>,
    hitbox: Option<Hitbox>,
    scroll_offset: Pixels,
}

/// Custom element painting the inline filename text, selection, cursor, and
/// IME composition underline; registers the window input handler while
/// focused (mirrors `CodeLanguageInputElement`).
pub struct ExplorerFilenameInputElement {
    /// The IME host entity registered as this input's window handler.
    pub ime_host: Entity<ExplorerFilenameImeHost>,
    /// The explorer panel state whose edit row this input renders.
    pub state: Entity<ExplorerState>,
}

impl IntoElement for ExplorerFilenameInputElement {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for ExplorerFilenameInputElement {
    type RequestLayoutState = ();
    type PrepaintState = ExplorerFilenamePrepaintState;

    fn id(&self) -> Option<ElementId> {
        Some(ElementId::Name("explorer-filename-input".into()))
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = px(EXPLORER_NODE_HEIGHT).into();
        style.flex_grow = 1.0;
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let theme = cx.global::<ThemeManager>().current_arc();
        let state = self.state.read(cx);
        let Some(edit) = state.edit.as_ref() else {
            return ExplorerFilenamePrepaintState {
                line: None,
                selection: None,
                cursor: None,
                hitbox: None,
                scroll_offset: px(0.0),
            };
        };
        let filename = &edit.filename;

        // Remember the bounds for IME hit-testing (interior mutability, pure read).
        filename.last_bounds.set(Some(bounds));

        let text: SharedString = filename.text.clone().into();
        let focused = filename
            .focus_handle
            .as_ref()
            .is_some_and(|handle| handle.is_focused(window));

        let base_run = TextRun {
            len: text.len(),
            font: window.text_style().font(),
            color: theme.colors.text_default,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let runs = if let Some(marked_range) =
            filename.marked_range.as_ref().filter(|_| !text.is_empty())
        {
            vec![
                TextRun {
                    len: marked_range.start,
                    ..base_run.clone()
                },
                TextRun {
                    len: marked_range.end - marked_range.start,
                    underline: Some(UnderlineStyle {
                        color: Some(theme.colors.text_default),
                        thickness: px(theme.dimensions.underline_thickness),
                        wavy: false,
                    }),
                    ..base_run.clone()
                },
                TextRun {
                    len: text.len().saturating_sub(marked_range.end),
                    ..base_run
                },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect()
        } else {
            vec![base_run]
        };

        let font_size = px(theme.typography.text_size * 0.9);
        let line = window
            .text_system()
            .shape_line(text, font_size, &runs, None);
        let line_height = bounds.size.height;
        let selection_range = filename.selection_range();
        let is_editing = self.state.read(cx).edit.is_some();

        let raw_cursor_x = line.x_for_index(if filename.reversed {
            filename.selection.start
        } else {
            filename.selection.end
        });

        // Compute horizontal scroll offset so cursor stays visible within input box bounds
        let available_w = bounds.size.width.max(px(0.0));
        let mut scroll_offset = filename.scroll_offset.get();
        if raw_cursor_x - scroll_offset > available_w {
            scroll_offset = raw_cursor_x - available_w;
        } else if raw_cursor_x - scroll_offset < px(0.0) {
            scroll_offset = raw_cursor_x;
        }
        let total_w = line.width();
        if total_w <= available_w {
            scroll_offset = px(0.0);
        } else if scroll_offset > total_w - available_w {
            scroll_offset = total_w - available_w;
        }
        let scroll_offset = scroll_offset.max(px(0.0));

        filename.scroll_offset.set(scroll_offset);

        let selection = if (focused || is_editing) && !selection_range.is_empty() {
            let start = line.x_for_index(selection_range.start) - scroll_offset;
            let end = line.x_for_index(selection_range.end) - scroll_offset;
            let sel_height = font_size * 1.35;
            let sel_y = bounds.top() + (line_height - sel_height) / 2.0;
            Some(fill(
                Bounds::from_corners(
                    point(bounds.left() + start, sel_y),
                    point(bounds.left() + end, sel_y + sel_height),
                ),
                theme.colors.selection,
            ))
        } else {
            None
        };
        let cursor = if (focused || is_editing) && selection_range.is_empty() {
            let cursor_x = raw_cursor_x - scroll_offset;
            let mut cursor_color = theme.colors.cursor;
            cursor_color.a = 1.0;
            let cursor_height = font_size * 1.25;
            let cursor_y = bounds.top() + (line_height - cursor_height) / 2.0;
            Some(fill(
                Bounds::new(
                    point(bounds.left() + cursor_x, cursor_y),
                    size(px(theme.dimensions.cursor_width.max(1.5)), cursor_height),
                ),
                cursor_color,
            ))
        } else {
            None
        };
        let hitbox = Some(window.insert_hitbox(bounds, HitboxBehavior::Normal));

        ExplorerFilenamePrepaintState {
            line: Some(line),
            selection,
            cursor,
            hitbox,
            scroll_offset,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(hitbox) = prepaint.hitbox.as_ref()
            && hitbox.is_hovered(window)
        {
            window.set_cursor_style(CursorStyle::IBeam, hitbox);
        }

        let focus_handle = self
            .state
            .read(cx)
            .edit
            .as_ref()
            .and_then(|edit| edit.filename.focus_handle.clone());
        if let Some(focus_handle) = focus_handle {
            window.handle_input(
                &focus_handle,
                ElementInputHandler::new(bounds, self.ime_host.clone()),
                cx,
            );
        }

        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            if let Some(selection) = prepaint.selection.take() {
                window.paint_quad(selection);
            }

            if let Some(line) = prepaint.line.take() {
                if let Some(edit) = self.state.read(cx).edit.as_ref() {
                    *edit.filename.last_layout.borrow_mut() = Some(line.clone());
                }
                let scroll_offset = prepaint.scroll_offset;
                let origin = point(bounds.origin.x - scroll_offset, bounds.origin.y);
                line.paint(
                    origin,
                    bounds.size.height,
                    TextAlign::Left,
                    None,
                    window,
                    cx,
                )
                .ok();
            }

            if let Some(cursor) = prepaint.cursor.take() {
                window.paint_quad(cursor);
            }
        });
    }
}
