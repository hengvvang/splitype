//! Recursive tiled node and splitter bar rendering.

use gpui::*;

use crate::shell::Shell;
use config::language::I18nStrings;
use splitter::tree::{NodeId, SplitAxis, SplitTree};
use theme::Theme;
use ui::split::chrome::{OverlayStyle, splitter_bar_h, splitter_bar_v};

impl Shell {
    pub(crate) fn render_window_panel_node(
        &mut self,
        node: &splitter::SplitTree<platform_contracts::PanelKind>,
        theme: &Theme,
        strings: &I18nStrings,
        leaf_count: usize,
        leaf_bounds: &std::collections::HashMap<NodeId, Bounds<Pixels>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let overlay_style = OverlayStyle::from_theme(theme);
        let shell = cx.entity().downgrade();

        match node {
            SplitTree::Leaf(container) => self.render_window_panel_tile(
                container.id,
                container.kind.clone(),
                theme,
                strings,
                leaf_count,
                false,
                leaf_bounds,
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
                let r = *ratio;

                let first_elem = self.render_window_panel_node(
                    first,
                    theme,
                    strings,
                    leaf_count,
                    leaf_bounds,
                    window,
                    cx,
                );
                let second_elem = self.render_window_panel_node(
                    second,
                    theme,
                    strings,
                    leaf_count,
                    leaf_bounds,
                    window,
                    cx,
                );

                match axis {
                    SplitAxis::Horizontal => {
                        let bar_shell = shell.clone();
                        let menu_shell = shell.clone();
                        let bar_active = self
                            .panels
                            .layout
                            .active_splitter_drag
                            .is_some_and(|drag| drag.split_id == split_id);

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
                                splitter_bar_h(
                                    ("tiled-root-bar-h", split_id),
                                    r,
                                    bar_active,
                                    &overlay_style,
                                )
                                .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                                    let start_pos = f32::from(event.position.x);
                                    let _ = bar_shell.update(cx, |shell, cx| {
                                        shell.panels.layout.start_splitter_drag(
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
                                            shell.panels.layout.open_border_menu(split_id, pos);
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
                                splitter_bar_v(
                                    ("tiled-root-bar-v", split_id),
                                    r,
                                    bar_active,
                                    &overlay_style,
                                )
                                .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                                    let start_pos = f32::from(event.position.y);
                                    let _ = bar_shell.update(cx, |shell, cx| {
                                        shell.panels.layout.start_splitter_drag(
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
                                            shell.panels.layout.open_border_menu(split_id, pos);
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
}
