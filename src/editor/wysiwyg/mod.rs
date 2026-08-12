//! WYSIWYG panel — the primary rendered editing view.
//!
//! WYSIWYG renders the document tree directly; the only panel-specific
//! behavior is re-normalizing quote/container structure after edits. The
//! row layout helpers live in `render`.

pub(crate) mod render;

use gpui::*;

use crate::editor::controller::{Editor, EditorMode};

impl Editor {
    /// Re-parse and replace the rendered tree, preserving selection and
    /// scroll state. Used after edits that can leave quote/container
    /// structures malformed in the rendered view.
    pub(crate) fn normalize_rendered_quote_structure(&mut self, cx: &mut Context<Self>) {
        if self.tab().mode != EditorMode::Wysiwyg {
            return;
        }

        // The tree is rebuilt from scratch below, so a structural anchor may
        // no longer fit; capture a global source range instead.
        let selection_snapshot = self.capture_source_selection_snapshot_global(cx);
        let source = self.doc().serialize_markdown(cx);
        self.rebuild_document_from_markdown(&source, cx);
        self.apply_selection_snapshot_in_current_mode(&selection_snapshot, cx);
        {
            let pane = self.active_pane_state();
            pane.focus.pending_scroll_active_block_into_view = true;
            pane.focus.pending_scroll_recheck_after_layout = true;
            pane.scroll.last_viewport_size = None;
        }
    }
}
