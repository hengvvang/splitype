//! Window-level tiled area layout — rendering and gestures for the outer
//! `WindowAreaKind` split tree (ExplorerState / Settings / Editor areas).
//!
//! The layout engine (tree, sessions, operations) lives in `crate::layout`;
//! the editor's inner panel layout rendering lives in
//! `crate::editor::panels::layout`. This module also aggregates the window
//! panel state ([`WindowPanels`]).

use crate::ui::components::popover::overlay;

use crate::ui::components::button::{icon_chip_button, small_pill_button};
use crate::ui::components::splitter::{splitter_bar_h, splitter_bar_v};

use crate::ui::components::menu_item::menu_item;
use crate::ui::components::popover::menu_panel;

use gpui::*;

use crate::editor::controller::*;
use crate::infra::i18n::I18nStrings;
use crate::layout::{
    AreaSplitMode, Axis, BorderMenuState, CornerDragModifier, CornerDragPreview, Direction,
    EditorInnerPanelDragAction, SplitTree, SplitterDragSession, WindowAreaDragAction,
    WindowAreaKind, WindowLayout,
};
use crate::theme::Theme;
use crate::windows::explorer::state::ExplorerState;
use crate::windows::settings::state::SettingsUiState;

impl Editor {
    pub(crate) fn render_tiled_layout(
        &mut self,
        theme: &Theme,
        strings: &I18nStrings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let root = self.panels.layout.window_area_tree.clone();
        let leaf_count = root.count_leaves();

        let layout_tree = if let Some(maximized_id) = self.panels.layout.maximized_window_area {
            if let Some(kind) = root.find_leaf_kind(maximized_id) {
                self.render_window_area_tile(
                    maximized_id,
                    kind,
                    theme,
                    strings,
                    leaf_count,
                    true,
                    window,
                    cx,
                )
            } else {
                self.render_window_area_node(
                    &root,
                    theme,
                    strings,
                    leaf_count,
                    window,
                    cx,
                )
            }
        } else {
            self.render_window_area_node(
                &root,
                theme,
                strings,
                leaf_count,
                window,
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
                    if let Some(drag) = ed.panels.layout.active_window_area_splitter_drag {
                        let current_pos = match drag.direction {
                            Axis::Horizontal => f32::from(pos.x),
                            Axis::Vertical => f32::from(pos.y),
                        };
                        let viewport = window.viewport_size();
                        let span = ed
                            .panels
                            .layout
                            .window_area_split_pixel_span(drag.split_id, viewport)
                            .unwrap_or_else(|| match drag.direction {
                                Axis::Horizontal => f32::from(viewport.width),
                                Axis::Vertical => f32::from(viewport.height),
                            });

                        if span > 1.0 {
                            let mut session = drag;
                            session.total_span = span;
                            ed.panels.layout.active_window_area_splitter_drag = Some(session);
                        }
                        ed.panels
                            .layout
                            .update_window_area_splitter_drag(current_pos);
                        changed = true;
                    } else if ed.panels.layout.active_window_area_corner_drag.is_some() {
                        let viewport = window.viewport_size();
                        let action = ed
                            .panels
                            .layout
                            .update_window_area_corner_drag(pos, viewport);
                        // Modifier actions still execute immediately.
                        if let Some(action) = action {
                            match action {
                                WindowAreaDragAction::Swap { from, to } => {
                                    ed.panels.layout.end_window_area_corner_drag();
                                    ed.panels.layout.swap_window_area_kinds(from, to);
                                }
                                // Shift + Settings corner: open the floating
                                // settings window (same as the app menu).
                                WindowAreaDragAction::OpenSettings => {
                                    ed.panels.layout.end_window_area_corner_drag();
                                    crate::windows::settings::open_settings_window(cx);
                                }
                                WindowAreaDragAction::Cancel => {
                                    ed.panels.layout.end_window_area_corner_drag();
                                }
                                _ => {} // Split/Join handled on mouse_up
                            }
                        }
                        changed = true;
                    } else if ed
                        .panels
                        .layout
                        .active_editor_inner_panel_splitter_drag
                        .is_some()
                    {
                        let (area_id, drag) = ed
                            .panels
                            .layout
                            .active_editor_inner_panel_splitter_drag
                            .unwrap();
                        let viewport = window.viewport_size();
                        let outer_rects = ed.panels.layout.window_area_rects(viewport);
                        if let Some(outer_rect) =
                            ed.panels.layout.window_area_rect(area_id, &outer_rects)
                        {
                            let current_pos = match drag.direction {
                                Axis::Horizontal => f32::from(pos.x) - outer_rect.x,
                                Axis::Vertical => f32::from(pos.y) - outer_rect.y,
                            };
                            let inner_size = size(px(outer_rect.width), px(outer_rect.height));
                            let span = ed
                                .panels
                                .layout
                                .editor_inner_panel_split_pixel_span(
                                    area_id,
                                    drag.split_id,
                                    inner_size,
                                )
                                .unwrap_or_else(|| match drag.direction {
                                    Axis::Horizontal => outer_rect.width,
                                    Axis::Vertical => outer_rect.height,
                                });
                            if span > 1.0 {
                                let mut session = drag;
                                session.total_span = span;
                                ed.panels.layout.active_editor_inner_panel_splitter_drag =
                                    Some((area_id, session));
                            }
                            ed.panels
                                .layout
                                .update_editor_inner_panel_splitter_drag(area_id, current_pos);
                            changed = true;
                        }
                    } else if ed
                        .panels
                        .layout
                        .active_editor_inner_panel_corner_drag
                        .is_some()
                    {
                        let (area_id, drag) = ed
                            .panels
                            .layout
                            .active_editor_inner_panel_corner_drag
                            .unwrap();
                        let viewport = window.viewport_size();
                        let outer_rects = ed.panels.layout.window_area_rects(viewport);
                        if let Some(outer_rect) =
                            ed.panels.layout.window_area_rect(area_id, &outer_rects)
                        {
                            let mut session = drag;
                            let inner_pos = point(
                                px(f32::from(pos.x) - outer_rect.x),
                                px(f32::from(pos.y) - outer_rect.y),
                            );
                            let inner_size = size(px(outer_rect.width), px(outer_rect.height));
                            let start_x = f32::from(session.start_pos.x);
                            let start_y = f32::from(session.start_pos.y);
                            if start_x > outer_rect.width || start_y > outer_rect.height {
                                session.start_pos =
                                    point(px(start_x - outer_rect.x), px(start_y - outer_rect.y));
                                ed.panels.layout.active_editor_inner_panel_corner_drag =
                                    Some((area_id, session));
                            }
                            let action = ed.panels.layout.update_editor_inner_panel_corner_drag(
                                area_id, inner_pos, inner_size,
                            );
                            if let Some(action) = action {
                                match action {
                                    EditorInnerPanelDragAction::Swap { from, to } => {
                                        ed.panels.layout.end_editor_inner_panel_corner_drag();
                                        ed.panels
                                            .layout
                                            .swap_editor_inner_panel_kinds(area_id, from, to);
                                    }
                                    EditorInnerPanelDragAction::Duplicate { .. } => {
                                        ed.panels.layout.end_editor_inner_panel_corner_drag();
                                    }
                                    EditorInnerPanelDragAction::Cancel => {
                                        ed.panels.layout.end_editor_inner_panel_corner_drag();
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
                    if ed.panels.layout.active_window_area_splitter_drag.is_some() {
                        ed.panels.layout.end_window_area_splitter_drag();
                        cx.notify();
                    }
                    if ed.panels.layout.active_window_area_corner_drag.is_some() {
                        match ed.panels.layout.finish_window_area_corner_drag() {
                            // Corner-drag split: same-kind area; Editor areas
                            // seed the new area per `mode` (deep-copied tab
                            // list + cloned inner layout, or a blank editor).
                            Some(WindowAreaDragAction::Split {
                                area_id,
                                direction,
                                ratio,
                                mode,
                            }) => {
                                ed.split_area(area_id, direction, ratio, mode, cx);
                            }
                            Some(WindowAreaDragAction::Join { from_area, into_area }) => {
                                ed.panels.layout.join_window_area(into_area, from_area);
                            }
                            Some(WindowAreaDragAction::Swap { from, to }) => {
                                ed.panels.layout.swap_window_area_kinds(from, to);
                            }
                            _ => {}
                        }
                        cx.notify();
                    }
                    if ed
                        .panels
                        .layout
                        .active_editor_inner_panel_splitter_drag
                        .is_some()
                    {
                        ed.panels.layout.end_editor_inner_panel_splitter_drag();
                        cx.notify();
                    }
                    if ed
                        .panels
                        .layout
                        .active_editor_inner_panel_corner_drag
                        .is_some()
                    {
                        match ed.panels.layout.finish_editor_inner_panel_corner_drag() {
                            Some((
                                area_id,
                                EditorInnerPanelDragAction::Split {
                                    panel_id,
                                    direction,
                                    ratio,
                                },
                            )) => {
                                ed.panels.layout.split_editor_inner_panel_with_ratio(
                                    area_id, panel_id, direction, ratio,
                                );
                            }
                            Some((
                                area_id,
                                EditorInnerPanelDragAction::Join {
                                    from_panel,
                                    into_panel,
                                },
                            )) => {
                                ed.panels
                                    .layout
                                    .join_editor_inner_panel(area_id, into_panel, from_panel);
                            }
                            _ => {}
                        }
                        cx.notify();
                    }
                });
            })
            .child(layout_tree);

        // Build the preview overlay for corner drag gestures.
        let preview_overlay = if let Some(drag) = &self.panels.layout.active_window_area_corner_drag
        {
            match drag.preview {
                CornerDragPreview::SplitPreview { direction, ratio } => {
                    // Calculate the pixel rect of the leaf being split.
                    let viewport = window.viewport_size();
                    let leaf_rects = self.panels.layout.window_area_rects(viewport);
                    let leaf_rect = self
                        .panels
                        .layout
                        .window_area_rect(drag.target_id, &leaf_rects);

                    if let Some(leaf_rect) = leaf_rect {
                        // Horizontal split = left|right → draw a VERTICAL line
                        // Vertical split = top|bottom → draw a HORIZONTAL line
                        let line = match direction {
                            Axis::Horizontal => div()
                                .absolute()
                                .left(px(leaf_rect.x + leaf_rect.width * ratio))
                                .top(px(leaf_rect.y))
                                .w(px(3.0))
                                .h(px(leaf_rect.height))
                                .bg(hsla(0.36, 0.73, 0.57, 0.8)),
                            Axis::Vertical => div()
                                .absolute()
                                .top(px(leaf_rect.y + leaf_rect.height * ratio))
                                .left(px(leaf_rect.x))
                                .h(px(3.0))
                                .w(px(leaf_rect.width))
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
                                        .left(px(leaf_rect.x))
                                        .top(px(leaf_rect.y))
                                        .w(px(leaf_rect.width))
                                        .h(px(leaf_rect.height))
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
                    target_id,
                    direction,
                } => {
                    let viewport = window.viewport_size();
                    let leaf_rects = self.panels.layout.window_area_rects(viewport);
                    let target_rect = self
                        .panels
                        .layout
                        .window_area_rect(target_id, &leaf_rects);

                    if let Some(target_rect) = target_rect {
                        let arrow_symbol = match direction {
                            Direction::Up => "▲",
                            Direction::Down => "▼",
                            Direction::Right => "▶",
                            Direction::Left => "◀",
                        };

                        Some(
                            div()
                                .absolute()
                                .left(px(target_rect.x))
                                .top(px(target_rect.y))
                                .w(px(target_rect.width))
                                .h(px(target_rect.height))
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
        let inner_preview_overlay = if let Some((area_id, ref drag)) =
            self.panels.layout.active_editor_inner_panel_corner_drag
        {
            match drag.preview {
                CornerDragPreview::SplitPreview { direction, ratio } => {
                    let viewport = window.viewport_size();
                    let outer_rects = self.panels.layout.window_area_rects(viewport);
                    if let Some(outer_rect) =
                        self.panels.layout.window_area_rect(area_id, &outer_rects)
                    {
                        let line = match direction {
                            Axis::Horizontal => div()
                                .absolute()
                                .left(px(outer_rect.x + outer_rect.width * ratio))
                                .top(px(outer_rect.y))
                                .w(px(3.0))
                                .h(px(outer_rect.height))
                                .bg(hsla(0.36, 0.73, 0.57, 0.8)),
                            Axis::Vertical => div()
                                .absolute()
                                .top(px(outer_rect.y + outer_rect.height * ratio))
                                .left(px(outer_rect.x))
                                .h(px(3.0))
                                .w(px(outer_rect.width))
                                .bg(hsla(0.36, 0.73, 0.57, 0.8)),
                        };
                        Some(
                            div()
                                .absolute()
                                .inset(px(0.0))
                                .child(
                                    div()
                                        .absolute()
                                        .left(px(outer_rect.x))
                                        .top(px(outer_rect.y))
                                        .w(px(outer_rect.width))
                                        .h(px(outer_rect.height))
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
                    target_id,
                    direction,
                } => {
                    let viewport = window.viewport_size();
                    let outer_rects = self.panels.layout.window_area_rects(viewport);
                    if let Some(outer_rect) =
                        self.panels.layout.window_area_rect(area_id, &outer_rects)
                    {
                        let inner_size = size(px(outer_rect.width), px(outer_rect.height));
                        let inner_rects = self
                            .panels
                            .layout
                            .editor_inner_panel_rects(area_id, inner_size);
                        if let Some(inner_rect) = self
                            .panels
                            .layout
                            .window_area_rect(target_id, &inner_rects)
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
                                    .left(px(outer_rect.x + inner_rect.x))
                                    .top(px(outer_rect.y + inner_rect.y))
                                    .w(px(inner_rect.width))
                                    .h(px(inner_rect.height))
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

        if let Some(border_menu) = self.panels.layout.active_window_area_border_menu {
            let menu_overlay = self.render_window_area_border_menu(border_menu, theme, cx);
            container.child(menu_overlay).into_any_element()
        } else {
            container.into_any_element()
        }
    }
    pub(crate) fn render_window_area_node(
        &mut self,
        node: &crate::layout::SplitTree<crate::layout::WindowAreaKind>,
        theme: &Theme,
        strings: &I18nStrings,
        leaf_count: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let editor = cx.entity().downgrade();

        match node {
            SplitTree::Leaf { id, kind } => self.render_window_area_tile(
                *id,
                *kind,
                theme,
                strings,
                leaf_count,
                false,
                window,
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

                let first_elem = self.render_window_area_node(
                    first,
                    theme,
                    strings,
                    leaf_count,
                    window,
                    cx,
                );
                let second_elem = self.render_window_area_node(
                    second,
                    theme,
                    strings,
                    leaf_count,
                    window,
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
                                splitter_bar_h(("tiled-splitter-bar-h", split_id), c)
                                    .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                                        let start_pos = f32::from(event.position.x);
                                        let _ = bar_editor.update(cx, |ed, cx| {
                                            ed.panels.layout.active_window_area_splitter_drag =
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
                                                ed.panels.layout.active_window_area_border_menu =
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
                                splitter_bar_v(("tiled-splitter-bar-v", split_id), c)
                                    .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                                        let start_pos = f32::from(event.position.y);
                                        let _ = bar_editor.update(cx, |ed, cx| {
                                            ed.panels.layout.active_window_area_splitter_drag =
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
                                                ed.panels.layout.active_window_area_border_menu =
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
    pub(crate) fn render_window_area_tile(
        &mut self,
        leaf_id: usize,
        kind: crate::layout::WindowAreaKind,
        theme: &Theme,
        strings: &I18nStrings,
        leaf_count: usize,
        is_maximized: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let gap = d.area_tile_gap;
        let radius = d.area_tile_radius;

        let header =
            self.render_window_area_header(leaf_id, kind, theme, leaf_count, is_maximized, cx);

        let body: AnyElement = match kind {
            WindowAreaKind::Editor => {
                self.render_editor_inner_panel_container(leaf_id, theme, strings, window, cx)
            }
            WindowAreaKind::Explorer => self.render_explorer_panel(leaf_id, theme, strings, cx),
            WindowAreaKind::Settings => self.render_tiled_settings_panel(leaf_id, theme, strings, cx),
        };

        let panel_status_bar = match kind {
            WindowAreaKind::Editor => {
                Some(self.render_editor_inner_panel_status_bar(leaf_id, kind, theme, strings, cx))
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
        // Mouse interaction with any part of the tile marks it as the focused
        // window area (visible via the focus border); Editor tiles additionally
        // become the active editor.
        let is_focused_area = self.panels.layout.focused_window_area == Some(leaf_id);
        let tile_focus = cx.entity().downgrade();
        let mut tile_card = div()
            .id(("tiled-area-card", leaf_id))
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .relative()
            .rounded(px(radius))
            .bg(c.dialog_surface)
            .border(px(if is_focused_area {
                d.dialog_border_width.max(1.5)
            } else {
                d.dialog_border_width
            }))
            .border_color(if is_focused_area {
                c.selection
            } else {
                c.dialog_border
            })
            .shadow_lg()
            .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                let _ = tile_focus.update(cx, |ed, cx| {
                    ed.panels.layout.focused_window_area = Some(leaf_id);
                    if kind == crate::layout::WindowAreaKind::Editor {
                        ed.panels.layout.activate_editor_area(leaf_id);
                    }
                    cx.notify();
                });
            })
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
                    ed.panels
                        .layout
                        .start_window_area_corner_drag(leaf_id, pos, modifier);
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

        let dropdown_open = self.panels.layout.open_window_area_dropdown == Some(leaf_id);
        if dropdown_open {
            let menu = self.render_window_area_dropdown_menu(leaf_id, kind, theme, cx);
            wrapped = wrapped.child(menu);
        }

        wrapped.into_any_element()
    }
    pub(crate) fn render_window_area_header(
        &mut self,
        leaf_id: usize,
        kind: crate::layout::WindowAreaKind,
        theme: &Theme,
        leaf_count: usize,
        is_maximized: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let editor = cx.entity().downgrade();

        let type_editor = editor.clone();
        let type_button = small_pill_button(c, d)
            .id(("area-header-type", leaf_id))
            .text_size(px(12.0))
            .text_color(c.text_default)
            .child(kind.name().to_string())
            .on_click(move |_event, _window, cx| {
                let _ = type_editor.update(cx, |ed, cx| {
                    ed.panels.layout.toggle_window_area_dropdown(leaf_id);
                    cx.notify();
                });
            });

        let split_h_editor = editor.clone();
        let split_h_button = icon_chip_button(c, d)
            .id(("area-btn-split-h", leaf_id))
            .child(
                svg()
                    .path("icon/panel/split-h.svg")
                    .size(px(14.0))
                    .text_color(c.dialog_muted),
            )
            .on_click(move |_event, _window, cx| {
                let _ = split_h_editor.update(cx, |ed, cx| {
                    // Same-kind split; Editor areas deep-copy their tabs.
                    ed.split_area(leaf_id, Axis::Horizontal, 0.5, AreaSplitMode::Copy, cx);
                    cx.notify();
                });
            });

        let split_v_editor = editor.clone();
        let split_v_button = icon_chip_button(c, d)
            .id(("area-btn-split-v", leaf_id))
            .child(
                svg()
                    .path("icon/panel/split-v.svg")
                    .size(px(14.0))
                    .text_color(c.dialog_muted),
            )
            .on_click(move |_event, _window, cx| {
                let _ = split_v_editor.update(cx, |ed, cx| {
                    // Same-kind split; Editor areas deep-copy their tabs.
                    ed.split_area(leaf_id, Axis::Vertical, 0.5, AreaSplitMode::Copy, cx);
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
            let max_button = icon_chip_button(c, d)
                .id(("area-btn-max", leaf_id))
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
                        ed.panels.layout.toggle_window_area_maximize(leaf_id);
                        cx.notify();
                    });
                });

            let close_editor = editor.clone();
            let close_button = icon_chip_button(c, d)
                .id(("area-btn-close", leaf_id))
                .child(
                    svg()
                        .path("icon/titlebar/chrome-close.svg")
                        .size(px(14.0))
                        .text_color(c.dialog_muted),
                )
                .on_click(move |_event, _window, cx| {
                    let _ = close_editor.update(cx, |ed, cx| {
                        ed.panels.layout.close_window_area(leaf_id);
                        cx.notify();
                    });
                });

            actions = actions.child(max_button).child(close_button);
        }

        // Build tab bar for Edit areas from THAT editor's own tab set.
        let mut left_section = div().flex().items_center().gap(px(8.0)).child(type_button);

        if kind == WindowAreaKind::Editor {
            let list = self.tab_list_for(leaf_id);
            let active_tab = list.active_tab;
            let tab_names: Vec<String> = list
                .tabs
                .iter()
                .map(|tab| {
                    tab.file
                        .path
                        .as_ref()
                        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                        .unwrap_or_else(|| "Untitled".to_string())
                })
                .collect();

            let mut tab_elements: Vec<AnyElement> = Vec::new();
            for (index, file_name) in tab_names.iter().enumerate() {
                let is_active = index == active_tab;

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

                let tab_editor = editor.clone();
                let close_editor = editor.clone();

                tab_elements.push(
                    small_pill_button(c, d)
                        .px(px(6.0))
                        .bg(tab_bg)
                        .text_size(px(11.0))
                        .child(
                            // Switch area: clicking the file name switches to this tab.
                            div()
                                .text_color(tab_text)
                                .child(file_name.clone())
                                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                                    let _ = tab_editor.update(cx, |ed, cx| {
                                        ed.with_current_tab_area(leaf_id, |ed| {
                                            ed.panels.layout.activate_editor_area(leaf_id);
                                            ed.activate_tab(leaf_id, index, cx);
                                        });
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
                                    let _ = close_editor.update(cx, |ed, cx| {
                                        ed.with_current_tab_area(leaf_id, |ed| {
                                            ed.panels.layout.activate_editor_area(leaf_id);
                                            ed.close_tab(leaf_id, index, cx);
                                        });
                                        cx.notify();
                                    });
                                }),
                        )
                        .into_any_element(),
                );
            }

            // "+" button opens a fresh untitled tab in THIS editor.
            let add_editor = editor.clone();
            tab_elements.push(
                div()
                    .size(px(18.0))
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
                        let _ = add_editor.update(cx, |ed, cx| {
                            ed.with_current_tab_area(leaf_id, |ed| {
                                ed.panels.layout.activate_editor_area(leaf_id);
                                ed.new_untitled_tab(leaf_id, cx);
                            });
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
            .child(left_section)
            .child(div().flex().items_center().gap(px(6.0)).child(actions))
            .into_any_element()
    }
    pub(crate) fn render_window_area_dropdown_menu(
        &mut self,
        leaf_id: usize,
        current_type: crate::layout::WindowAreaKind,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let editor = cx.entity().downgrade();

        let available_types = WindowAreaKind::all();

        menu_panel(c, d)
            .id(("area-dropdown-overlay", leaf_id))
            .absolute()
            .occlude()
            .top(px(28.0))
            .left(px(8.0))
            .w(px(d.menu_panel_width))
            .children(available_types.iter().enumerate().map(|(idx, kind)| {
                let kind = *kind;
                let is_current = kind == current_type;
                let option_editor = editor.clone();
                menu_item(("area-type-opt", idx), c, d)
                    .w_full()
                    .justify_between()
                    .bg(if is_current {
                        c.panel_row_selected
                    } else {
                        c.dialog_surface
                    })
                    .text_size(px(d.menu_text_size))
                    .font_weight(t.dialog_button_weight.to_font_weight())
                    .text_color(c.dialog_secondary_button_text)
                    .child(kind.name())
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
                            ed.panels.layout.change_window_area_kind(leaf_id, kind);
                            cx.notify();
                        });
                    })
                    .into_any_element()
            }))
            .into_any_element()
    }
    pub(crate) fn render_window_area_border_menu(
        &mut self,
        border_menu: crate::layout::BorderMenuState,
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

        overlay()
            .id("tiled-border-context-menu-wrapper")
            .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                let _ = dismiss_ed.update(cx, |ed, cx| {
                    ed.panels.layout.active_window_area_border_menu = None;
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
                        menu_item("border-menu-split-h", c, d)
                            .text_size(px(d.menu_text_size))
                            .font_weight(t.dialog_body_weight.to_font_weight())
                            .text_color(c.dialog_secondary_button_text)
                            .child("Split Horizontally")
                            .on_click(move |_event, _window, cx| {
                                let _ = split_h_ed.update(cx, |ed, cx| {
                                    ed.split_area(
                                        split_id,
                                        Axis::Horizontal,
                                        0.5,
                                        AreaSplitMode::Copy,
                                        cx,
                                    );
                                    cx.notify();
                                });
                            }),
                    )
                    .child(
                        menu_item("border-menu-split-v", c, d)
                            .text_size(px(d.menu_text_size))
                            .font_weight(t.dialog_body_weight.to_font_weight())
                            .text_color(c.dialog_secondary_button_text)
                            .child("Split Vertically")
                            .on_click(move |_event, _window, cx| {
                                let _ = split_v_ed.update(cx, |ed, cx| {
                                    ed.split_area(
                                        split_id,
                                        Axis::Vertical,
                                        0.5,
                                        AreaSplitMode::Copy,
                                        cx,
                                    );
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
                        menu_item("border-menu-swap", c, d)
                            .text_size(px(d.menu_text_size))
                            .font_weight(t.dialog_body_weight.to_font_weight())
                            .text_color(c.dialog_secondary_button_text)
                            .child("Swap Panels")
                            .on_click(move |_event, _window, cx| {
                                let _ = swap_ed.update(cx, |ed, cx| {
                                    ed.panels.layout.swap_window_area_split_sides(split_id);
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
                        menu_item("border-menu-close", c, d)
                            .text_size(px(d.menu_text_size))
                            .font_weight(t.dialog_body_weight.to_font_weight())
                            .text_color(c.dialog_secondary_button_text)
                            .child("Close Area")
                            .on_click(move |_event, _window, cx| {
                                let _ = close_ed.update(cx, |ed, cx| {
                                    ed.panels.layout.close_window_area(split_id);
                                    cx.notify();
                                });
                            }),
                    ),
            )
            .into_any_element()
    }
}

// ---------------------------------------------------------------------------
// Window panels aggregate
// ---------------------------------------------------------------------------

/// Sidebar and tiled-layout state of the editor window.
///
/// Pure state records; their rendering lives in `src/windows`.
#[derive(Default)]
pub struct WindowPanels {
    pub(crate) explorer: ExplorerState,
    pub(crate) layout: WindowLayout,
    pub(crate) settings: SettingsUiState,
}
