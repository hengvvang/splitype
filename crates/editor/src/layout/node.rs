//! Recursive rendering for split layout tree nodes, split containers, and pane viewports.

use gpui::*;
use splitter::SplitAxis;
use splitter::tree::SplitTree;

use crate::editor::Editor;
use config::language::I18nStrings;
use editor_contracts::PaneId;
use theme::Theme;

impl Editor {
    pub(crate) fn render_editor_pane_split_tree(
        &mut self,
        tree: &SplitTree<editor_contracts::PaneKind>,
        theme: &Theme,
        strings: &I18nStrings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let content = self.render_editor_pane_node(tree, theme, strings, window, cx);
        let border_menu = self.render_editor_pane_border_menu(theme, strings, cx);
        let corner_preview = self.render_editor_pane_corner_drag_preview(theme);

        div()
            .id("editor-split-tree")
            .size_full()
            .relative()
            .child(content)
            .children(border_menu)
            .children(corner_preview)
            .into_any_element()
    }

    pub(crate) fn render_editor_pane_node(
        &mut self,
        node: &SplitTree<editor_contracts::PaneKind>,
        theme: &Theme,
        _strings: &I18nStrings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let overlay_style = ui::split::chrome::OverlayStyle::from_theme(theme);

        match node {
            SplitTree::Leaf(container) => {
                let pane_id = PaneId(container.id);
                let inner_editor = cx.entity().downgrade();

                let inner_body = self.render_pane(pane_id, window, cx);

                let is_inner_maximized = self.session.root.tree.find_maximized_leaf().is_some();
                let corner_handles = if !is_inner_maximized {
                    Some(ui::split::chrome::corner_drag_handles(
                        "inner-corner",
                        pane_id.0,
                        d.pane_gap,
                        48.0,
                        &overlay_style,
                        move |modifier, pos, cx| {
                            let _ = inner_editor.update(cx, |ed, cx| {
                                // Convert the window-relative pointer into the
                                // pane layout's local space before recording
                                // the gesture start.
                                let local = ed
                                    .panel_rect
                                    .map(|rect| {
                                        point(
                                            px(f32::from(pos.x) - f32::from(rect.origin.x)),
                                            px(f32::from(pos.y) - f32::from(rect.origin.y)),
                                        )
                                    })
                                    .unwrap_or(pos);
                                ed.session_mut()
                                    .root
                                    .start_corner_drag(pane_id.0, local, modifier);
                                cx.notify();
                            });
                        },
                    ))
                } else {
                    None
                };

                if self.focused_pane_id.is_none() {
                    self.focused_pane_id = Some(pane_id);
                }

                let focus_editor = cx.entity().downgrade();
                let panel_gap = d.pane_gap;

                div()
                    .id(("pane-wrapper", pane_id.0))
                    .w_full()
                    .h_full()
                    .relative()
                    .child(
                        div()
                            .id(("pane-card", pane_id.0))
                            .absolute()
                            .inset(px(panel_gap))
                            .overflow_hidden()
                            .flex()
                            .flex_col()
                            .rounded(px(d.panel_tile_radius))
                            .bg(c.dialog_surface)
                            .border(px(d.dialog_border_width))
                            .border_color(c.dialog_border)
                            .shadow_lg()
                            .child(
                                div()
                                    .w_full()
                                    .flex_1()
                                    .min_h(px(0.0))
                                    .overflow_hidden()
                                    .child(inner_body),
                            )
                            .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                                let _ = focus_editor.update(cx, |ed, cx| {
                                    ed.focus_pane(pane_id, window, cx);
                                    cx.notify();
                                });
                            }),
                    )
                    .children(corner_handles)
                    .into_any_element()
            }
            SplitTree::Split {
                id,
                axis,
                ratio,
                first,
                second,
            } => {
                let split_id = *id;
                let r = *ratio;
                let first_elem = self.render_editor_pane_node(first, theme, _strings, window, cx);
                let second_elem = self.render_editor_pane_node(second, theme, _strings, window, cx);
                let inner_editor = cx.entity().downgrade();

                match axis {
                    SplitAxis::Horizontal => {
                        let bar_editor = inner_editor.clone();
                        let menu_editor = inner_editor.clone();
                        let bar_active = self
                            .session
                            .root
                            .active_splitter_drag
                            .is_some_and(|drag| drag.split_id == split_id);
                        div()
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
                                ui::split::chrome::splitter_bar_h(
                                    ("inner-root-bar-h", split_id),
                                    r,
                                    bar_active,
                                    &overlay_style,
                                )
                                .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                                    let start_pos = f32::from(event.position.x);
                                    let _ = bar_editor.update(cx, |ed, cx| {
                                        let local_start = ed
                                            .panel_rect
                                            .map(|rect| start_pos - f32::from(rect.origin.x))
                                            .unwrap_or(start_pos);
                                        ed.session_mut().root.start_splitter_drag(
                                            split_id,
                                            SplitAxis::Horizontal,
                                            local_start,
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
                                            ed.session_mut().root.open_border_menu(split_id, pos);
                                            cx.notify();
                                        });
                                    },
                                ),
                            )
                            .into_any_element()
                    }
                    SplitAxis::Vertical => {
                        let bar_editor = inner_editor.clone();
                        let menu_editor = inner_editor.clone();
                        let bar_active = self
                            .session
                            .root
                            .active_splitter_drag
                            .is_some_and(|drag| drag.split_id == split_id);
                        div()
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
                                ui::split::chrome::splitter_bar_v(
                                    ("inner-root-bar-v", split_id),
                                    r,
                                    bar_active,
                                    &overlay_style,
                                )
                                .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                                    let start_pos = f32::from(event.position.y);
                                    let _ = bar_editor.update(cx, |ed, cx| {
                                        let local_start = ed
                                            .panel_rect
                                            .map(|rect| start_pos - f32::from(rect.origin.y))
                                            .unwrap_or(start_pos);
                                        ed.session_mut().root.start_splitter_drag(
                                            split_id,
                                            SplitAxis::Vertical,
                                            local_start,
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
                                            ed.session_mut().root.open_border_menu(split_id, pos);
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

    pub(crate) fn render_pane(
        &mut self,
        pane_id: PaneId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        // With no open tab there is no document to edit: render a blank,
        // input-inert surface instead of an interactive pane.
        let Some(buffer) = self.session.active_tab().map(|tab| tab.buffer.clone()) else {
            let theme = cx.global::<theme::ThemeManager>().current_arc();
            return div()
                .id(("empty-pane", pane_id.0))
                .w_full()
                .h_full()
                .bg(theme.colors.editor_background)
                .into_any_element();
        };
        let document = buffer.read(cx).snapshot();

        if pane_id == self.active_pane_id() {
            self.apply_pending_focus(pane_id, window, cx);
        }

        let is_focused = self.focused_pane_id == Some(pane_id);
        let host = self.pane_host.clone();

        let is_outline_hovered = self.outline.is_hovered;
        if let Some(state) = self.pane_state_mut(pane_id) {
            // Syncing here every frame is the activation catch-up: a kind
            // switch (ensure_kind) happens just above, and the revision
            // guard makes this a no-op when nothing changed.
            let scroll = state.scroll.handle.clone();
            state.pane_mut().sync_document(&document, cx);
            let render_ctx = editor_contracts::PaneRenderContext {
                pane_id,
                is_focused,
                scroll: &scroll,
                host: &host,
                is_outline_hovered,
            };
            state.pane_mut().render(&render_ctx, window, cx)
        } else {
            div().into_any_element()
        }
    }
}
