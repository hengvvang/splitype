//! High-performance virtualized GPUI Element for the Source Code pane.
//! Implements Zed-style sub-pixel text shaping, viewport virtualization,
//! Tree-sitter syntax highlighting, indent guides, and multi-cursor rendering.

use editor_contracts::PaneId;
use gpui::*;
use theme::{ThemeManager, TypographyScope, TypographyStore};

use crate::state::SourceCodeState;
use crate::syntax::indent_guides::compute_indent_guide_columns;
use syntax_highlighter::highlight::build_line_text_runs;

/// High-performance virtualized rendering element for SourceCodeState.
pub struct EditorElement {
    state: SourceCodeState,
    pane_id: PaneId,
    is_focused: bool,
}

impl EditorElement {
    pub fn new(state: SourceCodeState, pane_id: PaneId, is_focused: bool) -> Self {
        Self {
            state,
            pane_id,
            is_focused,
        }
    }
}

pub struct SourceCodePrepaintState {
    pub(crate) line_height: f32,
    pub(crate) gutter_width: f32,
    pub(crate) editor_padding: f32,
    pub(crate) shaped_lines: Vec<(usize, ShapedLine)>,
    pub(crate) cursor_quads: Vec<PaintQuad>,
    pub(crate) selection_quads: Vec<PaintQuad>,
    pub(crate) active_line_quad: Option<PaintQuad>,
    pub(crate) search_match_quads: Vec<PaintQuad>,
    pub(crate) bracket_match_quads: Vec<PaintQuad>,
    pub(crate) indent_guide_quads: Vec<PaintQuad>,
    pub(crate) gutter_numbers: Vec<(usize, ShapedLine, bool)>,
    pub(crate) hitbox: Option<Hitbox>,
}

impl IntoElement for EditorElement {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for EditorElement {
    type RequestLayoutState = ();
    type PrepaintState = SourceCodePrepaintState;

