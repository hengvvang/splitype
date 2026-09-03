//! Display-snapshot-driven rendering: the editor element, its prepaint
//! state, and the pane shell that hosts it.
//!
//! Each frame the editor builds a [`DisplaySnapshot`] (folds + soft wraps
//! flattened into visual rows), the element shapes only the visible row
//! segments, and stores the resulting [`RowFrame`]s back on the editor for
//! mouse hit-testing.

use std::ops::Range;
use std::sync::Arc;

use gpui::{
    AnyElement, App, Bounds, Context, Element, ElementId, GlobalElementId, Hitbox, HitboxBehavior,
    InspectorElementId, InteractiveElement, IntoElement, LayoutId, PaintQuad, ParentElement,
    Pixels, ShapedLine, SharedString, StatefulInteractiveElement, Style, Styled, TextAlign,
    TextRun, Window, div, fill, point, px, relative, size,
};
use theme::{ThemeManager, TypographyScope, TypographyStore};

use crate::editor::SourceCodeEditor;
use crate::syntax::indent_guides::compute_indent_guide_columns;
use editor_contracts::{OutlineHost, PaneId, PaneOutlineHost, PaneRenderContext};
use syntax_highlighter::highlight::build_line_text_runs;

/// One visual row of the last rendered frame, used by mouse hit-testing.
#[derive(Clone, Debug)]
pub(crate) struct RowFrame {
    pub display_row: u32,
    pub buffer_row: u32,
    /// Whether this is the first visual row of its buffer row (gutter
    /// numbers and fold markers are drawn only here).
    pub is_first: bool,
    /// Byte range of this visual row's text segment within the document.
    pub range: Range<usize>,
}

impl SourceCodeEditor {
    /// Renders the pane shell: key/mouse routing, the scroll container, the
    /// editor element, and the floating outline HUD.
    pub fn render(
        &mut self,
        ctx: &PaneRenderContext,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.host = Some(ctx.host.clone());
        self.scroll = Some(ctx.scroll.clone());

        let theme = cx.global::<ThemeManager>().current_arc();
        let font_size = theme.typography.code_size.max(12.0);
        let line_height = (font_size * theme.typography.text_line_height).round();

        let viewport_width = f32::from(ctx.scroll.bounds().size.width);
        // Skip the first frame(s) before the scroll container has a real
        // width; otherwise wrap would clamp to the minimum column count.
        if viewport_width > 0.0 {
            self.ensure_wrap(viewport_width, cx);
        }

        // Warm the per-revision caches before the element reads them
        // during prepaint: the display row index and the matching bracket
        // (each recomputes only on invalidation). Outline headings and
        // foldable regions come from the background highlight pipeline.
        self.ensure_rows_cache();
        self.bracket_offset = self.matching_bracket();
        let headings = self.cached_outline_headings();
        let scroll_y = -f32::from(ctx.scroll.offset().y);
        let top_visible_line = (scroll_y / line_height).floor().max(0.0) as usize;
        let active_index = headings
            .iter()
            .rposition(|h| h.block_index <= top_visible_line)
            .or(if headings.is_empty() { None } else { Some(0) });

        let outline_host: Arc<dyn OutlineHost> = Arc::new(PaneOutlineHost {
            pane_id: ctx.pane_id,
            host: ctx.host.clone(),
        });
        let outline_hud = ui::render_floating_outline_hud(
            ctx.pane_id.0,
            &headings,
            active_index,
            ctx.is_outline_hovered,
            &theme,
            &outline_host,
        );

        let focus_handle = self.focus_handle.clone();
        let editor_entity = cx.entity();

        let mut outer = div()
            .id(ElementId::Name(
                format!("tiled-source-editor-{}", ctx.pane_id.0).into(),
            ))
            .key_context("EditorContent")
            .w_full()
            .h_full()
            .relative()
            .bg(theme.colors.editor_background)
            .track_focus(&focus_handle);

        let pane_id = ctx.pane_id;
        let host = ctx.host.clone();
        let host_key = host.clone();
        let host_down = host.clone();
        let host_move = host.clone();
        let host_up = host.clone();

        outer = outer.on_key_down(move |event, window, cx| {
            let handled = host_key.handle_pane_key_down(pane_id, event, window, cx);
            if handled {
                cx.stop_propagation();
            }
        });

        outer
            .child(
                div()
                    .id(ElementId::Name(
                        format!("tiled-source-scroll-{}", pane_id.0).into(),
                    ))
                    .w_full()
                    .h_full()
                    .overflow_y_scroll()
                    .track_scroll(ctx.scroll)
                    .on_mouse_down(gpui::MouseButton::Left, move |event, window, cx| {
                        host_down.handle_pane_mouse_down(pane_id, event, window, cx);
                    })
                    .on_mouse_move(move |event, window, cx| {
                        host_move.handle_pane_mouse_move(pane_id, event, window, cx);
                    })
                    .on_mouse_up(gpui::MouseButton::Left, move |event, window, cx| {
                        host_up.handle_pane_mouse_up(pane_id, event, window, cx);
                    })
                    .child(EditorElement::new(editor_entity, pane_id, ctx.is_focused)),
            )
            .child(outline_hud)
            .into_any_element()
    }
}

