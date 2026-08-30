use std::time::Instant;

use gpui::*;


use crate::engine::controller::*;
use editor_wysiwyg::document::BlockEntry;
use editor_wysiwyg::render::viewport::{
    build_planned_row_element, plan_document_rows, PlannedRow,
};
use theme::{Theme, ThemeDimensions, ThemeManager};

impl Editor {
    /// Builds this editor area's WYSIWYG document pane: the scrollable block editor.
    pub fn render_wysiwyg_pane(
        &mut self,
        pane_id: PaneId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let panel_id = self.panel_id;
        let theme = cx.global::<ThemeManager>().current_arc();
        let d = &theme.dimensions;

        let viewport_bounds = self
            .pane_state_ref(pane_id)
            .map(|state| state.scroll.handle.bounds())
            .unwrap_or_default();
        let is_initial_unbound =
            viewport_bounds.size.width == px(0.0) || viewport_bounds.size.height == px(0.0);
        let viewport_size = if !is_initial_unbound {
            viewport_bounds.size
        } else if let Some(rect) = self.panel_rect {
            let body_height = (f32::from(rect.size.height)
                - d.topbar_height
                - d.bottombar_height)
                .max(0.0);
            let inner_rects = self
                .session
                .root
                .leaf_rects(size(rect.size.width, px(body_height)));
            if let Some(leaf) = inner_rects.iter().find(|l| l.id == pane_id.0) {
                size(px(leaf.width), px(leaf.height))
            } else {
                rect.size
            }
        } else {
            window.viewport_size()
        };

        if is_initial_unbound {
            let window_handle = window.window_handle();
            cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(16))
                    .await;
                let _ = window_handle.update(cx, |_view, _window, cx| {
                    let _ = this.update(cx, |_editor, cx| {
                        cx.notify();
                    });
                });
            })
            .detach();
        }

        if pane_id == self.active_pane_id() {
            self.apply_pending_focus(pane_id, window, cx);
            self.apply_pending_autoscroll(pane_id, window, cx);
        }
        self.sync_scroll_viewport(pane_id, viewport_size, cx);

        self.ensure_document(cx);
        let blocks = self.doc().blocks();
        let editor = cx.entity().downgrade();
        let max_scroll_y = self
            .pane_state_ref(pane_id)
            .map(|state| f32::from(state.scroll.handle.max_offset().y.max(px(0.0))))
            .unwrap_or(0.0);
        let viewport_height = f32::from(viewport_size.height.max(px(1.0)));
        let viewport_width = f32::from(viewport_size.width.max(px(1.0)));
        let has_overflow = max_scroll_y > 0.5;

        let centered_width = editor_wysiwyg::render::layout::centered_column_width(viewport_width, &theme.dimensions);
        let current_scroll_y = self
            .pane_state_ref(pane_id)
            .map(|state| (-f32::from(state.scroll.handle.offset().y)).clamp(0.0, max_scroll_y))
            .unwrap_or(0.0);
        let scrollbar_geometry =
            Self::scrollbar_geometry(viewport_height, max_scroll_y, current_scroll_y);
        let track_height = scrollbar_geometry.track_height;
        let thumb_height = scrollbar_geometry.thumb_height;
        let thumb_top = scrollbar_geometry.thumb_top;

        let show_custom_scrollbar = has_overflow
            && self.pane_state_ref(pane_id).is_some_and(|state| {
                state.scroll.scrollbar_drag.is_some()
                    || state.scroll.scrollbar_hovered
                    || Instant::now() <= state.scroll.scrollbar_visible_until
            });

        let rows = plan_document_rows(blocks, d, cx);

        let overscroll_bottom = (viewport_height * 0.45).clamp(120.0, 400.0);
        let block_rows: Vec<AnyElement> = rows
            .iter()
            .map(|plan| {
                self.build_planned_row_element(
                    plan,
                    blocks,
                    centered_width,
                    &theme,
                    d,
                )
            })
            .collect();

        let scroll_handle = self
            .pane_state_ref(pane_id)
            .map(|state| state.scroll.handle.clone())
            .unwrap_or_default();
        let scroll_content = div()
            .id(ElementId::Name(
                format!("editor-scroll-inner-{panel_id}-{pane_id}").into(),
            ))
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .items_center()
            .bg(theme.colors.editor_background)
            .overflow_y_scroll()
            .scrollbar_width(px(0.0))
            .track_scroll(&scroll_handle)
            .can_drop(|dragged, _window, _cx| dragged.is::<ExternalPaths>())
            .on_drop::<ExternalPaths>(cx.listener(move |this, paths, window, cx| {
                this.defer_host_action(cx, move |host, cx| {
                    host.activate_panel(panel_id, cx)
                });
                this.on_external_paths_drop(paths, window, cx);
            }))
            .on_hover(cx.listener(move |this, hovered, window, cx| {
                this.on_editor_hover(pane_id, hovered, window, cx);
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event, window, cx| {
                    this.defer_host_action(cx, move |host, cx| {
                        host.activate_panel(panel_id, cx)
                    });
                    this.on_editor_mouse_down(event, window, cx);
                }),
            )
            .on_scroll_wheel(cx.listener(move |this, event, window, cx| {
                this.on_editor_scroll_wheel(pane_id, event, window, cx);
            }))
            .p(px(d.editor_padding))
            .pb(px(d.editor_padding + overscroll_bottom))
            .children(block_rows);

        let content_area = div()
            .id(ElementId::Name(
                format!("editor-scroll-{panel_id}-{pane_id}").into(),
            ))
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(0.0))
            .min_h(px(0.0))
            .bg(theme.colors.editor_background)
            .relative()
            .child(scroll_content);

        let content_area = if show_custom_scrollbar {
            let scrollbar_editor = editor.clone();
            let track_origin_y = f32::from(viewport_bounds.origin.y);
            content_area.child(
                div()
                    .id(ElementId::Name(
                        format!("editor-scrollbar-thumb-{panel_id}-{pane_id}").into(),
                    ))
                    .absolute()
                    .occlude()
                    .top(px(thumb_top))
                    .right(px(d.scrollbar_right))
                    .w(px(d.scrollbar_width))
                    .h(px(thumb_height))
                    .rounded(px(theme::dimensions::FULL_CORNER_RADIUS))
                    .bg(theme.colors.scrollbar_thumb)
                    .cursor_pointer()
                    .on_hover(cx.listener(move |this, hovered, window, cx| {
                        this.on_editor_hover(pane_id, hovered, window, cx);
                    }))
                    .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                        let pointer_offset_y =
                            f32::from(event.position.y) - track_origin_y - thumb_top;
                        let _ = scrollbar_editor.update(cx, |editor, cx| {
                            cx.stop_propagation();
                            {
                                editor.defer_host_action(cx, move |host, cx| {
                                    host.activate_panel(panel_id, cx);
                                });
                                editor.start_scrollbar_drag(
                                    pane_id,
                                    pointer_offset_y,
                                    track_height,
                                    thumb_height,
                                    max_scroll_y,
                                    window,
                                    cx,
                                );
                            }
                        });
                    })
                    .child(
                        canvas(
                            |_, _, _| (),
                            move |_thumb_bounds, _, window, _| {
                                window.on_mouse_event({
                                    let editor = editor.clone();
                                    move |_event: &MouseUpEvent, phase, window, cx| {
                                        if !phase.bubble() {
                                             return;
                                        }
                                        let _ = editor.update(cx, |editor, cx| {
                                            editor.end_scrollbar_drag(pane_id, window, cx);
                                        });
                                    }
                                });

                                window.on_mouse_event({
                                    let editor = editor.clone();
                                    move |event: &MouseMoveEvent, phase, window, cx| {
                                        if !phase.bubble() || !event.dragging() {
                                            return;
                                        }

                                        let pointer_y_in_track =
                                            f32::from(event.position.y) - track_origin_y;
                                        let _ = editor.update(cx, |editor, cx| {
                                            editor.update_scrollbar_drag(
                                                pane_id,
                                                pointer_y_in_track,
                                                window,
                                                cx,
                                            );
                                        });
                                    }
                                });
                            },
                        )
                        .size_full(),
                    ),
            )
        } else {
            content_area
        };

        let outline_hud = self.render_floating_outline_hud(pane_id, crate::engine::session::PaneKindId::WYSIWYG, &theme, cx);
        content_area.child(outline_hud).into_any_element()
    }

    /// Materializes one planned render row into its element tree.
    fn build_planned_row_element(
        &self,
        plan: &PlannedRow,
        blocks: &[BlockEntry],
        centered_width: f32,
        theme: &Theme,
        d: &ThemeDimensions,
    ) -> AnyElement {
        build_planned_row_element(
            plan,
            blocks,
            centered_width,
            theme,
            d,
            |row: Div, _entity_id: EntityId| -> Div { row },
        )
    }
}
