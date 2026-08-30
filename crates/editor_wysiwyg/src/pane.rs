//! WysiwygPaneState — the WYSIWYG mode's view state (Pane implementation).

use std::ops::Range;

use gpui::App;

use crate::state::{FocusState, SelectionState};
use editor_model::{EditorDocument, EditorPaneKind, Pane};
use editor_outline::OutlineNode;

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
        doc.outline_headings(cx)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
