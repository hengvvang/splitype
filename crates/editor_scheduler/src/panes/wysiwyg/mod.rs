//! WYSIWYG panel glue — editor-side re-normalization of the rendered
//! tree. The mode's view state ([`WysiwygPaneState`]) lives in the
//! `editor_wysiwyg` crate.

use gpui::*;

use crate::engine::controller::Editor;

impl Editor {
    /// Re-parse and replace the rendered tree, preserving selection and
    /// scroll state. Used after edits that can leave quote/container
    /// structures malformed in the rendered view.
    pub(crate) fn normalize_rendered_quote_structure(&mut self, cx: &mut Context<Self>) {
        if !self.is_wysiwyg() {
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
            pane.scroll.pending_autoscroll = Some(crate::engine::controller::AutoscrollStrategy::Fit {
                margin: px(20.0),
            });
            pane.scroll.last_viewport_size = None;
        }
    }
}
