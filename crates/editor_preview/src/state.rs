//! PreviewPane state — read-only block tree, selection and sync markers.

use std::ops::Range;

use gpui::App;

use crate::node::PreviewBlock;
use crate::selection::{PreviewEndpoint, PreviewSelectionRange};
use editor::{outline_headings_from_markdown, EditorDocument, EditorPaneKind, OutlineNode, Pane};

/// Read-only block tree shown in the preview panel.
#[derive(Default)]
pub struct PreviewState {
    pub blocks: Vec<PreviewBlock>,
    pub selection: Option<PreviewSelectionRange>,
    pub drag_anchor: Option<PreviewEndpoint>,
    pub source_hash: u64,
    /// Document revision the preview tree was last synced at; `None` until
    /// the first build.
    pub synced_revision: Option<u64>,
}

impl Pane for PreviewState {
    fn kind(&self) -> EditorPaneKind {
        EditorPaneKind::Preview
    }

    fn document_source(&self, doc: &dyn EditorDocument, cx: &App) -> String {
        doc.serialize_markdown(cx)
    }

    fn set_search_matches(&mut self, _matches: &[(Range<usize>, bool)]) {
        // Preview is a read-only render; there is nothing to highlight.
    }

    fn outline_items(&self, doc: &dyn EditorDocument, cx: &App) -> Vec<OutlineNode> {
        outline_headings_from_markdown(&doc.serialize_markdown(cx))
    }
}
