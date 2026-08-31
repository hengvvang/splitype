use core_contracts::{SearchActiveField, SearchHost, SearchIme, SearchStateView};
use gpui::*;
use std::sync::Arc;
use theme::ThemeManager;

pub struct SearchInputPrepaintState {
    line: Option<ShapedLine>,
    selection: Option<PaintQuad>,
    cursor: Option<PaintQuad>,
    hitbox: Option<Hitbox>,
}

pub struct SearchInputElement {
    pub(crate) view: Arc<dyn SearchStateView>,
    pub(crate) ime: Arc<dyn SearchIme>,
    pub(crate) host: Arc<dyn SearchHost>,
    pub(crate) field: SearchActiveField,
    pub(crate) placeholder: SharedString,
}

impl IntoElement for SearchInputElement {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for SearchInputElement {
    type RequestLayoutState = ();
    type PrepaintState = SearchInputPrepaintState;

    fn id(&self) -> Option<ElementId> {
        match self.field {
            SearchActiveField::Query => Some(ElementId::Name("search-query-input".into())),
            SearchActiveField::Replace => Some(ElementId::Name("search-replace-input".into())),
        }
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
        style.size.height = px(22.0).max(window.line_height()).into();
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
        let snap = self.view.snapshot(self.field, cx);
        let (text, marked_range, selection_range, cursor_offset, is_focused) = (
            snap.text,
            snap.marked_range,
            snap.selection_range,
            snap.cursor_offset,
            snap.focus_handle
                .as_ref()
                .is_some_and(|h| h.is_focused(window)),
        );

        self.host.set_input_last_bounds(self.field, bounds, cx);

        let display_text: SharedString = if text.is_empty() && !is_focused {
            self.placeholder.clone()
        } else {
            text.clone().into()
        };

        let is_placeholder = text.is_empty() && !is_focused;
        let text_color = if is_placeholder {
            theme.colors.dialog_muted
        } else {
            theme.colors.text_default
        };

        let base_run = TextRun {
            len: display_text.len(),
            font: window.text_style().font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        let runs = if !is_placeholder {
            if let Some(marked_range) = marked_range.as_ref().filter(|_| !text.is_empty()) {
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
            }
        } else {
            vec![base_run]
        };

        let font_size = px(12.0);
        let line = window
            .text_system()
            .shape_line(display_text, font_size, &runs, None);
        let line_height = bounds.size.height;

        let selection = if is_focused && !selection_range.is_empty() && !is_placeholder {
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

        let cursor = if is_focused && selection_range.is_empty() {
            let cursor_x = if is_placeholder {
                px(0.0)
            } else {
                line.x_for_index(cursor_offset)
            };
            let mut cursor_color = theme.colors.cursor;
            cursor_color.a = 1.0;
            Some(fill(
                Bounds::new(
                    point(bounds.left() + cursor_x, bounds.top() + px(2.0)),
                    size(px(theme.dimensions.cursor_width), line_height - px(4.0)),
                ),
                cursor_color,
            ))
        } else {
            None
        };

        let hitbox = Some(window.insert_hitbox(bounds, HitboxBehavior::Normal));

        SearchInputPrepaintState {
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

        let focus_handle = self.view.snapshot(self.field, cx).focus_handle;

        if let Some(ref focus_handle) = focus_handle {
            if focus_handle.is_focused(window) {
                self.ime.handle_input(self.field, bounds, window, cx);
            }
        }

        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection);
        }

        if let Some(line) = prepaint.line.take() {
            line.paint(
                bounds.origin,
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
    }
}
