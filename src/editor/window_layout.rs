//! Window-level tiled area layout — rendering and gestures for the outer
//! `WindowAreaKind` split tree (ExplorerState / Settings / Editor areas).
//!
//! The layout engine (tree, sessions, operations) lives in `crate::splitter`;
//! the editor's inner panel layout rendering lives in
//! `crate::editor::panel_layout`. This module also aggregates the editor
//! window's panel state ([`WindowPanels`]).

use crate::ui::menu_item::menu_item;
use crate::ui::popover::menu_panel;

use gpui::*;

use crate::app::window_area::WindowAreaKind;
use crate::app::window_area::WindowLayout;
use crate::editor::corner_drag_preview::render_corner_drag_preview;
use crate::editor::explorer::state::ExplorerState;
use crate::editor::outline::state::OutlinePanelState;
use crate::editor::settings::SettingsUiState;
use crate::infra::i18n::I18nStrings;
use crate::infra::theme::Theme;
use crate::splitter::{Axis, CornerDragModifier};
use splitype_splitter::container::SplitterContainer;
use splitype_splitter::policy::DragPolicy;
use splitype_splitter::sessions::{MODIFIER_THRESHOLD_PX, id_at_point};
use splitype_splitter::tree::SplitTree;

use super::controller::*;

/// Icon path for a window-area top-bar button, per area kind.
///
/// Every `WindowAreaKind` owns its own copies of the top-bar icons
/// (decoupling — see `assets/icons/README.md`), so a button's asset
/// path depends on the kind of the area it renders in.
///
/// NOTE: the on-disk icon directories are still named `titlebar/` /
/// `statusbar/`; they move to `topbar/` / `bottombar/` in the asset
/// rename pass that follows this refactor.
pub(crate) fn area_topbar_icon(kind: WindowAreaKind, name: &str) -> SharedString {
    let dir = match kind {
        WindowAreaKind::Explorer => "explorer",
        WindowAreaKind::Editor => "editor",
        WindowAreaKind::Settings => "settings",
    };
    format!("icons/{dir}/topbar/{name}.svg").into()
}

