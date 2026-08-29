//! Inner editor pane dragging, corner drag gesture tracking, and drop target resolution.

use gpui::*;

use crate::editor::engine::controller::*;
use splitter::SplitAxis;

impl Editor {
    pub(crate) fn update_inner_drag(&mut self, pos: Point<Pixels>, _window: &Window) -> bool {
        // Inner splitter drag: drive this editor's own session container
        // through the shared container API.
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

        // Inner corner drag: translate the pointer into the dragging
        // area's local space (fixing up the recorded start position),
        // refresh the facts, then apply the host's immediate shortcuts:
        // Ctrl past the threshold swaps the dragged pane with the
        // hovered one, Shift ends the gesture as a no-op. Plain drags
        // defer to the inner drag policy on mouse-up.
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
                // Write the corrected start pos back onto the panel's own
                // session, then let the root update the facts (hover,
                // direction, dock target).
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

    pub(crate) fn finish_inner_drag(&mut self, window: &Window, cx: &mut Context<Self>) {
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
}
