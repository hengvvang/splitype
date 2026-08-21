//! Inline filename editor GPUI Element: shaping, layout, selection, and cursor paint.

use gpui::*;

use crate::app::shell::Shell;
use crate::explorer::state::state::EXPLORER_NODE_HEIGHT;
use crate::infra::theme::ThemeManager;

pub(crate) fn shape_filename_line(window: &mut Window, text: &str) -> ShapedLine {
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

pub(crate) struct ExplorerFilenamePrepaintState {
    line: Option<ShapedLine>,
    selection: Option<PaintQuad>,
    cursor: Option<PaintQuad>,
    hitbox: Option<Hitbox>,
}

/// Custom element painting the inline filename text, selection, cursor, and
/// IME composition underline; registers the window input handler while
/// focused (mirrors `CodeLanguageInputElement`).
pub(crate) struct ExplorerFilenameInputElement {
    pub(crate) editor: Entity<Shell>,
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
        let Some(edit) = self.editor.read(cx).panels.explorer.edit.clone() else {
            return ExplorerFilenamePrepaintState {
                line: None,
                selection: None,
                cursor: None,
                hitbox: None,
            };
        };
        let filename = &edit.filename;

        // Remember the bounds for IME hit-testing.
        self.editor.update(cx, |shell, _cx| {
            if let Some(edit) = shell.panels.explorer.edit.as_mut() {
                edit.filename.last_bounds = Some(bounds);
            }
        });

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

        let font_size = window.text_style().font_size.to_pixels(window.rem_size());
        let line = window
            .text_system()
            .shape_line(text, font_size, &runs, None);
        let line_height = bounds.size.height;
        let selection_range = filename.selection_range();
        let selection = if focused && !selection_range.is_empty() {
            let start = line.x_for_index(selection_range.start);
            let end = line.x_for_index(selection_range.end);
            Some(fill(
                Bounds::from_corners(
                    point(bounds.left() + start, bounds.top()),
                    point(bounds.left() + end, bounds.bottom()),
                ),
                theme.colors.selection,
            ))
        } else {
            None
        };
        let cursor = if focused && selection_range.is_empty() {
            let cursor_x = line.x_for_index(if filename.reversed {
                filename.selection.start
            } else {
                filename.selection.end
            });
            let mut cursor_color = theme.colors.cursor;
            cursor_color.a = 1.0;
            Some(fill(
                Bounds::new(
                    point(bounds.left() + cursor_x, bounds.top()),
                    size(px(theme.dimensions.cursor_width), line_height),
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
            .editor
            .read(cx)
            .panels
            .explorer
            .edit
            .as_ref()
            .and_then(|edit| edit.filename.focus_handle.clone());
        if let Some(focus_handle) = focus_handle
            && focus_handle.is_focused(window)
        {
            window.handle_input(
                &focus_handle,
                ElementInputHandler::new(bounds, self.editor.clone()),
                cx,
            );
        }

        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection);
        }

        if let Some(line) = prepaint.line.take() {
            line.paint(bounds.origin, bounds.size.height, window, cx)
                .ok();
        }

        if let Some(cursor) = prepaint.cursor.take() {
            window.paint_quad(cursor);
        }
    }
}
