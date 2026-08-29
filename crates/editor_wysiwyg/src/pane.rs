//! WysiwygPaneState — the WYSIWYG mode's view state (Pane implementation).

use std::ops::Range;

use gpui::App;

use crate::state::{FocusState, SelectionState};
use editor::{outline_headings_from_markdown, EditorDocument, EditorPaneKind, OutlineNode, Pane};

/// View state specific to a WYSIWYG editor pane.
#[derive(Default)]
pub struct WysiwygPaneState {
    pub focus: FocusState,
    pub selection: SelectionState,
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

    fn outline_items(&self, doc: &dyn EditorDocument, cx: &App) -> Vec<OutlineNode> {
        let mut headings = doc.outline_headings(cx);
        if headings.is_empty() {
            headings = outline_headings_from_markdown(&doc.serialize_markdown(cx));
        }
        headings
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