/// Map a theme to the layout crate's border-menu style parameters.
///
/// Shared by the outer window-area border menu and the editor's inner-panel
/// border menu so both render identically.
pub(crate) fn border_menu_style(theme: &Theme) -> crate::splitter::interaction::MenuStyle {
    let c = &theme.colors;
    let d = &theme.dimensions;
    let t = &theme.typography;
    crate::splitter::interaction::MenuStyle {
        surface: c.dialog_surface,
        border: c.dialog_border,
        border_width: d.dialog_border_width,
        radius: d.menu_panel_radius,
        width: d.menu_panel_width,
        padding: d.menu_panel_padding,
        gap: d.menu_panel_gap,
        text: c.dialog_secondary_button_text,
        text_size: d.menu_text_size,
        text_weight: t.dialog_body_weight.to_font_weight(),
        item_height: d.menu_item_height,
        item_padding_x: d.menu_item_padding_x,
        item_radius: d.menu_item_radius,
        item_hover: c.panel_row_hover,
        separator_margin_x: d.menu_separator_margin_x,
        separator_margin_y: d.menu_separator_margin_y,
        separator_height: d.menu_separator_height,
    }
}
impl Editor {
    pub(crate) fn render_tiled_layout(
        &mut self,
        theme: &Theme,
        strings: &I18nStrings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let root = self.panels.layout.tree.clone();
        let leaf_count = root.count_leaves();

        // The maximized flag lives on its panel (panel-level state).
        let maximized_id = {
            let mut ids = Vec::new();
            root.leaf_ids(&mut ids);
            ids.into_iter()
                .find(|id| root.find_leaf(*id).is_some_and(|p| p.maximized))
        };
        let layout_tree = if let Some(maximized_id) = maximized_id {
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
                self.render_window_area_node(&root, theme, strings, leaf_count, window, cx)
            }
        } else {
            self.render_window_area_node(&root, theme, strings, leaf_count, window, cx)
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
                    let viewport = window.viewport_size();
                    if splitype_splitter::interaction::update_window_drag(
                        &mut ed.panels.layout,
                        pos,
                        viewport,
                    ) {
                        changed = true;
                    }
                    // Outer gesture shortcuts (host-owned, immediate):
                    // Ctrl past the threshold swaps the dragged area with
                    // the hovered one and ends the gesture. Shift drags
                    // defer to the drag policy on mouse-up (clone window);
                    // they never show the visual indicator.
                    // The corner-drag session lives on the dragging panel
                    // itself; find it via the root.
                    if let Some(drag_panel) = ed.panels.layout.corner_drag_panel() {
                        let drag = ed
                            .panels
                            .layout
                            .tree
                            .find_leaf(drag_panel)
                            .and_then(|p| p.active_corner_drag);
                        if let Some(drag) = drag {
                            let dx = f32::from(pos.x - drag.start_pos.x);
                            let dy = f32::from(pos.y - drag.start_pos.y);
                            let dist = (dx * dx + dy * dy).sqrt();
                            if dist >= MODIFIER_THRESHOLD_PX
                                && drag.modifier == CornerDragModifier::Ctrl
                            {
                                let rects = ed.panels.layout.leaf_rects(viewport);
                                if let Some(over) = id_at_point(&rects, pos) {
                                    if over != drag.target_id {
                                        ed.swap_window_area_kinds(drag.target_id, over);
                                    }
                                }
                                ed.panels.layout.end_corner_drag();
                            }
                        }
                    }
                    // Inner splitter drag: locate the session whose container
                    // holds the active drag, and drive it through the shared
                    // container API.
                    if let Some((area_id, session)) = ed
                        .editor_sessions
                        .iter_mut()
                        .find(|(_, s)| s.root.active_splitter_drag.is_some())
                    {
                        let drag = session.root.active_splitter_drag.unwrap();
                        let viewport = window.viewport_size();
                        let outer_rects = ed.panels.layout.leaf_rects(viewport);
                        if let Some(outer_rect) = ed.panels.layout.leaf_rect(*area_id, &outer_rects)
                        {
                            let current_pos = match drag.direction {
                                Axis::Horizontal => f32::from(pos.x) - outer_rect.x,
                                Axis::Vertical => f32::from(pos.y) - outer_rect.y,
                            };
                            let inner_size = size(px(outer_rect.width), px(outer_rect.height));
                            let span = session
                                .root
                                .split_pixel_span(drag.split_id, inner_size)
                                .unwrap_or_else(|| match drag.direction {
                                    Axis::Horizontal => outer_rect.width,
                                    Axis::Vertical => outer_rect.height,
                                });
                            if span > 1.0 {
                                let mut refreshed = drag;
                                refreshed.total_span = span;
                                session.root.active_splitter_drag = Some(refreshed);
                            }
                            session.root.update_splitter_drag(current_pos);
                            changed = true;
                        }
                    } else if let Some((area_id, session)) = ed
                        .editor_sessions
                        .iter_mut()
                        .find(|(_, s)| s.root.corner_drag_panel().is_some())
                    {
                        let area_id = *area_id;
                        let drag_panel = session.root.corner_drag_panel().unwrap();
                        let drag = session
                            .root
                            .tree
                            .find_leaf(drag_panel)
                            .and_then(|p| p.active_corner_drag)
                            .unwrap();
                        let viewport = window.viewport_size();
                        let outer_rects = ed.panels.layout.leaf_rects(viewport);
                        let mut pending_swap: Option<(usize, usize)> = None;
                        if let Some(outer_rect) = ed.panels.layout.leaf_rect(area_id, &outer_rects)
                        {
                            let mut updated = drag;
                            let inner_pos = point(
                                px(f32::from(pos.x) - outer_rect.x),
                                px(f32::from(pos.y) - outer_rect.y),
                            );
                            let inner_size = size(px(outer_rect.width), px(outer_rect.height));
                            let start_x = f32::from(updated.start_pos.x);
                            let start_y = f32::from(updated.start_pos.y);
                            if start_x > outer_rect.width || start_y > outer_rect.height {
                                updated.start_pos =
                                    point(px(start_x - outer_rect.x), px(start_y - outer_rect.y));
                            }
                            // Write the corrected start pos back onto the
                            // panel's own session, then let the root
                            // update the facts (hover, direction).
                            if let Some(panel) =
                                session.root.tree.find_leaf_mut(drag_panel)
                            {
                                panel.active_corner_drag = Some(updated);
                            }
                            session.root.update_corner_drag(inner_pos, inner_size);
                            // Inner gesture shortcuts (host-owned,
                            // immediate): Ctrl past the threshold swaps
                            // the dragged panel with the hovered one;
                            // Shift past the threshold ends the gesture as
                            // a no-op. Plain drags defer to the inner drag
                            // policy on mouse-up.
                            let drag = session
                                .root
                                .tree
                                .find_leaf(drag_panel)
                                .and_then(|p| p.active_corner_drag);
                            if let Some(drag) = drag {
                                let dx = f32::from(inner_pos.x - drag.start_pos.x);
                                let dy = f32::from(inner_pos.y - drag.start_pos.y);
                                let dist = (dx * dx + dy * dy).sqrt();
                                if dist >= MODIFIER_THRESHOLD_PX {
                                    match drag.modifier {
                                        CornerDragModifier::Ctrl => {
                                            let rects = session.root.leaf_rects(inner_size);
                                            if let Some(over) = id_at_point(&rects, inner_pos) {
                                                if over != drag.target_id {
                                                    pending_swap = Some((drag.target_id, over));
                                                }
                                            }
                                            session.root.end_corner_drag();
                                        }
                                        CornerDragModifier::Shift => {
                                            session.root.end_corner_drag();
                                        }
                                        CornerDragModifier::None | CornerDragModifier::Alt => {}
                                    }
                                }
                            }
                            changed = true;
                        }
                        if let Some((from, to)) = pending_swap {
                            ed.swap_editor_inner_panel_kinds(area_id, from, to);
                        }
                    }
                    if changed {
                        cx.notify();
                    }
                });
            })
            .on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                let _ = root_editor_up.update(cx, |ed, cx| {
                    // Outer corner drag end: the window-level drag policy
                    // interprets the raw facts (plain = split/join with
                    // content seed, Shift = clone the window, Ctrl = swap,
                    // Alt = nothing).
                    if let Some(facts) =
                        splitype_splitter::interaction::finish_window_drag(&mut ed.panels.layout)
                    {
                        let viewport = window.viewport_size();
                        match facts.modifier {
                            CornerDragModifier::None => {
                                <SplitterContainer<WindowAreaKind> as DragPolicy<WindowAreaKind>>::on_plain_drag(
                                    &mut ed.panels.layout, &facts, viewport, cx,
                                )
                            }
                            CornerDragModifier::Shift => {
                                <SplitterContainer<WindowAreaKind> as DragPolicy<WindowAreaKind>>::on_shift_drag(
                                    &mut ed.panels.layout, &facts, viewport, cx,
                                )
                            }
                            CornerDragModifier::Ctrl => {
                                <SplitterContainer<WindowAreaKind> as DragPolicy<WindowAreaKind>>::on_ctrl_drag(
                                    &mut ed.panels.layout, &facts, viewport, cx,
                                )
                            }
                            CornerDragModifier::Alt => {
                                <SplitterContainer<WindowAreaKind> as DragPolicy<WindowAreaKind>>::on_alt_drag(
                                    &mut ed.panels.layout, &facts, viewport, cx,
                                )
                            }
                        }
                        cx.notify();
                    }
                    // Inner splitter drag end.
                    let splitter_pending = ed
                        .editor_sessions
                        .iter_mut()
                        .find(|(_, s)| s.root.active_splitter_drag.is_some());
                    if let Some((_, session)) = splitter_pending {
                        session.root.end_splitter_drag();
                        cx.notify();
                    }
                    // Inner corner drag end: finish the gesture through the
                    // shared container, then let the inner-panel policy
                    // interpret the facts (Shift is a no-op override).
                    let corner_pending = ed
                        .editor_sessions
                        .iter_mut()
                        .find(|(_, s)| s.root.corner_drag_panel().is_some())
                        .map(|(area_id, session)| {
                            let facts = session.root.finish_corner_drag();
                            (*area_id, facts)
                        });
                    if let Some((area_id, Some(facts))) = corner_pending {
                        let viewport = window.viewport_size();
                        let inner_size = {
                            let outer_rects = ed.panels.layout.leaf_rects(viewport);
                            ed.panels
                                .layout
                                .leaf_rect(area_id, &outer_rects)
                                .map(|rect| size(px(rect.width), px(rect.height)))
                                .unwrap_or(viewport)
                        };
                        if let Some(session) = ed.editor_sessions.get_mut(&area_id) {
                            match facts.modifier {
                                CornerDragModifier::None => {
                                    <SplitterContainer<EditorInnerPanelKind> as DragPolicy<EditorInnerPanelKind>>::on_plain_drag(
                                        &mut session.root, &facts, inner_size, cx,
                                    )
                                }
                                CornerDragModifier::Shift => {
                                    <SplitterContainer<EditorInnerPanelKind> as DragPolicy<EditorInnerPanelKind>>::on_shift_drag(
                                        &mut session.root, &facts, inner_size, cx,
                                    )
                                }
                                CornerDragModifier::Ctrl => {
                                    <SplitterContainer<EditorInnerPanelKind> as DragPolicy<EditorInnerPanelKind>>::on_ctrl_drag(
                                        &mut session.root, &facts, inner_size, cx,
                                    )
                                }
                                CornerDragModifier::Alt => {
                                    <SplitterContainer<EditorInnerPanelKind> as DragPolicy<EditorInnerPanelKind>>::on_alt_drag(
                                        &mut session.root, &facts, inner_size, cx,
                                    )
                                }
                            }
                        }
                        cx.notify();
                    }
                });
            })
            .child(layout_tree);

        // Build the preview overlay for corner drag gestures. Host policy:
        // only plain (no-modifier) drags show an indicator; modifier drags
        // have their own immediate behaviors.
        let overlay_style = splitype_splitter::interaction::OverlayStyle {
            accent: theme.colors.split_indicator,
            tile_radius: theme.dimensions.area_tile_radius,
            border: theme.colors.dialog_border,
            selection: theme.colors.selection,
            active: theme.colors.focus_accent,
            ..Default::default()
        };
        let preview_overlay = self.panels.layout.corner_drag_panel().and_then(|panel_id| {
            let drag = self
                .panels
                .layout
                .tree
                .find_leaf(panel_id)
                .and_then(|p| p.active_corner_drag)?;
            if drag.modifier != CornerDragModifier::None {
                return None;
            }
            render_corner_drag_preview(
                &self.panels.layout,
                &drag,
                window.viewport_size(),
                &overlay_style,
            )
        });
        let container = container.children(preview_overlay);

        if let Some(border_menu) = self.panels.layout.active_border_menu {
            let menu_overlay = self.render_window_area_border_menu(border_menu, theme, cx);
            container.child(menu_overlay).into_any_element()
        } else {
            container.into_any_element()
        }
    }
    pub(crate) fn render_window_area_node(
        &mut self,
        node: &crate::splitter::SplitTree<crate::app::window_area::WindowAreaKind>,
        theme: &Theme,
        strings: &I18nStrings,
        leaf_count: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let overlay_style = splitype_splitter::interaction::OverlayStyle {
            accent: c.split_indicator,
            tile_radius: theme.dimensions.area_tile_radius,
            border: c.dialog_border,
            selection: c.selection,
            active: c.focus_accent,
            ..Default::default()
        };
        let editor = cx.entity().downgrade();

        match node {
            SplitTree::Leaf(container) => self.render_window_area_tile(
                container.id,
                container.kind,
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

                let first_elem =
                    self.render_window_area_node(first, theme, strings, leaf_count, window, cx);
                let second_elem =
                    self.render_window_area_node(second, theme, strings, leaf_count, window, cx);

                match direction {
                    Axis::Horizontal => {
                        let bar_editor = editor.clone();
                        let menu_editor = editor.clone();
                        let bar_active = self
                            .panels
                            .layout
                            .active_splitter_drag
                            .is_some_and(|drag| drag.split_id == split_id);

                        // The split areas tile seamlessly; the splitter bar
                        // floats as an overlay on the boundary at `r`.
                        div()
                            .id(("tiled-split-h", split_id))
                            .w_full()
                            .h_full()
                            .flex()
                            .flex_row()
                            .min_w(px(0.0))
                            .min_h(px(0.0))
                            .relative()
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
                                    .h_full()
                                    .overflow_hidden()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .min_h(px(0.0))
                                    .child(second_elem),
                            )
                            .child(
                                // Splitter bar between the two seamless areas.
                                splitype_splitter::interaction::splitter_bar_h(
                                    ("tiled-root-bar-h", split_id),
                                    r,
                                    bar_active,
                                    &overlay_style,
                                )
                                .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                                    let start_pos = f32::from(event.position.x);
                                    let _ = bar_editor.update(cx, |ed, cx| {
                                        splitype_splitter::interaction::start_splitter_drag(
                                            &mut ed.panels.layout,
                                            split_id,
                                            Axis::Horizontal,
                                            start_pos,
                                            r,
                                        );
                                        cx.notify();
                                    });
                                })
                                .on_mouse_down(
                                    MouseButton::Right,
                                    move |event, _window, cx| {
                                        let pos = event.position;
                                        let _ = menu_editor.update(cx, |ed, cx| {
                                            splitype_splitter::interaction::open_border_menu(
                                                &mut ed.panels.layout,
                                                split_id,
                                                dir,
                                                pos,
                                            );
                                            cx.notify();
                                        });
                                    },
                                ),
                            )
                            .into_any_element()
                    }
                    Axis::Vertical => {
                        let bar_editor = editor.clone();
                        let menu_editor = editor.clone();
                        let bar_active = self
                            .panels
                            .layout
                            .active_splitter_drag
                            .is_some_and(|drag| drag.split_id == split_id);

                        div()
                            .id(("tiled-split-v", split_id))
                            .w_full()
                            .h_full()
                            .flex()
                            .flex_col()
                            .min_w(px(0.0))
                            .min_h(px(0.0))
                            .relative()
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
                                    .w_full()
                                    .overflow_hidden()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .min_h(px(0.0))
                                    .child(second_elem),
                            )
                            .child(
                                // Splitter bar between the two seamless areas.
                                splitype_splitter::interaction::splitter_bar_v(
                                    ("tiled-root-bar-v", split_id),
                                    r,
                                    bar_active,
                                    &overlay_style,
                                )
                                .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                                    let start_pos = f32::from(event.position.y);
                                    let _ = bar_editor.update(cx, |ed, cx| {
                                        splitype_splitter::interaction::start_splitter_drag(
                                            &mut ed.panels.layout,
                                            split_id,
                                            Axis::Vertical,
                                            start_pos,
                                            r,
                                        );
                                        cx.notify();
                                    });
                                })
                                .on_mouse_down(
                                    MouseButton::Right,
                                    move |event, _window, cx| {
                                        let pos = event.position;
                                        let _ = menu_editor.update(cx, |ed, cx| {
                                            splitype_splitter::interaction::open_border_menu(
                                                &mut ed.panels.layout,
                                                split_id,
                                                dir,
                                                pos,
                                            );
                                            cx.notify();
                                        });
                                    },
                                ),
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
        kind: crate::app::window_area::WindowAreaKind,
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

        let topbar = match kind {
            WindowAreaKind::Editor => {
                self.render_editor_topbar(leaf_id, kind, theme, leaf_count, is_maximized, cx)
            }
            WindowAreaKind::Explorer => {
                self.render_explorer_topbar(leaf_id, kind, theme, leaf_count, is_maximized, cx)
            }
            WindowAreaKind::Settings => {
                self.render_settings_topbar(leaf_id, kind, theme, leaf_count, is_maximized, cx)
            }
        };

        let midcontainer: AnyElement = match kind {
            WindowAreaKind::Editor => {
                self.render_editor_midcontainer(leaf_id, theme, strings, window, cx)
            }
            WindowAreaKind::Explorer => {
                self.render_explorer_midcontainer(leaf_id, theme, strings, cx)
            }
            WindowAreaKind::Settings => {
                self.render_settings_midcontainer(leaf_id, theme, strings, cx)
            }
        };

        let bottombar = match kind {
            WindowAreaKind::Editor => {
                Some(self.render_editor_bottombar(leaf_id, theme, strings, cx))
            }
            WindowAreaKind::Explorer => Some(self.render_explorer_bottombar(leaf_id, theme, cx)),
            WindowAreaKind::Settings => Some(self.render_settings_bottombar(leaf_id, theme, cx)),
        };

        let midcontainer_container = div()
            .w_full()
            .flex_1()
            .min_h(px(0.0))
            .relative()
            .child(midcontainer);

        // Tile card with overflow hidden (no corner handles inside, to avoid clipping).
        // Mouse interaction with any part of the tile marks it as the focused
        // window area and, for Editor tiles, the active editor.
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
            .border(px(d.dialog_border_width))
            .border_color(c.dialog_border)
            .shadow_lg()
            .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                let _ = tile_focus.update(cx, |ed, cx| {
                    ed.panels.layout.focused_area = Some(leaf_id);
                    if kind == crate::app::window_area::WindowAreaKind::Editor {
                        ed.panels.layout.activate_area(leaf_id);
                    }
                    cx.notify();
                });
            })
            .child(topbar)
            .child(midcontainer_container);

        if let Some(bb) = bottombar {
            tile_card = tile_card.child(bb);
        }

        // Corner drag handles positioned at the four outer corners of the tile card.
        let editor_corner = cx.entity().downgrade();
        let corner_handles = splitype_splitter::interaction::corner_drag_handles(
            "area-corner",
            leaf_id,
            gap,
            20.0,
            false,
            false,
            move |modifier, pos, cx| {
                let _ = editor_corner.update(cx, |ed, cx| {
                    ed.panels.layout.start_corner_drag(leaf_id, pos, modifier);
                    cx.notify();
                });
            },
        );

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

        // The dropdown flag lives on the panel itself.
        let dropdown_open = self
            .panels
            .layout
            .tree
            .find_leaf(leaf_id)
            .is_some_and(|p| p.open_dropdown);
        if dropdown_open {
            let menu = self.render_area_type_dropdown_menu(leaf_id, kind, theme, cx);
            wrapped = wrapped.child(menu);
        }

        wrapped.into_any_element()
    }

    pub(crate) fn render_area_type_dropdown_menu(
        &mut self,
        leaf_id: usize,
        current_type: crate::app::window_area::WindowAreaKind,
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
                            .path(area_topbar_icon(current_type, "check"))
                            .size(px(13.0))
                            .text_color(c.dialog_primary_button_bg)
                            .into_any_element()
                    } else {
                        div().w(px(13.0)).into_any_element()
                    })
                    .on_click(move |_event, _window, cx| {
                        let _ = option_editor.update(cx, |ed, cx| {
                            ed.change_window_area_kind(leaf_id, kind);
                            cx.notify();
                        });
                    })
                    .into_any_element()
            }))
            .into_any_element()
    }
    pub(crate) fn render_window_area_border_menu(
        &mut self,
        border_menu: crate::splitter::BorderMenuState,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let editor = cx.entity().downgrade();
        let split_id = border_menu.split_id;

        let menu_style = border_menu_style(theme);

        let split_h_ed = editor.clone();
        let split_h: Box<dyn Fn(&mut App)> = Box::new(move |app| {
            let _ = split_h_ed.update(app, |ed, cx| {
                ed.split_area(split_id, Axis::Horizontal, 0.5, true, cx);
                cx.notify();
            });
        });
        let split_v_ed = editor.clone();
        let split_v: Box<dyn Fn(&mut App)> = Box::new(move |app| {
            let _ = split_v_ed.update(app, |ed, cx| {
                ed.split_area(split_id, Axis::Vertical, 0.5, true, cx);
                cx.notify();
            });
        });
        let swap_ed = editor.clone();
        let swap: Box<dyn Fn(&mut App)> = Box::new(move |app| {
            let _ = swap_ed.update(app, |ed, cx| {
                ed.panels.layout.swap_split_sides(split_id);
                cx.notify();
            });
        });
        let close_ed = editor.clone();
        let close: Box<dyn Fn(&mut App)> = Box::new(move |app| {
            let _ = close_ed.update(app, |ed, cx| {
                ed.close_window_area(split_id);
                cx.notify();
            });
        });
        let dismiss_ed = editor.clone();
        let dismiss: Box<dyn Fn(&mut App)> = Box::new(move |app| {
            let _ = dismiss_ed.update(app, |ed, cx| {
                ed.panels.layout.active_border_menu = None;
                cx.notify();
            });
        });

        crate::splitter::interaction::render_border_menu(
            border_menu.position,
            vec![
                crate::splitter::interaction::BorderMenuItem {
                    label: "Split Horizontally",
                    on_activate: split_h,
                },
                crate::splitter::interaction::BorderMenuItem {
                    label: "Split Vertically",
                    on_activate: split_v,
                },
                crate::splitter::interaction::BorderMenuItem {
                    label: "Swap Panels",
                    on_activate: swap,
                },
                crate::splitter::interaction::BorderMenuItem {
                    label: "Close Area",
                    on_activate: close,
                },
            ],
            &menu_style,
            dismiss,
        )
    }
}

// ---------------------------------------------------------------------------
// Window panels aggregate
// ---------------------------------------------------------------------------

/// Sidebar and tiled-layout state of the editor window.
///
/// Pure state records; rendering lives in `crate::editor::window_layout`
/// (outer layout), `crate::explorer`, and `crate::settings`. The per-area
/// editor sessions and inner-panel operations live on the `Editor` entity
/// (see `crate::editor::session_ops`).
pub struct WindowPanels {
    pub(crate) explorer: ExplorerState,
    pub(crate) layout: WindowLayout,
    pub(crate) outline: OutlinePanelState,
    pub(crate) settings: SettingsUiState,
}

impl Default for WindowPanels {
    fn default() -> Self {
        Self {
            explorer: ExplorerState::default(),
            layout: crate::app::window_area::default_layout(),
            outline: OutlinePanelState::default(),
            settings: SettingsUiState::default(),
        }
    }
}
