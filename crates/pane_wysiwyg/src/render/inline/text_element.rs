//! BlockTextElement — GPUI Element for styled inline text.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::*;

pub use super::code_input::CodeLanguageInputElement;
use super::shaping::{build_code_text_runs, build_text_runs, unrounded_line_height};
use crate::model::block::Block;
use crate::render::text_layout::*;
use theme::ThemeManager;

/// Interactive GPUI element that paints and handles mouse/keyboard events
/// for a block's text content.
pub struct BlockTextElement {
    input: Entity<Block>,
    is_placeholder: bool,
}

impl BlockTextElement {
    pub fn new(input: Entity<Block>, is_placeholder: bool) -> Self {
        Self {
            input,
            is_placeholder,
        }
    }
}

/// Prepared text layout and paint geometry for one `BlockTextElement` frame.
pub struct PrepaintState {
    lines: Vec<WrappedLine>,
    source_line_numbers: Vec<ShapedLine>,
    source_line_number_gutter_width: Pixels,
    cursor: Option<PaintQuad>,
    selection: Vec<PaintQuad>,
    code_backgrounds: Vec<PaintQuad>,
    line_height: Pixels,
    hitbox: Hitbox,
}

impl IntoElement for BlockTextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for BlockTextElement {
    type RequestLayoutState = Rc<RefCell<Option<Vec<WrappedLine>>>>;
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
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
        let input = self.input.read(cx);
        let shared_text = input.shared_display_text();
        let is_placeholder = self.is_placeholder;
        let show_source_line_numbers = input.show_source_line_numbers();
        let source_line_count = source_line_count(shared_text.as_ref());
        let style = window.text_style();

        let (display_text, text_color): (SharedString, Hsla) = if is_placeholder {
            (
                theme.placeholders.empty_editing.clone().into(),
                theme.colors.text_placeholder,
            )
        } else {
            (shared_text, style.color)
        };

        let run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        let runs: Vec<TextRun> = if !is_placeholder {
            if input.kind().uses_code_highlighting() {
                build_code_text_runs(
                    input,
                    &display_text,
                    &run,
                    px(theme.dimensions.underline_thickness),
                    &theme.colors,
                )
            } else {
                build_text_runs(
                    input,
                    &display_text,
                    &run,
                    px(theme.dimensions.underline_thickness),
                    theme.colors.text_link,
                    theme.colors.markdown_marker,
                    theme.colors.footnote_backref,
                    theme.colors.text_highlight_bg,
                )
            }
        } else {
            vec![run]
        };

        let font_size = style.font_size.to_pixels(window.rem_size());
        let line_height = unrounded_line_height(window);
        let source_line_number_gutter_width = show_source_line_numbers
            .then(|| source_line_number_gutter_width(source_line_count, font_size))
            .unwrap_or(px(0.0));

        let shared_lines = Rc::new(RefCell::new(None));
        let shared_lines_clone = shared_lines.clone();

        let mut layout_style = Style::default();
        layout_style.size.width = relative(1.).into();
        layout_style.min_size.width = px(0.0).into();
        layout_style.max_size.width = relative(1.).into();

        let layout_id = window.request_measured_layout(
            layout_style,
            move |known_dimensions, available_space, window, _cx| {
                let wrap_width = known_dimensions.width.or(match available_space.width {
                    AvailableSpace::Definite(x) => Some(x),
                    AvailableSpace::MinContent => None,
                    AvailableSpace::MaxContent => Some(window.viewport_size().width.max(px(1.0))),
                });
                let text_wrap_width =
                    wrap_width.map(|width| (width - source_line_number_gutter_width).max(px(1.0)));

                match window.text_system().shape_text(
                    display_text.clone(),
                    font_size,
                    &runs,
                    text_wrap_width,
                    None,
                ) {
                    Ok(lines) => {
                        let mut total_size: Size<Pixels> = Size::default();
                        for line in &lines {
                            let ls = line.size(line_height);
                            total_size.height += ls.height.max(line_height);
                            total_size.width = total_size.width.max(ls.width);
                        }
                        total_size.height = total_size.height.max(line_height);
                        total_size.width += source_line_number_gutter_width;
                        if wrap_width.is_some() {
                            *shared_lines_clone.borrow_mut() = Some(lines.into_vec());
                        }
                        total_size
                    }
                    Err(_) => Size {
                        width: px(0.0),
                        height: line_height,
                    },
                }
            },
        );

