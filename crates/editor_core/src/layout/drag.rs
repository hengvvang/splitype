//! Inner editor pane dragging, corner drag gesture tracking, and drop target resolution.

use gpui::*;
use splitter::SplitAxis;
use theme::Theme;

use crate::editor::Editor;

impl Editor {
    pub fn update_inner_drag(&mut self, pos: Point<Pixels>, _window: &Window) -> bool {
        if self.session.root.active_splitter_drag.is_some() {
            let session = &mut self.session;
            let drag = session.root.active_splitter_drag.unwrap();
            let Some(outer_rect) = self.panel_rect else {
                return false;
            };
            let origin = outer_rect.origin;
            let rect_size = outer_rect.size;
            let current_pos = match drag.axis {
                SplitAxis::Horizontal => f32::from(pos.x) - f32::from(origin.x),
                SplitAxis::Vertical => f32::from(pos.y) - f32::from(origin.y),
            };
            let inner_size = size(rect_size.width, rect_size.height);
            let span = session
                .root
                .split_pixel_span(drag.split_id, inner_size)
                .unwrap_or_else(|| match drag.axis {
                    SplitAxis::Horizontal => f32::from(rect_size.width),
                    SplitAxis::Vertical => f32::from(rect_size.height),
                });
            if span > 1.0 {
                let mut refreshed = drag;
                refreshed.total_span = span;
                session.root.active_splitter_drag = Some(refreshed);
            }
            session.root.update_splitter_drag(current_pos);
            return true;
        }

        if self.session.root.corner_drag_panel().is_some() {
            let session = &mut self.session;
            let drag_panel = session.root.corner_drag_panel().unwrap();
            let drag = session
                .root
                .tree
                .find_leaf(drag_panel)
                .and_then(|p| p.active_corner_drag)
                .unwrap();
            let mut handled = false;
            if let Some(outer_rect) = self.panel_rect {
                let origin = outer_rect.origin;
                let rect_size = outer_rect.size;
                let mut updated = drag;
                let inner_pos = point(
                    px(f32::from(pos.x) - f32::from(origin.x)),
                    px(f32::from(pos.y) - f32::from(origin.y)),
                );
                let inner_size = size(rect_size.width, rect_size.height);
                let start_x = f32::from(updated.start_pos.x);
                let start_y = f32::from(updated.start_pos.y);
                if start_x > f32::from(rect_size.width) || start_y > f32::from(rect_size.height) {
                    updated.start_pos = point(
                        px(start_x - f32::from(origin.x)),
                        px(start_y - f32::from(origin.y)),
                    );
                }
                if let Some(panel) = session.root.tree.find_leaf_mut(drag_panel) {
                    panel.active_corner_drag = Some(updated);
                }
                session.root.update_corner_drag(inner_pos, inner_size);
                handled = true;
            }
            return handled;
        }
        false
    }

    pub fn finish_inner_drag(&mut self, window: &Window, cx: &mut Context<Self>) {
        if self.session.root.active_splitter_drag.is_some() {
            self.session.root.end_splitter_drag();
            cx.notify();
            return;
        }
        if self.session.root.corner_drag_panel().is_some() {
            let viewport = window.viewport_size();
            let inner_size = self.panel_rect.map(|rect| rect.size).unwrap_or(viewport);
            let _ = self.session.root.apply_corner_drag(inner_size);
            cx.notify();
        }
    }

    #[allow(dead_code)]
    pub(crate) fn on_pane_drag_move(
        &mut self,
        _drag: splitter::sessions::CornerDragSession,
        _event: &MouseMoveEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
    }

    #[allow(dead_code)]
    pub(crate) fn on_pane_drag_end(
        &mut self,
        _drag: splitter::sessions::CornerDragSession,
        _event: &MouseUpEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
    }

    pub(crate) fn render_editor_pane_corner_drag_preview(
        &self,
        theme: &Theme,
    ) -> Option<AnyElement> {
        let drag_panel = self.session.root.corner_drag_panel()?;
        let drag = self
            .session
            .root
            .tree
            .find_leaf(drag_panel)?
            .active_corner_drag?;
        if drag.modifier != splitter::sessions::CornerDragModifier::None
            && drag.modifier != splitter::sessions::CornerDragModifier::Ctrl
            && drag.modifier != splitter::sessions::CornerDragModifier::Shift
        {
            return None;
        }
        let overlay_style = splitter::interaction::OverlayStyle {
            accent: theme.colors.split_indicator,
            tile_radius: theme.dimensions.panel_tile_radius,
            border: theme.colors.dialog_border,
            selection: theme.colors.selection,
            active: theme.colors.focus_accent,
            surface: theme.colors.dialog_surface,
            text: theme.colors.dialog_title,
        };
        let inner_size = self
            .panel_rect
            .map(|r| r.size)
            .unwrap_or_else(|| size(px(800.0), px(600.0)));
        ui::render_corner_drag_preview(
            &self.session.root,
            &drag,
            inner_size,
            &overlay_style,
        )
    }

    pub(crate) fn render_editor_pane_splitter_drag_preview(
        &self,
        _theme: &Theme,
    ) -> Option<AnyElement> {
        None
    }
}