/// Virtualized rendering element for the source code editor.
pub struct EditorElement {
    editor: gpui::Entity<SourceCodeEditor>,
    pane_id: PaneId,
    is_focused: bool,
}

impl EditorElement {
    pub fn new(editor: gpui::Entity<SourceCodeEditor>, pane_id: PaneId, is_focused: bool) -> Self {
        Self {
            editor,
            pane_id,
            is_focused,
        }
    }
}

pub struct SourceCodePrepaintState {
    pub(crate) line_height: f32,
    pub(crate) gutter_width: f32,
    pub(crate) editor_padding: f32,
    pub(crate) shaped_lines: Vec<(u32, ShapedLine)>,
    pub(crate) cursor_quads: Vec<PaintQuad>,
    pub(crate) selection_quads: Vec<PaintQuad>,
    pub(crate) active_line_quads: Vec<PaintQuad>,
    pub(crate) search_match_quads: Vec<PaintQuad>,
    pub(crate) bracket_match_quads: Vec<PaintQuad>,
    pub(crate) marked_range_quads: Vec<PaintQuad>,
    pub(crate) indent_guide_quads: Vec<PaintQuad>,
    pub(crate) gutter_numbers: Vec<(u32, ShapedLine, bool)>,
    pub(crate) fold_markers: Vec<(u32, ShapedLine)>,
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

        let visible_lines = {
            let editor = self.editor.read(cx);
            editor.snapshot().visible_line_count()
        };
        let content_height = visible_lines as f32 * line_height + editor_padding * 2.0;

        let mut style = Style::default();
        style.size.width = relative(1.0).into();
        style.size.height = px(content_height).into();
        style.min_size.height = relative(1.0).into();

