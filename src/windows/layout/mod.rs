//! Tiled pane layout — splitting, resizing, and rearranging views.
//!
//! The layout state model lives in `editor::window::layout`; this module
//! renders the split tree and hosts the drag/resize interactions.

use gpui::*;

use crate::editor::controller::*;
use crate::editor::window::layout::{
    Axis, BorderMenuState, CornerDragAction, CornerDragModifier, CornerDragPreview, Direction,
    EditTabState, EditorPanel, PaneKind, SplitTree, SplitterDragSession,
};
use crate::infra::i18n::I18nStrings;
use crate::theme::Theme;
use crate::windows::editor::render_empty_panel_prompt;

impl Editor {
    pub(crate) fn render_tiled_layout(
        &mut self,
        content_area: AnyElement,
        theme: &Theme,
        strings: &I18nStrings,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let root = self.panels.layout.root.clone();
        let leaf_count = root.count_leaves();
        let mut primary_content = Some(content_area);

        let layout_tree = if let Some(maximized_id) = self.panels.layout.maximized_leaf {
            if let Some(area_type) = root.find_leaf_area(maximized_id) {
                self.render_area_tile(
                    maximized_id,
                    area_type,
                    &mut primary_content,
                    theme,
                    strings,
                    leaf_count,
                    true,
                    cx,
                )
            } else {
                self.render_tiled_layout_node(
                    &root,
                    &mut primary_content,
                    theme,
                    strings,
                    leaf_count,
                    cx,
                )
            }
        } else {
            self.render_tiled_layout_node(
                &root,
                &mut primary_content,
                theme,
                strings,
                leaf_count,
                cx,
            )
        };

        let root_editor_move = cx.entity().downgrade();
        let root_editor_up = cx.entity().downgrade();

        let container = div()
            .id("tiled-layout-root")
            .w_full()
            .h_full()
            .flex_1()
            .min_w(px(0.0))
            .min_h(px(0.0))
            .relative()
            .on_mouse_move(move |event, window, cx| {
                let pos = event.position;
                let _ = root_editor_move.update(cx, |ed, cx| {
                    let mut changed = false;
                    if let Some(drag) = ed.panels.layout.active_splitter_drag {
                        let current_pos = match drag.direction {
                            Axis::Horizontal => f32::from(pos.x),
                            Axis::Vertical => f32::from(pos.y),
                        };
                        let viewport = window.viewport_size();
                        let span = ed
                            .panels
                            .layout
                            .get_split_pixel_span(drag.split_id, viewport)
                            .unwrap_or_else(|| match drag.direction {
                                Axis::Horizontal => f32::from(viewport.width),
                                Axis::Vertical => f32::from(viewport.height),
                            });

                        if span > 1.0 {
                            let mut session = drag;
                            session.total_span = span;
                            ed.panels.layout.active_splitter_drag = Some(session);
                        }
                        ed.panels.layout.update_splitter_drag(current_pos);
                        changed = true;
                    } else if ed.panels.layout.active_corner_drag.is_some() {
                        let viewport = window.viewport_size();
                        let action = ed.panels.layout.update_corner_drag(pos, viewport);
                        // Modifier actions still execute immediately.
                        if let Some(action) = action {
                            match action {
                                CornerDragAction::Swap { from, to } => {
                                    ed.panels.layout.end_corner_drag();
                                    ed.panels.layout.swap_area_types(from, to);
                                }
                                CornerDragAction::Duplicate { .. } => {
                                    ed.panels.layout.end_corner_drag();
                                }
                                CornerDragAction::Cancel => {
                                    ed.panels.layout.end_corner_drag();
                                }
                                _ => {} // Split/Join handled on mouse_up
                            }
                        }
                        changed = true;
                    } else if ed.panels.layout.active_inner_splitter_drag.is_some() {
                        let (container_id, drag) =
                            ed.panels.layout.active_inner_splitter_drag.unwrap();
                        let viewport = window.viewport_size();
                        let outer_rects = ed.panels.layout.collect_leaf_rects(viewport);
                        if let Some((_eid, ex, ey, ew, eh)) = ed
                            .panels
                            .layout
                            .get_leaf_pixel_rect(container_id, &outer_rects)
                        {
                            let current_pos = match drag.direction {
                                Axis::Horizontal => f32::from(pos.x) - ex,
                                Axis::Vertical => f32::from(pos.y) - ey,
                            };
                            let inner_size = size(px(ew), px(eh));
                            let span = ed
                                .panels
                                .layout
                                .get_inner_split_pixel_span(container_id, drag.split_id, inner_size)
                                .unwrap_or_else(|| match drag.direction {
                                    Axis::Horizontal => ew,
                                    Axis::Vertical => eh,
                                });
                            if span > 1.0 {
                                let mut session = drag;
                                session.total_span = span;
                                ed.panels.layout.active_inner_splitter_drag =
                                    Some((container_id, session));
                            }
                            ed.panels
                                .layout
                                .update_inner_splitter_drag(container_id, current_pos);
                            changed = true;
                        }
                    } else if ed.panels.layout.active_inner_corner_drag.is_some() {
                        let (container_id, drag) =
                            ed.panels.layout.active_inner_corner_drag.unwrap();
                        let viewport = window.viewport_size();
                        let outer_rects = ed.panels.layout.collect_leaf_rects(viewport);
                        if let Some((_eid, ex, ey, ew, eh)) = ed
                            .panels
                            .layout
                            .get_leaf_pixel_rect(container_id, &outer_rects)
                        {
                            let mut session = drag;
                            let inner_pos =
                                point(px(f32::from(pos.x) - ex), px(f32::from(pos.y) - ey));
                            let inner_size = size(px(ew), px(eh));
                            let start_x = f32::from(session.start_pos.x);
                            let start_y = f32::from(session.start_pos.y);
                            if start_x > ew || start_y > eh {
                                session.start_pos = point(px(start_x - ex), px(start_y - ey));
                                ed.panels.layout.active_inner_corner_drag =
                                    Some((container_id, session));
                            }
                            let action = ed.panels.layout.update_inner_corner_drag(
                                container_id,
                                inner_pos,
                                inner_size,
                            );
                            if let Some(action) = action {
                                match action {
                                    CornerDragAction::Swap { from, to } => {
                                        ed.panels.layout.end_inner_corner_drag();
                                        ed.panels.layout.inner_swap_area_types(
                                            container_id,
                                            from,
                                            to,
                                        );
                                    }
                                    CornerDragAction::Duplicate { .. } => {
                                        ed.panels.layout.end_inner_corner_drag();
                                    }
                                    CornerDragAction::Cancel => {
                                        ed.panels.layout.end_inner_corner_drag();
                                    }
                                    _ => {}
                                }
                            }
                            changed = true;
                        }
                    }
                    if changed {
                        cx.notify();
                    }
                });
            })
            .on_mouse_up(MouseButton::Left, move |_event, _window, cx| {
                let _ = root_editor_up.update(cx, |ed, cx| {
                    if ed.panels.layout.active_splitter_drag.is_some() {
                        ed.panels.layout.end_splitter_drag();
                        cx.notify();
                    }
                    if ed.panels.layout.active_corner_drag.is_some() {
                        match ed.panels.layout.finish_corner_drag() {
                            Some(CornerDragAction::Split {
                                leaf_id,
                                direction,
                                ratio,
                            }) => {
                                ed.panels
                                    .layout
                                    .split_area_with_ratio(leaf_id, direction, ratio);
                            }
                            Some(CornerDragAction::Join { from, into }) => {
                                ed.panels.layout.join_area(into, from);
                            }
                            Some(CornerDragAction::Swap { from, to }) => {
                                ed.panels.layout.swap_area_types(from, to);
                            }
                            _ => {}
                        }
                        cx.notify();
                    }
                    if ed.panels.layout.active_inner_splitter_drag.is_some() {
                        ed.panels.layout.end_inner_splitter_drag();
                        cx.notify();
                    }
                    if ed.panels.layout.active_inner_corner_drag.is_some() {
                        match ed.panels.layout.finish_inner_corner_drag() {
                            Some((
                                container_id,
                                CornerDragAction::Split {
                                    leaf_id,
                                    direction,
                                    ratio,
                                },
                            )) => {
                                ed.panels.layout.inner_split_area_with_ratio(
                                    container_id,
                                    leaf_id,
                                    direction,
                                    ratio,
                                );
                            }
                            Some((container_id, CornerDragAction::Join { from, into })) => {
                                ed.panels.layout.inner_join_area(container_id, into, from);
                            }
                            _ => {}
                        }
                        cx.notify();
                    }
                });
            })
            .child(layout_tree);

        // Build the preview overlay for corner drag gestures.
        let preview_overlay = if let Some(drag) = &self.panels.layout.active_corner_drag {
            match drag.preview {
                CornerDragPreview::SplitPreview { direction, ratio } => {
                    // Calculate the pixel rect of the leaf being split.
                    let viewport = _window.viewport_size();
                    let leaf_rects = self.panels.layout.collect_leaf_rects(viewport);
                    let leaf_rect = self
                        .panels
                        .layout
                        .get_leaf_pixel_rect(drag.leaf_id, &leaf_rects);

                    if let Some((_, lx, ly, lw, lh)) = leaf_rect {
                        // Horizontal split = left|right → draw a VERTICAL line
                        // Vertical split = top|bottom → draw a HORIZONTAL line
                        let line = match direction {
                            Axis::Horizontal => div()
                                .absolute()
                                .left(px(lx + lw * ratio))
                                .top(px(ly))
                                .w(px(3.0))
                                .h(px(lh))
                                .bg(hsla(0.36, 0.73, 0.57, 0.8)),
                            Axis::Vertical => div()
                                .absolute()
                                .top(px(ly + lh * ratio))
                                .left(px(lx))
                                .h(px(3.0))
                                .w(px(lw))
                                .bg(hsla(0.36, 0.73, 0.57, 0.8)),
                        };

                        // Also draw a semi-transparent highlight over the leaf
                        Some(
                            div()
                                .absolute()
                                .inset(px(0.0))
                                .child(
                                    div()
                                        .absolute()
                                        .left(px(lx))
                                        .top(px(ly))
                                        .w(px(lw))
                                        .h(px(lh))
                                        .rounded(px(theme.dimensions.area_tile_radius))
                                        .bg(hsla(0.36, 0.73, 0.57, 0.08)),
                                )
                                .child(line),
                        )
                    } else {
                        None
                    }
                }
                CornerDragPreview::JoinPreview {
                    target_leaf_id,
                    direction,
                } => {
                    let viewport = _window.viewport_size();
                    let leaf_rects = self.panels.layout.collect_leaf_rects(viewport);
                    let target_rect = self
                        .panels
                        .layout
                        .get_leaf_pixel_rect(target_leaf_id, &leaf_rects);

                    if let Some((_, rx, ry, rw, rh)) = target_rect {
                        let arrow_symbol = match direction {
                            Direction::Up => "▲",
                            Direction::Down => "▼",
                            Direction::Right => "▶",
                            Direction::Left => "◀",
                        };

                        Some(
                            div()
                                .absolute()
                                .left(px(rx))
                                .top(px(ry))
                                .w(px(rw))
                                .h(px(rh))
                                .rounded(px(theme.dimensions.area_tile_radius))
                                .bg(hsla(0.36, 0.73, 0.57, 0.25))
                                .border(px(2.0))
                                .border_color(hsla(0.36, 0.73, 0.57, 0.8))
                                .flex()
                                .flex_col()
                                .items_center()
                                .justify_center()
                                .child(
                                    div()
                                        .px(px(12.0))
                                        .py(px(6.0))
                                        .rounded_md()
                                        .bg(hsla(0.0, 0.0, 0.0, 0.75))
                                        .text_color(hsla(0.0, 0.0, 1.0, 0.95))
                                        .text_size(px(15.0))
                                        .font_weight(FontWeight::BOLD)
                                        .child(format!("{} Join Area", arrow_symbol)),
                                ),
                        )
                    } else {
                        Some(
                            div()
                                .absolute()
                                .inset(px(0.0))
                                .bg(hsla(0.36, 0.73, 0.57, 0.15)),
                        )
                    }
                }
                CornerDragPreview::Dragging => None,
            }
        } else {
            None
        };
        let container = container.children(preview_overlay);

        // INNER_PREVIEW_INSERT
        let inner_preview_overlay =
            if let Some((container_id, ref drag)) = self.panels.layout.active_inner_corner_drag {
                match drag.preview {
                    CornerDragPreview::SplitPreview { direction, ratio } => {
                        let viewport = _window.viewport_size();
                        let outer_rects = self.panels.layout.collect_leaf_rects(viewport);
                        if let Some((_eid, ex, ey, ew, eh)) = self
                            .panels
                            .layout
                            .get_leaf_pixel_rect(container_id, &outer_rects)
                        {
                            let line = match direction {
                                Axis::Horizontal => div()
                                    .absolute()
                                    .left(px(ex + ew * ratio))
                                    .top(px(ey))
                                    .w(px(3.0))
                                    .h(px(eh))
                                    .bg(hsla(0.36, 0.73, 0.57, 0.8)),
                                Axis::Vertical => div()
                                    .absolute()
                                    .top(px(ey + eh * ratio))
                                    .left(px(ex))
                                    .h(px(3.0))
                                    .w(px(ew))
                                    .bg(hsla(0.36, 0.73, 0.57, 0.8)),
                            };
                            Some(
                                div()
                                    .absolute()
                                    .inset(px(0.0))
                                    .child(
                                        div()
                                            .absolute()
                                            .left(px(ex))
                                            .top(px(ey))
                                            .w(px(ew))
                                            .h(px(eh))
                                            .rounded(px(theme.dimensions.area_tile_radius))
                                            .bg(hsla(0.36, 0.73, 0.57, 0.08)),
                                    )
                                    .child(line),
                            )
                        } else {
                            None
                        }
                    }
                    CornerDragPreview::JoinPreview {
                        target_leaf_id,
                        direction,
                    } => {
                        let viewport = _window.viewport_size();
                        let outer_rects = self.panels.layout.collect_leaf_rects(viewport);
                        if let Some((_eid, ex, ey, ew, eh)) = self
                            .panels
                            .layout
                            .get_leaf_pixel_rect(container_id, &outer_rects)
                        {
                            let inner_size = size(px(ew), px(eh));
                            let inner_rects = self
                                .panels
                                .layout
                                .collect_inner_leaf_rects(container_id, inner_size);
                            if let Some((_id, rx, ry, rw, rh)) = self
                                .panels
                                .layout
                                .get_leaf_pixel_rect(target_leaf_id, &inner_rects)
                            {
                                let arrow_symbol = match direction {
                                    Direction::Up => "N",
                                    Direction::Down => "S",
                                    Direction::Right => "E",
                                    Direction::Left => "W",
                                };
                                Some(
                                    div()
                                        .absolute()
                                        .left(px(ex + rx))
                                        .top(px(ey + ry))
                                        .w(px(rw))
                                        .h(px(rh))
                                        .rounded(px(theme.dimensions.area_tile_radius))
                                        .bg(hsla(0.36, 0.73, 0.57, 0.25))
                                        .border(px(2.0))
                                        .border_color(hsla(0.36, 0.73, 0.57, 0.8))
                                        .flex()
                                        .flex_col()
                                        .items_center()
                                        .justify_center()
                                        .child(
                                            div()
                                                .px(px(12.0))
                                                .py(px(6.0))
                                                .rounded_md()
                                                .bg(hsla(0.0, 0.0, 0.0, 0.75))
                                                .text_color(hsla(0.0, 0.0, 1.0, 0.95))
                                                .text_size(px(15.0))
                                                .font_weight(FontWeight::BOLD)
                                                .child(format!("{} Join Area", arrow_symbol)),
                                        ),
                                )
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    CornerDragPreview::Dragging => None,
                }
            } else {
                None
            };
        let container = container.children(inner_preview_overlay);

        if let Some(border_menu) = self.panels.layout.active_border_menu {
            let menu_overlay = self.render_border_context_menu_overlay(border_menu, theme, cx);
            container.child(menu_overlay).into_any_element()
        } else {
            container.into_any_element()
        }
    }
    pub(crate) fn render_tiled_layout_node(
        &mut self,
        node: &crate::editor::window::layout::SplitTree<crate::editor::window::layout::PaneKind>,
        primary_content: &mut Option<AnyElement>,
        theme: &Theme,
        strings: &I18nStrings,
        leaf_count: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let editor = cx.entity().downgrade();

        match node {
            SplitTree::Leaf { id, area_type } => self.render_area_tile(
                *id,
                *area_type,
                primary_content,
                theme,
                strings,
                leaf_count,
                false,
                cx,
            ),
            SplitTree::Split {
                id,
                direction,
                ratio,
                first,
                second,
            } => {
                let split_id = *id;
                let dir = *direction;
                let r = *ratio;

                let first_elem = self.render_tiled_layout_node(
                    first,
                    primary_content,
                    theme,
                    strings,
                    leaf_count,
                    cx,
                );
                let second_elem = self.render_tiled_layout_node(
                    second,
                    primary_content,
                    theme,
                    strings,
                    leaf_count,
                    cx,
                );

                match direction {
                    Axis::Horizontal => {
                        let bar_editor = editor.clone();
                        let menu_editor = editor.clone();

                        div()
                            .id(("tiled-split-h", split_id))
                            .w_full()
                            .h_full()
                            .flex()
                            .flex_row()
                            .min_w(px(0.0))
                            .min_h(px(0.0))
                            .child(
                                div()
                                    .w(relative(r))
                                    .h_full()
                                    .overflow_hidden()
                                    .flex()
                                    .flex_col()
                                    .flex_shrink_0()
                                    .min_w(px(0.0))
                                    .min_h(px(0.0))
                                    .child(first_elem),
                            )
                            .child(
                                // Splitter bar between the two padded tiles.
                                div()
                                    .id(("tiled-splitter-bar-h", split_id))
                                    .w(px(2.0))
                                    .h_full()
                                    .flex_shrink_0()
                                    .cursor_col_resize()
                                    .bg(c.dialog_border)
                                    .hover(|this| this.bg(c.selection))
                                    .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                                        let start_pos = f32::from(event.position.x);
                                        let _ = bar_editor.update(cx, |ed, cx| {
                                            ed.panels.layout.active_splitter_drag =
                                                Some(SplitterDragSession {
                                                    split_id,
                                                    direction: Axis::Horizontal,
                                                    start_pointer_pos: start_pos,
                                                    start_ratio: r,
                                                    total_span: 1000.0,
                                                });
                                            cx.notify();
                                        });
                                    })
                                    .on_mouse_down(
                                        MouseButton::Right,
                                        move |event, _window, cx| {
                                            let pos = event.position;
                                            let _ = menu_editor.update(cx, |ed, cx| {
                                                ed.panels.layout.active_border_menu =
                                                    Some(BorderMenuState {
                                                        split_id,
                                                        direction: dir,
                                                        position: pos,
                                                    });
                                                cx.notify();
                                            });
                                        },
                                    ),
                            )
                            .child(
                                div()
                                    .h_full()
                                    .overflow_hidden()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .min_h(px(0.0))
                                    .child(second_elem),
                            )
                            .into_any_element()
                    }
                    Axis::Vertical => {
                        let bar_editor = editor.clone();
                        let menu_editor = editor.clone();

                        div()
                            .id(("tiled-split-v", split_id))
                            .w_full()
                            .h_full()
                            .flex()
                            .flex_col()
                            .min_w(px(0.0))
                            .min_h(px(0.0))
                            .child(
                                div()
                                    .h(relative(r))
                                    .w_full()
                                    .overflow_hidden()
                                    .flex()
                                    .flex_col()
                                    .flex_shrink_0()
                                    .min_w(px(0.0))
                                    .min_h(px(0.0))
                                    .child(first_elem),
                            )
                            .child(
                                // Splitter bar between the two padded tiles.
                                div()
                                    .id(("tiled-splitter-bar-v", split_id))
                                    .h(px(2.0))
                                    .w_full()
                                    .flex_shrink_0()
                                    .cursor_row_resize()
                                    .bg(c.dialog_border)
                                    .hover(|this| this.bg(c.selection))
                                    .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                                        let start_pos = f32::from(event.position.y);
                                        let _ = bar_editor.update(cx, |ed, cx| {
                                            ed.panels.layout.active_splitter_drag =
                                                Some(SplitterDragSession {
                                                    split_id,
                                                    direction: Axis::Vertical,
                                                    start_pointer_pos: start_pos,
                                                    start_ratio: r,
                                                    total_span: 700.0,
                                                });
                                            cx.notify();
                                        });
                                    })
                                    .on_mouse_down(
                                        MouseButton::Right,
                                        move |event, _window, cx| {
                                            let pos = event.position;
                                            let _ = menu_editor.update(cx, |ed, cx| {
                                                ed.panels.layout.active_border_menu =
                                                    Some(BorderMenuState {
                                                        split_id,
                                                        direction: dir,
                                                        position: pos,
                                                    });
                                                cx.notify();
                                            });
                                        },
                                    ),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .overflow_hidden()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .min_h(px(0.0))
                                    .child(second_elem),
                            )
                            .into_any_element()
                    }
                }
            }
        }
    }
    pub(crate) fn render_area_tile(
        &mut self,
        leaf_id: usize,
        area_type: crate::editor::window::layout::PaneKind,
        primary_content: &mut Option<AnyElement>,
        theme: &Theme,
        strings: &I18nStrings,
        leaf_count: usize,
        is_maximized: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let gap = d.area_tile_gap;
        let radius = d.area_tile_radius;

        let header =
            self.render_area_header(leaf_id, area_type, theme, leaf_count, is_maximized, cx);

        let body: AnyElement = match area_type {
            PaneKind::Editor => {
                self.render_tiled_edit_container_panel(leaf_id, primary_content, theme, strings, cx)
            }
            PaneKind::Workspace => self.render_tiled_workspace_files_panel(theme, strings, cx),
            PaneKind::Settings => self.render_tiled_settings_panel(theme, strings, cx),
        };

        let panel_status_bar = match area_type {
            PaneKind::Editor => {
                Some(self.render_panel_status_bar(leaf_id, area_type, theme, strings, cx))
            }
            _ => None,
        };

        let body_container = div()
            .w_full()
            .flex_1()
            .min_h(px(0.0))
            .relative()
            .child(body);

        // Tile card with overflow hidden (no corner handles inside, to avoid clipping).
        let mut tile_card = div()
            .id(("tiled-area-card", leaf_id))
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .relative()
            .rounded(px(radius))
            .bg(c.dialog_surface)
            .border(px(d.dialog_border_width))
            .border_color(c.dialog_border)
            .shadow_lg()
            .child(header)
            .child(body_container);

        if let Some(sb) = panel_status_bar {
            tile_card = tile_card.child(sb);
        }

        // Corner drag handles positioned at the four outer corners of the tile card.
        let editor_corner = cx.entity().downgrade();
        let make_outer_corner = |id_str: &'static str, top: bool, left: bool| {
            let editor_corner = editor_corner.clone();
            let mut corner_div = div()
                .id((id_str, leaf_id))
                .absolute()
                .size(px(20.0))
                .cursor_crosshair();

            if top {
                corner_div = corner_div.top(px(gap));
            } else {
                corner_div = corner_div.bottom(px(gap));
            }
            if left {
                corner_div = corner_div.left(px(gap));
            } else {
                corner_div = corner_div.right(px(gap));
            }

            corner_div.on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                let pos = event.position;
                let modifier = if event.modifiers.control {
                    CornerDragModifier::Swap
                } else if event.modifiers.shift {
                    CornerDragModifier::Duplicate
                } else {
                    CornerDragModifier::None
                };
                let _ = editor_corner.update(cx, |ed, cx| {
                    ed.panels.layout.start_corner_drag(leaf_id, pos, modifier);
                    cx.notify();
                });
            })
        };

        let corner_handles = div()
            .id(("area-corners", leaf_id))
            .absolute()
            .inset(px(-gap))
            .child(make_outer_corner("area-corner-tl", true, true))
            .child(make_outer_corner("area-corner-tr", true, false))
            .child(make_outer_corner("area-corner-bl", false, true))
            .child(make_outer_corner("area-corner-br", false, false));

        // Wrap in a padded container so the gap is uniform.
        let mut wrapped = div()
            .id(("tiled-area-wrapper", leaf_id))
            .w_full()
            .h_full()
            .min_w(px(0.0))
            .min_h(px(0.0))
            .p(px(gap))
            .relative()
            .child(tile_card)
            .child(corner_handles);

        let dropdown_open = self.panels.layout.active_dropdown_leaf == Some(leaf_id);
        if dropdown_open {
            let menu = self.render_area_dropdown_menu(leaf_id, area_type, theme, cx);
            wrapped = wrapped.child(menu);
        }

        wrapped.into_any_element()
    }
    pub(crate) fn render_area_header(
        &mut self,
        leaf_id: usize,
        area_type: crate::editor::window::layout::PaneKind,
        theme: &Theme,
        leaf_count: usize,
        is_maximized: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let editor = cx.entity().downgrade();

        let type_editor = editor.clone();
        let type_button = div()
            .id(("area-header-type", leaf_id))
            .h(px(22.0))
            .px(px(8.0))
            .flex()
            .items_center()
            .gap(px(4.0))
            .rounded(px(d.menu_item_radius))
            .bg(c.dialog_secondary_button_bg)
            .hover(|this| this.bg(c.dialog_secondary_button_hover))
            .cursor_pointer()
            .text_size(px(12.0))
            .text_color(c.text_default)
            .child(area_type.name().to_string())
            .on_click(move |_event, _window, cx| {
                let _ = type_editor.update(cx, |ed, cx| {
                    ed.panels.layout.toggle_dropdown(leaf_id);
                    cx.notify();
                });
            });

        let split_h_editor = editor.clone();
        let split_h_button = div()
            .id(("area-btn-split-h", leaf_id))
            .p(px(4.0))
            .rounded(px(d.menu_item_radius))
            .hover(|this| this.bg(c.dialog_secondary_button_hover))
            .cursor_pointer()
            .child(
                svg()
                    .path("icon/panel/split-h.svg")
                    .size(px(14.0))
                    .text_color(c.dialog_muted),
            )
            .on_click(move |_event, _window, cx| {
                let _ = split_h_editor.update(cx, |ed, cx| {
                    ed.panels.layout.split_area(leaf_id, Axis::Horizontal);
                    cx.notify();
                });
            });

        let split_v_editor = editor.clone();
        let split_v_button = div()
            .id(("area-btn-split-v", leaf_id))
            .p(px(4.0))
            .rounded(px(d.menu_item_radius))
            .hover(|this| this.bg(c.dialog_secondary_button_hover))
            .cursor_pointer()
            .child(
                svg()
                    .path("icon/panel/split-v.svg")
                    .size(px(14.0))
                    .text_color(c.dialog_muted),
            )
            .on_click(move |_event, _window, cx| {
                let _ = split_v_editor.update(cx, |ed, cx| {
                    ed.panels.layout.split_area(leaf_id, Axis::Vertical);
                    cx.notify();
                });
            });

        let mut actions = div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .child(split_h_button)
            .child(split_v_button);

        if leaf_count > 1 {
            let max_editor = editor.clone();
            let max_button = div()
                .id(("area-btn-max", leaf_id))
                .p(px(4.0))
                .rounded(px(d.menu_item_radius))
                .hover(|this| this.bg(c.dialog_secondary_button_hover))
                .cursor_pointer()
                .child(
                    svg()
                        .path(if is_maximized {
                            "icon/titlebar/chrome-restore.svg"
                        } else {
                            "icon/titlebar/chrome-maximize.svg"
                        })
                        .size(px(14.0))
                        .text_color(c.dialog_muted),
                )
                .on_click(move |_event, _window, cx| {
                    let _ = max_editor.update(cx, |ed, cx| {
                        ed.panels.layout.toggle_maximize(leaf_id);
                        cx.notify();
                    });
                });

            let close_editor = editor.clone();
            let close_button = div()
                .id(("area-btn-close", leaf_id))
                .p(px(4.0))
                .rounded(px(d.menu_item_radius))
                .hover(|this| this.bg(c.dialog_secondary_button_hover))
                .cursor_pointer()
                .child(
                    svg()
                        .path("icon/titlebar/chrome-close.svg")
                        .size(px(14.0))
                        .text_color(c.dialog_muted),
                )
                .on_click(move |_event, _window, cx| {
                    let _ = close_editor.update(cx, |ed, cx| {
                        ed.panels.layout.close_area(leaf_id);
                        cx.notify();
                    });
                });

            actions = actions.child(max_button).child(close_button);
        }

        // Build tab bar for Edit areas.
        let mut left_section = div().flex().items_center().gap(px(8.0)).child(type_button);

        if area_type == PaneKind::Editor {
            let tabs = self
                .panels
                .layout
                .edit_tabs
                .entry(leaf_id)
                .or_insert_with(EditTabState::new);

            // Sync current file with tab list.
            if let Some(ref path) = self.file.path {
                if !tabs.open_paths.contains(path) {
                    tabs.open_paths.push(path.clone());
                    tabs.active_index = tabs.open_paths.len() - 1;
                } else {
                    // Update active index to match current file.
                    if let Some(pos) = tabs.open_paths.iter().position(|p| p == path) {
                        tabs.active_index = pos;
                    }
                }
            }

            let mut tab_elements: Vec<AnyElement> = Vec::new();
            for (_i, path) in tabs.open_paths.iter().enumerate() {
                let file_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Untitled".to_string());
                let is_active = Some(path.as_path()) == tabs.active_path().map(|p| p.as_path());

                let tab_bg = if is_active {
                    c.dialog_surface
                } else {
                    hsla(0.0, 0.0, 0.0, 0.0)
                };
                let tab_text = if is_active {
                    c.text_default
                } else {
                    c.dialog_muted
                };

                let switch_path = path.clone();
                let close_path = path.clone();
                let tab_editor = editor.clone();
                let close_editor = editor.clone();

                tab_elements.push(
                    div()
                        .h(px(22.0))
                        .px(px(6.0))
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .rounded(px(d.menu_item_radius))
                        .bg(tab_bg)
                        .hover(|this| this.bg(c.dialog_secondary_button_hover))
                        .cursor_pointer()
                        .text_size(px(11.0))
                        .child(
                            // Switch area: clicking the file name switches to this tab.
                            div()
                                .text_color(tab_text)
                                .child(file_name.clone())
                                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                                    let p = switch_path.clone();
                                    let _ = tab_editor.update(cx, |ed, cx| {
                                        if let Some(tabs) =
                                            ed.panels.layout.edit_tabs.get_mut(&leaf_id)
                                        {
                                            if let Some(pos) =
                                                tabs.open_paths.iter().position(|tp| tp == &p)
                                            {
                                                tabs.active_index = pos;
                                            }
                                        }
                                        let _ = ed.replace_document_from_path(&p, cx);
                                        cx.notify();
                                    });
                                }),
                        )
                        .child(
                            // Close button: separate click area.
                            div()
                                .size(px(12.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(2.0))
                                .hover(|this| this.bg(c.dialog_secondary_button_bg.opacity(0.6)))
                                .cursor_pointer()
                                .child(
                                    svg()
                                        .path("icon/titlebar/chrome-close.svg")
                                        .size(px(8.0))
                                        .text_color(c.dialog_muted),
                                )
                                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                                    let p = close_path.clone();
                                    let _ = close_editor.update(cx, |ed, cx| {
                                        if let Some(tabs) =
                                            ed.panels.layout.edit_tabs.get_mut(&leaf_id)
                                        {
                                            if tabs.open_paths.len() > 1 {
                                                if let Some(pos) =
                                                    tabs.open_paths.iter().position(|tp| tp == &p)
                                                {
                                                    let was_active = pos == tabs.active_index;
                                                    tabs.open_paths.remove(pos);
                                                    if was_active {
                                                        if tabs.active_index
                                                            >= tabs.open_paths.len()
                                                        {
                                                            tabs.active_index = tabs
                                                                .open_paths
                                                                .len()
                                                                .saturating_sub(1);
                                                        }
                                                        if let Some(new_path) =
                                                            tabs.active_path().cloned()
                                                        {
                                                            let _ = ed.replace_document_from_path(
                                                                &new_path, cx,
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        cx.notify();
                                    });
                                }),
                        )
                        .into_any_element(),
                );
            }

            // Add "+" button to open new file.
            let add_editor = editor.clone();
            tab_elements.push(
                div()
                    .size(px(22.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(d.menu_item_radius))
                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                    .cursor_pointer()
                    .text_size(px(14.0))
                    .text_color(c.dialog_muted)
                    .child("+")
                    .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                        let _ = add_editor.update(cx, |_ed, cx| {
                            cx.notify();
                        });
                    })
                    .into_any_element(),
            );

            left_section = left_section.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.0))
                    .children(tab_elements),
            );
        }

        div()
            .id(("area-header", leaf_id))
            .h(px(28.0))
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .px(px(8.0))
            .border_b(px(1.0))
            .border_color(c.dialog_border)
            .child(left_section)
            .child(div().flex().items_center().gap(px(6.0)).child(actions))
            .into_any_element()
    }
    pub(crate) fn render_area_dropdown_menu(
        &mut self,
        leaf_id: usize,
        current_type: crate::editor::window::layout::PaneKind,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let editor = cx.entity().downgrade();

        let available_types = PaneKind::all();

        div()
            .id(("area-dropdown-overlay", leaf_id))
            .absolute()
            .occlude()
            .top(px(28.0))
            .left(px(8.0))
            .w(px(d.menu_panel_width))
            .p(px(d.menu_panel_padding))
            .flex()
            .flex_col()
            .gap(px(d.menu_panel_gap))
            .bg(c.dialog_surface)
            .border(px(d.dialog_border_width))
            .border_color(c.dialog_border)
            .rounded(px(d.menu_panel_radius))
            .shadow_lg()
            .children(available_types.iter().enumerate().map(|(idx, area_type)| {
                let area_type = *area_type;
                let is_current = area_type == current_type;
                let option_editor = editor.clone();
                div()
                    .id(("area-type-opt", idx))
                    .w_full()
                    .h(px(d.menu_item_height))
                    .px(px(d.menu_item_padding_x))
                    .flex()
                    .items_center()
                    .justify_between()
                    .rounded(px(d.menu_item_radius))
                    .bg(if is_current {
                        c.dialog_secondary_button_hover
                    } else {
                        c.dialog_surface
                    })
                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                    .cursor_pointer()
                    .text_size(px(d.menu_text_size))
                    .font_weight(t.dialog_button_weight.to_font_weight())
                    .text_color(c.dialog_secondary_button_text)
                    .child(area_type.name())
                    .child(if is_current {
                        svg()
                            .path("icon/panel/check.svg")
                            .size(px(13.0))
                            .text_color(c.dialog_primary_button_bg)
                            .into_any_element()
                    } else {
                        div().w(px(13.0)).into_any_element()
                    })
                    .on_click(move |_event, _window, cx| {
                        let _ = option_editor.update(cx, |ed, cx| {
                            ed.panels.layout.change_area_type(leaf_id, area_type);
                            cx.notify();
                        });
                    })
                    .into_any_element()
            }))
            .into_any_element()
    }
    pub(crate) fn render_border_context_menu_overlay(
        &mut self,
        border_menu: crate::editor::window::layout::BorderMenuState,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let editor = cx.entity().downgrade();
        let split_id = border_menu.split_id;

        let left_pos = f32::from(border_menu.position.x);
        let top_pos = f32::from(border_menu.position.y);

        let split_h_ed = editor.clone();
        let split_v_ed = editor.clone();
        let swap_ed = editor.clone();
        let close_ed = editor.clone();
        let dismiss_ed = editor.clone();

        div()
            .id("tiled-border-context-menu-wrapper")
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .bottom_0()
            .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                let _ = dismiss_ed.update(cx, |ed, cx| {
                    ed.panels.layout.active_border_menu = None;
                    cx.notify();
                });
            })
            .child(
                div()
                    .id("tiled-border-context-menu")
                    .absolute()
                    .occlude()
                    .top(px(top_pos))
                    .left(px(left_pos))
                    .w(px(d.menu_panel_width))
                    .p(px(d.menu_panel_padding))
                    .flex()
                    .flex_col()
                    .gap(px(d.menu_panel_gap))
                    .bg(c.dialog_surface)
                    .border(px(d.dialog_border_width))
                    .border_color(c.dialog_border)
                    .rounded(px(d.menu_panel_radius))
                    .shadow_lg()
                    .child(
                        div()
                            .id("border-menu-split-h")
                            .w_full()
                            .h(px(d.menu_item_height))
                            .px(px(d.menu_item_padding_x))
                            .flex()
                            .items_center()
                            .rounded(px(d.menu_item_radius))
                            .bg(c.dialog_surface)
                            .hover(|this| this.bg(c.dialog_secondary_button_hover))
                            .active(|this| this.opacity(0.92))
                            .cursor_pointer()
                            .text_size(px(d.menu_text_size))
                            .font_weight(t.dialog_body_weight.to_font_weight())
                            .text_color(c.dialog_secondary_button_text)
                            .child("Split Horizontally")
                            .on_click(move |_event, _window, cx| {
                                let _ = split_h_ed.update(cx, |ed, cx| {
                                    ed.panels.layout.split_area(split_id, Axis::Horizontal);
                                    cx.notify();
                                });
                            }),
                    )
                    .child(
                        div()
                            .id("border-menu-split-v")
                            .w_full()
                            .h(px(d.menu_item_height))
                            .px(px(d.menu_item_padding_x))
                            .flex()
                            .items_center()
                            .rounded(px(d.menu_item_radius))
                            .bg(c.dialog_surface)
                            .hover(|this| this.bg(c.dialog_secondary_button_hover))
                            .active(|this| this.opacity(0.92))
                            .cursor_pointer()
                            .text_size(px(d.menu_text_size))
                            .font_weight(t.dialog_body_weight.to_font_weight())
                            .text_color(c.dialog_secondary_button_text)
                            .child("Split Vertically")
                            .on_click(move |_event, _window, cx| {
                                let _ = split_v_ed.update(cx, |ed, cx| {
                                    ed.panels.layout.split_area(split_id, Axis::Vertical);
                                    cx.notify();
                                });
                            }),
                    )
                    .child(
                        div()
                            .id("border-menu-sep-1")
                            .mx(px(d.menu_separator_margin_x))
                            .my(px(d.menu_separator_margin_y))
                            .h(px(d.menu_separator_height))
                            .bg(c.dialog_border),
                    )
                    .child(
                        div()
                            .id("border-menu-swap")
                            .w_full()
                            .h(px(d.menu_item_height))
                            .px(px(d.menu_item_padding_x))
                            .flex()
                            .items_center()
                            .rounded(px(d.menu_item_radius))
                            .bg(c.dialog_surface)
                            .hover(|this| this.bg(c.dialog_secondary_button_hover))
                            .active(|this| this.opacity(0.92))
                            .cursor_pointer()
                            .text_size(px(d.menu_text_size))
                            .font_weight(t.dialog_body_weight.to_font_weight())
                            .text_color(c.dialog_secondary_button_text)
                            .child("Swap Panels")
                            .on_click(move |_event, _window, cx| {
                                let _ = swap_ed.update(cx, |ed, cx| {
                                    ed.panels.layout.swap_split_children(split_id);
                                    cx.notify();
                                });
                            }),
                    )
                    .child(
                        div()
                            .id("border-menu-sep-2")
                            .mx(px(d.menu_separator_margin_x))
                            .my(px(d.menu_separator_margin_y))
                            .h(px(d.menu_separator_height))
                            .bg(c.dialog_border),
                    )
                    .child(
                        div()
                            .id("border-menu-close")
                            .w_full()
                            .h(px(d.menu_item_height))
                            .px(px(d.menu_item_padding_x))
                            .flex()
                            .items_center()
                            .rounded(px(d.menu_item_radius))
                            .bg(c.dialog_surface)
                            .hover(|this| this.bg(c.dialog_secondary_button_hover))
                            .active(|this| this.opacity(0.92))
                            .cursor_pointer()
                            .text_size(px(d.menu_text_size))
                            .font_weight(t.dialog_body_weight.to_font_weight())
                            .text_color(c.dialog_secondary_button_text)
                            .child("Close Area")
                            .on_click(move |_event, _window, cx| {
                                let _ = close_ed.update(cx, |ed, cx| {
                                    ed.panels.layout.close_area(split_id);
                                    cx.notify();
                                });
                            }),
                    ),
            )
            .into_any_element()
    }
    pub(crate) fn render_tiled_edit_container_panel(
        &mut self,
        container_id: usize,
        primary_content: &mut Option<AnyElement>,
        theme: &Theme,
        strings: &I18nStrings,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let inner_tree = self
            .panels
            .layout
            .get_or_create_edit_inner_layout(container_id)
            .clone();

        let inner_rendered = self.render_edit_inner_node(
            &inner_tree,
            container_id,
            primary_content,
            theme,
            strings,
            cx,
        );

        let dropdown = if let Some((_cid, inner_id)) = self.panels.layout.active_inner_dropdown {
            if _cid == container_id {
                let current_type = self
                    .panels
                    .layout
                    .get_or_create_edit_inner_layout(container_id)
                    .find_leaf_area(inner_id)
                    .unwrap_or(crate::editor::window::layout::EditorPanel::SourceCode);
                Some(self.render_inner_area_dropdown_menu(
                    container_id,
                    inner_id,
                    current_type,
                    theme,
                    cx,
                ))
            } else {
                None
            }
        } else {
            None
        };

        let mut container = div()
            .w_full()
            .h_full()
            .relative()
            .p(px(2.0))
            .bg(c.editor_background)
            .child(inner_rendered);

        if let Some(dropdown) = dropdown {
            container = container.child(dropdown);
        }

        container.into_any_element()
    }
    pub(crate) fn render_edit_inner_node(
        &mut self,
        node: &crate::editor::window::layout::SplitTree<crate::editor::window::layout::EditorPanel>,
        container_id: usize,
        primary_content: &mut Option<AnyElement>,
        theme: &Theme,
        strings: &I18nStrings,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;

        match node {
            SplitTree::Leaf {
                id: inner_id,
                area_type,
            } => {
                let inner_id = *inner_id;
                let area_type = *area_type;
                let inner_editor = cx.entity().downgrade();

                let has_content = self.file.path.is_some() || self.file.dirty;
                let workspace_open = self.panels.workspace.root.is_some();

                let inner_body: AnyElement = match area_type {
                    // Block — WYSIWYG block editor from document.blocks().
                    EditorPanel::Wysiwyg => {
                        if let Some(content) = primary_content.take() {
                            content
                        } else if has_content {
                            self.render_tiled_preview_panel(primary_content, theme, strings, cx)
                        } else if workspace_open {
                            render_empty_panel_prompt(c, "Open a file")
                        } else {
                            render_empty_panel_prompt(c, "No content")
                        }
                    }
                    // Source — interactive source code editor.  Uses a cached
                    // block in source-document mode.  Edits sync to the shared
                    // document via the block's Changed event.
                    EditorPanel::SourceCode => {
                        if has_content {
                            self.refresh_source_panel_block(cx);
                            self.render_source_editor_panel(theme, cx)
                        } else if workspace_open {
                            render_empty_panel_prompt(c, "Open a file")
                        } else {
                            render_empty_panel_prompt(c, "No content")
                        }
                    }
                    EditorPanel::Preview => {
                        if has_content {
                            self.render_tiled_preview_panel(primary_content, theme, strings, cx)
                        } else if workspace_open {
                            render_empty_panel_prompt(c, "Open a file to preview")
                        } else {
                            render_empty_panel_prompt(c, "No preview content")
                        }
                    }
                    EditorPanel::Outline => {
                        if has_content {
                            self.render_tiled_outline_panel(theme, strings, cx)
                        } else if workspace_open {
                            render_empty_panel_prompt(c, "Open a file to show outline")
                        } else {
                            render_empty_panel_prompt(c, "No outline content")
                        }
                    }
                };

                let make_inner_corner = |id_str: &'static str, top: bool, left: bool| {
                    let inner_editor = inner_editor.clone();
                    let mut corner_div = div()
                        .id((id_str, inner_id))
                        .absolute()
                        .occlude()
                        .size(px(10.0))
                        .cursor_crosshair()
                        .rounded(px(4.0));

                    if top {
                        corner_div = corner_div.top(px(2.0));
                    } else {
                        corner_div = corner_div.bottom(px(2.0));
                    }
                    if left {
                        corner_div = corner_div.left(px(2.0));
                    } else {
                        corner_div = corner_div.right(px(2.0));
                    }

                    corner_div.on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                        let pos = event.position;
                        let modifier = if event.modifiers.control {
                            CornerDragModifier::Swap
                        } else if event.modifiers.shift {
                            CornerDragModifier::Duplicate
                        } else {
                            CornerDragModifier::None
                        };
                        let _ = inner_editor.update(cx, |ed, cx| {
                            ed.panels.layout.start_inner_corner_drag(
                                container_id,
                                inner_id,
                                pos,
                                modifier,
                            );
                            cx.notify();
                        });
                    })
                };

                // Auto-focus first inner panel if none is focused.
                if self.panels.layout.focused_inner_panel.is_none() {
                    self.panels.layout.focused_inner_panel = Some((container_id, inner_id));
                }

                let focus_editor = cx.entity().downgrade();

                div()
                    .w_full()
                    .h_full()
                    .flex()
                    .flex_col()
                    .relative()
                    .rounded(px(d.area_tile_radius))
                    .bg(c.dialog_surface)
                    .border(px(d.dialog_border_width))
                    .border_color(c.dialog_border)
                    .shadow_lg()
                    .child(div().w_full().flex_1().min_h(px(0.0)).child(inner_body))
                    .child(make_inner_corner("edit-sub-tl", true, true))
                    .child(make_inner_corner("edit-sub-tr", true, false))
                    .child(make_inner_corner("edit-sub-bl", false, true))
                    .child(make_inner_corner("edit-sub-br", false, false))
                    .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                        let _ = focus_editor.update(cx, |ed, cx| {
                            ed.panels.layout.focused_inner_panel = Some((container_id, inner_id));
                            cx.notify();
                        });
                    })
                    .into_any_element()
            }
            SplitTree::Split {
                id,
                direction,
                ratio,
                first,
                second,
            } => {
                let split_id = *id;
                let dir = *direction;
                let r = *ratio;
                let first_elem = self.render_edit_inner_node(
                    first,
                    container_id,
                    primary_content,
                    theme,
                    strings,
                    cx,
                );
                let second_elem = self.render_edit_inner_node(
                    second,
                    container_id,
                    primary_content,
                    theme,
                    strings,
                    cx,
                );

                let inner_editor = cx.entity().downgrade();

                match direction {
                    Axis::Horizontal => {
                        let bar_editor = inner_editor.clone();
                        let menu_editor = inner_editor.clone();
                        div()
                            .w_full()
                            .h_full()
                            .flex()
                            .flex_row()
                            .min_w(px(0.0))
                            .min_h(px(0.0))
                            .child(
                                div()
                                    .w(relative(r))
                                    .h_full()
                                    .overflow_hidden()
                                    .flex()
                                    .flex_col()
                                    .flex_shrink_0()
                                    .min_w(px(0.0))
                                    .min_h(px(0.0))
                                    .child(first_elem),
                            )
                            .child(
                                div()
                                    .id(("inner-splitter-bar-h", split_id))
                                    .w(px(2.0))
                                    .h_full()
                                    .flex_shrink_0()
                                    .cursor_col_resize()
                                    .bg(c.dialog_border)
                                    .hover(|this| this.bg(c.selection))
                                    .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                                        let start_pos = f32::from(event.position.x);
                                        let _ = bar_editor.update(cx, |ed, cx| {
                                            ed.panels.layout.active_inner_splitter_drag = Some((
                                                container_id,
                                                SplitterDragSession {
                                                    split_id,
                                                    direction: Axis::Horizontal,
                                                    start_pointer_pos: start_pos,
                                                    start_ratio: r,
                                                    total_span: 1000.0,
                                                },
                                            ));
                                            cx.notify();
                                        });
                                    })
                                    .on_mouse_down(
                                        MouseButton::Right,
                                        move |event, _window, cx| {
                                            let pos = event.position;
                                            let _ = menu_editor.update(cx, |ed, cx| {
                                                ed.panels.layout.active_inner_border_menu =
                                                    Some(BorderMenuState {
                                                        split_id,
                                                        direction: dir,
                                                        position: pos,
                                                    });
                                                cx.notify();
                                            });
                                        },
                                    ),
                            )
                            .child(
                                div()
                                    .h_full()
                                    .overflow_hidden()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .min_h(px(0.0))
                                    .child(second_elem),
                            )
                            .into_any_element()
                    }
                    Axis::Vertical => {
                        let bar_editor = inner_editor.clone();
                        let menu_editor = inner_editor.clone();
                        div()
                            .w_full()
                            .h_full()
                            .flex()
                            .flex_col()
                            .min_w(px(0.0))
                            .min_h(px(0.0))
                            .child(
                                div()
                                    .h(relative(r))
                                    .w_full()
                                    .overflow_hidden()
                                    .flex()
                                    .flex_col()
                                    .flex_shrink_0()
                                    .min_w(px(0.0))
                                    .min_h(px(0.0))
                                    .child(first_elem),
                            )
                            .child(
                                div()
                                    .id(("inner-splitter-bar-v", split_id))
                                    .h(px(2.0))
                                    .w_full()
                                    .flex_shrink_0()
                                    .cursor_row_resize()
                                    .bg(c.dialog_border)
                                    .hover(|this| this.bg(c.selection))
                                    .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                                        let start_pos = f32::from(event.position.y);
                                        let _ = bar_editor.update(cx, |ed, cx| {
                                            ed.panels.layout.active_inner_splitter_drag = Some((
                                                container_id,
                                                SplitterDragSession {
                                                    split_id,
                                                    direction: Axis::Vertical,
                                                    start_pointer_pos: start_pos,
                                                    start_ratio: r,
                                                    total_span: 700.0,
                                                },
                                            ));
                                            cx.notify();
                                        });
                                    })
                                    .on_mouse_down(
                                        MouseButton::Right,
                                        move |event, _window, cx| {
                                            let pos = event.position;
                                            let _ = menu_editor.update(cx, |ed, cx| {
                                                ed.panels.layout.active_inner_border_menu =
                                                    Some(BorderMenuState {
                                                        split_id,
                                                        direction: dir,
                                                        position: pos,
                                                    });
                                                cx.notify();
                                            });
                                        },
                                    ),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .overflow_hidden()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .min_h(px(0.0))
                                    .child(second_elem),
                            )
                            .into_any_element()
                    }
                }
            }
        }
    }
    pub(crate) fn render_inner_area_dropdown_menu(
        &mut self,
        container_id: usize,
        inner_id: usize,
        current_area_type: crate::editor::window::layout::EditorPanel,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let editor = cx.entity().downgrade();

        let available_types = EditorPanel::all();

        div()
            .id(("inner-area-dropdown-overlay", inner_id))
            .absolute()
            .occlude()
            .bottom(px(0.0))
            .left(px(0.0))
            .w(px(d.menu_panel_width))
            .p(px(d.menu_panel_padding))
            .flex()
            .flex_col()
            .gap(px(d.menu_panel_gap))
            .bg(c.dialog_surface)
            .border(px(d.dialog_border_width))
            .border_color(c.dialog_border)
            .rounded(px(d.menu_panel_radius))
            .shadow_lg()
            .children(available_types.iter().enumerate().map(|(idx, area_type)| {
                let area_type = *area_type;
                let is_current = area_type == current_area_type;
                let option_editor = editor.clone();
                div()
                    .id(("inner-area-type-opt", idx))
                    .w_full()
                    .h(px(d.menu_item_height))
                    .px(px(d.menu_item_padding_x))
                    .flex()
                    .items_center()
                    .justify_between()
                    .rounded(px(d.menu_item_radius))
                    .bg(if is_current {
                        c.dialog_secondary_button_hover
                    } else {
                        c.dialog_surface
                    })
                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                    .cursor_pointer()
                    .text_size(px(d.menu_text_size))
                    .font_weight(t.dialog_body_weight.to_font_weight())
                    .text_color(c.dialog_secondary_button_text)
                    .child(div().child(area_type.name()))
                    .child(if is_current {
                        svg()
                            .path("icon/panel/check.svg")
                            .size(px(13.0))
                            .text_color(c.dialog_primary_button_bg)
                            .into_any_element()
                    } else {
                        div().w(px(13.0)).into_any_element()
                    })
                    .on_click(move |_event, _window, cx| {
                        let _ = option_editor.update(cx, |ed, cx| {
                            ed.panels.layout.change_inner_area_type(
                                container_id,
                                inner_id,
                                area_type,
                            );
                            cx.notify();
                        });
                    })
            }))
            .into_any_element()
    }
}
