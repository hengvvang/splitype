//! High-performance virtualized GPUI Element for Source Code pane.
//! Implements Zed-style sub-pixel text shaping, viewport virtualization,
//! Tree-sitter syntax highlighting, and exact character-level hit-testing.

use gpui::*;

use crate::editor::engine::controller::{Editor, PaneId};
use crate::editor::panes::source_code::highlight::build_line_text_runs;
use crate::infra::theme::{ThemeManager, TypographyScope, TypographyStore};

pub(crate) struct SourceCodeViewElement {
    pub(crate) editor: Entity<Editor>,
    pub(crate) pane_id: PaneId,
}

pub(crate) struct SourceCodePrepaintState {
    pub(crate) line_height: f32,
    pub(crate) gutter_width: f32,
    pub(crate) editor_padding: f32,
    pub(crate) shaped_lines: Vec<(usize, ShapedLine)>,
    pub(crate) cursor_quad: Option<PaintQuad>,
    pub(crate) selection_quads: Vec<PaintQuad>,
    pub(crate) active_line_quad: Option<PaintQuad>,
    pub(crate) gutter_numbers: Vec<(usize, ShapedLine, bool)>, // (row, shaped_number, is_active)
    pub(crate) hitbox: Option<Hitbox>,
}

impl IntoElement for SourceCodeViewElement {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for SourceCodeViewElement {
    type RequestLayoutState = ();
    type PrepaintState = SourceCodePrepaintState;

