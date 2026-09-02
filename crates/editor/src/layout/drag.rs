//! Inner editor pane dragging: coordinate transforms into the pane layout's
//! local space, drag finishing and pane-state reconciliation.

use gpui::*;
use splitter::policy::CornerDragResult;
use theme::Theme;

use crate::editor::Editor;

impl Editor {
    /// Routes window-level pointer moves into the pane layout's local
    /// coordinate space and advances the active drag gesture.
    pub fn update_inner_drag(&mut self, pos: Point<Pixels>, _window: &Window) -> bool {
        let Some(outer_rect) = self.panel_rect else {
            return false;
        };
        let inner_pos = point(
            px(f32::from(pos.x) - f32::from(outer_rect.origin.x)),
            px(f32::from(pos.y) - f32::from(outer_rect.origin.y)),
        );
        self.session
            .root
            .update_drag_gesture(inner_pos, outer_rect.size)
    }

    /// Finishes the active drag gesture and applies its result to the pane
    /// states — the pane-level counterpart of the window shell's lifecycle
    /// handlers.
    pub fn finish_inner_drag(&mut self, window: &Window, cx: &mut Context<Self>) {
        let inner_size = self
            .panel_rect
            .map(|rect| rect.size)
            .unwrap_or_else(|| window.viewport_size());
        let Some(result) = self.session.root.finish_drag_gesture(inner_size) else {
            return;
        };
        match result {
            // Pane states are created lazily on the next render with the
            // new leaf's kind; nothing to move.
            CornerDragResult::Split { .. } => {}
            CornerDragResult::Join { removed_id, .. } => {
                self.forget_pane_state(removed_id);
            }
            CornerDragResult::Swap { a, b } => {
                self.swap_pane_states(a, b);
            }
            CornerDragResult::MoveAndDock {
                source_id,
                target_id,
                new_leaf_id,
                dock_target,
                ..
            } => {
                self.move_and_dock_pane_states(source_id, target_id, new_leaf_id, dock_target);
            }
            // Inner panes share one document per tab; window cloning is a
            // shell-level concept and is not offered at the pane level
            // (the Shift-drag preview is suppressed there).
            CornerDragResult::CloneWindow { .. } | CornerDragResult::None => {}
        }
        cx.notify();
    }

    /// Render the corner-drag indicator for the inner pane layout.
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
        let overlay_style = ui::split::chrome::OverlayStyle::from_theme(theme);
        let inner_size = self
            .panel_rect
            .map(|r| r.size)
            .unwrap_or_else(|| size(px(800.0), px(600.0)));
        ui::split::drag_preview::render_corner_drag_preview(
            &self.session.root,
            &drag,
            inner_size,
            &overlay_style,
        )
    }
}
