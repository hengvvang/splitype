//! WYSIWYG panel — the primary rendered editing view.
//!
//! WYSIWYG renders the document tree directly; the only panel-specific
//! behavior is re-normalizing quote/container structure after edits.

use gpui::*;

use crate::editor::controller::{Editor, EditorMode};
use crate::model::block::BlockData;

impl Editor {
    /// Re-parse and replace the rendered tree, preserving selection and
    /// scroll state. Used after edits that can leave quote/container
    /// structures malformed in the rendered view.
    pub(crate) fn normalize_rendered_quote_structure(&mut self, cx: &mut Context<Self>) {
        if self.mode != EditorMode::Wysiwyg {
            return;
        }

        let selection_snapshot = self.capture_source_selection_snapshot(cx);
        let source = self.document.to_markdown(cx);
        let mut roots = Self::parse_document(cx, &source);
        if roots.is_empty() {
            roots.push(Self::new_block(cx, BlockData::paragraph(String::new())));
        }
        self.document.replace_blocks(roots, cx);
        self.rebuild_table_runtimes(cx);
        self.rebuild_image_runtimes(cx);
        self.apply_selection_snapshot_in_current_mode(&selection_snapshot, cx);
        self.focus.pending_scroll_active_block_into_view = true;
        self.focus.pending_scroll_recheck_after_layout = true;
        self.scroll.last_viewport_size = None;
    }
}