    fn id(&self) -> Option<ElementId> {
        Some(ElementId::Name(
            format!("source-code-editor-{}", self.pane_id.0).into(),
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

        let visible_lines = self
            .state
            .fold_map
            .visible_line_count(self.state.line_count() as u32);
        let content_height = (visible_lines as f32) * line_height + editor_padding * 2.0;

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
        if let Ok(mut lb) = self.state.last_bounds.try_borrow_mut() {
            *lb = Some(bounds);
        }
        let font = TypographyStore::default_font(TypographyScope::Code);

        let focus_handle = self.state.focus_handle.borrow().clone();
        let is_focused =
            self.is_focused || focus_handle.as_ref().is_some_and(|h| h.is_focused(window));

        let gutter_width = if self.state.settings.line_numbers {
            self.state.gutter_layout(font_size).width()
        } else {
            8.0
        };

        let visible_bounds = window.content_mask().bounds;
        let scroll_y = f32::from(bounds.top() - visible_bounds.top());
        let viewport_height = f32::from(visible_bounds.size.height.max(bounds.size.height));

        let total_buffer_rows = self.state.line_count() as u32;
        let total_visible_lines = self.state.fold_map.visible_line_count(total_buffer_rows);

        let start_visible_row_f = ((-scroll_y - editor_padding) / line_height).floor();
        let start_visible_row =
            (start_visible_row_f.max(0.0) as u32).min(total_visible_lines.saturating_sub(1));
        let visible_count = ((viewport_height / line_height).ceil() as u32) + 8;
        let end_visible_row = (start_visible_row + visible_count).min(total_visible_lines);

        let cursor_offset = self.state.cursor();
        let (primary_cursor_line, _primary_cursor_col) = self.state.line_and_column(cursor_offset);
        let matching_bracket_offset = self.state.matching_bracket();

        let mut shaped_lines = Vec::with_capacity((end_visible_row - start_visible_row) as usize);
        let mut gutter_numbers = Vec::with_capacity((end_visible_row - start_visible_row) as usize);
        let mut selection_quads = Vec::new();
        let mut cursor_quads = Vec::new();
        let mut active_line_quad = None;
        let mut search_match_quads = Vec::new();
        let mut bracket_match_quads = Vec::new();
        let mut indent_guide_quads = Vec::new();

        let text_origin_x = bounds.left() + px(gutter_width + 12.0);
        let char_width = font_size * 0.6;

        let spans = self
            .state
            .highlight_cache
            .as_ref()
            .map(|c| c.spans.as_slice())
            .unwrap_or(&[]);

        for visible_row in start_visible_row..end_visible_row {
            let buffer_row = self
                .state
                .fold_map
                .visible_row_to_buffer_row(visible_row, total_buffer_rows)
                as usize;
            let row_range = self.state.line_range(buffer_row);
            let line_start = row_range.start.min(self.state.text.len());
            let line_end = row_range.end.min(self.state.text.len());
            let line_str = &self.state.text[line_start..line_end];

            let runs = build_line_text_runs(
                line_str,
                row_range.clone(),
                spans,
                font.clone(),
                &theme.colors,
            );

            let shaped_line = window.text_system().shape_line(
                SharedString::new(line_str),
                px(font_size),
                &runs,
                None,
            );

            let line_y = bounds.top() + px(editor_padding + (visible_row as f32) * line_height);

            // 1. Active line highlight (subtle background bar like Zed)
            if self.state.settings.highlight_active_line
                && is_focused
                && buffer_row == primary_cursor_line
            {
                active_line_quad = Some(fill(
                    Bounds::new(
                        point(bounds.left() + px(gutter_width), line_y),
                        size(bounds.size.width - px(gutter_width), px(line_height)),
                    ),
                    theme.colors.selection.opacity(0.12),
                ));
            }

            // 2. Indent guides (clean vertical alignment lines)
            let indent_cols = compute_indent_guide_columns(line_str, self.state.tab_map.tab_size);
            for col in indent_cols {
                let guide_x = text_origin_x + px(col as f32 * char_width);
                indent_guide_quads.push(fill(
                    Bounds::new(point(guide_x, line_y), size(px(1.0), px(line_height))),
                    theme.colors.dialog_border.opacity(0.25),
                ));
            }

            // 3. Selection quads across all selections
            for sel in self.state.selections.all() {
                if !sel.is_empty() && sel.start() <= row_range.end && sel.end() >= row_range.start {
                    let sel_start_in_line = sel
                        .start()
                        .saturating_sub(row_range.start)
                        .min(line_str.len());
                    let sel_end_in_line = sel
                        .end()
                        .saturating_sub(row_range.start)
                        .min(line_str.len());

                    let x_start = shaped_line.x_for_index(sel_start_in_line);
                    let x_end = if sel_end_in_line == line_str.len() && sel.end() > row_range.end {
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

            // 4. Search match highlights on this line
            for (m_range, is_active) in &self.state.search_matches {
                if m_range.start <= row_range.end && m_range.end >= row_range.start {
                    let m_start_in_line = m_range
                        .start
                        .saturating_sub(row_range.start)
                        .min(line_str.len());
                    let m_end_in_line = m_range
                        .end
                        .saturating_sub(row_range.start)
                        .min(line_str.len());

                    let x_start = shaped_line.x_for_index(m_start_in_line);
                    let x_end = shaped_line.x_for_index(m_end_in_line);

                    if x_end > x_start {
                        let color = if *is_active {
                            theme.colors.focus_accent.opacity(0.4)
                        } else {
                            theme.colors.selection.opacity(0.6)
                        };
                        search_match_quads.push(fill(
                            Bounds::new(
                                point(text_origin_x + x_start, line_y),
                                size(x_end - x_start, px(line_height)),
                            ),
                            color,
                        ));
                    }
                }
            }

            // 5. Matching bracket highlight
            if let Some(match_off) = matching_bracket_offset {
                if match_off >= row_range.start && match_off < row_range.end {
                    let col_in_line = match_off - row_range.start;
                    let x_start = shaped_line.x_for_index(col_in_line);
                    let x_end = shaped_line.x_for_index((col_in_line + 1).min(line_str.len()));
                    bracket_match_quads.push(fill(
                        Bounds::new(
                            point(text_origin_x + x_start, line_y + px(line_height - 2.0)),
                            size((x_end - x_start).max(px(char_width)), px(2.0)),
                        ),
                        theme.colors.focus_accent,
                    ));
                }
            }

            // 6. Cursors on this line
            if is_focused {
                for sel in self.state.selections.all() {
                    let (c_line, c_col) = self.state.line_and_column(sel.head);
                    if c_line == buffer_row {
                        let cursor_x = shaped_line.x_for_index(c_col.min(line_str.len()));
                        cursor_quads.push(fill(
                            Bounds::new(
                                point(text_origin_x + cursor_x, line_y),
                                size(px(theme.dimensions.cursor_width.max(2.0)), px(line_height)),
                            ),
                            theme.colors.cursor,
                        ));
                    }
                }
            }

            shaped_lines.push((visible_row as usize, shaped_line));

            // 7. Gutter line numbers & formatting
            if self.state.settings.line_numbers {
                let gutter_layout = self.state.gutter_layout(font_size);
                let num_str = gutter_layout.format_line_number(buffer_row as u32);
                let is_active_row = buffer_row == primary_cursor_line;
                let num_color = if is_active_row {
                    theme.colors.text_default
                } else {
                    theme.colors.dialog_muted
                };

                let num_run = TextRun {
                    len: num_str.len(),
                    font: font.clone(),
                    color: num_color,
                    ..Default::default()
                };
                let shaped_num = window.text_system().shape_line(
                    SharedString::new(num_str),
                    px(font_size),
                    &[num_run],
                    None,
                );
                gutter_numbers.push((visible_row as usize, shaped_num, is_active_row));
            }
        }

        let hitbox = Some(window.insert_hitbox(bounds, HitboxBehavior::Normal));

        SourceCodePrepaintState {
            line_height,
            gutter_width,
            editor_padding,
            shaped_lines,
            cursor_quads,
            selection_quads,
            active_line_quad,
            search_match_quads,
            bracket_match_quads,
            indent_guide_quads,
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

        // 1. Paint Gutter background (seamless minimalist style matching Zed)
        let gutter_bounds = Bounds::new(
            bounds.origin,
            size(px(prepaint.gutter_width), bounds.size.height),
        );
        window.paint_quad(fill(gutter_bounds, theme.colors.editor_background));

        // 2. Paint active line background
        if let Some(active_quad) = prepaint.active_line_quad.take() {
            window.paint_quad(active_quad);
        }

        // 3. Paint indent guides
        for guide in prepaint.indent_guide_quads.drain(..) {
            window.paint_quad(guide);
        }

        // 4. Paint search match quads
        for search_quad in prepaint.search_match_quads.drain(..) {
            window.paint_quad(search_quad);
        }

        // 5. Paint selection quads
        for sel_quad in prepaint.selection_quads.drain(..) {
            window.paint_quad(sel_quad);
        }

        // 6. Paint bracket matching underlines
        for b_quad in prepaint.bracket_match_quads.drain(..) {
            window.paint_quad(b_quad);
        }

        // 7. Paint Gutter line numbers (right-aligned with 10px padding)
        for (visible_row, shaped_num, _) in prepaint.gutter_numbers.drain(..) {
            let line_y = bounds.top()
                + px(prepaint.editor_padding + (visible_row as f32) * prepaint.line_height);
            let num_x = bounds.left() + px(prepaint.gutter_width - 10.0) - shaped_num.width;
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

        // 8. Paint shaped syntax text lines
        let text_origin_x = bounds.left() + px(prepaint.gutter_width + 12.0);
        for (visible_row, shaped_line) in prepaint.shaped_lines.drain(..) {
            let line_y = bounds.top()
                + px(prepaint.editor_padding + (visible_row as f32) * prepaint.line_height);
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

        // 9. Paint all cursor carets
        for c_quad in prepaint.cursor_quads.drain(..) {
            window.paint_quad(c_quad);
        }
    }
}
