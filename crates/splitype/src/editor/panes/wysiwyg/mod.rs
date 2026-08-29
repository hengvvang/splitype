//! WYSIWYG panel — the primary rendered editing view.
//!
//! WYSIWYG renders the document tree directly; the only panel-specific
//! behavior is re-normalizing quote/container structure after edits. The
//! row layout helpers live in `render`.


use std::ops::Range;

use gpui::*;

use crate::editor::engine::controller::{Editor, FocusState, SelectionState};
use crate::editor::engine::session::EditorPaneKind;
use editor_core::{outline_headings_from_markdown, EditorDocument, OutlineNode, Pane};

/// View state specific to a WYSIWYG editor pane.
#[derive(Default)]
pub(crate) struct WysiwygPaneState {
    pub(crate) focus: FocusState,
    pub(crate) selection: SelectionState,
}

impl Pane for WysiwygPaneState {
    fn kind(&self) -> EditorPaneKind {
        EditorPaneKind::Wysiwyg
    }

    fn document_source(&self, doc: &dyn EditorDocument, cx: &App) -> String {
        doc.serialize_markdown(cx)
    }

    fn set_search_matches(&mut self, _matches: &[(Range<usize>, bool)]) {
        // WYSIWYG highlights search matches at the block level: the editor
        // syncs `block.search_matches` on the block entities directly, so
        // the pane state carries nothing.
    }


    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn outline_items(&self, doc: &dyn EditorDocument, cx: &App) -> Vec<OutlineNode> {
        let mut headings = doc.outline_headings(cx);
        if headings.is_empty() {
            headings = outline_headings_from_markdown(&doc.serialize_markdown(cx));
        }
        headings
    }
}

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
            pane.scroll.pending_autoscroll = Some(crate::editor::engine::controller::AutoscrollStrategy::Fit {
                margin: px(20.0),
            });
            pane.scroll.last_viewport_size = None;
        }
    }
}
