//! Window-level tiled area layout — rendering and gestures for the outer
//! `PanelKindId` split tree (Explorer / Settings / Editor panel views).
//!
//! The layout engine (tree, sessions, operations) lives in `crate::splitter`;
//! the editor.s pane layout rendering lives in
//! `editor::panel_layout`. The window panel state aggregate lives in
//! `crate::app::window::panels`.

use ui::menu_item::menu_item;
use ui::popover::menu_panel;

use gpui::*;

use crate::app::shell::Shell;

use workspace::panel_topbar_icon;
use ui::corner_drag_preview::render_corner_drag_preview;
use config::language::I18nStrings;
use theme::{Theme, ThemeManager};
use splitter::policy::CornerDragResult;
use splitter::sessions::CornerDragModifier;
use splitter::tree::{NodeId, SplitAxis, SplitTree};

impl Shell {
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
        let layout_tree = if let Some(maximized_leaf) = root.find_maximized_leaf() {
            self.render_window_panel_tile(
                maximized_leaf.id,
                maximized_leaf.kind,
                theme,
                strings,
                leaf_count,
                true,
                window,
                cx,
            )
        } else {
            self.render_window_panel_node(&root, theme, strings, leaf_count, window, cx)
        };
        let root_shell_move = cx.entity().downgrade();
        let root_shell_up = cx.entity().downgrade();
        let root_shell_up_out = cx.entity().downgrade();

