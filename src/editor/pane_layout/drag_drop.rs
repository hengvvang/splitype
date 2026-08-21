//! Inner editor pane dragging, corner drag gesture tracking, and drop target resolution.

use gpui::*;

use crate::editor::controller::*;
use crate::editor::session::EditorPaneKind;
use crate::splitter::{CornerDragModifier, SplitAxis};
use splitype_splitter::container::SplitterContainer;
use splitype_splitter::policy::DragPolicy;
use splitype_splitter::sessions::{id_at_point, past_shortcut_threshold};

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
        // Ctrl past the threshold swaps the dragged panel with the
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
            let mut pending_swap: Option<(usize, usize)> = None;
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
                // direction).
                if let Some(panel) = session.root.tree.find_leaf_mut(drag_panel) {
                    panel.active_corner_drag = Some(updated);
                }
                session.root.update_corner_drag(inner_pos, inner_size);
                let drag = session
                    .root
                    .tree
                    .find_leaf(drag_panel)
                    .and_then(|p| p.active_corner_drag);
                if let Some(drag) = drag {
                    if past_shortcut_threshold(&drag) {
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
                handled = true;
            }
            if let Some((from, to)) = pending_swap {
                self.swap_pane_kinds(from, to);
            }
            return handled;
        }
        false
    }

    /// End the inner-level drag gesture of the area currently dragging on
    /// mouse release: finish splitter-bar drags, and run the pane
    /// drag policy for corner drags.
    pub(crate) fn finish_inner_drag(&mut self, window: &Window, cx: &mut Context<Self>) {
        // Inner splitter bar drag end.
        if self.session.root.active_splitter_drag.is_some() {
            self.session.root.end_splitter_drag();
            cx.notify();
        }
        // Inner corner drag end: finish the gesture through the shared
        // container, then let the pane policy interpret the facts
        // (Shift is a no-op override).
        let facts = if self.session.root.corner_drag_panel().is_some() {
            self.session.root.finish_corner_drag()
        } else {
            None
        };
        if let Some(facts) = facts {
            let viewport = window.viewport_size();
            let inner_size = self.panel_rect.map(|rect| rect.size).unwrap_or(viewport);
            let session = &mut self.session;
            match facts.modifier {
                CornerDragModifier::None => {
                    let _ = <SplitterContainer<EditorPaneKind> as DragPolicy<
                            EditorPaneKind,
                        >>::on_plain_drag(
                            &mut session.root, &facts, inner_size
                        );
                }
                CornerDragModifier::Shift => {
                    let _ = <SplitterContainer<EditorPaneKind> as DragPolicy<
                            EditorPaneKind,
                        >>::on_shift_drag(
                            &mut session.root, &facts, inner_size
                        );
                }
                CornerDragModifier::Ctrl => <SplitterContainer<EditorPaneKind> as DragPolicy<
                    EditorPaneKind,
                >>::on_ctrl_drag(
                    &mut session.root, &facts, inner_size
                ),
                CornerDragModifier::Alt => <SplitterContainer<EditorPaneKind> as DragPolicy<
                    EditorPaneKind,
                >>::on_alt_drag(
                    &mut session.root, &facts, inner_size
                ),
            }
            cx.notify();
        }
    }
}
