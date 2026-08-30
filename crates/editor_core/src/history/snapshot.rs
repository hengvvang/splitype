//! Selection snapshot capture and universal caret restoration.

use gpui::*;

use crate::engine::controller::{Editor, UndoSelectionSnapshot};

impl Editor {
    pub(crate) fn empty_selection_snapshot() -> UndoSelectionSnapshot {
        UndoSelectionSnapshot {
            range: 0..0,
            reversed: false,
            block_anchor: None,
        }
    }

    pub(crate) fn capture_source_selection_snapshot(&self, cx: &App) -> UndoSelectionSnapshot {
        self.capture_source_selection_snapshot_global(cx)
    }

    pub(crate) fn capture_source_selection_snapshot_global(
        &self,
        cx: &App,
    ) -> UndoSelectionSnapshot {
        let pane_id = self.active_pane_id();
        if let Some(source) = self.pane_state_ref(pane_id).and_then(|s| s.as_source_code()) {
            let range = source
                .selection
                .clone()
                .unwrap_or_else(|| source.cursor..source.cursor);
            return UndoSelectionSnapshot {
                range,
                reversed: false,
                block_anchor: None,
            };
        }
        if let Some(doc) = self.active_doc() {
            if let Some(first) = doc.first_root() {
                let b = first.read(cx);
                return UndoSelectionSnapshot {
                    range: b.selected_range.clone(),
                    reversed: b.selection_reversed,
                    block_anchor: None,
                };
            }
        }
        self.tab().undo.last_selection_snapshot.clone()
    }

    pub(crate) fn apply_selection_snapshot_in_current_mode(
        &mut self,
        snapshot: &UndoSelectionSnapshot,
        cx: &mut Context<Self>,
    ) {
        let pane_id = self.active_pane_id();
        self.sync_source_pane(pane_id, cx);
        if let Some(source) = self.pane_state_mut(pane_id).and_then(|p| p.as_source_code_mut()) {
            let len = source.text.len();
            let pos = snapshot.range.end.min(len);
            source.move_to(pos, false);
        }
    }
}
