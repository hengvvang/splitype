//! BlockTextElement — GPUI Element for styled inline text.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::*;

pub use super::code_input::CodeLanguageInputElement;
use super::shaping::{build_code_text_runs, build_text_runs, unrounded_line_height};
use crate::editor::geometry::text_layout::*;
use crate::editor::document::block::Block;
use crate::infra::theme::ThemeManager;

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

#[cfg(test)]
mod tests {
    use super::{
        link_at_position, source_line_number_gutter_width, source_line_number_tops,
        source_text_bounds, wrapped_line_height,
    };
    use crate::editor::document::block::Block;
    use crate::model::block::table::TableCellPosition;
    use crate::model::inline::text::BlockText;
    use crate::model::parse::{BlockData, BlockKind};
    use gpui::{
        AppContext, Bounds, Hsla, Modifiers, MouseButton, MouseDownEvent, SharedString,
        TestAppContext, TextAlign, TextRun, VisualTestContext, font, point, px, rgba, size,
    };

    fn shaped_lines(
        text: &str,
        width: gpui::Pixels,
        cx: &mut VisualTestContext,
    ) -> Vec<gpui::WrappedLine> {
        cx.update(|window, _app| {
            window
                .text_system()
                .shape_text(
                    text.to_string().into(),
                    px(16.0),
                    &[TextRun {
                        len: text.len(),
                        font: font(".SystemUIFont"),
                        color: Hsla::from(rgba(0xffffffff)),
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    }],
                    Some(width),
                    None,
                )
                .expect("text should shape")
                .into_vec()
        })
    }

    #[test]
    fn source_line_number_gutter_grows_with_digit_count() {
        let one_digit = source_line_number_gutter_width(9, px(16.0));
        let two_digits = source_line_number_gutter_width(10, px(16.0));
        let three_digits = source_line_number_gutter_width(100, px(16.0));

        assert_eq!(one_digit, two_digits);
        assert!(three_digits > two_digits);
    }

    #[test]
    fn source_text_bounds_are_offset_by_gutter_width() {
        let bounds = Bounds::new(point(px(10.0), px(20.0)), size(px(300.0), px(120.0)));
        let text_bounds = source_text_bounds(bounds, px(48.0));

        assert_eq!(text_bounds.left(), px(58.0));
        assert_eq!(text_bounds.top(), px(20.0));
        assert_eq!(text_bounds.size.width, px(252.0));
        assert_eq!(text_bounds.size.height, px(120.0));
    }

    #[gpui::test]
    async fn source_line_number_tops_follow_soft_wrapped_hard_lines(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let lines = shaped_lines(
            "this line should wrap before the next hard line\nsecond",
            px(92.0),
            cx,
        );
        assert!(
            !lines[0].wrap_boundaries().is_empty(),
            "first hard line should soft-wrap"
        );

        let tops = source_line_number_tops(&lines, px(20.0));
        assert_eq!(tops[0], px(0.0));
        assert_eq!(tops[1], wrapped_line_height(&lines[0], px(20.0)));
    }