    fn id(&self) -> Option<ElementId> {
        Some(ElementId::Name(
            format!("source-code-view-{}", self.pane_id.0).into(),
        ))
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
        let theme = cx.global::<ThemeManager>().current_arc();
        let font_size = theme.typography.code_size.max(12.0);
        let line_height = (font_size * theme.typography.text_line_height).round();
        let editor_padding = theme.dimensions.editor_padding;

        let total_lines = self
            .editor
            .read(cx)
            .pane_state_ref(self.pane_id)
            .map(|s| s.source_code.line_count())
            .unwrap_or(1);

        let content_height = (total_lines as f32) * line_height + editor_padding * 2.0;

        let mut style = Style::default();
        style.size.width = relative(1.0).into();
        style.size.height = px(content_height).into();
        style.min_size.height = relative(1.0).into();

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
        let font_size = theme.typography.code_size.max(12.0);
        let line_height = (font_size * theme.typography.text_line_height).round();
        let editor_padding = theme.dimensions.editor_padding;
        let font = TypographyStore::default_font(TypographyScope::Code);

        let (text, line_ranges, cursor, selection, spans, is_focused) = {
            let editor_ref = self.editor.read(cx);
            let state = editor_ref.pane_state_ref(self.pane_id);
            let focus_handle = state.and_then(|s| s.source_code.focus_handle.clone());
            let is_focused = focus_handle.as_ref().map_or(false, |h| h.is_focused(window));
            if let Some(state) = state {
                let sc = &state.source_code;
                (
                    sc.text.clone(),
                    sc.line_ranges.clone(),
                    sc.cursor,
                    sc.selection.clone(),
                    sc.highlight_cache
                        .as_ref()
                        .map(|h| h.spans.clone())
                        .unwrap_or_default(),
                    is_focused,
                )
            } else {
                (
                    String::new(),
                    vec![0..0],
                    0,
                    None,
                    Vec::new(),
                    is_focused,
                )
            }
        };

        // Record bounds for IME candidate popup window
        let pane_id = self.pane_id;
        self.editor.update(cx, |ed, _cx| {
            if let Some(state) = ed.pane_state_mut(pane_id) {
                state.source_code.last_bounds = Some(bounds);
            }
        });

        let total_lines = if line_ranges.is_empty() { 1 } else { line_ranges.len() };
        let line_digits = total_lines.to_string().len();
        let gutter_width = (line_digits as f32 * (font_size * 0.6) + 24.0).max(36.0);

        // Virtualized viewport calculation: calculate which rows are visible
        let visible_bounds = window.content_mask().bounds;
        let scroll_y = f32::from(bounds.top() - visible_bounds.top());
        let viewport_height = f32::from(visible_bounds.size.height.max(bounds.size.height));

        let start_row_f = ((-scroll_y - editor_padding) / line_height).floor();
        let start_row = (start_row_f.max(0.0) as usize).min(total_lines.saturating_sub(1));
        let visible_count = ((viewport_height / line_height).ceil() as usize) + 6;
        let end_row = (start_row + visible_count).min(total_lines);

        let (cursor_line, cursor_col) = {
            let clamped = cursor.min(text.len());
            let mut line = 0;
            let mut col = 0;
            for (idx, r) in line_ranges.iter().enumerate() {
                if clamped >= r.start && clamped <= r.end {
                    line = idx;
                    col = clamped - r.start;
                    break;
                }
            }
            (line, col)
        };

        let mut shaped_lines = Vec::with_capacity(end_row - start_row);
        let mut gutter_numbers = Vec::with_capacity(end_row - start_row);
        let mut selection_quads = Vec::new();
        let mut cursor_quad = None;
        let mut active_line_quad = None;

        let text_origin_x = bounds.left() + px(gutter_width + 12.0);

        for row in start_row..end_row {
            let row_range = if row < line_ranges.len() {
                line_ranges[row].clone()
            } else {
                text.len()..text.len()
            };
            let line_start = row_range.start.min(text.len());
            let line_end = row_range.end.min(text.len());
            let line_str = &text[line_start..line_end];

            let runs = build_line_text_runs(
                line_str,
                row_range.clone(),
                &spans,
                font.clone(),
                &theme.colors,
            );

            let shaped_line = window.text_system().shape_line(
                SharedString::new(line_str),
                px(font_size),
                &runs,
                None,
            );

            let line_y = bounds.top() + px(editor_padding + (row as f32) * line_height);

            // Active line background
            if is_focused && row == cursor_line {
                active_line_quad = Some(fill(
                    Bounds::new(
                        point(bounds.left() + px(gutter_width), line_y),
                        size(bounds.size.width - px(gutter_width), px(line_height)),
                    ),
                    theme.colors.source_mode_block_bg,
                ));
            }

            // Selection quads on this line
            if let Some(ref sel) = selection {
                if sel.start < sel.end && sel.start <= row_range.end && sel.end >= row_range.start {
                    let sel_start_in_line = sel.start.saturating_sub(row_range.start).min(line_str.len());
                    let sel_end_in_line = sel.end.saturating_sub(row_range.start).min(line_str.len());

                    let x_start = shaped_line.x_for_index(sel_start_in_line);
                    let x_end = if sel_end_in_line == line_str.len() && sel.end > row_range.end {
                        shaped_line.x_for_index(sel_end_in_line) + px(font_size * 0.5)
                    } else {
                        shaped_line.x_for_index(sel_end_in_line)
                    };

                    if x_end > x_start {
                        selection_quads.push(fill(
                            Bounds::new(
                                point(text_origin_x + x_start, line_y),
                                size(x_end - x_start, px(line_height)),
                            ),
                            theme.colors.selection,
                        ));
                    }
                }
            }

            // Cursor quad
            if is_focused && row == cursor_line && selection.is_none() {
                let cursor_x = shaped_line.x_for_index(cursor_col.min(line_str.len()));
                cursor_quad = Some(fill(
                    Bounds::new(
                        point(text_origin_x + cursor_x, line_y),
                        size(px(theme.dimensions.cursor_width.max(2.0)), px(line_height)),
                    ),
                    theme.colors.cursor,
                ));
            }

            // Gutter line numbers
            let line_num_str = format!("{}", row + 1);
            let num_color = if row == cursor_line {
                theme.colors.text_default
            } else {
                theme.colors.dialog_muted
            };
            let num_run = TextRun {
                len: line_num_str.len(),
                font: font.clone(),
                color: num_color,
                ..Default::default()
            };
            let shaped_num = window.text_system().shape_line(
                SharedString::new(line_num_str),
                px(font_size),
                &[num_run],
                None,
            );

            gutter_numbers.push((row, shaped_num, row == cursor_line));
            shaped_lines.push((row, shaped_line));
        }

        let hitbox = Some(window.insert_hitbox(bounds, HitboxBehavior::Normal));

        SourceCodePrepaintState {
            line_height,
            gutter_width,
            editor_padding,
            shaped_lines,
            cursor_quad,
            selection_quads,
            active_line_quad,
            gutter_numbers,
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
        let theme = cx.global::<ThemeManager>().current_arc();

        if let Some(hitbox) = prepaint.hitbox.as_ref() {
            if hitbox.is_hovered(window) {
                window.set_cursor_style(CursorStyle::IBeam, hitbox);
            }
        }

        let focus_handle = self
            .editor
            .read(cx)
            .pane_state_ref(self.pane_id)
            .and_then(|s| s.source_code.focus_handle.clone());

        if let Some(ref focus_handle) = focus_handle {
            if focus_handle.is_focused(window) {
                window.handle_input(
                    focus_handle,
                    ElementInputHandler::new(bounds, self.editor.clone()),
                    cx,
                );
            }
        }

        // 1. Paint Gutter background & border
        let gutter_bounds = Bounds::new(
            bounds.origin,
            size(px(prepaint.gutter_width), bounds.size.height),
        );
        window.paint_quad(fill(gutter_bounds, theme.colors.editor_background));
        window.paint_quad(fill(
            Bounds::new(
                point(bounds.left() + px(prepaint.gutter_width - 1.0), bounds.top()),
                size(px(1.0), bounds.size.height),
            ),
            theme.colors.table_border,
        ));

        // 2. Paint active line highlight
        if let Some(active_quad) = prepaint.active_line_quad.take() {
            window.paint_quad(active_quad);
        }

        // 3. Paint selection quads
        for sel_quad in prepaint.selection_quads.drain(..) {
            window.paint_quad(sel_quad);
        }

        // 4. Paint Gutter line numbers
        for (row, shaped_num, _) in prepaint.gutter_numbers.drain(..) {
            let line_y = bounds.top() + px(prepaint.editor_padding + (row as f32) * prepaint.line_height);
            let num_x = bounds.left() + px(prepaint.gutter_width - 8.0) - shaped_num.width;
            shaped_num
                .paint(
                    point(num_x, line_y),
                    px(prepaint.line_height),
                    TextAlign::Left,
                    None,
                    window,
                    cx,
                )
                .ok();
        }

        // 5. Paint shaped text lines
        let text_origin_x = bounds.left() + px(prepaint.gutter_width + 12.0);
        for (row, shaped_line) in prepaint.shaped_lines.drain(..) {
            let line_y = bounds.top() + px(prepaint.editor_padding + (row as f32) * prepaint.line_height);
            shaped_line
                .paint(
                    point(text_origin_x, line_y),
                    px(prepaint.line_height),
                    TextAlign::Left,
                    None,
                    window,
                    cx,
                )
                .ok();
        }

        // 6. Paint cursor caret
        if let Some(cursor_quad) = prepaint.cursor_quad.take() {
            window.paint_quad(cursor_quad);
        }
    }
}
