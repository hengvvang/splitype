//! Editor canvas mouse handling — hover, wheel, and scrollbar drag.
//!
//! These are thin wrappers that toggle UI state (scrollbar visibility,
//! menu dismissal, table-axis preview clearing) and drive the scrollbar
//! drag session. Page-key and table-cell navigation lives in `navigation`.

use std::time::Duration;

use gpui::*;

use crate::editor::controller::*;

impl Editor {
    pub(crate) fn bump_scrollbar_visibility(&mut self, cx: &mut Context<Self>) {
        let duration = Duration::from_millis(900);
        self.tab_mut().scroll.scrollbar_visible_until = Instant::now() + duration;

        let weak_editor = cx.entity().downgrade();
        self.tab_mut().scroll.scrollbar_fade_task = Some(cx.spawn(
            async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
                cx.background_executor()
                    .timer(duration + Duration::from_millis(50))
                    .await;
                let _ = weak_editor.update(cx, |this, cx| {
                    this.tab_mut().scroll.scrollbar_fade_task = None;
                    cx.notify();
                });
            },
        ));

        cx.notify();
    }

    pub(crate) fn on_editor_hover(
        &mut self,
        hovered: &bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.tab_mut().scroll.scrollbar_hovered = *hovered;
        if *hovered {
            self.bump_scrollbar_visibility(cx);
        } else {
            cx.notify();
        }
    }

    pub(crate) fn on_editor_mouse_down(
        &mut self,
        _event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dismiss_menu_bar_from_body(cx);
        self.clear_table_axis_preview(cx);
        self.clear_table_axis_selection(cx);
    }

    pub(crate) fn on_editor_scroll_wheel(
        &mut self,
        _event: &ScrollWheelEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.bump_scrollbar_visibility(cx);
    }

    pub(crate) fn start_scrollbar_drag(
        &mut self,
        pointer_offset_y: f32,
        track_height: f32,
        thumb_height: f32,
        max_scroll_y: f32,
        cx: &mut Context<Self>,
    ) {
        self.tab_mut().scroll.scrollbar_drag = Some(crate::editor::controller::ScrollbarDragSession {
            pointer_offset_y: pointer_offset_y.clamp(0.0, thumb_height.max(0.0)),
            track_height,
            thumb_height,
            max_scroll_y,
        });
        self.tab_mut().focus.pending_scroll_active_block_into_view = false;
        self.tab_mut().focus.pending_scroll_recheck_after_layout = false;
        self.bump_scrollbar_visibility(cx);
        cx.notify();
    }

    pub(crate) fn update_scrollbar_drag(
        &mut self,
        pointer_y_in_track: f32,
        cx: &mut Context<Self>,
    ) {
        let Some(drag) = self.tab().scroll.scrollbar_drag else {
            return;
        };

        let travel = (drag.track_height - drag.thumb_height).max(0.0);
        let thumb_top = (pointer_y_in_track - drag.pointer_offset_y).clamp(0.0, travel);
        let scroll_y = Self::scroll_offset_for_thumb_top(
            thumb_top,
            drag.track_height,
            drag.thumb_height,
            drag.max_scroll_y,
        );

        let mut offset = self.tab().scroll.handle.offset();
        offset.y = -px(scroll_y);
        self.tab().scroll.handle.set_offset(offset);
        self.bump_scrollbar_visibility(cx);
        cx.notify();
    }

    pub(crate) fn end_scrollbar_drag(&mut self, cx: &mut Context<Self>) {
        if self.tab_mut().scroll.scrollbar_drag.take().is_some() {
            self.bump_scrollbar_visibility(cx);
            cx.notify();
        }
    }
}