        (window.request_layout(style, [], cx), ())
    }

    #[allow(clippy::too_many_arguments)]
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

        // The visible display-row window, computed before the editor read
        // so only visible rows are walked (never the whole document).
        let visible_bounds = window.content_mask().bounds;
        let scroll_y = f32::from(bounds.top() - visible_bounds.top());
        let viewport_height = f32::from(visible_bounds.size.height.max(bounds.size.height));
        let start_visible_row = ((-scroll_y - editor_padding) / line_height)
            .floor()
            .max(0.0) as u32;
        let visible_count = ((viewport_height / line_height).ceil() as u32) + 8;
        let end_visible_row = start_visible_row + visible_count;

        // One editor read gathers every piece of frame data; the row loop
        // below only uses owned copies.
        let (
            text,
            frames,
            line_count,
            primary_head_display_row,
            primary_cursor_buffer_row,
            highlight_active_line,
            line_numbers,
            tab_size,
            highlight,
            search_matches,
            selection_ranges,
            cursor_rows,
            gutter_width,
            marked_range,
            matching_bracket_offset,
            is_focused,
        ) = {
            let editor = self.editor.read(cx);
            let snapshot = editor.snapshot();
            let is_focused = self.is_focused || editor.focus_handle.is_focused(window);
            let line_count = editor.line_count();

            // Walk only the visible display rows, mapping each back to its
            // buffer row through the row index (O(log n) per row). Folded
            // and foldable markers are looked up per visible row.
            let mut frames: Vec<(RowFrame, Option<bool>)> = Vec::new();
            let total_rows = snapshot.rows.total.min(end_visible_row);
            for display_row in start_visible_row..total_rows {
                let buffer_row = snapshot.rows.buffer_row_at(display_row) as usize;
                let start_row = snapshot.rows.starts[buffer_row];
                let wrap_index = display_row - start_row;
                let line = editor.line_str(buffer_row);
                if wrap_index >= snapshot.wrap.line_rows(buffer_row) {
                    continue; // display row inside a folded region
                }
                let segment =
                    snapshot
                        .wrap
                        .row_range(buffer_row, wrap_index as usize, 0..line.len());
                let line_start = editor.line_start_offset(buffer_row);
                let folded = editor.folds().is_folded(buffer_row as u32);
                let foldable = !folded && editor.foldable_at(buffer_row as u32).is_some();
                frames.push((
                    RowFrame {
                        display_row,
                        buffer_row: buffer_row as u32,
                        is_first: wrap_index == 0,
                        range: line_start + segment.start..line_start + segment.end,
                    },
                    (folded || foldable).then_some(folded),
                ));
            }

            let primary_head_display_row = snapshot.offset_to_display_point(editor.cursor()).row;
            let primary_cursor_buffer_row = editor.point_of(editor.cursor()).0 as u32;
            let selection_ranges: Vec<(usize, usize)> = editor
                .selections()
                .iter()
                .map(|s| (s.start(), s.end()))
                .collect();
            let cursor_rows: Vec<(usize, u32)> = editor
                .selections()
                .iter()
                .map(|s| {
                    let dp = snapshot.offset_to_display_point(s.head);
                    (s.head, dp.row)
                })
                .collect();

            (
                editor.text().clone(),
                frames,
                line_count,
                primary_head_display_row,
                primary_cursor_buffer_row,
                editor.settings().highlight_active_line,
                editor.settings().line_numbers,
                editor.settings().tab_size,
                editor.highlight_result(),
                editor.search_matches().to_vec(),
                selection_ranges,
                cursor_rows,
                editor.gutter_width_px(cx),
                editor.marked_range(),
                editor.bracket_offset,
                is_focused,
            )
        };

        let mut shaped_lines = Vec::new();
        let mut gutter_numbers = Vec::new();
        let mut fold_markers = Vec::new();
        let mut selection_quads = Vec::new();
        let mut cursor_quads = Vec::new();
        let mut active_line_quads = Vec::new();
        let mut search_match_quads = Vec::new();
        let mut bracket_match_quads = Vec::new();
        let mut marked_range_quads = Vec::new();
        let mut indent_guide_quads = Vec::new();

        let text_origin_x = bounds.left() + px(gutter_width + 12.0);
        let char_width = font_size * 0.6;
        let gutter_layout = crate::gutter::GutterLayout::new(line_count, font_size);

        for (frame, fold_marker) in &frames {
            let line_y = bounds.top() + px(editor_padding + frame.display_row as f32 * line_height);
            let segment = text.slice_owned(frame.range.clone());
            let spans = highlight
                .as_ref()
                .map(|result| result.spans.as_slice())
                .unwrap_or(&[]);
            let runs = build_line_text_runs(
                &segment,
                frame.range.clone(),
                spans,
                font.clone(),
                &theme.colors,
            );
            let shaped_line = window.text_system().shape_line(
                SharedString::new(segment.clone()),
                px(font_size),
                &runs,
                None,
            );

            // 1. Active line highlight (subtle background bar).
            if highlight_active_line && is_focused && frame.display_row == primary_head_display_row
            {
                active_line_quads.push(fill(
                    Bounds::new(
                        point(bounds.left() + px(gutter_width), line_y),
                        size(bounds.size.width - px(gutter_width), px(line_height)),
                    ),
                    theme.colors.selection.opacity(0.12),
                ));
            }

            // 2. Indent guides on the leading whitespace of the buffer line.
            if frame.is_first {
                let indent_cols = compute_indent_guide_columns(&segment, tab_size);
                for col in indent_cols {
                    let guide_x = text_origin_x + px(col as f32 * char_width);
                    indent_guide_quads.push(fill(
                        Bounds::new(point(guide_x, line_y), size(px(1.0), px(line_height))),
                        theme.colors.dialog_border.opacity(0.25),
                    ));
                }
            }

            // 3. Selection quads across all selections.
            for (sel_start, sel_end) in &selection_ranges {
                let clip_start = (*sel_start).max(frame.range.start);
                let clip_end = (*sel_end).min(frame.range.end);
                if clip_start >= clip_end {
                    if *sel_start < frame.range.end && *sel_end > frame.range.end {
                        // Selection continues onto the next visual row:
                        // fill this row to its end.
                        let x_start = shaped_line.x_for_index(clip_start - frame.range.start);
                        let x_end = shaped_line.width + px(char_width);
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
                    continue;
                }
                let x_start = shaped_line.x_for_index(clip_start - frame.range.start);
                let x_end = shaped_line.x_for_index(clip_end - frame.range.start);
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

            // 4. Search match highlights on this row.
            for (m_range, is_active) in &search_matches {
                let m_start = m_range.start.max(frame.range.start);
                let m_end = m_range.end.min(frame.range.end);
                if m_start < m_end {
                    let x_start = shaped_line.x_for_index(m_start - frame.range.start);
                    let x_end = shaped_line.x_for_index(m_end - frame.range.start);
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

            // 5. Matching bracket underline.
            if let Some(match_off) = matching_bracket_offset {
                if match_off >= frame.range.start && match_off < frame.range.end {
                    let col_in_segment = match_off - frame.range.start;
                    let x_start = shaped_line.x_for_index(col_in_segment);
                    let x_end = shaped_line.x_for_index((col_in_segment + 1).min(segment.len()));
                    bracket_match_quads.push(fill(
                        Bounds::new(
                            point(text_origin_x + x_start, line_y + px(line_height - 2.0)),
                            size((x_end - x_start).max(px(char_width)), px(2.0)),
                        ),
                        theme.colors.focus_accent,
                    ));
                }
            }

            // 6. IME marked-range underline.
            if let Some(marked) = &marked_range {
                let m_start = marked.start.max(frame.range.start);
                let m_end = marked.end.min(frame.range.end);
                if m_start < m_end {
                    let x_start = shaped_line.x_for_index(m_start - frame.range.start);
                    let x_end = shaped_line.x_for_index(m_end - frame.range.start);
                    marked_range_quads.push(fill(
                        Bounds::new(
                            point(text_origin_x + x_start, line_y + px(line_height - 2.0)),
                            size((x_end - x_start).max(px(1.0)), px(2.0)),
                        ),
                        theme.colors.text_default.opacity(0.8),
                    ));
                }
            }

            // 7. Cursors on this row.
            if is_focused {
                for (head, dp_row) in &cursor_rows {
                    if *dp_row == frame.display_row {
                        let head_in_segment =
                            head.saturating_sub(frame.range.start).min(segment.len());
                        let cursor_x = shaped_line.x_for_index(head_in_segment);
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

            shaped_lines.push((frame.display_row, shaped_line));

            // 8. Gutter line numbers and fold markers (first visual row only).
            if frame.is_first {
                if line_numbers {
                    let num_str = gutter_layout.format_line_number(frame.buffer_row);
                    let is_active_row = frame.buffer_row == primary_cursor_buffer_row;
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
                    gutter_numbers.push((frame.display_row, shaped_num, is_active_row));
                }

                if let Some(folded) = fold_marker {
                    let marker = if *folded { "▾" } else { "▸" };
                    let marker_run = TextRun {
                        len: marker.len(),
                        font: font.clone(),
                        color: theme.colors.dialog_muted,
                        ..Default::default()
                    };
                    let shaped_marker = window.text_system().shape_line(
                        SharedString::new(marker),
                        px(font_size),
                        &[marker_run],
                        None,
                    );
                    fold_markers.push((frame.display_row, shaped_marker));
                }
            }
        }

        // Store the frame layout for mouse hit-testing and the bounds for
        // coordinate math.
        let stored_frames: Vec<RowFrame> = frames.iter().map(|(frame, _)| frame.clone()).collect();
        self.editor.update(cx, |editor, _cx| {
            editor.frame_rows = stored_frames;
            editor.set_last_bounds(bounds);
        });

        let hitbox = Some(window.insert_hitbox(bounds, HitboxBehavior::Normal));

        SourceCodePrepaintState {
            line_height,
            gutter_width,
            editor_padding,
            shaped_lines,
            cursor_quads,
            selection_quads,
            active_line_quads,
            search_match_quads,
            bracket_match_quads,
            marked_range_quads,
            indent_guide_quads,
            gutter_numbers,
            fold_markers,
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
                window.set_cursor_style(gpui::CursorStyle::IBeam, hitbox);
            }
        }

        // 1. Gutter background.
        let gutter_bounds = Bounds::new(
            bounds.origin,
            size(px(prepaint.gutter_width), bounds.size.height),
        );
        window.paint_quad(fill(gutter_bounds, theme.colors.editor_background));

        // 2. Active line background.
        for active_quad in prepaint.active_line_quads.drain(..) {
            window.paint_quad(active_quad);
        }

        // 3. Indent guides.
        for guide in prepaint.indent_guide_quads.drain(..) {
            window.paint_quad(guide);
        }

        // 4. Search match quads.
        for search_quad in prepaint.search_match_quads.drain(..) {
            window.paint_quad(search_quad);
        }

        // 5. Selection quads.
        for sel_quad in prepaint.selection_quads.drain(..) {
            window.paint_quad(sel_quad);
        }

        // 6. Bracket matching underlines.
        for b_quad in prepaint.bracket_match_quads.drain(..) {
            window.paint_quad(b_quad);
        }

        // 7. IME marked-range underlines.
        for m_quad in prepaint.marked_range_quads.drain(..) {
            window.paint_quad(m_quad);
        }

        // 8. Gutter line numbers (right-aligned with 10px padding).
        for (visible_row, shaped_num, _) in prepaint.gutter_numbers.drain(..) {
            let line_y = bounds.top()
                + px(prepaint.editor_padding + visible_row as f32 * prepaint.line_height);
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

        // 9. Fold markers at the left of the gutter.
        for (visible_row, shaped_marker) in prepaint.fold_markers.drain(..) {
            let line_y = bounds.top()
                + px(prepaint.editor_padding + visible_row as f32 * prepaint.line_height);
            shaped_marker
                .paint(
                    point(bounds.left() + px(6.0), line_y),
                    px(prepaint.line_height),
                    TextAlign::Left,
                    None,
                    window,
                    cx,
                )
                .ok();
        }

        // 10. Shaped syntax text lines.
        let text_origin_x = bounds.left() + px(prepaint.gutter_width + 12.0);
        for (visible_row, shaped_line) in prepaint.shaped_lines.drain(..) {
            let line_y = bounds.top()
                + px(prepaint.editor_padding + visible_row as f32 * prepaint.line_height);
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

        // 11. Cursor carets.
        for c_quad in prepaint.cursor_quads.drain(..) {
            window.paint_quad(c_quad);
        }

        // 12. IME input bridge: route composition through the editor entity.
        let focus_handle = self.editor.read(cx).focus_handle.clone();
        if focus_handle.is_focused(window) {
            window.handle_input(
                &focus_handle,
                gpui::ElementInputHandler::new(bounds, self.editor.clone()),
                cx,
            );
        }
    }
}
