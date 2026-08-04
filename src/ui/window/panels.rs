//! Tiled pane rendering: outer layout, area tiles, inner edit panels,
//! preview/source/settings panels, and border context menus.

use gpui::*;

use crate::editor::controller::*;
use crate::editor::layout::{
    Axis, BorderMenuState, CornerDragAction, CornerDragModifier, CornerDragPreview, Direction,
    EditTabState, EditorPanel, PaneKind, SettingsTab, SplitTree, SplitterDragSession,
};
use crate::infra::i18n::I18nStrings;
use crate::theme::{Theme, ThemeManager};
use crate::ui::window::render::render_empty_panel_prompt;
use crate::ui::window::switch::Switch;

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
        node: &crate::editor::layout::SplitTree<crate::editor::layout::PaneKind>,
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
        area_type: crate::editor::layout::PaneKind,
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
        area_type: crate::editor::layout::PaneKind,
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
        current_type: crate::editor::layout::PaneKind,
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
        border_menu: crate::editor::layout::BorderMenuState,
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

    pub(crate) fn render_tiled_workspace_files_panel(
        &mut self,
        theme: &Theme,
        strings: &I18nStrings,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.sync_workspace_models(cx);
        let editor = cx.entity().downgrade();
        self.render_workspace_files_tree(theme, strings, &editor)
    }

    pub(crate) fn render_tiled_outline_panel(
        &mut self,
        theme: &Theme,
        strings: &I18nStrings,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.sync_workspace_models(cx);
        let editor = cx.entity().downgrade();
        self.render_workspace_outline_tree(theme, strings, &editor)
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
                    .unwrap_or(crate::editor::layout::EditorPanel::SourceCode);
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
        node: &crate::editor::layout::SplitTree<crate::editor::layout::EditorPanel>,
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
        current_area_type: crate::editor::layout::EditorPanel,
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

    pub(crate) fn render_tiled_preview_panel(
        &mut self,
        _primary_content: &mut Option<AnyElement>,
        theme: &Theme,
        _strings: &I18nStrings,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;

        self.refresh_preview_blocks(cx);

        // Render each preview block inside a read-only shell that captures all
        // interaction events. The visual rendering is identical to the Block
        // panel, but mouse clicks, keyboard input, and focus are suppressed so
        // the Preview channel remains a truly read-only view.
        let editor = cx.entity().downgrade();
        let block_elements: Vec<AnyElement> = self
            .preview
            .blocks
            .iter()
            .map(|entity| {
                let block_id = entity.entity_id();
                let preview_editor = editor.clone();
                div()
                    .w_full()
                    .flex_shrink_0()
                    .mt(px(d.block_gap))
                    .cursor_default()
                    .capture_any_mouse_down(move |_event, _window, cx| {
                        cx.stop_propagation();
                    })
                    .capture_key_down(move |_event, _window, cx| {
                        cx.stop_propagation();
                    })
                    .on_mouse_down(MouseButton::Right, move |event, window, cx| {
                        let _ = preview_editor.update(cx, |editor, cx| {
                            editor.on_block_context_menu_mouse_down(block_id, event, window, cx);
                        });
                    })
                    .child(entity.clone())
                    .into_any_element()
            })
            .collect();

        div()
            .w_full()
            .h_full()
            .relative()
            .bg(c.editor_background)
            .child(
                div()
                    .id("tiled-preview-scroll")
                    .w_full()
                    .h_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .overflow_y_scroll()
                    .p(px(d.editor_padding))
                    .children(block_elements),
            )
            .into_any_element()
    }

    /// Render the interactive source editor for the Source channel.
    ///
    /// Uses a cached Block entity in source-document mode — the same
    /// look and feel as the original Source view mode: line numbers,
    /// monospace font, raw text editing, cursor, and selection.
    pub(crate) fn render_source_editor_panel(
        &mut self,
        theme: &Theme,
        _cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;

        let content: AnyElement = if let Some(ref block) = self.source_panel.block {
            div()
                .w_full()
                .flex_shrink_0()
                .child(block.clone())
                .into_any_element()
        } else {
            div().into_any_element()
        };

        div()
            .id("tiled-source-editor")
            .w_full()
            .h_full()
            .relative()
            .bg(c.editor_background)
            .child(
                div()
                    .id("tiled-source-scroll")
                    .w_full()
                    .h_full()
                    .flex()
                    .flex_col()
                    .overflow_y_scroll()
                    .p(px(d.editor_padding))
                    .child(content),
            )
            .into_any_element()
    }

    pub(crate) fn render_tiled_settings_panel(
        &mut self,
        theme: &Theme,
        _strings: &I18nStrings,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let active_tab = self.panels.layout.settings_tab;

        let mut inner_border_color = c.dialog_border;
        inner_border_color.a *= 0.4;

        // --- Left Sidebar (3 Main Tabs: Interface, Editing, Keymap) ---
        let mut left_nav_items = Vec::new();
        for (tab_idx, tab) in SettingsTab::all().iter().enumerate() {
            let is_active = active_tab == *tab;
            let editor = cx.entity().downgrade();
            let tab_item = *tab;

            left_nav_items.push(
                div()
                    .id(("pref-tab", tab_idx))
                    .px(px(12.0))
                    .py(px(8.0))
                    .rounded(px(d.menu_item_radius))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .bg(if is_active {
                        c.dialog_secondary_button_hover
                    } else {
                        c.dialog_surface
                    })
                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .font_weight(if is_active {
                                gpui::FontWeight::BOLD
                            } else {
                                gpui::FontWeight::NORMAL
                            })
                            .text_color(if is_active {
                                c.text_default
                            } else {
                                c.dialog_muted
                            })
                            .child(tab.name()),
                    )
                    .on_click(move |_event, _window, cx| {
                        let _ = editor.update(cx, |ed, cx| {
                            ed.panels.layout.settings_tab = tab_item;
                            cx.notify();
                        });
                    })
                    .into_any_element(),
            );
        }

        let left_nav = div()
            .w(px(160.0))
            .h_full()
            .flex_shrink_0()
            .p(px(8.0))
            .border_r_1()
            .border_color(c.dialog_border)
            .flex()
            .flex_col()
            .gap(px(2.0))
            .children(left_nav_items);

        // --- Right Content Area ---
        let mut sections: Vec<AnyElement> = Vec::new();

        // Helper closures / local constructors to produce shallow type-erased elements
        let make_row = |title: &'static str,
                        desc: &'static str,
                        control: AnyElement,
                        theme: &Theme,
                        border_col: Hsla|
         -> AnyElement {
            let tc = &theme.colors;
            let td = &theme.dimensions;
            div()
                .w_full()
                .h(px(56.0))
                .px(px(16.0))
                .rounded(px(td.menu_panel_radius))
                .bg(tc.dialog_surface)
                .border_1()
                .border_color(border_col)
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(
                            div()
                                .text_size(px(12.5))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(tc.text_default)
                                .child(title),
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(tc.dialog_muted)
                                .child(desc),
                        ),
                )
                .child(control)
                .into_any_element()
        };

        let render_zed_stepper =
            |id_dec: &'static str,
             id_inc: &'static str,
             val_num: String,
             unit_str: &'static str,
             is_editing: bool,
             on_dec: Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>,
             on_inc: Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>,
             on_click_center: Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>,
             theme: &Theme|
             -> AnyElement {
                let tc = &theme.colors;
                let td = &theme.dimensions;

                let mut center_box = div()
                    .id(ElementId::Name(format!("{}-center", id_dec).into()))
                    .cursor_pointer()
                    .h_full()
                    .flex_1()
                    .min_w(px(0.0))
                    .px(px(4.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(3.0))
                    .bg(if is_editing {
                        tc.dialog_surface
                    } else {
                        tc.dialog_secondary_button_bg
                    })
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(tc.text_default)
                            .child(val_num),
                    );

                if is_editing {
                    center_box = center_box
                        .border_1()
                        .border_color(tc.dialog_primary_button_bg)
                        .child(div().w(px(1.5)).h(px(12.0)).bg(tc.dialog_primary_button_bg));
                }

                if !unit_str.is_empty() {
                    center_box = center_box.child(
                        div()
                            .text_size(px(11.0))
                            .text_color(tc.dialog_muted)
                            .child(unit_str),
                    );
                }

                let center_box = center_box.on_click(on_click_center);

                div()
                    .flex()
                    .items_center()
                    .w(px(145.0))
                    .h(px(28.0))
                    .rounded(px(td.menu_item_radius))
                    .border_1()
                    .border_color(tc.dialog_border)
                    .bg(tc.dialog_secondary_button_bg)
                    .child(
                        div()
                            .id(id_dec)
                            .cursor_pointer()
                            .h_full()
                            .w(px(32.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .hover(|this| this.bg(tc.dialog_secondary_button_hover))
                            .text_size(px(13.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(tc.text_default)
                            .child("-")
                            .on_click(on_dec),
                    )
                    .child(div().w(px(1.0)).h_full().bg(tc.dialog_border))
                    .child(center_box)
                    .child(div().w(px(1.0)).h_full().bg(tc.dialog_border))
                    .child(
                        div()
                            .id(id_inc)
                            .cursor_pointer()
                            .h_full()
                            .w(px(32.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .hover(|this| this.bg(tc.dialog_secondary_button_hover))
                            .text_size(px(13.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(tc.text_default)
                            .child("+")
                            .on_click(on_inc),
                    )
                    .into_any_element()
            };

        let make_section = |sec_id: &'static str,
                            title: &'static str,
                            is_expanded: bool,
                            toggle_fn: Box<dyn Fn(&gpui::ClickEvent, &mut Window, &mut App)>,
                            items: Vec<AnyElement>,
                            theme: &Theme|
         -> AnyElement {
            let tc = &theme.colors;
            let td = &theme.dimensions;

            let header = div()
                .id(sec_id)
                .w_full()
                .px(px(14.0))
                .py(px(10.0))
                .cursor_pointer()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    svg()
                        .path(if is_expanded {
                            "icon/panel/chevron-down.svg"
                        } else {
                            "icon/panel/chevron-right.svg"
                        })
                        .size(px(14.0))
                        .text_color(tc.text_default),
                )
                .child(
                    div()
                        .text_size(px(13.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(tc.text_default)
                        .child(title),
                )
                .on_click(move |ev, window, cx| toggle_fn(ev, window, cx));

            let mut card = div()
                .relative()
                .w_full()
                .rounded(px(td.menu_panel_radius))
                .bg(tc.dialog_surface)
                .border_1()
                .border_color(tc.dialog_border)
                .flex()
                .flex_col()
                .child(header);

            if is_expanded && !items.is_empty() {
                let body = div()
                    .w_full()
                    .px(px(10.0))
                    .pb(px(10.0))
                    .pt(px(2.0))
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .children(items);

                card = card.child(body);
            }

            card.into_any_element()
        };

        match active_tab {
            SettingsTab::Interface => {
                // Section 1: Visual Theme & Language
                let sec1_key = "theme";
                let is_sec1_expanded = self
                    .panels
                    .layout
                    .settings_expanded_sections
                    .contains(sec1_key);
                let mut sec1_items = Vec::new();

                let theme_ed = cx.entity().downgrade();
                let available_themes = cx.global::<ThemeManager>().available_themes();
                let raw_theme_name = theme.name.clone();
                let current_theme_name: String = match raw_theme_name.as_str() {
                    "Velotype" => "Dark".to_string(),
                    "Velotype Light" => "Light".to_string(),
                    other => other.to_string(),
                };

                let lang_ed = cx.entity().downgrade();
                let lang_options = [("en-US", "English (en-US)"), ("zh-CN", "简体中文 (zh-CN)")];
                let current_lang = "English (en-US)";

                let is_theme_open =
                    self.panels.layout.open_settings_dropdown.as_deref() == Some("theme");
                let is_lang_open =
                    self.panels.layout.open_settings_dropdown.as_deref() == Some("lang");

                if is_sec1_expanded {
                    let theme_icon_path = if current_theme_name == "Light" {
                        "icon/panel/sun.svg"
                    } else {
                        "icon/panel/moon.svg"
                    };

                    let mut theme_btn_wrap = div().relative().child(
                        div()
                            .id("pref-btn-theme")
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .justify_between()
                            .w(px(145.0))
                            .h(px(28.0))
                            .px(px(8.0))
                            .rounded(px(d.menu_item_radius))
                            .bg(c.dialog_secondary_button_bg)
                            .hover(|this| this.bg(c.dialog_secondary_button_hover))
                            .border_1()
                            .border_color(c.dialog_border)
                            .text_size(px(12.0))
                            .text_color(c.text_default)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .flex()
                                    .items_center()
                                    .gap(px(6.0))
                                    .child(
                                        svg()
                                            .path(theme_icon_path)
                                            .size(px(13.0))
                                            .text_color(c.text_default),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w(px(0.0))
                                            .truncate()
                                            .child(current_theme_name.clone()),
                                    ),
                            )
                            .child(
                                div().flex_shrink_0().pl(px(4.0)).child(
                                    svg()
                                        .path("icon/panel/select-chevron.svg")
                                        .size(px(14.0))
                                        .text_color(c.dialog_muted),
                                ),
                            )
                            .on_click({
                                let theme_ed = theme_ed.clone();
                                move |_ev, _win, cx| {
                                    let _ = theme_ed.update(cx, |ed, cx| {
                                        if ed.panels.layout.open_settings_dropdown.as_deref()
                                            == Some("theme")
                                        {
                                            ed.panels.layout.open_settings_dropdown = None;
                                        } else {
                                            ed.panels.layout.open_settings_dropdown =
                                                Some("theme".to_string());
                                        }
                                        cx.notify();
                                    });
                                }
                            }),
                    );

                    if is_theme_open {
                        let mut menu_items = Vec::new();
                        for t_entry in available_themes {
                            let t_id = t_entry.id.clone();
                            let display_label: String = match t_entry.name.as_str() {
                                "Velotype" | "Dark" => "Dark".to_string(),
                                "Velotype Light" | "Light" => "Light".to_string(),
                                other => other.to_string(),
                            };
                            let is_selected = display_label == current_theme_name;
                            let item_ed = theme_ed.clone();
                            let item_icon = if display_label == "Light" {
                                "icon/panel/sun.svg"
                            } else {
                                "icon/panel/moon.svg"
                            };

                            menu_items.push(
                                div()
                                    .id(ElementId::Name(format!("theme-item-{}", t_id).into()))
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .px(px(10.0))
                                    .py(px(6.0))
                                    .rounded(px(4.0))
                                    .bg(if is_selected {
                                        c.dialog_secondary_button_hover
                                    } else {
                                        c.dialog_surface
                                    })
                                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                                    .text_size(px(12.0))
                                    .text_color(c.text_default)
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(6.0))
                                            .child(
                                                svg()
                                                    .path(item_icon)
                                                    .size(px(13.0))
                                                    .text_color(c.text_default),
                                            )
                                            .child(display_label),
                                    )
                                    .child(if is_selected {
                                        svg()
                                            .path("icon/panel/check.svg")
                                            .size(px(13.0))
                                            .text_color(c.dialog_primary_button_bg)
                                            .into_any_element()
                                    } else {
                                        div().w(px(13.0)).into_any_element()
                                    })
                                    .on_click(move |_ev, _win, cx| {
                                        let _ = item_ed.update(cx, |ed, cx| {
                                            cx.update_global::<ThemeManager, _>(|manager, _cx| {
                                                let _ = manager.set_theme_by_id(&t_id);
                                            });
                                            ed.panels.layout.open_settings_dropdown = None;
                                            cx.notify();
                                        });
                                    })
                                    .into_any_element(),
                            );
                        }

                        theme_btn_wrap = theme_btn_wrap.child(gpui::deferred(
                            div()
                                .absolute()
                                .top_full()
                                .right_0()
                                .mt(px(4.0))
                                .w(px(160.0))
                                .occlude()
                                .bg(c.dialog_surface)
                                .border_1()
                                .border_color(c.dialog_border)
                                .rounded(px(6.0))
                                .shadow_lg()
                                .p(px(4.0))
                                .flex()
                                .flex_col()
                                .gap(px(2.0))
                                .children(menu_items),
                        ));
                    }

                    sec1_items.push(make_row(
                        "Interface Theme",
                        "Customize overall application color scheme and appearance",
                        theme_btn_wrap.into_any_element(),
                        theme,
                        inner_border_color,
                    ));

                    let mut lang_btn_wrap = div().relative().child(
                        div()
                            .id("pref-btn-lang")
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .justify_between()
                            .w(px(145.0))
                            .h(px(28.0))
                            .px(px(8.0))
                            .rounded(px(d.menu_item_radius))
                            .bg(c.dialog_secondary_button_bg)
                            .hover(|this| this.bg(c.dialog_secondary_button_hover))
                            .border_1()
                            .border_color(c.dialog_border)
                            .text_size(px(12.0))
                            .text_color(c.text_default)
                            .child(div().flex_1().min_w(px(0.0)).truncate().child(current_lang))
                            .child(
                                div().flex_shrink_0().pl(px(4.0)).child(
                                    svg()
                                        .path("icon/panel/select-chevron.svg")
                                        .size(px(14.0))
                                        .text_color(c.dialog_muted),
                                ),
                            )
                            .on_click({
                                let lang_ed = lang_ed.clone();
                                move |_ev, _win, cx| {
                                    let _ = lang_ed.update(cx, |ed, cx| {
                                        if ed.panels.layout.open_settings_dropdown.as_deref()
                                            == Some("lang")
                                        {
                                            ed.panels.layout.open_settings_dropdown = None;
                                        } else {
                                            ed.panels.layout.open_settings_dropdown =
                                                Some("lang".to_string());
                                        }
                                        cx.notify();
                                    });
                                }
                            }),
                    );

                    if is_lang_open {
                        let mut menu_items = Vec::new();
                        for (code, label) in lang_options {
                            let is_selected = label == current_lang;
                            let item_ed = lang_ed.clone();

                            menu_items.push(
                                div()
                                    .id(ElementId::Name(format!("lang-item-{}", code).into()))
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .px(px(10.0))
                                    .py(px(6.0))
                                    .rounded(px(4.0))
                                    .bg(if is_selected {
                                        c.dialog_secondary_button_hover
                                    } else {
                                        c.dialog_surface
                                    })
                                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                                    .text_size(px(12.0))
                                    .text_color(c.text_default)
                                    .child(label)
                                    .child(if is_selected {
                                        svg()
                                            .path("icon/panel/check.svg")
                                            .size(px(13.0))
                                            .text_color(c.dialog_primary_button_bg)
                                            .into_any_element()
                                    } else {
                                        div().w(px(13.0)).into_any_element()
                                    })
                                    .on_click(move |_ev, _win, cx| {
                                        let _ = item_ed.update(cx, |ed, cx| {
                                            ed.panels.layout.open_settings_dropdown = None;
                                            cx.notify();
                                        });
                                    })
                                    .into_any_element(),
                            );
                        }

                        lang_btn_wrap = lang_btn_wrap.child(gpui::deferred(
                            div()
                                .absolute()
                                .top_full()
                                .right_0()
                                .mt(px(4.0))
                                .w(px(160.0))
                                .occlude()
                                .bg(c.dialog_surface)
                                .border_1()
                                .border_color(c.dialog_border)
                                .rounded(px(6.0))
                                .shadow_lg()
                                .p(px(4.0))
                                .flex()
                                .flex_col()
                                .gap(px(2.0))
                                .children(menu_items),
                        ));
                    }

                    sec1_items.push(make_row(
                        "Display Language",
                        "Select preferred language for editor UI and dialogs",
                        lang_btn_wrap.into_any_element(),
                        theme,
                        inner_border_color,
                    ));
                }

                let sec1_ed = cx.entity().downgrade();
                sections.push(make_section(
                    "pref-sec-theme",
                    "Visual Theme & Language",
                    is_sec1_expanded,
                    Box::new(move |_ev, _win, cx| {
                        let _ = sec1_ed.update(cx, |ed, cx| {
                            ed.panels.layout.toggle_settings_section(sec1_key);
                            cx.notify();
                        });
                    }),
                    sec1_items,
                    theme,
                ));

                // Section 2: Status Bar Options
                let sec2_key = "status_bar";
                let is_sec2_expanded = self
                    .panels
                    .layout
                    .settings_expanded_sections
                    .contains(sec2_key);
                let mut sec2_items = Vec::new();

                if is_sec2_expanded {
                    let sub1_ed = cx.entity().downgrade();
                    let ctrl_sb_main = Switch::new("switch-sb-main")
                        .checked(self.panels.layout.pref_show_status_bar)
                        .on_click(move |_ev, _win, cx| {
                            let _ = sub1_ed.update(cx, |ed, cx| {
                                ed.panels.layout.pref_show_status_bar =
                                    !ed.panels.layout.pref_show_status_bar;
                                cx.notify();
                            });
                        })
                        .into_any_element();

                    sec2_items.push(make_row(
                        "Status Bar Visibility",
                        "Show or hide the persistent bottom status bar across window",
                        ctrl_sb_main,
                        theme,
                        inner_border_color,
                    ));

                    let sub2_ed = cx.entity().downgrade();
                    let ctrl_sb_words = Switch::new("switch-sb-words")
                        .checked(self.panels.layout.pref_show_word_count)
                        .on_click(move |_ev, _win, cx| {
                            let _ = sub2_ed.update(cx, |ed, cx| {
                                ed.panels.layout.pref_show_word_count =
                                    !ed.panels.layout.pref_show_word_count;
                                cx.notify();
                            });
                        })
                        .into_any_element();

                    sec2_items.push(make_row(
                        "Word Count Badge",
                        "Display real-time document word count in status bar",
                        ctrl_sb_words,
                        theme,
                        inner_border_color,
                    ));

                    let sub3_ed = cx.entity().downgrade();
                    let ctrl_sb_pos = Switch::new("switch-sb-pos")
                        .checked(self.panels.layout.pref_show_cursor_pos)
                        .on_click(move |_ev, _win, cx| {
                            let _ = sub3_ed.update(cx, |ed, cx| {
                                ed.panels.layout.pref_show_cursor_pos =
                                    !ed.panels.layout.pref_show_cursor_pos;
                                cx.notify();
                            });
                        })
                        .into_any_element();

                    sec2_items.push(make_row(
                        "Cursor Position Badge",
                        "Display line and column coordinates in status bar",
                        ctrl_sb_pos,
                        theme,
                        inner_border_color,
                    ));

                    let sub4_ed = cx.entity().downgrade();
                    let ctrl_sb_sidebar = Switch::new("switch-sb-sidebar")
                        .checked(self.panels.layout.pref_show_sidebar_toggle)
                        .on_click(move |_ev, _win, cx| {
                            let _ = sub4_ed.update(cx, |ed, cx| {
                                ed.panels.layout.pref_show_sidebar_toggle =
                                    !ed.panels.layout.pref_show_sidebar_toggle;
                                cx.notify();
                            });
                        })
                        .into_any_element();

                    sec2_items.push(make_row(
                        "Sidebar Toggle Button",
                        "Display button to toggle file tree sidebar in status bar",
                        ctrl_sb_sidebar,
                        theme,
                        inner_border_color,
                    ));

                    let sub5_ed = cx.entity().downgrade();
                    let ctrl_sb_mode = Switch::new("switch-sb-mode")
                        .checked(self.panels.layout.pref_show_mode_switch)
                        .on_click(move |_ev, _win, cx| {
                            let _ = sub5_ed.update(cx, |ed, cx| {
                                ed.panels.layout.pref_show_mode_switch =
                                    !ed.panels.layout.pref_show_mode_switch;
                                cx.notify();
                            });
                        })
                        .into_any_element();

                    sec2_items.push(make_row(
                        "Mode Switch Button",
                        "Display button to switch Edit/Preview modes in status bar",
                        ctrl_sb_mode,
                        theme,
                        inner_border_color,
                    ));
                }

                let sec2_ed = cx.entity().downgrade();
                sections.push(make_section(
                    "pref-sec-sb",
                    "Status Bar Options",
                    is_sec2_expanded,
                    Box::new(move |_ev, _win, cx| {
                        let _ = sec2_ed.update(cx, |ed, cx| {
                            ed.panels.layout.toggle_settings_section(sec2_key);
                            cx.notify();
                        });
                    }),
                    sec2_items,
                    theme,
                ));
            }
            SettingsTab::Editing => {
                // Section 1: Typography & Formatting
                let sec1_key = "typography";
                let is_sec1_expanded = self
                    .panels
                    .layout
                    .settings_expanded_sections
                    .contains(sec1_key);
                let mut sec1_items = Vec::new();

                if is_sec1_expanded {
                    let font_dec = cx.entity().downgrade();
                    let font_inc = cx.entity().downgrade();
                    let font_ctr = cx.entity().downgrade();
                    let curr_size = self.panels.layout.pref_font_size;
                    let is_editing_font =
                        self.panels.layout.editing_settings_stepper.as_deref() == Some("font");

                    let ctrl_font = render_zed_stepper(
                        "font-dec",
                        "font-inc",
                        format!("{}", curr_size),
                        "px",
                        is_editing_font,
                        Box::new(move |_ev, _win, cx| {
                            let _ = font_dec.update(cx, |ed, cx| {
                                ed.panels.layout.editing_settings_stepper = None;
                                if ed.panels.layout.pref_font_size > 8 {
                                    ed.panels.layout.pref_font_size -= 1;
                                    cx.notify();
                                }
                            });
                        }),
                        Box::new(move |_ev, _win, cx| {
                            let _ = font_inc.update(cx, |ed, cx| {
                                ed.panels.layout.editing_settings_stepper = None;
                                if ed.panels.layout.pref_font_size < 48 {
                                    ed.panels.layout.pref_font_size += 1;
                                    cx.notify();
                                }
                            });
                        }),
                        Box::new(move |_ev, _win, cx| {
                            let _ = font_ctr.update(cx, |ed, cx| {
                                ed.panels.layout.editing_settings_stepper =
                                    Some("font".to_string());
                                ed.panels.layout.pref_font_size =
                                    match ed.panels.layout.pref_font_size {
                                        12 => 14,
                                        14 => 16,
                                        16 => 18,
                                        18 => 20,
                                        20 => 24,
                                        24 => 12,
                                        _ => 14,
                                    };
                                cx.notify();
                            });
                        }),
                        theme,
                    );

                    sec1_items.push(make_row(
                        "Editor Font Size",
                        "Baseline font size in pixels for text editor content",
                        ctrl_font,
                        theme,
                        inner_border_color,
                    ));

                    let lh_dec = cx.entity().downgrade();
                    let lh_inc = cx.entity().downgrade();
                    let lh_ctr = cx.entity().downgrade();
                    let curr_lh = self.panels.layout.pref_line_height;
                    let is_editing_lh = self.panels.layout.editing_settings_stepper.as_deref()
                        == Some("line_height");

                    let ctrl_lh = render_zed_stepper(
                        "lh-dec",
                        "lh-inc",
                        format!("{:.1}", curr_lh),
                        "",
                        is_editing_lh,
                        Box::new(move |_ev, _win, cx| {
                            let _ = lh_dec.update(cx, |ed, cx| {
                                ed.panels.layout.editing_settings_stepper = None;
                                if ed.panels.layout.pref_line_height > 1.05 {
                                    ed.panels.layout.pref_line_height =
                                        (ed.panels.layout.pref_line_height - 0.1).max(1.0);
                                    cx.notify();
                                }
                            });
                        }),
                        Box::new(move |_ev, _win, cx| {
                            let _ = lh_inc.update(cx, |ed, cx| {
                                ed.panels.layout.editing_settings_stepper = None;
                                if ed.panels.layout.pref_line_height < 3.0 {
                                    ed.panels.layout.pref_line_height =
                                        (ed.panels.layout.pref_line_height + 0.1).min(3.0);
                                    cx.notify();
                                }
                            });
                        }),
                        Box::new(move |_ev, _win, cx| {
                            let _ = lh_ctr.update(cx, |ed, cx| {
                                ed.panels.layout.editing_settings_stepper =
                                    Some("line_height".to_string());
                                ed.panels.layout.pref_line_height =
                                    if (ed.panels.layout.pref_line_height - 1.2).abs() < 0.05 {
                                        1.4
                                    } else if (ed.panels.layout.pref_line_height - 1.4).abs() < 0.05
                                    {
                                        1.6
                                    } else if (ed.panels.layout.pref_line_height - 1.6).abs() < 0.05
                                    {
                                        1.8
                                    } else if (ed.panels.layout.pref_line_height - 1.8).abs() < 0.05
                                    {
                                        2.0
                                    } else {
                                        1.2
                                    };
                                cx.notify();
                            });
                        }),
                        theme,
                    );

                    sec1_items.push(make_row(
                        "Line Height Multiplier",
                        "Adjust vertical line spacing ratio for reading comfort",
                        ctrl_lh,
                        theme,
                        inner_border_color,
                    ));
                }

                let sec1_ed = cx.entity().downgrade();
                sections.push(make_section(
                    "pref-sec-typo",
                    "Typography & Formatting",
                    is_sec1_expanded,
                    Box::new(move |_ev, _win, cx| {
                        let _ = sec1_ed.update(cx, |ed, cx| {
                            ed.panels.layout.toggle_settings_section(sec1_key);
                            cx.notify();
                        });
                    }),
                    sec1_items,
                    theme,
                ));

                // Section 2: Markdown & Assets
                let sec2_key = "markdown";
                let is_sec2_expanded = self
                    .panels
                    .layout
                    .settings_expanded_sections
                    .contains(sec2_key);
                let mut sec2_items = Vec::new();

                let img_ed = cx.entity().downgrade();
                let img_options = [
                    (0, "Save to Local Assets"),
                    (1, "Copy to Document Folder"),
                    (2, "Insert Direct Link"),
                ];
                let curr_img_idx = self.panels.layout.pref_image_paste_action % img_options.len();
                let curr_img_label = img_options[curr_img_idx].1;
                let is_img_open =
                    self.panels.layout.open_settings_dropdown.as_deref() == Some("image");

                if is_sec2_expanded {
                    let tbl_ed = cx.entity().downgrade();
                    let ctrl_tbl = Switch::new("switch-table-headers")
                        .checked(self.panels.layout.pref_show_table_headers)
                        .on_click(move |_ev, _win, cx| {
                            let _ = tbl_ed.update(cx, |ed, cx| {
                                ed.panels.layout.pref_show_table_headers =
                                    !ed.panels.layout.pref_show_table_headers;
                                crate::infra::config::settings::EditorSettings::set_show_table_headers(
                                    cx,
                                    ed.panels.layout.pref_show_table_headers,
                                );
                                cx.notify();
                            });
                        })
                        .into_any_element();

                    sec2_items.push(make_row(
                        "Table Column Headers",
                        "Automatically render header row when formatting markdown tables",
                        ctrl_tbl,
                        theme,
                        inner_border_color,
                    ));

                    let mut img_btn_wrap = div().relative().child(
                        div()
                            .id("pref-btn-img")
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .justify_between()
                            .w(px(145.0))
                            .h(px(28.0))
                            .px(px(8.0))
                            .rounded(px(d.menu_item_radius))
                            .bg(c.dialog_secondary_button_bg)
                            .hover(|this| this.bg(c.dialog_secondary_button_hover))
                            .border_1()
                            .border_color(c.dialog_border)
                            .text_size(px(12.0))
                            .text_color(c.text_default)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .truncate()
                                    .child(curr_img_label),
                            )
                            .child(
                                div().flex_shrink_0().pl(px(4.0)).child(
                                    svg()
                                        .path("icon/panel/select-chevron.svg")
                                        .size(px(14.0))
                                        .text_color(c.dialog_muted),
                                ),
                            )
                            .on_click({
                                let img_ed = img_ed.clone();
                                move |_ev, _win, cx| {
                                    let _ = img_ed.update(cx, |ed, cx| {
                                        if ed.panels.layout.open_settings_dropdown.as_deref()
                                            == Some("image")
                                        {
                                            ed.panels.layout.open_settings_dropdown = None;
                                        } else {
                                            ed.panels.layout.open_settings_dropdown =
                                                Some("image".to_string());
                                        }
                                        cx.notify();
                                    });
                                }
                            }),
                    );

                    if is_img_open {
                        let mut menu_items = Vec::new();
                        for (idx, label) in img_options {
                            let is_selected = idx == curr_img_idx;
                            let item_ed = img_ed.clone();

                            menu_items.push(
                                div()
                                    .id(ElementId::Name(format!("img-item-{}", idx).into()))
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .px(px(10.0))
                                    .py(px(6.0))
                                    .rounded(px(4.0))
                                    .bg(if is_selected {
                                        c.dialog_secondary_button_hover
                                    } else {
                                        c.dialog_surface
                                    })
                                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                                    .text_size(px(12.0))
                                    .text_color(c.text_default)
                                    .child(label)
                                    .child(if is_selected {
                                        svg()
                                            .path("icon/panel/check.svg")
                                            .size(px(13.0))
                                            .text_color(c.dialog_primary_button_bg)
                                            .into_any_element()
                                    } else {
                                        div().w(px(13.0)).into_any_element()
                                    })
                                    .on_click(move |_ev, _win, cx| {
                                        let _ = item_ed.update(cx, |ed, cx| {
                                            ed.panels.layout.pref_image_paste_action = idx;
                                            ed.panels.layout.open_settings_dropdown = None;
                                            cx.notify();
                                        });
                                    })
                                    .into_any_element(),
                            );
                        }

                        img_btn_wrap = img_btn_wrap.child(gpui::deferred(
                            div()
                                .absolute()
                                .top_full()
                                .right_0()
                                .mt(px(4.0))
                                .w(px(160.0))
                                .occlude()
                                .bg(c.dialog_surface)
                                .border_1()
                                .border_color(c.dialog_border)
                                .rounded(px(6.0))
                                .shadow_lg()
                                .p(px(4.0))
                                .flex()
                                .flex_col()
                                .gap(px(2.0))
                                .children(menu_items),
                        ));
                    }

                    sec2_items.push(make_row(
                        "Image Paste Action",
                        "Default storage location when pasting images into document",
                        img_btn_wrap.into_any_element(),
                        theme,
                        inner_border_color,
                    ));
                }

                let sec2_ed = cx.entity().downgrade();
                sections.push(make_section(
                    "pref-sec-md",
                    "Markdown & Assets",
                    is_sec2_expanded,
                    Box::new(move |_ev, _win, cx| {
                        let _ = sec2_ed.update(cx, |ed, cx| {
                            ed.panels.layout.toggle_settings_section(sec2_key);
                            cx.notify();
                        });
                    }),
                    sec2_items,
                    theme,
                ));

                // Section 3: Startup Behavior
                let sec3_key = "startup";
                let is_sec3_expanded = self
                    .panels
                    .layout
                    .settings_expanded_sections
                    .contains(sec3_key);
                let mut sec3_items = Vec::new();

                let startup_ed = cx.entity().downgrade();
                let startup_options = [(0, "New Blank Document"), (1, "Open Last Opened File")];
                let curr_startup_idx =
                    self.panels.layout.pref_startup_option % startup_options.len();
                let curr_startup_label = startup_options[curr_startup_idx].1;
                let is_startup_open =
                    self.panels.layout.open_settings_dropdown.as_deref() == Some("startup");

                if is_sec3_expanded {
                    let mut startup_btn_wrap = div().relative().child(
                        div()
                            .id("pref-btn-startup")
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .justify_between()
                            .w(px(145.0))
                            .h(px(28.0))
                            .px(px(8.0))
                            .rounded(px(d.menu_item_radius))
                            .bg(c.dialog_secondary_button_bg)
                            .hover(|this| this.bg(c.dialog_secondary_button_hover))
                            .border_1()
                            .border_color(c.dialog_border)
                            .text_size(px(12.0))
                            .text_color(c.text_default)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .truncate()
                                    .child(curr_startup_label),
                            )
                            .child(
                                div().flex_shrink_0().pl(px(4.0)).child(
                                    svg()
                                        .path("icon/panel/select-chevron.svg")
                                        .size(px(14.0))
                                        .text_color(c.dialog_muted),
                                ),
                            )
                            .on_click({
                                let startup_ed = startup_ed.clone();
                                move |_ev, _win, cx| {
                                    let _ = startup_ed.update(cx, |ed, cx| {
                                        if ed.panels.layout.open_settings_dropdown.as_deref()
                                            == Some("startup")
                                        {
                                            ed.panels.layout.open_settings_dropdown = None;
                                        } else {
                                            ed.panels.layout.open_settings_dropdown =
                                                Some("startup".to_string());
                                        }
                                        cx.notify();
                                    });
                                }
                            }),
                    );

                    if is_startup_open {
                        let mut menu_items = Vec::new();
                        for (idx, label) in startup_options {
                            let is_selected = idx == curr_startup_idx;
                            let item_ed = startup_ed.clone();

                            menu_items.push(
                                div()
                                    .id(ElementId::Name(format!("startup-item-{}", idx).into()))
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .px(px(10.0))
                                    .py(px(6.0))
                                    .rounded(px(4.0))
                                    .bg(if is_selected {
                                        c.dialog_secondary_button_hover
                                    } else {
                                        c.dialog_surface
                                    })
                                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                                    .text_size(px(12.0))
                                    .text_color(c.text_default)
                                    .child(label)
                                    .child(if is_selected {
                                        svg()
                                            .path("icon/panel/check.svg")
                                            .size(px(13.0))
                                            .text_color(c.dialog_primary_button_bg)
                                            .into_any_element()
                                    } else {
                                        div().w(px(13.0)).into_any_element()
                                    })
                                    .on_click(move |_ev, _win, cx| {
                                        let _ = item_ed.update(cx, |ed, cx| {
                                            ed.panels.layout.pref_startup_option = idx;
                                            ed.panels.layout.open_settings_dropdown = None;
                                            cx.notify();
                                        });
                                    })
                                    .into_any_element(),
                            );
                        }

                        startup_btn_wrap = startup_btn_wrap.child(gpui::deferred(
                            div()
                                .absolute()
                                .top_full()
                                .right_0()
                                .mt(px(4.0))
                                .w(px(160.0))
                                .occlude()
                                .bg(c.dialog_surface)
                                .border_1()
                                .border_color(c.dialog_border)
                                .rounded(px(6.0))
                                .shadow_lg()
                                .p(px(4.0))
                                .flex()
                                .flex_col()
                                .gap(px(2.0))
                                .children(menu_items),
                        ));
                    }

                    sec3_items.push(make_row(
                        "On Startup",
                        "Choose default document state when launching Velotype editor",
                        startup_btn_wrap.into_any_element(),
                        theme,
                        inner_border_color,
                    ));
                }

                let sec3_ed = cx.entity().downgrade();
                sections.push(make_section(
                    "pref-sec-startup",
                    "Startup Behavior",
                    is_sec3_expanded,
                    Box::new(move |_ev, _win, cx| {
                        let _ = sec3_ed.update(cx, |ed, cx| {
                            ed.panels.layout.toggle_settings_section(sec3_key);
                            cx.notify();
                        });
                    }),
                    sec3_items,
                    theme,
                ));
            }
            SettingsTab::Keymap => {
                // Section 1: Document Actions
                let sec1_key = "doc_actions";
                let is_sec1_expanded = self
                    .panels
                    .layout
                    .settings_expanded_sections
                    .contains(sec1_key);
                let mut sec1_items = Vec::new();

                if is_sec1_expanded {
                    let doc_shortcuts = [
                        (
                            "Save Document",
                            "Save active file changes to disk",
                            "Ctrl + S",
                        ),
                        (
                            "Save Document As",
                            "Save active document with a new name",
                            "Ctrl + Shift + S",
                        ),
                        (
                            "New Window",
                            "Open a new editor window instance",
                            "Ctrl + N",
                        ),
                        (
                            "Close Window",
                            "Close the currently focused editor window",
                            "Ctrl + W",
                        ),
                    ];

                    for (name, desc, sc) in doc_shortcuts.iter() {
                        let ctrl_sc = div()
                            .px(px(8.0))
                            .py(px(2.0))
                            .rounded(px(d.menu_item_radius))
                            .bg(c.dialog_secondary_button_hover)
                            .text_size(px(11.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(c.text_default)
                            .child(*sc)
                            .into_any_element();

                        sec1_items.push(make_row(*name, *desc, ctrl_sc, theme, inner_border_color));
                    }
                }

                let sec1_ed = cx.entity().downgrade();
                sections.push(make_section(
                    "pref-sec-doc-actions",
                    "Document Actions",
                    is_sec1_expanded,
                    Box::new(move |_ev, _win, cx| {
                        let _ = sec1_ed.update(cx, |ed, cx| {
                            ed.panels.layout.toggle_settings_section(sec1_key);
                            cx.notify();
                        });
                    }),
                    sec1_items,
                    theme,
                ));

                // Section 2: Interface & View Controls
                let sec2_key = "view_controls";
                let is_sec2_expanded = self
                    .panels
                    .layout
                    .settings_expanded_sections
                    .contains(sec2_key);
                let mut sec2_items = Vec::new();

                if is_sec2_expanded {
                    let view_shortcuts = [
                        (
                            "Toggle View Mode",
                            "Switch between Edit, Preview, and Dual view layouts",
                            "Ctrl + M",
                        ),
                        (
                            "Toggle Workspace Tree",
                            "Show or collapse the left file navigation sidebar",
                            "Ctrl + E",
                        ),
                        (
                            "Quit Application",
                            "Safely exit application and save session",
                            "Ctrl + Q",
                        ),
                    ];

                    for (name, desc, sc) in view_shortcuts.iter() {
                        let ctrl_sc = div()
                            .px(px(8.0))
                            .py(px(2.0))
                            .rounded(px(d.menu_item_radius))
                            .bg(c.dialog_secondary_button_hover)
                            .text_size(px(11.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(c.text_default)
                            .child(*sc)
                            .into_any_element();

                        sec2_items.push(make_row(*name, *desc, ctrl_sc, theme, inner_border_color));
                    }
                }

                let sec2_ed = cx.entity().downgrade();
                sections.push(make_section(
                    "pref-sec-view-controls",
                    "Interface & View Controls",
                    is_sec2_expanded,
                    Box::new(move |_ev, _win, cx| {
                        let _ = sec2_ed.update(cx, |ed, cx| {
                            ed.panels.layout.toggle_settings_section(sec2_key);
                            cx.notify();
                        });
                    }),
                    sec2_items,
                    theme,
                ));
            }
        }

        let right_content = div()
            .id("pref-right-content")
            .relative()
            .flex_1()
            .h_full()
            .p(px(14.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .children(sections);

        // --- Main Root Layout ---
        div()
            .w_full()
            .h_full()
            .flex()
            .flex_row()
            .bg(c.editor_background)
            .child(left_nav)
            .child(right_content)
            .into_any_element()
    }
}
