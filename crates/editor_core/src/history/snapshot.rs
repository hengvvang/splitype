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
        _snapshot: &UndoSelectionSnapshot,
        _cx: &mut Context<Self>,
    ) {
    }
}