    #[gpui::test]
    async fn link_hit_matches_only_rendered_link_text(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let block = cx.new(|cx| {
            Block::with_data(
                cx,
                BlockData::new(
                    BlockKind::Paragraph,
                    BlockText::from_markdown("[link](https://example.com)"),
                ),
            )
        });

        let display_text = block.read_with(cx, |block, _cx| block.display_text().to_string());
        let lines = shaped_lines(&display_text, px(320.0), cx);
        let (hit, miss_right) = block.read_with(cx, |block, _cx| {
            let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(320.0), px(20.0)));
            let span = block
                .inline_spans()
                .iter()
                .find(|span| span.link.is_some())
                .expect("link span should exist");
            let layout = &lines[0];
            let start = layout
                .position_for_index(span.range.start, px(20.0))
                .expect("start position");
            let end = layout
                .position_for_index(span.range.end, px(20.0))
                .expect("end position");
            let hit = point((start.x + end.x) / 2.0, px(10.0));
            let miss_right = point(end.x + px(24.0), px(10.0));
            (
                link_at_position(block, &lines, bounds, px(20.0), hit)
                    .map(|link| link.open_target.clone()),
                link_at_position(block, &lines, bounds, px(20.0), miss_right)
                    .map(|link| link.open_target.clone()),
            )
        });

        assert_eq!(hit, Some("https://example.com".to_string()));
        assert_eq!(miss_right, None);
    }

    #[gpui::test]
    async fn secondary_click_follows_link_while_plain_click_edits(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let block = cx.new(|cx| {
            Block::with_data(
                cx,
                BlockData::new(
                    BlockKind::Paragraph,
                    BlockText::from_markdown("a [link](https://example.com) bbbb"),
                ),
            )
        });

        let display_text = block.read_with(cx, |block, _cx| block.display_text().to_string());
        let lines = shaped_lines(&display_text, px(320.0), cx);
        let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(320.0), px(20.0)));

        let link_position = block.read_with(cx, |block, _cx| {
            let span = block
                .inline_spans()
                .iter()
                .find(|span| span.link.is_some())
                .expect("link span should exist");
            let layout = &lines[0];
            let start = layout
                .position_for_index(span.range.start, px(20.0))
                .expect("start position");
            let end = layout
                .position_for_index(span.range.end, px(20.0))
                .expect("end position");
            point((start.x + end.x) / 2.0, px(10.0))
        });

        block.update(cx, |block, _cx| {
            block.push_last_paint(bounds, lines, px(20.0));
            block.selected_range = 0..0;
        });

        let mut event = MouseDownEvent {
            button: MouseButton::Left,
            position: link_position,
            modifiers: Modifiers::default(),
            click_count: 1,
            first_mouse: false,
        };

        // A plain click on the link moves the caret into the text for editing.
        cx.update(|window, app| {
            block.update(app, |block, cx| block.on_mouse_down(&event, window, cx));
        });
        block.read_with(cx, |block, _cx| {
            assert_ne!(block.selected_range, 0..0);
        });

        // Cmd/Ctrl+click follows the link instead: the caret is left untouched
        // and no drag-selection begins.
        block.update(cx, |block, _cx| block.selected_range = 0..0);
        event.modifiers = Modifiers::secondary_key();
        cx.update(|window, app| {
            block.update(app, |block, cx| block.on_mouse_down(&event, window, cx));
        });
        block.read_with(cx, |block, _cx| {
            assert_eq!(block.selected_range, 0..0);
            assert!(!block.is_selecting);
        });
    }

    #[gpui::test]
    async fn link_hit_respects_center_alignment(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let block = cx.new(|cx| {
            let mut block = Block::with_data(
                cx,
                BlockData::new(
                    BlockKind::Paragraph,
                    BlockText::from_markdown("[link](https://example.com)"),
                ),
            );
            block.set_table_cell_mode(
                TableCellPosition { row: 0, column: 0 },
                crate::model::block::table::TableColumnAlignment::Center,
            );
            block
        });

        let display_text = block.read_with(cx, |block, _cx| block.display_text().to_string());
        let lines = shaped_lines(&display_text, px(240.0), cx);
        let (miss_left, hit_center) = block.read_with(cx, |block, _cx| {
            let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(240.0), px(20.0)));
            let span = block
                .inline_spans()
                .iter()
                .find(|span| span.link.is_some())
                .expect("link span should exist");
            let layout = &lines[0];
            let origin_x = super::aligned_line_left(layout, bounds, block.text_align());
            let start = layout
                .position_for_index(span.range.start, px(20.0))
                .expect("start position");
            let end = layout
                .position_for_index(span.range.end, px(20.0))
                .expect("end position");
            let miss_left = point(origin_x - px(12.0), px(10.0));
            let hit_center = point(origin_x + (start.x + end.x) / 2.0, px(10.0));
            (
                link_at_position(block, &lines, bounds, px(20.0), miss_left)
                    .map(|link| link.open_target.clone()),
                link_at_position(block, &lines, bounds, px(20.0), hit_center)
                    .map(|link| link.open_target.clone()),
            )
        });

        assert_eq!(miss_left, None);
        assert_eq!(hit_center, Some("https://example.com".to_string()));
    }

    #[gpui::test]
    async fn text_runs_apply_inline_html_color_and_background(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let block = cx.new(|cx| {
            Block::with_data(
                cx,
                BlockData::new(
                    BlockKind::Paragraph,
                    BlockText::from_markdown(
                        "before <span style='color:blue;background-color:#ff0'>marked</span>",
                    ),
                ),
            )
        });

        block.read_with(cx, |block, _cx| {
            let display_text: SharedString = block.display_text().to_string().into();
            let base_run = TextRun {
                len: display_text.len(),
                font: font(".SystemUIFont"),
                color: Hsla::from(rgba(0xffffffff)),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let runs = super::build_text_runs(
                block,
                &display_text,
                &base_run,
                px(1.0),
                Hsla::from(rgba(0x0066ccff)),
                Hsla::from(rgba(0x00ff88ff)),
                Hsla::from(rgba(0x9aa5ceff)),
                Hsla::from(rgba(0xffff00ff)),
            );
            let marked_run = runs.last().expect("styled text should create a final run");

            assert_eq!(block.display_text(), "before marked");
            assert_eq!(marked_run.len, "marked".len());
            assert_eq!(marked_run.color, Hsla::from(rgba(0x0000ffff)));
            assert_eq!(
                marked_run.background_color,
                Some(Hsla::from(rgba(0xffff00ff)))
            );
        });
    }

    #[gpui::test]
    async fn projected_delimiter_markers_render_in_marker_color(cx: &mut TestAppContext) {
        let block = cx.new(|cx| {
            Block::with_data(
                cx,
                BlockData::new(
                    BlockKind::Paragraph,
                    BlockText::from_markdown("x^2^ 与[^note]"),
                ),
            )
        });
        block.update(cx, |block, _cx| {
            let len = block.display_len();
            block.selected_range = 0..len;
            block.rebuild_inline_projection(0..len, None);
        });

        block.read_with(cx, |block, _cx| {
            let display_text: SharedString = block.display_text().to_string().into();
            let base_run = TextRun {
                len: display_text.len(),
                font: font(".SystemUIFont"),
                color: Hsla::from(rgba(0xffffffff)),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let marker_color = Hsla::from(rgba(0x00ff88ff));
            let runs = super::build_text_runs(
                block,
                &display_text,
                &base_run,
                px(1.0),
                Hsla::from(rgba(0x0066ccff)),
                marker_color,
                Hsla::from(rgba(0x9aa5ceff)),
                Hsla::from(rgba(0xffff00ff)),
            );

            let mut offset = 0usize;
            let mut saw_marker = false;
            for run in runs {
                let segment = &display_text[offset..offset + run.len];
                if matches!(segment, "^" | "[^" | "]") {
                    assert_eq!(run.color, marker_color, "marker {segment:?} not colored");
                    saw_marker = true;
                }
                offset += run.len;
            }
            assert!(saw_marker, "no delimiter markers found in {display_text:?}");
        });
    }

    #[gpui::test]
    async fn footnote_reference_id_renders_in_footnote_color(cx: &mut TestAppContext) {
        let block = cx.new(|cx| {
            Block::with_data(
                cx,
                BlockData::new(
                    BlockKind::Paragraph,
                    BlockText::from_markdown("引用[^note]"),
                ),
            )
        });
        block.update(cx, |block, _cx| {
            let len = block.display_len();
            block.selected_range = 0..len;
            block.rebuild_inline_projection(0..len, None);
        });

        block.read_with(cx, |block, _cx| {
            let display_text: SharedString = block.display_text().to_string().into();
            assert_eq!(display_text.as_ref(), "引用[^note]");
            let base_run = TextRun {
                len: display_text.len(),
                font: font(".SystemUIFont"),
                color: Hsla::from(rgba(0xffffffff)),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let marker_color = Hsla::from(rgba(0x00ff88ff));
            let footnote_color = Hsla::from(rgba(0x9aa5ceff));
            let highlight_color = Hsla::from(rgba(0xffd1664d));
            let runs = super::build_text_runs(
                block,
                &display_text,
                &base_run,
                px(1.0),
                Hsla::from(rgba(0x0066ccff)),
                marker_color,
                footnote_color,
                highlight_color,
            );

            let mut offset = 0usize;
            let mut saw_id = false;
            for run in runs {
                let segment = &display_text[offset..offset + run.len];
                if segment == "note" {
                    assert_eq!(
                        run.color, footnote_color,
                        "footnote id should share the definition head color"
                    );
                    saw_id = true;
                } else if matches!(segment, "[^" | "]") {
                    assert_eq!(run.color, marker_color);
                }
                offset += run.len;
            }
            assert!(saw_id, "footnote id not found in {display_text:?}");
        });
    }

    #[gpui::test]
    async fn inline_latex_delimiters_render_in_marker_color(cx: &mut TestAppContext) {
        let block = cx.new(|cx| {
            Block::with_data(
                cx,
                BlockData::new(
                    BlockKind::Paragraph,
                    BlockText::from_markdown("公式 $x^2+y^2$ 和 \\(a+b\\) 结束"),
                ),
            )
        });

        block.read_with(cx, |block, _cx| {
            let display_text: SharedString = block.display_text().to_string().into();
            let base_run = TextRun {
                len: display_text.len(),
                font: font(".SystemUIFont"),
                color: Hsla::from(rgba(0xffffffff)),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let marker_color = Hsla::from(rgba(0xa855f7ff)); // purple
            let footnote_color = Hsla::from(rgba(0x9aa5ceff));
            let highlight_color = Hsla::from(rgba(0xffd1664d));
            let runs = super::build_text_runs(
                block,
                &display_text,
                &base_run,
                px(1.0),
                Hsla::from(rgba(0x0066ccff)),
                marker_color,
                footnote_color,
                highlight_color,
            );

            let mut offset = 0usize;
            let mut saw_dollar = 0usize;
            let mut saw_paren = 0usize;
            for run in runs {
                let segment = &display_text[offset..offset + run.len];
                if segment == "$" {
                    assert_eq!(run.color, marker_color, "dollar delimiter should be purple");
                    saw_dollar += 1;
                } else if segment == "\\(" || segment == "\\)" {
                    assert_eq!(run.color, marker_color, "paren delimiter should be purple");
                    saw_paren += 1;
                } else if segment == "x^2+y^2" || segment == "a+b" {
                    assert_eq!(run.color, base_run.color, "math body should be base color");
                }
                offset += run.len;
            }
            assert_eq!(saw_dollar, 2, "both dollar delimiters should be colored");
            assert_eq!(saw_paren, 2, "both paren delimiters should be colored");
        });
    }

    #[gpui::test]
    async fn soft_wrapped_range_segments_stay_within_wrap_width(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let text = "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz";
        let lines = shaped_lines(text, px(80.0), cx);
        assert!(
            !lines[0].wrap_boundaries().is_empty(),
            "test text should soft-wrap"
        );

        let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(80.0), px(120.0)));
        let segments = super::range_segment_bounds(
            &lines,
            bounds,
            px(20.0),
            text,
            0..text.len(),
            TextAlign::Left,
        );

        assert!(segments.len() > 1);
        for segment in segments {
            assert!(segment.left() >= bounds.left());
            assert!(segment.right() <= bounds.right() + px(0.5));
        }
    }

    #[gpui::test]
    async fn wrapped_link_hit_matches_only_visible_segments(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let label = "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz";
        let block = cx.new(|cx| {
            Block::with_data(
                cx,
                BlockData::new(
                    BlockKind::Paragraph,
                    BlockText::from_markdown(&format!("[{label}](https://example.com)")),
                ),
            )
        });

        let display_text = block.read_with(cx, |block, _cx| block.display_text().to_string());
        let lines = shaped_lines(&display_text, px(80.0), cx);
        assert!(
            !lines[0].wrap_boundaries().is_empty(),
            "link text should soft-wrap"
        );

        let (hit, miss_right) = block.read_with(cx, |block, _cx| {
            let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(80.0), px(120.0)));
            let span = block
                .inline_spans()
                .iter()
                .find(|span| span.link.is_some())
                .expect("link span should exist");
            let segments = super::range_segment_bounds(
                &lines,
                bounds,
                px(20.0),
                &display_text,
                span.range.clone(),
                block.text_align(),
            );
            assert!(segments.len() > 1);
            let second_segment = segments[1];
            let hit = point(
                (second_segment.left() + second_segment.right()) / 2.0,
                (second_segment.top() + second_segment.bottom()) / 2.0,
            );
            let miss_right = point(second_segment.right() + px(24.0), hit.y);
            (
                link_at_position(block, &lines, bounds, px(20.0), hit)
                    .map(|link| link.open_target.clone()),
                link_at_position(block, &lines, bounds, px(20.0), miss_right)
                    .map(|link| link.open_target.clone()),
            )
        });

        assert_eq!(hit, Some("https://example.com".to_string()));
        assert_eq!(miss_right, None);
    }

    #[gpui::test]
    async fn wrapped_hard_line_top_accumulates_soft_wrap_height(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let text = "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz\nnext";
        let lines = shaped_lines(text, px(80.0), cx);
        assert_eq!(lines.len(), 2);
        assert!(
            !lines[0].wrap_boundaries().is_empty(),
            "first hard line should soft-wrap"
        );

        let first_height = lines[0].size(px(20.0)).height;
        assert!(first_height > px(20.0));
        assert_eq!(super::wrapped_line_top(&lines, px(20.0), 1), first_height);
    }
}
