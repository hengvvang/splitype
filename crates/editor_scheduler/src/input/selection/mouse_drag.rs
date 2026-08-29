use gpui::*;

use crate::engine::controller::{
    CrossBlockDrag, CrossBlockSelection, CrossBlockSelectionEndpoint, Editor, PaneId,
};
use editor_wysiwyg::document::block::Block;

impl Editor {
    pub(crate) fn begin_cross_block_drag_at_point(
        &mut self,
        pane_id: PaneId,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let had_selection = self
            .pane_state_ref(pane_id)
            .and_then(|state| state.selection())
            .is_some_and(|selection| selection.has_cross_block());
        let changed_visuals = self.clear_cross_block_selection_visuals(cx);
        let changed = had_selection || changed_visuals;
        let drag = self
            .cross_block_endpoint_for_point(position, cx)
            .map(|anchor| CrossBlockDrag { anchor });
        if let Some(state) = self.pane_state_mut(pane_id) {
            if let Some(selection) = state.selection_mut() {
                selection.cross_block = None;
                selection.cross_block_drag = drag;
            }
        }
        if changed {
            cx.notify();
        }
    }

    pub(crate) fn on_editor_capture_mouse_down(
        &mut self,
        pane_id: PaneId,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.button != MouseButton::Left {
            cx.propagate();
            return;
        }

        if !self.is_wysiwyg() {
            cx.propagate();
            return;
        }

        if let Some(state) = self.pane_state_mut(pane_id) {
            if let Some(selection) = state.selection_mut() {
                selection.select_all_cycle = None;
            }
        }
        self.begin_cross_block_drag_at_point(pane_id, event.position, cx);
        cx.propagate();
    }

    pub(crate) fn on_editor_mouse_move(
        &mut self,
        pane_id: PaneId,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !event.dragging() {
            return;
        }
        let Some(drag) = self
            .pane_state_ref(pane_id)
            .and_then(|state| state.selection())
            .and_then(|selection| selection.cross_block_drag)
        else {
            return;
        };
        let Some(focus) = self.cross_block_endpoint_for_point(event.position, cx) else {
            return;
        };

        if self
            .pane_state_ref(pane_id)
            .and_then(|state| state.selection())
            .and_then(|selection| selection.cross_block)
            .is_none()
            && drag.anchor.entity_id == focus.entity_id
        {
            return;
        }

        let selection = CrossBlockSelection {
            anchor: drag.anchor,
            focus,
        };
        let is_empty = self.is_cross_block_selection_empty(selection);
        if let Some(state) = self.pane_state_mut(pane_id) {
            if let Some(sel) = state.selection_mut() {
                if is_empty {
                    sel.cross_block = None;
                } else {
                    sel.cross_block = Some(selection);
                }
            }
        }
        self.sync_cross_block_selection_visuals(cx);
        cx.notify();
    }

    pub(crate) fn on_editor_mouse_up(
        &mut self,
        pane_id: PaneId,
        _event: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(state) = self.pane_state_mut(pane_id) {
            if let Some(selection) = state.selection_mut() {
                selection.cross_block_drag = None;
            }
        }
        self.end_block_pointer_selection_sessions(cx);
    }

    pub(crate) fn cross_block_endpoint_for_point(
        &self,
        position: Point<Pixels>,
        cx: &App,
    ) -> Option<CrossBlockSelectionEndpoint> {
        let mut previous: Option<(Entity<Block>, Bounds<Pixels>)> = None;
        for entries in self.doc().blocks() {
            let entity = entries.entity.clone();
            let Some(bounds) = entity
                .read(cx)
                .last_paint()
                .map(|paint| paint.bounds)
            else {
                continue;
            };

            if position.y < bounds.top() {
                if let Some((previous, _)) = previous {
                    let offset = previous.read(cx).display_len();
                    return Some(CrossBlockSelectionEndpoint {
                        entity_id: previous.entity_id(),
                        offset,
                    });
                }
                return Some(CrossBlockSelectionEndpoint {
                    entity_id: entity.entity_id(),
                    offset: 0,
                });
            }

            if position.y <= bounds.bottom() {
                let offset = entity.read(cx).index_for_mouse_position(position);
                return Some(CrossBlockSelectionEndpoint {
                    entity_id: entity.entity_id(),
                    offset,
                });
            }

            previous = Some((entity, bounds));
        }

        previous.map(|(entity, _)| CrossBlockSelectionEndpoint {
            entity_id: entity.entity_id(),
            offset: entity.read(cx).display_len(),
        })
    }
}
