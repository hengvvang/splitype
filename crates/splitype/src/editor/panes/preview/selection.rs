use gpui::*;
use crate::editor::engine::controller::{Editor, PaneId};
use editor_preview::{PreviewEndpoint, PreviewSelectionRange};

impl Editor {
    pub(crate) fn on_preview_mouse_down(
        &mut self,
        pane_id: PaneId,
        block_index: usize,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let Some(pane) = self.pane_state_mut(pane_id) else {
            return;
        };
        let Some(preview) = pane.as_preview_mut() else {
            return;
        };
        let Some(block) = preview.blocks.get(block_index) else {
            return;
        };
        let offset = block.index_for_mouse_position(position);
        preview.drag_anchor = Some(PreviewEndpoint { block_index, offset });
        preview.selection = None;
        cx.notify();
    }

    pub(crate) fn on_preview_mouse_move(
        &mut self,
        pane_id: PaneId,
        block_index: usize,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let Some(pane) = self.pane_state_mut(pane_id) else {
            return;
        };
        let Some(preview) = pane.as_preview_mut() else {
            return;
        };
        let Some(anchor) = preview.drag_anchor else {
            return;
        };
        let Some(block) = preview.blocks.get(block_index) else {
            return;
        };
        let offset = block.index_for_mouse_position(position);
        let focus = PreviewEndpoint { block_index, offset };
        let selection = PreviewSelectionRange::new(anchor, focus);
        if selection.is_empty() {
            preview.selection = None;
        } else {
            preview.selection = Some(selection);
        }
        cx.notify();
    }

    pub(crate) fn on_preview_mouse_up(
        &mut self,
        pane_id: PaneId,
        cx: &mut Context<Self>,
    ) {
        if let Some(preview) = self.pane_state_mut(pane_id).and_then(|p| p.as_preview_mut()) {
            preview.drag_anchor = None;
            cx.notify();
        }
    }

    pub(crate) fn preview_selected_text(&self, _cx: &App) -> Option<String> {
        let pane_id = self.active_pane_id();
        let preview = self.pane_state_ref(pane_id)?.as_preview()?;
        let selection = preview.selection?;
        if selection.is_empty() {
            return None;
        }

        let mut lines = Vec::new();
        for (index, block) in preview.blocks.iter().enumerate() {
            let len = block.display_len();
            if let Some(range) = selection.range_for_block(index, len) {
                let text = block.display_text();
                let start = range.start.min(text.len());
                let end = range.end.min(text.len());
                if start < end {
                    lines.push(text[start..end].to_string());
                }
            }
        }

        if lines.is_empty() {
            None
        } else {
            Some(lines.join("\n\n"))
        }
    }
}