        (layout_id, shared_lines)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let theme = cx.global::<ThemeManager>().current_arc();
        let input = self.input.read(cx);
        let editor_selection_range = input
            .editor_selection_range
            .as_ref()
            .filter(|range| !range.is_empty())
            .cloned();
        let selected_range = editor_selection_range
            .clone()
            .unwrap_or_else(|| input.selected_range.clone());
        let cursor = input.cursor_offset();
        let line_height = unrounded_line_height(window);
        let focused = input.focus_handle.is_focused(window);
        let show_inline_code_backgrounds = !input.is_verbatim_mode();
        let show_source_line_numbers = input.show_source_line_numbers();
        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());

        let lines = request_layout.borrow_mut().take().unwrap_or_default();
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
        let source_line_number_gutter_width = show_source_line_numbers
            .then(|| source_line_number_gutter_width(lines.len().max(1), font_size))
            .unwrap_or(px(0.0));
        let text_bounds = source_text_bounds(bounds, source_line_number_gutter_width);
        let source_line_numbers = if show_source_line_numbers {
            let run_color = theme.colors.text_placeholder;
            (1..=lines.len().max(1))
                .map(|line_number| {
                    let label = line_number.to_string();
                    window.text_system().shape_line(
                        SharedString::from(label.clone()),
                        font_size,
                        &[TextRun {
                            len: label.len(),
                            font: style.font(),
                            color: run_color,
                            background_color: None,
                            underline: None,
                            strikethrough: None,
                        }],
                        None,
                    )
                })
                .collect()
        } else {
            Vec::new()
        };

        let cursor_opacity = input.cursor_opacity();
        let cursor_color = {
            let mut c = theme.colors.cursor;
            c.a *= cursor_opacity;
            c
        };
        let cursor_width = theme.dimensions.cursor_width;
        let selection_color = theme.colors.selection;
        let text_align = input.text_align();

        let (selection_quads, cursor_quad) =
            if (focused || editor_selection_range.is_some()) && !lines.is_empty() {
                if self.is_placeholder {
                    // Placeholder: cursor after the placeholder text
                    let layout = &lines[0];
                    let origin_x = aligned_line_left(layout, text_bounds, text_align);
                    let cursor_pos = layout
                        .position_for_index(0, line_height)
                        .unwrap_or_default();
                    (
                        vec![],
                        Some(fill(
                            Bounds::new(
                                point(origin_x + cursor_pos.x, text_bounds.top() + cursor_pos.y),
                                size(px(cursor_width), line_height),
                            ),
                            cursor_color,
                        )),
                    )
                } else if selected_range.is_empty() {
                    // No selection: just draw the cursor
                    let text = input.display_text();
                    (
                        vec![],
                        cursor_bounds_for_offset(
                            &lines,
                            text_bounds,
                            line_height,
                            text,
                            cursor,
                            text_align,
                            px(cursor_width),
                        )
                        .map(|bounds| fill(bounds, cursor_color)),
                    )
                } else {
                    let text = input.display_text();
                    let quads = range_segment_bounds(
                        &lines,
                        text_bounds,
                        line_height,
                        text,
                        selected_range,
                        text_align,
                    )
                    .into_iter()
                    .map(|bounds| fill(bounds, selection_color))
                    .collect();
                    (quads, None)
                }
            } else {
                (vec![], None)
            };

        // Compute code-span and highlight background quads with rounded corners and padding.
        let mut code_quads = Vec::new();
        if !self.is_placeholder {
            let text = input.display_text();
            let code_color = theme.colors.code_bg;
            let highlight_color = theme.colors.text_highlight_bg;
            let pad_x = px(theme.dimensions.code_bg_pad_x);
            let pad_y = px(theme.dimensions.code_bg_pad_y);
            let radius = px(theme.dimensions.code_bg_radius);
            for span in input.inline_spans() {
                if span.range.is_empty() {
                    continue;
                }
                if show_inline_code_backgrounds && span.style.code {
                    for segment in range_segment_bounds(
                        &lines,
                        text_bounds,
                        line_height,
                        text,
                        span.range.clone(),
                        text_align,
                    ) {
                        let quad_bounds = Bounds::from_corners(
                            point(segment.left() - pad_x, segment.top() - pad_y),
                            point(segment.right() + pad_x, segment.bottom() + pad_y),
                        );
                        code_quads.push({
                            let mut q = fill(quad_bounds, code_color);
                            q.corner_radii = Corners::all(radius);
                            q
                        });
                    }
                } else if span.style.highlight {
                    for segment in range_segment_bounds(
                        &lines,
                        text_bounds,
                        line_height,
                        text,
                        span.range.clone(),
                        text_align,
                    ) {
                        let quad_bounds = Bounds::from_corners(
                            point(segment.left() - px(2.0), segment.top() - pad_y),
                            point(segment.right() + px(2.0), segment.bottom() + pad_y),
                        );
                        code_quads.push({
                            let mut q = fill(quad_bounds, highlight_color);
                            q.corner_radii = Corners::all(radius);
                            q
                        });
                    }
                }
            }

            // Search match background highlights with rounded corners and active borders
            for (match_range, is_active) in &input.search_matches {
                if match_range.is_empty() {
                    continue;
                }
                let bg_color = if *is_active {
                    theme.colors.app_menu_active.opacity(0.65)
                } else {
                    theme.colors.app_menu_active.opacity(0.25)
                };
                for segment in range_segment_bounds(
                    &lines,
                    text_bounds,
                    line_height,
                    text,
                    match_range.clone(),
                    text_align,
                ) {
                    let quad_bounds = Bounds::from_corners(
                        point(segment.left() - px(1.0), segment.top() - pad_y),
                        point(segment.right() + px(1.0), segment.bottom() + pad_y),
                    );
                    code_quads.push({
                        let mut q = fill(quad_bounds, bg_color);
                        q.corner_radii = Corners::all(radius);
                        if *is_active {
                            q.border_color = theme.colors.app_menu_active;
                            q.border_widths = Edges::all(px(1.5));
                        }
                        q
                    });
                }
            }
        }

        PrepaintState {
            lines,
            source_line_numbers,
            source_line_number_gutter_width,
            cursor: cursor_quad,
            selection: selection_quads,
            code_backgrounds: code_quads,
            line_height,
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
        let (focus_handle, hovering_link) = {
            let input = self.input.read(cx);
            let text_bounds = source_text_bounds(bounds, prepaint.source_line_number_gutter_width);
            let hovering_link = !self.is_placeholder
                && !input.is_verbatim_mode()
                && prepaint.hitbox.is_hovered(window)
                && link_at_position(
                    input,
                    &prepaint.lines,
                    text_bounds,
                    prepaint.line_height,
                    window.mouse_position(),
                )
                .is_some();
            (input.focus_handle.clone(), hovering_link)
        };

        if hovering_link {
            // The hand cursor only appears while the Cmd/Ctrl follow modifier is
            // held (matching the gesture that opens the link); a plain hover keeps
            // the text cursor. The editor root repaints on follow-modifier
            // toggles, so this re-evaluates even when the pointer stays still.
            if window.modifiers().secondary() {
                window.set_cursor_style(CursorStyle::PointingHand, &prepaint.hitbox);
            }
        }

        if focus_handle.is_focused(window) {
            let text_bounds = source_text_bounds(bounds, prepaint.source_line_number_gutter_width);
            window.handle_input(
                &focus_handle,
                ElementInputHandler::new(text_bounds, self.input.clone()),
                cx,
            );
        }

        // Paint code backgrounds behind text.
        for code_bg in prepaint.code_backgrounds.drain(..) {
            window.paint_quad(code_bg);
        }

        for selection in prepaint.selection.drain(..) {
            window.paint_quad(selection);
        }

        let line_height = prepaint.line_height;
        let lines = std::mem::take(&mut prepaint.lines);
        let text_align = self.input.read(cx).text_align();
        let text_bounds = source_text_bounds(bounds, prepaint.source_line_number_gutter_width);
        let line_number_tops = source_line_number_tops(&lines, line_height);
        let line_number_gap = px(SOURCE_LINE_NUMBER_GAP);
        let line_numbers = std::mem::take(&mut prepaint.source_line_numbers);
        for (line_number, y_offset) in line_numbers.iter().zip(line_number_tops.iter()) {
            let line_number_width = line_number.x_for_index(line_number.len());
            line_number
                .paint(
                    point(
                        text_bounds.left() - line_number_gap - line_number_width,
                        bounds.origin.y + *y_offset,
                    ),
                    line_height,
                    TextAlign::Left,
                    None,
                    window,
                    cx,
                )
                .ok();
        }

        let mut y_offset = Pixels::default();
        for line in &lines {
            let origin_x = aligned_line_left(line, text_bounds, text_align);
            line.paint(
                point(origin_x, text_bounds.origin.y + y_offset),
                line_height,
                TextAlign::Left,
                None,
                window,
                cx,
            )
            .ok();
            y_offset += wrapped_line_height(line, line_height);
        }

        if focus_handle.is_focused(window)
            && let Some(cursor) = prepaint.cursor.take()
        {
            window.paint_quad(cursor);
        }

        self.input.update(cx, |input, _cx| {
            input.push_last_paint(text_bounds, lines, line_height);
        });
    }
}