        let titlebar_height = ui::custom_titlebar::custom_titlebar_height_for_target_os(
            std::env::consts::OS,
            Decorations::Server,
            &theme.dimensions,
        );

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
                let _ = root_shell_move.update(cx, |shell, cx| {
                    let mut changed = false;
                    let viewport = window.viewport_size();
                    let body_height = (f32::from(viewport.height) - titlebar_height).max(0.0);
                    let body_size = size(viewport.width, px(body_height));
                    let body_pos = point(pos.x, px((f32::from(pos.y) - titlebar_height).max(0.0)));
                    if splitter::interaction::update_splitter_drag(
                        &mut shell.panels.layout,
                        body_pos,
                        body_size,
                    ) {
                        changed = true;
                    }
                    // Outer gesture shortcuts (host-owned, immediate):
                    // Forward to every editor entity — only the one with an
                    // active drag reports a change.
                    let editors: Vec<Entity<editor_core::Editor>> = shell
                        .panel_views
                        .values()
                        .filter_map(|view| view.as_any().downcast_ref::<editor_core::EditorPanelView>())
                        .map(|p| p.editor.clone())
                        .collect();
                    for editor in editors {
                        if editor.update(cx, |editor, _cx| editor.update_inner_drag(pos, window)) {
                            changed = true;
                        }
                    }
                    if changed {
                        cx.notify();
                    }
                });
            })
            .on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                let _ =
                    root_shell_up.update(cx, |shell, cx| shell.finish_drag_gestures(window, cx));
            })
            .on_mouse_up_out(MouseButton::Left, move |_event, window, cx| {
                let _ = root_shell_up_out
                    .update(cx, |shell, cx| shell.finish_drag_gestures(window, cx));
            })
            .child(layout_tree);

        // Build the preview overlay for corner drag gestures.
        let overlay_style = splitter::interaction::OverlayStyle {
            accent: theme.colors.split_indicator,
            tile_radius: theme.dimensions.panel_tile_radius,
            border: theme.colors.dialog_border,
            selection: theme.colors.selection,
            active: theme.colors.focus_accent,
            surface: theme.colors.dialog_surface,
            text: theme.colors.dialog_title,
        };
        let body_height = (f32::from(window.viewport_size().height) - titlebar_height).max(0.0);
        let body_size = size(window.viewport_size().width, px(body_height));
        let preview_overlay = self.panels.layout.corner_drag_panel().and_then(|panel_id| {
            let drag = self
                .panels
                .layout
                .tree
                .find_leaf(panel_id)
                .and_then(|p| p.active_corner_drag)?;
            if drag.modifier != CornerDragModifier::None
                && drag.modifier != CornerDragModifier::Ctrl
                && drag.modifier != CornerDragModifier::Shift
            {
                return None;
            }
            render_corner_drag_preview(
                &self.panels.layout,
                &drag,
                body_size,
                &overlay_style,
            )
        });
        let container = container.children(preview_overlay);

        if let Some(border_menu) = self.panels.layout.active_border_menu {
            let menu_overlay = self.render_window_panel_border_menu(border_menu, theme, cx);
            container.child(menu_overlay).into_any_element()
        } else {
            container.into_any_element()
        }
    }
    pub(crate) fn finish_drag_gestures(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.panels.layout.active_splitter_drag.is_some() {
            self.panels.layout.end_splitter_drag();
            cx.notify();
            return;
        }

        let theme = cx.global::<ThemeManager>().current_arc();
        let titlebar_height = ui::custom_titlebar::custom_titlebar_height_for_target_os(
            std::env::consts::OS,
            Decorations::Server,
            &theme.dimensions,
        );
        let body_height = (f32::from(window.viewport_size().height) - titlebar_height).max(0.0);
        let body_size = size(window.viewport_size().width, px(body_height));

        match self.panels.layout.apply_corner_drag(body_size) {
            CornerDragResult::Split { new_leaf_id, .. } => {
                self.seed_split_panel(new_leaf_id, cx);
            }
            CornerDragResult::Join { into_id: _, removed_id } => {
                self.handle_joined_panel(removed_id, cx);
            }
            CornerDragResult::MoveAndDock { source_id, target_id, new_leaf_id, dock_target, .. } => {
                self.handle_moved_and_docked_panel(source_id, target_id, new_leaf_id, dock_target, cx);
            }
            CornerDragResult::Swap { a, b } => {
                self.handle_swapped_panels(a, b, cx);
            }
            CornerDragResult::CloneWindow { container, .. } => {
                self.clone_container_into_new_window(container, cx);
            }
            CornerDragResult::None => {}
        }
        cx.notify();
        let editors: Vec<Entity<editor_core::Editor>> = self
            .panel_views
            .values()
            .filter_map(|view| view.as_any().downcast_ref::<editor_core::EditorPanelView>())
            .map(|p| p.editor.clone())
            .collect();
        for editor in editors {
            editor.update(cx, |editor, cx| editor.finish_inner_drag(window, cx));
        }
    }

    pub(crate) fn render_window_panel_node(
        &mut self,
        node: &splitter::SplitTree<workspace::PanelKindId>,
        theme: &Theme,
        strings: &I18nStrings,
        leaf_count: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let overlay_style = splitter::interaction::OverlayStyle {
            accent: c.split_indicator,
            tile_radius: theme.dimensions.panel_tile_radius,
            border: c.dialog_border,
            selection: c.selection,
            active: c.focus_accent,
            surface: c.dialog_surface,
            text: c.dialog_title,
        };
        let shell = cx.entity().downgrade();

        match node {
            SplitTree::Leaf(container) => self.render_window_panel_tile(
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
                axis,
                ratio,
                first,
                second,
            } => {
                let split_id = *id;
                let split_axis = *axis;
                let r = *ratio;

                let first_elem =
                    self.render_window_panel_node(first, theme, strings, leaf_count, window, cx);
                let second_elem =
                    self.render_window_panel_node(second, theme, strings, leaf_count, window, cx);

                match axis {
                    SplitAxis::Horizontal => {
                        let bar_shell = shell.clone();
                        let menu_shell = shell.clone();
                        let bar_active = self
                            .panels
                            .layout
                            .active_splitter_drag
                            .is_some_and(|drag| drag.split_id == split_id);

                        // The split panel_contents tile seamlessly; the splitter bar
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
                                // Splitter bar between the two seamless panel_contents.
                                splitter::interaction::splitter_bar_h(
                                    ("tiled-root-bar-h", split_id),
                                    r,
                                    bar_active,
                                    &overlay_style,
                                )
                                .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                                    let start_pos = f32::from(event.position.x);
                                    let _ = bar_shell.update(cx, |shell, cx| {
                                        splitter::interaction::start_splitter_drag(
                                            &mut shell.panels.layout,
                                            split_id,
                                            SplitAxis::Horizontal,
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
                                        let _ = menu_shell.update(cx, |shell, cx| {
                                            splitter::interaction::open_border_menu(
                                                &mut shell.panels.layout,
                                                split_id,
                                                split_axis,
                                                pos,
                                            );
                                            cx.notify();
                                        });
                                    },
                                ),
                            )
                            .into_any_element()
                    }
                    SplitAxis::Vertical => {
                        let bar_shell = shell.clone();
                        let menu_shell = shell.clone();
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
                                // Splitter bar between the two seamless panel_contents.
                                splitter::interaction::splitter_bar_v(
                                    ("tiled-root-bar-v", split_id),
                                    r,
                                    bar_active,
                                    &overlay_style,
                                )
                                .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                                    let start_pos = f32::from(event.position.y);
                                    let _ = bar_shell.update(cx, |shell, cx| {
                                        splitter::interaction::start_splitter_drag(
                                            &mut shell.panels.layout,
                                            split_id,
                                            SplitAxis::Vertical,
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
                                        let _ = menu_shell.update(cx, |shell, cx| {
                                            splitter::interaction::open_border_menu(
                                                &mut shell.panels.layout,
                                                split_id,
                                                split_axis,
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
    pub(crate) fn render_window_panel_tile(
        &mut self,
        leaf_id: NodeId,
        kind: workspace::PanelKindId,
        theme: &Theme,
        strings: &I18nStrings,
        leaf_count: usize,
        is_maximized: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let gap = d.panel_tile_gap;
        let radius = d.panel_tile_radius;

        let panel_id = workspace::PanelId(leaf_id);
        let render_ctx = workspace::PanelRenderContext {
            panel_id,
            leaf_count,
            is_maximized,
            theme,
            strings,
        };

        if !self.panel_views.contains_key(&panel_id) {
            self.sync_panel_kind(panel_id, kind == workspace::PanelKindId::EDITOR, cx);
        }

        let panel_card: AnyElement = if let Some(view) = self.panel_views.get_mut(&panel_id) {
            let rendered = view.render(&render_ctx, _window, cx);
            if view.kind() == workspace::PanelKindId::EDITOR {
                rendered
            } else {
                let card = div()
                    .id(("panel-card", leaf_id))
                    .w_full()
                    .h_full()
                    .flex()
                    .flex_col()
                    .relative()
                    .overflow_hidden()
                    .rounded(px(radius))
                    .bg(c.dialog_surface)
                    .border(px(d.dialog_border_width))
                    .border_color(c.dialog_border)
                    .shadow_lg()
                    .child(rendered);
                card.into_any_element()
            }
        } else {
            div().into_any_element()
        };

        // Mouse interaction with any part of the tile marks it as the focused
        // window panel and, for Editor tiles, the active editor.
        let tile_focus = cx.entity().downgrade();
        // Wrap in a padded container so the gap is uniform.
        let mut wrapped = div()
            .id(("panel-wrapper", leaf_id))
            .w_full()
            .h_full()
            .min_w(px(0.0))
            .min_h(px(0.0))
            .p(px(gap))
            .relative()
            .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                let _ = tile_focus.update(cx, |shell, cx| {
                    shell.panels.layout.focused_leaf = Some(leaf_id);
                    if kind == workspace::PanelKindId::EDITOR {
                        shell.panels.layout.activate_leaf(leaf_id);
                    }
                    cx.notify();
                });
            })
            .child(panel_card);

        let titlebar_height = ui::custom_titlebar::custom_titlebar_height_for_target_os(
            std::env::consts::OS,
            Decorations::Server,
            &theme.dimensions,
        );

        // Corner drag handles positioned at the four outer corners of the tile card.
        let shell_corner = cx.entity().downgrade();
        let corner_handles = splitter::interaction::corner_drag_handles(
            "panel-corner",
            leaf_id,
            gap,
            20.0,
            false,
            false,
            move |modifier, pos, cx| {
                let _ = shell_corner.update(cx, |shell, cx| {
                    let body_pos = point(pos.x, px((f32::from(pos.y) - titlebar_height).max(0.0)));
                    shell
                        .panels
                        .layout
                        .start_corner_drag(leaf_id, body_pos, modifier);
                    cx.notify();
                });
            },
        );
        wrapped = wrapped.child(corner_handles);

        // The dropdown flag lives on the panel itself.
        let dropdown_open = self
            .panels
            .layout
            .tree
            .find_leaf(leaf_id)
            .is_some_and(|p| p.open_dropdown);
        if dropdown_open {
            let menu = self.render_panel_type_dropdown_menu(leaf_id, kind, theme, cx);
            wrapped = wrapped.child(menu);
        }

        wrapped.into_any_element()
    }

    pub(crate) fn render_panel_type_dropdown_menu(
        &mut self,
        leaf_id: NodeId,
        current_kind: workspace::PanelKindId,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let shell = cx.entity().downgrade();

        let registry = workspace::PanelRegistry::global().lock().unwrap();
        let available_descriptors = registry.all_descriptors();

        menu_panel(c, d)
            .id(("panel-dropdown-overlay", leaf_id))
            .absolute()
            .occlude()
            .top(px(28.0))
            .left(px(8.0))
            .w(px(d.menu_panel_width))
            .children(available_descriptors.into_iter().enumerate().map(|(idx, desc)| {
                let kind_id = desc.kind();
                let is_current = kind_id == current_kind;
                let option_shell = shell.clone();
                let display_name = desc.display_name();
                menu_item(("panel-type-opt", idx), c, d)
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
                    .child(display_name)
                    .child(if is_current {
                        svg()
                            .path(panel_topbar_icon(current_kind, "check"))
                            .size(px(13.0))
                            .text_color(c.dialog_primary_button_bg)
                            .into_any_element()
                    } else {
                        div().w(px(13.0)).into_any_element()
                    })
                    .on_click(move |_event, _window, cx| {
                        let _ = option_shell.update(cx, |shell, cx| {
                            shell.change_panel_kind(leaf_id, kind_id, cx);
                            cx.notify();
                        });
                    })
                    .into_any_element()
            }))
            .into_any_element()
    }
    pub(crate) fn render_window_panel_border_menu(
        &mut self,
        border_menu: splitter::BorderMenuState,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let shell = cx.entity().downgrade();
        let split_id = border_menu.split_id;

        let menu_style = workspace::border_menu_style(theme);

        let split_h_shell = shell.clone();
        let split_h: Box<dyn Fn(&mut App)> = Box::new(move |app| {
            let _ = split_h_shell.update(app, |shell, cx| {
                shell.split_panel(split_id, SplitAxis::Horizontal, 0.5, true, cx);
                cx.notify();
            });
        });
        let split_v_shell = shell.clone();
        let split_v: Box<dyn Fn(&mut App)> = Box::new(move |app| {
            let _ = split_v_shell.update(app, |shell, cx| {
                shell.split_panel(split_id, SplitAxis::Vertical, 0.5, true, cx);
                cx.notify();
            });
        });
        let swap_shell = shell.clone();
        let swap: Box<dyn Fn(&mut App)> = Box::new(move |app| {
            let _ = swap_shell.update(app, |shell, cx| {
                shell.panels.layout.swap_split_sides(split_id);
                cx.notify();
            });
        });
        let close_shell = shell.clone();
        let close: Box<dyn Fn(&mut App)> = Box::new(move |app| {
            let _ = close_shell.update(app, |shell, cx| {
                shell.close_panel(split_id, cx);
                cx.notify();
            });
        });
        let dismiss_shell = shell.clone();
        let dismiss: Box<dyn Fn(&mut App)> = Box::new(move |app| {
            let _ = dismiss_shell.update(app, |shell, cx| {
                shell.panels.layout.active_border_menu = None;
                cx.notify();
            });
        });

        splitter::interaction::render_border_menu(
            border_menu.position,
            vec![
                splitter::interaction::BorderMenuItem {
                    label: "Split Horizontally",
                    on_activate: split_h,
                },
                splitter::interaction::BorderMenuItem {
                    label: "Split Vertically",
                    on_activate: split_v,
                },
                splitter::interaction::BorderMenuItem {
                    label: "Swap Panels",
                    on_activate: swap,
                },
                splitter::interaction::BorderMenuItem {
                    label: "Close Panel",
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
