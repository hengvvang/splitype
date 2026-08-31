//! Window-level tiled area layout — rendering and gestures for the outer
//! `PanelKind` split tree (Explorer / Settings / Editor panel views).

pub mod border_menu;
pub mod panel_tile;
pub mod render;

use gpui::*;

use crate::shell::Shell;
use config::language::I18nStrings;
use splitter::policy::CornerDragResult;
use splitter::sessions::CornerDragModifier;
use splitter::tree::NodeId;
use theme::{Theme, ThemeManager};
use ui::corner_drag_preview::render_corner_drag_preview;
use window::WindowLayout;

/// Sidebar and tiled-layout state of the window.
pub struct WindowPanels {
    pub(crate) layout: WindowLayout,
}

impl Default for WindowPanels {
    fn default() -> Self {
        Self {
            layout: Self::default_layout(),
        }
    }
}

impl WindowPanels {
    /// Builds the default window layout from the registered panel
    /// capabilities: one sidebar panel on the left and one document panel
    /// on the right. No built-in kind names are hardcoded here, so a
    /// third-party sidebar or document plugin can take over the default
    /// slots just by declaring the matching capability.
    pub(crate) fn default_layout() -> WindowLayout {
        let descriptors = window::PanelRegistry::registered_descriptors().unwrap_or_default();
        let sidebar = descriptors
            .iter()
            .find(|descriptor| descriptor.capabilities().sidebar);
        let documents = descriptors
            .iter()
            .find(|descriptor| descriptor.capabilities().documents);
        let left_id = window::PanelId(1);
        let right_id = window::PanelId(2);
        let builder = window::WindowBuilder::new();
        match (sidebar, documents) {
            (Some(sidebar), Some(documents)) => builder.with_split(
                left_id,
                sidebar.kind(),
                right_id,
                documents.kind(),
                0.3,
                right_id,
            ),
            (None, Some(documents)) => builder.with_single_panel(left_id, documents.kind()),
            (Some(sidebar), None) => builder.with_single_panel(left_id, sidebar.kind()),
            (None, None) => {
                let kind = descriptors
                    .first()
                    .map(|descriptor| descriptor.kind())
                    .expect(
                        "at least one panel descriptor must be registered before a window opens",
                    );
                builder.with_single_panel(left_id, kind)
            }
        }
        .take_layout()
        .expect("window builder must produce a layout")
    }
}

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
        let titlebar_height = ui::custom_titlebar::custom_titlebar_height_for_target_os(
            std::env::consts::OS,
            Decorations::Server,
            &theme.dimensions,
        );
        let body_height = (f32::from(window.viewport_size().height) - titlebar_height).max(0.0);
        let body_size = size(window.viewport_size().width, px(body_height));
        let leaf_bounds: std::collections::HashMap<NodeId, Bounds<Pixels>> = self
            .panels
            .layout
            .leaf_rects(body_size)
            .into_iter()
            .map(|rect| {
                (
                    rect.id,
                    Bounds {
                        origin: point(px(rect.x), px(rect.y + titlebar_height)),
                        size: size(px(rect.width), px(rect.height)),
                    },
                )
            })
            .collect();

        let layout_tree = if let Some(maximized_leaf) = root.find_maximized_leaf() {
            self.render_window_panel_tile(
                maximized_leaf.id,
                maximized_leaf.kind.clone(),
                theme,
                strings,
                leaf_count,
                true,
                &leaf_bounds,
                window,
                cx,
            )
        } else {
            self.render_window_panel_node(
                &root,
                theme,
                strings,
                leaf_count,
                &leaf_bounds,
                window,
                cx,
            )
        };
        let root_shell_move = cx.entity().downgrade();
        let root_shell_up = cx.entity().downgrade();
        let root_shell_up_out = cx.entity().downgrade();

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
                    for view in shell.panel_views.values_mut() {
                        if view.handle_inner_mouse_move(pos, window, cx) {
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
            render_corner_drag_preview(&self.panels.layout, &drag, body_size, &overlay_style)
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
            CornerDragResult::Join {
                into_id: _,
                removed_id,
            } => {
                self.handle_joined_panel(removed_id, cx);
            }
            CornerDragResult::MoveAndDock {
                source_id,
                target_id,
                new_leaf_id,
                dock_target,
                ..
            } => {
                self.handle_moved_and_docked_panel(
                    source_id,
                    target_id,
                    new_leaf_id,
                    dock_target,
                    cx,
                );
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
        for view in self.panel_views.values_mut() {
            view.finish_inner_gestures(window, cx);
        }
    }
}
